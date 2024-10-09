// commands.rs

// Copyright (C) 2018-2024 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::borrow;
use std::convert::TryFrom as _;
use std::env;
use std::ffi;
use std::fmt;
use std::fs;
use std::io;
use std::ops;
use std::ops::Deref as _;
use std::path;
use std::process;
use std::thread;
use std::time;

use anyhow::Context as _;

use libc::sync;

use nitrokey::ConfigureOtp;
use nitrokey::Device;
use nitrokey::GenerateOtp;
use nitrokey::GetPasswordSafe;

use crate::args;
use crate::config;
use crate::output;
use crate::pinentry;
use crate::Context;

const NITROCLI_EXT_PREFIX: &str = "nitrocli-";

const OTP_NAME_LENGTH: usize = 15;

const PWS_NAME_LENGTH: usize = 11;
const PWS_LOGIN_LENGTH: usize = 32;
const PWS_PASSWORD_LENGTH: usize = 20;

/// Set `libnitrokey`'s log level based on the execution context's verbosity.
fn set_log_level(ctx: &mut Context<'_>) {
  let log_lvl = match ctx.config.verbosity {
    // The error log level is what libnitrokey uses by default. As such,
    // there is no harm in us setting that as well when the user did not
    // ask for higher verbosity.
    0 => nitrokey::LogLevel::Error,
    1 => nitrokey::LogLevel::Warning,
    2 => nitrokey::LogLevel::Info,
    3 => nitrokey::LogLevel::DebugL1,
    4 => nitrokey::LogLevel::Debug,
    _ => nitrokey::LogLevel::DebugL2,
  };
  nitrokey::set_log_level(log_lvl);
}

/// Create a filter string from the program configuration.
fn format_filter(config: &config::Config) -> String {
  let mut filters = Vec::new();
  if let Some(model) = config.model {
    filters.push(format!("model={}", model.as_ref()));
  }
  if !config.serial_numbers.is_empty() {
    let serial_numbers = config
      .serial_numbers
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>();
    filters.push(format!("serial number in [{}]", serial_numbers.join(", ")));
  }
  if let Some(path) = &config.usb_path {
    filters.push(format!("usb path={}", path));
  }
  if filters.is_empty() {
    String::new()
  } else {
    format!(" (filter: {})", filters.join(", "))
  }
}

/// Find a Nitrokey device that matches the given requirements
fn find_device(config: &config::Config) -> anyhow::Result<nitrokey::DeviceInfo> {
  let devices = nitrokey::list_devices().context("Failed to enumerate Nitrokey devices")?;
  let nkmodel = config.model.map(nitrokey::Model::from);
  let mut iter = devices
    .into_iter()
    .filter(|device| nkmodel.is_none() || device.model == nkmodel)
    .filter(|device| {
      config.serial_numbers.is_empty()
        || device
          .serial_number
          .map(|sn| config.serial_numbers.contains(&sn))
          .unwrap_or_default()
    })
    .filter(|device| config.usb_path.is_none() || config.usb_path.as_ref() == Some(&device.path));

  let device = iter
    .next()
    .with_context(|| format!("Nitrokey device not found{}", format_filter(config)))?;

  anyhow::ensure!(
    iter.next().is_none(),
    "Multiple Nitrokey devices found{}.  Use the --model, --serial-number, and --usb-path options \
    to select one",
    format_filter(config)
  );
  Ok(device)
}

/// Connect to a Nitrokey device that matches the given requirements
fn connect<'mgr>(
  manager: &'mgr mut nitrokey::Manager,
  config: &config::Config,
) -> anyhow::Result<nitrokey::DeviceWrapper<'mgr>> {
  let device_info = find_device(config)?;
  manager
    .connect_path(device_info.path.deref())
    .with_context(|| {
      format!(
        "Failed to connect to Nitrokey device at path {}",
        device_info.path
      )
    })
}

/// Connect to any Nitrokey device and do something with it.
fn with_device<F>(ctx: &mut Context<'_>, op: F) -> anyhow::Result<()>
where
  F: FnOnce(&mut Context<'_>, nitrokey::DeviceWrapper<'_>) -> anyhow::Result<()>,
{
  let mut manager =
    nitrokey::take().context("Failed to acquire access to Nitrokey device manager")?;

  set_log_level(ctx);

  let device = connect(&mut manager, &ctx.config)?;
  op(ctx, device)
}

/// Connect to a Nitrokey Storage device and do something with it.
fn with_storage_device<F>(ctx: &mut Context<'_>, op: F) -> anyhow::Result<()>
where
  F: FnOnce(&mut Context<'_>, nitrokey::Storage<'_>) -> anyhow::Result<()>,
{
  let mut manager =
    nitrokey::take().context("Failed to acquire access to Nitrokey device manager")?;

  set_log_level(ctx);

  if let Some(model) = ctx.config.model {
    if model != args::DeviceModel::Storage {
      anyhow::bail!("This command is only available on the Nitrokey Storage");
    }
  } else {
    ctx.config.model = Some(args::DeviceModel::Storage);
  }

  let device = connect(&mut manager, &ctx.config)?;
  if let nitrokey::DeviceWrapper::Storage(storage) = device {
    op(ctx, storage)
  } else {
    panic!("connect returned a wrong model: {}", device.get_model())
  }
}

/// Connect to any Nitrokey device, retrieve a password safe handle, and
/// do something with it.
fn with_password_safe<F>(ctx: &mut Context<'_>, mut op: F) -> anyhow::Result<()>
where
  F: FnMut(&mut Context<'_>, nitrokey::PasswordSafe<'_, '_>) -> anyhow::Result<()>,
{
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::User, &device)?;
    try_with_pin_and_data(ctx, &pin_entry, (), move |ctx, _, pin| {
      let pws = device.get_password_safe(pin).or_else(|err| {
        Err(err)
          .context("Could not access the password safe")
          .map_err(|err| ((), err))
      })?;

      op(ctx, pws).map_err(|err| ((), err))
    })
  })?;
  Ok(())
}

/// Authenticate the given device using the given PIN type and operation.
fn authenticate<'mgr, D, A, F>(
  ctx: &mut Context<'_>,
  device: D,
  pin_type: args::PinType,
  op: F,
) -> anyhow::Result<A>
where
  D: Device<'mgr>,
  F: FnMut(&mut Context<'_>, D, &str) -> Result<A, (D, anyhow::Error)>,
{
  let pin_entry = pinentry::PinEntry::from(pin_type, &device)?;

  try_with_pin_and_data(ctx, &pin_entry, device, op)
}

/// Authenticate the given device with the user PIN.
fn authenticate_user<'mgr, T>(
  ctx: &mut Context<'_>,
  device: T,
) -> anyhow::Result<nitrokey::User<'mgr, T>>
where
  T: Device<'mgr>,
{
  authenticate(ctx, device, args::PinType::User, |_ctx, device, pin| {
    device.authenticate_user(pin).or_else(|(x, err)| {
      Err(err)
        .context("Failed to authenticate as user")
        .map_err(|err| (x, err))
    })
  })
}

/// Authenticate the given device with the admin PIN.
fn authenticate_admin<'mgr, T>(
  ctx: &mut Context<'_>,
  device: T,
) -> anyhow::Result<nitrokey::Admin<'mgr, T>>
where
  T: Device<'mgr>,
{
  authenticate(ctx, device, args::PinType::Admin, |_ctx, device, pin| {
    device.authenticate_admin(pin).or_else(|(x, err)| {
      Err(err)
        .context("Failed to authenticate as admin")
        .map_err(|err| (x, err))
    })
  })
}

/// Return a string representation of the given volume status.
fn get_volume_status(status: &nitrokey::VolumeStatus) -> &'static str {
  if status.active {
    if status.read_only {
      "read-only"
    } else {
      "active"
    }
  } else {
    "inactive"
  }
}

/// Try to execute the given function with a pin queried using pinentry.
///
/// This function will query the pin of the given type from the user
/// using pinentry.  It will then execute the given function.  If this
/// function returns a result, the result will be passed on.  If it
/// returns a `CommandError::WrongPassword`, the user will be asked
/// again to enter the pin.  Otherwise, this function returns an error
/// containing the given error message.  The user will have at most
/// three tries to get the pin right.
///
/// The data argument can be used to pass on data between the tries.  At
/// the first try, this function will call `op` with `data`.  At the
/// second or third try, it will call `op` with the data returned by the
/// previous call to `op`.
fn try_with_pin_and_data_with_pinentry<D, F, R>(
  ctx: &mut Context<'_>,
  pin_entry: &pinentry::PinEntry,
  data: D,
  mut op: F,
) -> anyhow::Result<R>
where
  F: FnMut(&mut Context<'_>, D, &str) -> Result<R, (D, anyhow::Error)>,
{
  let mut data = data;
  let mut retry = 3;
  let mut error_msg = None;
  loop {
    let pin = pinentry::inquire(ctx, pin_entry, pinentry::Mode::Query, error_msg)?;
    match op(ctx, data, &pin) {
      Ok(result) => return Ok(result),
      Err((new_data, err)) => match err.downcast::<nitrokey::Error>() {
        Ok(err) => match err {
          nitrokey::Error::CommandError(nitrokey::CommandError::WrongPassword) => {
            pinentry::clear(pin_entry).context("Failed to clear cached secret")?;
            retry -= 1;

            if retry > 0 {
              error_msg = Some("Wrong password, please reenter");
              data = new_data;
              continue;
            }
            anyhow::bail!(err);
          }
          err => anyhow::bail!(err),
        },
        Err(err) => anyhow::bail!(err),
      },
    };
  }
}

/// Try to execute the given function with a PIN.
fn try_with_pin_and_data<D, F, R>(
  ctx: &mut Context<'_>,
  pin_entry: &pinentry::PinEntry,
  data: D,
  mut op: F,
) -> anyhow::Result<R>
where
  F: FnMut(&mut Context<'_>, D, &str) -> Result<R, (D, anyhow::Error)>,
{
  let pin = match pin_entry.pin_type() {
    // Ideally we would not clone here, but that would require us to
    // restrict op to work with an immutable Context, which is not
    // possible given that some clients print data.
    args::PinType::Admin => ctx.admin_pin.clone(),
    args::PinType::User => ctx.user_pin.clone(),
  };

  if let Some(pin) = pin {
    let pin = pin
      .to_str()
      .context("Failed to read PIN: Invalid Unicode data found")?;
    op(ctx, data, pin).map_err(|(_, err)| err)
  } else {
    try_with_pin_and_data_with_pinentry(ctx, pin_entry, data, op)
  }
}

/// Try to execute the given function with a pin queried using pinentry.
///
/// This function behaves exactly as `try_with_pin_and_data`, but
/// it refrains from passing any data to it.
fn try_with_pin<F>(
  ctx: &mut Context<'_>,
  pin_entry: &pinentry::PinEntry,
  mut op: F,
) -> anyhow::Result<()>
where
  F: FnMut(&str) -> anyhow::Result<()>,
{
  try_with_pin_and_data(ctx, pin_entry, (), |_ctx, data, pin| {
    op(pin).map_err(|err| (data, err))
  })
}

/// Pretty print the status of a Nitrokey Storage.
fn print_storage_status(
  ctx: &mut Context<'_>,
  status: &nitrokey::StorageStatus,
  sd_card_usage: &ops::Range<u8>,
) -> anyhow::Result<()> {
  println!(
    ctx,
    r#"  Storage:
    SD card ID:        {id:#x}
    SD card usage:     {usagestart}% .. {usageend}% not written
    firmware:          {fw}
    storage keys:      {sk}
    volumes:
      unencrypted:     {vu}
      encrypted:       {ve}
      hidden:          {vh}"#,
    id = status.serial_number_sd_card,
    usagestart = sd_card_usage.start,
    usageend = sd_card_usage.end,
    fw = if status.firmware_locked {
      "locked"
    } else {
      "unlocked"
    },
    sk = if status.stick_initialized {
      "created"
    } else {
      "not created"
    },
    vu = get_volume_status(&status.unencrypted_volume),
    ve = get_volume_status(&status.encrypted_volume),
    vh = get_volume_status(&status.hidden_volume),
  )?;
  Ok(())
}

/// Read a value from stdin if the given string is set to "-".
fn value_or_stdin<'s>(ctx: &mut Context<'_>, s: &'s str) -> anyhow::Result<borrow::Cow<'s, str>> {
  if s == "-" {
    let mut s = String::new();
    let _ = ctx
      .stdin
      .read_to_string(&mut s)
      .context("Failed to read from stdin")?;
    Ok(borrow::Cow::from(s))
  } else {
    Ok(borrow::Cow::from(s))
  }
}

/// Validate the length of strings provided by the user.
///
/// The input must be a slice of tuples of the name of the string, the string itself and the
/// maximum length.
fn ensure_string_lengths(data: &[(&str, &str, usize)]) -> anyhow::Result<()> {
  let mut invalid_strings = Vec::new();
  for (label, value, max_length) in data {
    let length = value.as_bytes().len();
    if length > *max_length {
      invalid_strings.push((label, length, max_length));
    }
  }
  match invalid_strings.len() {
    0 => Ok(()),
    1 => {
      let (label, length, max_length) = invalid_strings[0];
      Err(anyhow::anyhow!(
        "The provided {} is too long (actual length: {} bytes, maximum length: {} bytes)",
        label,
        length,
        max_length
      ))
    }
    _ => {
      let mut msg = String::from("Multiple provided strings are too long:");
      for (label, length, max_length) in invalid_strings {
        msg.push_str(&format!(
          "\n  {} (actual length: {} bytes, maximum length: {} bytes)",
          label, length, max_length
        ));
      }
      Err(anyhow::anyhow!(msg))
    }
  }
}

/// Pretty print the status that is common to all Nitrokey devices.
fn print_status(
  ctx: &mut Context<'_>,
  model: nitrokey::Model,
  serial_number: nitrokey::SerialNumber,
  firmware_version: nitrokey::FirmwareVersion,
  user_retry_count: u8,
  admin_retry_count: u8,
) -> anyhow::Result<()> {
  println!(
    ctx,
    r#"Status:
  model:             {model}
  serial number:     {id}
  firmware version:  {fwv}
  user retry count:  {urc}
  admin retry count: {arc}"#,
    model = model,
    id = serial_number,
    fwv = firmware_version,
    urc = user_retry_count,
    arc = admin_retry_count,
  )?;

  Ok(())
}

/// Inquire the status of the nitrokey.
pub fn status(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |ctx, device| {
    if let nitrokey::DeviceWrapper::Storage(device) = device {
      // TODO: Extract serial number from storage status, see
      //       https://todo.sr.ht/~ireas/nitrokey-rs/1
      let serial_number = device
        .get_serial_number()
        .context("Failed to retrieve serial number")?;
      let status = device
        .get_storage_status()
        .context("Failed to retrieve storage status")?;

      print_status(
        ctx,
        device.get_model(),
        serial_number,
        status.firmware_version,
        status.user_retry_count,
        status.admin_retry_count,
      )?;

      let sd_card_usage = device
        .get_sd_card_usage()
        .context("Failed to retrieve SD card usage")?;
      print_storage_status(ctx, &status, &sd_card_usage)
    } else {
      let status = device
        .get_status()
        .context("Could not query the device status")?;
      let user_retry_count = device
        .get_user_retry_count()
        .context("Failed to retrieve user retry count")?;
      let admin_retry_count = device
        .get_admin_retry_count()
        .context("Failed to retrieve admin retry count")?;
      print_status(
        ctx,
        device.get_model(),
        status.serial_number,
        status.firmware_version,
        user_retry_count,
        admin_retry_count,
      )
    }
  })
}

/// List the attached Nitrokey devices.
pub fn list(ctx: &mut Context<'_>, no_connect: bool) -> anyhow::Result<()> {
  set_log_level(ctx);

  let device_infos =
    nitrokey::list_devices().context("Failed to list connected Nitrokey devices")?;
  if device_infos.is_empty() {
    println!(ctx, "No Nitrokey device connected")?;
  } else {
    println!(ctx, "USB path\tmodel\tserial number")?;
    let mut manager =
      nitrokey::take().context("Failed to acquire access to Nitrokey device manager")?;

    for device_info in device_infos {
      let model = device_info
        .model
        .map(|m| m.to_string())
        .unwrap_or_else(|| "unknown".into());
      let serial_number = match device_info.serial_number {
        Some(serial_number) => serial_number.to_string(),
        None => {
          // Storage devices do not have the serial number present in
          // the device information. We have to connect to them to
          // retrieve the information.
          if no_connect {
            "N/A".to_string()
          } else {
            let device = manager
              .connect_path(device_info.path.clone())
              .context("Failed to connect to Nitrokey")?;
            device
              .get_serial_number()
              .context("Failed to retrieve device serial number")?
              .to_string()
          }
        }
      };

      println!(ctx, "{}\t{}\t{}", device_info.path, model, serial_number)?;
    }
  }

  Ok(())
}

/// Fill the SD card with random data
pub fn fill(ctx: &mut Context<'_>, attach: bool) -> anyhow::Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let mut initial_progress = 0;
    if attach {
      let status = device
        .get_operation_status()
        .context("Failed to query operation status")?;
      match status {
        nitrokey::OperationStatus::Ongoing(progress) => initial_progress = progress,
        nitrokey::OperationStatus::Idle => anyhow::bail!("No fill operation in progress"),
      }
    } else {
      let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;

      // Similar to reset, we want the user to re-enter the admin PIN
      // even if is cached to avoid accidental data loss.
      pinentry::clear(&pin_entry).context("Failed to clear cached secret")?;

      try_with_pin(ctx, &pin_entry, |pin| {
        device.fill_sd_card(pin).context("Failed to fill SD card")
      })?;
    }

    let mut progress_bar = output::ProgressBar::new(initial_progress);
    progress_bar.draw(ctx)?;

    while !progress_bar.is_finished() {
      thread::sleep(time::Duration::from_secs(1));

      let status = device
        .get_operation_status()
        .context("Failed to query operation status")?;
      match status {
        nitrokey::OperationStatus::Ongoing(progress) => progress_bar.update(progress)?,
        nitrokey::OperationStatus::Idle => progress_bar.finish(),
      };
      progress_bar.draw(ctx)?;
    }

    Ok(())
  })
}

/// Perform a factory reset.
pub fn reset(ctx: &mut Context<'_>, only_aes_key: bool) -> anyhow::Result<()> {
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;

    // To force the user to enter the admin PIN before performing a
    // factory reset, we clear the pinentry cache for the admin PIN.
    pinentry::clear(&pin_entry).context("Failed to clear cached secret")?;

    try_with_pin(ctx, &pin_entry, |pin| {
      if only_aes_key {
        // Similar to the else arm, we have to execute this command to avoid WrongPassword errors
        let _ = device.get_user_retry_count();
        device
          .build_aes_key(pin)
          .context("Failed to rebuild AES key")
      } else {
        device
          .factory_reset(pin)
          .context("Failed to reset to factory settings")?;
        // Work around for a timing issue between factory_reset and
        // build_aes_key, see
        // https://github.com/Nitrokey/nitrokey-storage-firmware/issues/80
        thread::sleep(time::Duration::from_secs(3));
        // Another work around for spurious WrongPassword returns of
        // build_aes_key after a factory reset on Pro devices.
        // https://github.com/Nitrokey/nitrokey-pro-firmware/issues/57
        let _ = device.get_user_retry_count();
        device
          .build_aes_key(nitrokey::DEFAULT_ADMIN_PIN)
          .context("Failed to rebuild AES key")
      }
    })
  })
}

/// Change the configuration of the unencrypted volume.
pub fn unencrypted_set(
  ctx: &mut Context<'_>,
  mode: args::UnencryptedVolumeMode,
) -> anyhow::Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;
    let mode = match mode {
      args::UnencryptedVolumeMode::ReadWrite => nitrokey::VolumeMode::ReadWrite,
      args::UnencryptedVolumeMode::ReadOnly => nitrokey::VolumeMode::ReadOnly,
    };

    // The unencrypted volume may reconnect, so be sure to flush caches to
    // disk.
    unsafe { sync() };

    try_with_pin(ctx, &pin_entry, |pin| {
      device
        .set_unencrypted_volume_mode(pin, mode)
        .context("Failed to change unencrypted volume mode")
    })
  })
}

/// Open the encrypted volume on the Nitrokey.
pub fn encrypted_open(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::User, &device)?;

    // We may forcefully close a hidden volume, if active, so be sure to
    // flush caches to disk.
    unsafe { sync() };

    try_with_pin(ctx, &pin_entry, |pin| {
      device
        .enable_encrypted_volume(pin)
        .context("Failed to open encrypted volume")
    })
  })
}

/// Close the previously opened encrypted volume.
pub fn encrypted_close(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_storage_device(ctx, |_ctx, mut device| {
    // Flush all filesystem caches to disk. We are mostly interested in
    // making sure that the encrypted volume on the Nitrokey we are
    // about to close is not closed while not all data was written to
    // it.
    unsafe { sync() };

    device
      .disable_encrypted_volume()
      .context("Failed to close encrypted volume")
  })
}

/// Create a hidden volume.
pub fn hidden_create(ctx: &mut Context<'_>, slot: u8, start: u8, end: u8) -> anyhow::Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pwd_entry = pinentry::PwdEntry::from(&device)?;
    let pwd = if let Some(pwd) = &ctx.password {
      pwd
        .to_str()
        .context("Failed to read password: Invalid Unicode data found")
        .map(ToOwned::to_owned)
    } else {
      pinentry::choose(ctx, &pwd_entry).context("Failed to select new PIN")
    }?;

    device
      .create_hidden_volume(slot, start, end, &pwd)
      .context("Failed to create hidden volume")
  })
}

/// Open a hidden volume.
pub fn hidden_open(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pwd_entry = pinentry::PwdEntry::from(&device)?;
    let pwd = if let Some(pwd) = &ctx.password {
      pwd
        .to_str()
        .context("Failed to read password: Invalid Unicode data found")
        .map(ToOwned::to_owned)
    } else {
      pinentry::inquire(ctx, &pwd_entry, pinentry::Mode::Query, None)
        .context("Failed to inquire PIN")
    }?;

    // We may forcefully close an encrypted volume, if active, so be sure
    // to flush caches to disk.
    unsafe { sync() };

    device
      .enable_hidden_volume(&pwd)
      .context("Failed to open hidden volume")
  })
}

/// Close a previously opened hidden volume.
pub fn hidden_close(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_storage_device(ctx, |_ctx, mut device| {
    unsafe { sync() };

    device
      .disable_hidden_volume()
      .context("Failed to close hidden volume")
  })
}

/// Return a String representation of the given Option.
fn format_option<T: fmt::Display>(option: Option<T>) -> String {
  match option {
    Some(value) => format!("{}", value),
    None => "not set".to_string(),
  }
}

/// Read the Nitrokey configuration.
pub fn config_get(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |ctx, device| {
    let config = device.get_config().context("Failed to get configuration")?;
    println!(
      ctx,
      r#"Config:
  num lock binding:         {nl}
  caps lock binding:        {cl}
  scroll lock binding:      {sl}
  require user PIN for OTP: {otp}"#,
      nl = format_option(config.num_lock),
      cl = format_option(config.caps_lock),
      sl = format_option(config.scroll_lock),
      otp = config.user_password,
    )?;
    Ok(())
  })
}

/// Write the Nitrokey configuration.
pub fn config_set(ctx: &mut Context<'_>, args: args::ConfigSetArgs) -> anyhow::Result<()> {
  let num_lock = args::ConfigOption::try_from(args.no_num_lock, args.num_lock, "numlock")
    .context("Failed to apply num lock configuration")?;
  let caps_lock = args::ConfigOption::try_from(args.no_caps_lock, args.caps_lock, "capslock")
    .context("Failed to apply caps lock configuration")?;
  let scroll_lock =
    args::ConfigOption::try_from(args.no_scroll_lock, args.scroll_lock, "scrollock")
      .context("Failed to apply scroll lock configuration")?;
  let otp_pin = if args.otp_pin {
    Some(true)
  } else if args.no_otp_pin {
    Some(false)
  } else {
    None
  };

  with_device(ctx, |ctx, device| {
    let mut device = authenticate_admin(ctx, device)?;
    let config = device
      .get_config()
      .context("Failed to get current configuration")?;
    let config = nitrokey::Config {
      num_lock: num_lock.or(config.num_lock),
      caps_lock: caps_lock.or(config.caps_lock),
      scroll_lock: scroll_lock.or(config.scroll_lock),
      user_password: otp_pin.unwrap_or(config.user_password),
    };
    device
      .write_config(config)
      .context("Failed to set new configuration")
  })
}

/// Lock the Nitrokey device.
pub fn lock(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |_ctx, mut device| {
    device.lock().context("Failed to lock the device")
  })
}

fn get_otp<T>(slot: u8, algorithm: args::OtpAlgorithm, device: &mut T) -> anyhow::Result<String>
where
  T: GenerateOtp,
{
  match algorithm {
    args::OtpAlgorithm::Hotp => device.get_hotp_code(slot),
    args::OtpAlgorithm::Totp => device.get_totp_code(slot),
  }
  .context("Failed to generate OTP")
}

fn get_unix_timestamp() -> anyhow::Result<u64> {
  time::SystemTime::now()
    .duration_since(time::UNIX_EPOCH)
    .context("Current system time is before the Unix epoch")
    .map(|duration| duration.as_secs())
}

/// Generate a one-time password on the Nitrokey device.
pub fn otp_get(
  ctx: &mut Context<'_>,
  slot: u8,
  algorithm: args::OtpAlgorithm,
  time: Option<u64>,
) -> anyhow::Result<()> {
  with_device(ctx, |ctx, mut device| {
    if algorithm == args::OtpAlgorithm::Totp {
      device
        .set_time(
          match time {
            Some(time) => time,
            None => get_unix_timestamp().context("Failed to retrieve current time")?,
          },
          true,
        )
        .context("Failed to set new time")?;
    }
    let config = device
      .get_config()
      .context("Failed to get get current device configuration")?;
    let otp = if config.user_password {
      let mut user = authenticate_user(ctx, device)?;
      get_otp(slot, algorithm, &mut user)
    } else {
      get_otp(slot, algorithm, &mut device)
    }?;
    println!(ctx, "{}", otp)?;
    Ok(())
  })
}

/// Format a byte vector as a hex string.
fn format_bytes(bytes: &[u8]) -> String {
  bytes
    .iter()
    .map(|c| format!("{:02x}", c))
    .collect::<Vec<_>>()
    .join("")
}

/// Prepare an ASCII secret string for libnitrokey.
///
/// libnitrokey expects secrets as hexadecimal strings.  This function transforms an ASCII string
/// into a hexadecimal string or returns an error if the given string contains non-ASCII
/// characters.
fn prepare_ascii_secret(secret: &str) -> anyhow::Result<String> {
  if secret.is_ascii() {
    Ok(format_bytes(secret.as_bytes()))
  } else {
    anyhow::bail!("The given secret is not an ASCII string as expected")
  }
}

/// Prepare a base32 secret string for libnitrokey.
fn prepare_base32_secret(secret: &str) -> anyhow::Result<String> {
  // Some sites display the base32 secret in groups separated by spaces, we want to ignore them.
  let mut secret = secret.replace(' ', "");
  let () = secret.make_ascii_lowercase();

  base32::decode(base32::Alphabet::Rfc4648Lower { padding: false }, &secret)
    .map(|vec| format_bytes(&vec))
    .context("Failed to parse base32 secret")
}

/// Prepare a secret string in the given format for libnitrokey.
fn prepare_secret(
  secret: borrow::Cow<'_, str>,
  format: args::OtpSecretFormat,
) -> anyhow::Result<String> {
  match format {
    args::OtpSecretFormat::Ascii => prepare_ascii_secret(&secret),
    args::OtpSecretFormat::Base32 => prepare_base32_secret(&secret),
    args::OtpSecretFormat::Hex => {
      // We need to ensure to provide a string with an even number of
      // characters in it, just because that's what libnitrokey
      // expects. So prepend a '0' if that is not the case.
      let mut secret = secret.into_owned();
      if secret.len() % 2 != 0 {
        secret.insert(0, '0')
      }
      Ok(secret)
    }
  }
}

/// Configure a one-time password slot on the Nitrokey device.
pub fn otp_set(ctx: &mut Context<'_>, args: args::OtpSetArgs) -> anyhow::Result<()> {
  // Ideally, we would also like to verify the length of the secret. But the maximum length is
  // determined by the firmware version of the device and we don't want to run an additional
  // command just to determine the firmware version.
  ensure_string_lengths(&[("slot name", &args.name, OTP_NAME_LENGTH)])?;

  let secret = value_or_stdin(ctx, &args.secret)?;
  let secret = prepare_secret(secret, args.format)?;

  let data = nitrokey::OtpSlotData::new(args.slot, args.name, secret, args.digits.into());
  let (algorithm, counter, time_window) = (args.algorithm, args.counter, args.time_window);
  with_device(ctx, |ctx, device| {
    let mut device = authenticate_admin(ctx, device)?;
    match algorithm {
      args::OtpAlgorithm::Hotp => device.write_hotp_slot(data, counter),
      args::OtpAlgorithm::Totp => device.write_totp_slot(data, time_window),
    }
    .context("Failed to write OTP slot")?;
    Ok(())
  })
}

/// Clear an OTP slot.
pub fn otp_clear(
  ctx: &mut Context<'_>,
  slot: u8,
  algorithm: args::OtpAlgorithm,
) -> anyhow::Result<()> {
  with_device(ctx, |ctx, device| {
    let mut device = authenticate_admin(ctx, device)?;
    match algorithm {
      args::OtpAlgorithm::Hotp => device.erase_hotp_slot(slot),
      args::OtpAlgorithm::Totp => device.erase_totp_slot(slot),
    }
    .context("Failed to clear OTP slot")?;
    Ok(())
  })
}

fn print_otp_status(
  ctx: &mut Context<'_>,
  algorithm: args::OtpAlgorithm,
  device: &nitrokey::DeviceWrapper<'_>,
  all: bool,
) -> anyhow::Result<()> {
  let mut slot: u8 = 0;
  loop {
    let result = match algorithm {
      args::OtpAlgorithm::Hotp => device.get_hotp_slot_name(slot),
      args::OtpAlgorithm::Totp => device.get_totp_slot_name(slot),
    };
    slot = slot
      .checked_add(1)
      .context("Encountered integer overflow when iterating OTP slots")?;
    let name = match result {
      Ok(name) => name,
      Err(nitrokey::Error::LibraryError(nitrokey::LibraryError::InvalidSlot)) => return Ok(()),
      Err(nitrokey::Error::CommandError(nitrokey::CommandError::SlotNotProgrammed)) => {
        if all {
          "[not programmed]".to_string()
        } else {
          continue;
        }
      }
      Err(err) => return Err(err).context("Failed to check OTP slot"),
    };
    println!(ctx, "{}\t{}\t{}", algorithm, slot - 1, name)?;
  }
}

/// Print the status of the OTP slots.
pub fn otp_status(ctx: &mut Context<'_>, all: bool) -> anyhow::Result<()> {
  with_device(ctx, |ctx, device| {
    println!(ctx, "alg\tslot\tname")?;
    print_otp_status(ctx, args::OtpAlgorithm::Hotp, &device, all)?;
    print_otp_status(ctx, args::OtpAlgorithm::Totp, &device, all)?;
    Ok(())
  })
}

/// Clear the PIN stored by various operations.
pub fn pin_clear(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |_ctx, device| {
    pinentry::clear(&pinentry::PinEntry::from(args::PinType::Admin, &device)?)
      .context("Failed to clear admin PIN")?;
    pinentry::clear(&pinentry::PinEntry::from(args::PinType::User, &device)?)
      .context("Failed to clear user PIN")?;
    Ok(())
  })
}

/// Choose a PIN of the given type.
///
/// If the user has set the respective environment variable for the
/// given PIN type, it will be used.
fn choose_pin(
  ctx: &mut Context<'_>,
  pin_entry: &pinentry::PinEntry,
  new: bool,
) -> anyhow::Result<String> {
  let new_pin = match pin_entry.pin_type() {
    args::PinType::Admin => {
      if new {
        &ctx.new_admin_pin
      } else {
        &ctx.admin_pin
      }
    }
    args::PinType::User => {
      if new {
        &ctx.new_user_pin
      } else {
        &ctx.user_pin
      }
    }
  };

  if let Some(new_pin) = new_pin {
    new_pin
      .to_str()
      .context("Failed to read PIN: Invalid Unicode data found")
      .map(ToOwned::to_owned)
  } else {
    pinentry::choose(ctx, pin_entry).context("Failed to select PIN")
  }
}

/// Change a PIN.
pub fn pin_set(ctx: &mut Context<'_>, pin_type: args::PinType) -> anyhow::Result<()> {
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(pin_type, &device)?;
    let new_pin = choose_pin(ctx, &pin_entry, true)?;

    try_with_pin(ctx, &pin_entry, |current_pin| match pin_type {
      args::PinType::Admin => device
        .change_admin_pin(current_pin, &new_pin)
        .context("Failed to change admin PIN"),
      args::PinType::User => device
        .change_user_pin(current_pin, &new_pin)
        .context("Failed to change user PIN"),
    })?;

    // We just changed the PIN but confirmed the action with the old PIN,
    // which may have caused it to be cached. Since it no longer applies,
    // make sure to evict the corresponding entry from the cache.
    pinentry::clear(&pin_entry)
  })
}

/// Unblock and reset the user PIN.
pub fn pin_unblock(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::User, &device)?;
    let user_pin = choose_pin(ctx, &pin_entry, false)?;
    let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;

    try_with_pin(ctx, &pin_entry, |admin_pin| {
      device
        .unlock_user_pin(admin_pin, &user_pin)
        .context("Failed to unblock user PIN")
    })
  })
}

fn print_pws_data(
  ctx: &mut Context<'_>,
  description: &'static str,
  result: Result<String, nitrokey::Error>,
  quiet: bool,
) -> anyhow::Result<()> {
  let value = result.context("Failed to access PWS slot")?;
  if quiet {
    println!(ctx, "{}", value)?;
  } else {
    println!(ctx, "{} {}", description, value)?;
  }
  Ok(())
}

/// Read a PWS slot.
pub fn pws_get(
  ctx: &mut Context<'_>,
  slot: u8,
  show_name: bool,
  show_login: bool,
  show_password: bool,
  quiet: bool,
) -> anyhow::Result<()> {
  with_password_safe(ctx, |ctx, pws| {
    let slot = pws.get_slot(slot).context("Failed to access PWS slot")?;

    let show_all = !show_name && !show_login && !show_password;
    if show_all || show_name {
      print_pws_data(ctx, "name:    ", slot.get_name(), quiet)?;
    }
    if show_all || show_login {
      print_pws_data(ctx, "login:   ", slot.get_login(), quiet)?;
    }
    if show_all || show_password {
      print_pws_data(ctx, "password:", slot.get_password(), quiet)?;
    }
    Ok(())
  })
}

fn ensure_pws_string_lengths(
  name: Option<&str>,
  login: Option<&str>,
  password: Option<&str>,
) -> anyhow::Result<()> {
  let mut data = Vec::new();
  if let Some(name) = name {
    data.push(("slot name", name, PWS_NAME_LENGTH));
  }
  if let Some(login) = login {
    data.push(("login", login, PWS_LOGIN_LENGTH));
  }
  if let Some(password) = password {
    data.push(("password", password, PWS_PASSWORD_LENGTH));
  }
  ensure_string_lengths(&data)
}

/// Add a new PWS slot.
pub fn pws_add(
  ctx: &mut Context<'_>,
  name: &str,
  login: &str,
  password: &str,
  slot_idx: Option<u8>,
) -> anyhow::Result<()> {
  let password = value_or_stdin(ctx, password)?;
  ensure_pws_string_lengths(Some(name), Some(login), Some(&password))?;
  with_password_safe(ctx, |ctx, mut pws| {
    let slots = pws.get_slots()?;

    let slot_idx = if let Some(slot_idx) = slot_idx {
      // If the user specified a slot, make sure that it is not programmed
      if let Some(slot) = slots.get(usize::from(slot_idx)) {
        if slot.is_some() {
          Err(anyhow::anyhow!(
            "The PWS slot {} is already programmed",
            slot_idx
          ))
        } else {
          Ok(slot_idx)
        }
      } else {
        Err(anyhow::anyhow!(
          "Encountered invalid slot index: {}",
          slot_idx
        ))
      }
    } else {
      // If the user did not specify a slot, we try to find the first unprogrammed slot
      if let Some(slot_idx) = slots.iter().position(Option::is_none) {
        u8::try_from(slot_idx).context("Unexpected number of PWS slots")
      } else {
        Err(anyhow::anyhow!("All PWS slots are already programmed"))
      }
    }?;

    pws
      .write_slot(slot_idx, name, login, password.as_ref())
      .context("Failed to write PWS slot")?;
    println!(ctx, "Added PWS slot {}", slot_idx)?;
    Ok(())
  })
}

/// Update a PWS slot.
pub fn pws_update(
  ctx: &mut Context<'_>,
  slot_idx: u8,
  name: Option<&str>,
  login: Option<&str>,
  password: Option<&str>,
) -> anyhow::Result<()> {
  if name.is_none() && login.is_none() && password.is_none() {
    anyhow::bail!("You have to set at least one of --name, --login, or --password");
  }

  let password = password.map(|s| value_or_stdin(ctx, s)).transpose()?;
  ensure_pws_string_lengths(name, login, password.as_deref())?;

  with_password_safe(ctx, |_ctx, mut pws| {
    let slot = pws.get_slot(slot_idx).context("Failed to query PWS slot")?;
    let name = name
      .map(|s| Ok(borrow::Cow::from(s)))
      .unwrap_or_else(|| slot.get_name().map(borrow::Cow::from))
      .context("Failed to query current slot name")?;
    let login = login
      .map(|s| Ok(borrow::Cow::from(s)))
      .unwrap_or_else(|| slot.get_login().map(borrow::Cow::from))
      .context("Failed to query current slot login")?;
    let password = password
      .as_ref()
      .map(|s| Ok(borrow::Cow::from(s.as_ref())))
      .unwrap_or_else(|| slot.get_password().map(borrow::Cow::from))
      .context("Failed to query current slot password")?;
    pws
      .write_slot(slot_idx, name.as_ref(), login.as_ref(), password.as_ref())
      .context("Failed to write PWS slot")
  })
}

/// Clear a PWS slot.
pub fn pws_clear(ctx: &mut Context<'_>, slot: u8) -> anyhow::Result<()> {
  with_password_safe(ctx, |_ctx, mut pws| {
    pws.erase_slot(slot).context("Failed to clear PWS slot")
  })
}

fn print_pws_slot(
  ctx: &mut Context<'_>,
  index: usize,
  slot: Option<nitrokey::PasswordSlot<'_, '_, '_>>,
) -> anyhow::Result<()> {
  let name = if let Some(slot) = slot {
    slot.get_name().context("Failed to read PWS slot name")?
  } else {
    "[not programmed]".to_string()
  };
  println!(ctx, "{}\t{}", index, name)?;
  Ok(())
}

/// Print the status of all PWS slots.
pub fn pws_status(ctx: &mut Context<'_>, all: bool) -> anyhow::Result<()> {
  with_password_safe(ctx, |ctx, pws| {
    let slots = pws.get_slots().context("Failed to read PWS slot status")?;
    println!(ctx, "slot\tname")?;
    for (i, &slot) in slots
      .iter()
      .enumerate()
      .filter(|(_, &slot)| all || slot.is_some())
    {
      print_pws_slot(ctx, i, slot)?;
    }
    Ok(())
  })
}

/// Find and list all available extensions.
///
/// The logic used in this function should use the same criteria as
/// `resolve_extension`.
pub(crate) fn discover_extensions(path_var: &ffi::OsStr) -> anyhow::Result<Vec<String>> {
  let dirs = env::split_paths(path_var);
  let mut commands = Vec::new();

  for dir in dirs {
    match fs::read_dir(&dir) {
      Ok(entries) => {
        for entry in entries {
          let entry = entry?;
          let path = entry.path();
          if path.is_file() {
            let name = entry.file_name();
            let file = name.to_string_lossy();
            if file.starts_with(NITROCLI_EXT_PREFIX) {
              let mut file = file.into_owned();
              file.replace_range(..NITROCLI_EXT_PREFIX.len(), "");
              commands.push(file);
            }
          }
        }
      }
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => (),
      x => x
        .map(|_| ())
        .with_context(|| format!("Failed to iterate entries of directory {}", dir.display()))?,
    }
  }
  Ok(commands)
}

/// Resolve an extension provided by name to an actual path.
///
/// Extensions are (executable) files that have the "nitrocli-" prefix
/// and are discoverable via the `PATH` environment variable.
///
/// The logic used in this function should use the same criteria as
/// `discover_extensions`.
pub(crate) fn resolve_extension(
  path_var: &ffi::OsStr,
  ext_name: &ffi::OsStr,
) -> anyhow::Result<path::PathBuf> {
  let mut bin_name = ffi::OsString::from(NITROCLI_EXT_PREFIX);
  bin_name.push(ext_name);

  for dir in env::split_paths(path_var) {
    let mut bin_path = dir.clone();
    bin_path.push(&bin_name);
    // Note that we deliberately do not check whether the file we found
    // is executable. If it is not we will just fail later on with a
    // permission denied error. The reasons for this behavior are two
    // fold:
    // 1) Checking whether a file is executable in Rust is painful (as
    //    of 1.37 there exists the PermissionsExt trait but it is
    //    available only for Unix based systems).
    // 2) It is considered a better user experience to resolve to an
    //    extension even if it later turned out to be not usable over
    //    not showing it and silently doing nothing -- mostly because
    //    anything residing in PATH should be executable anyway and
    //    given that its name also starts with nitrocli- we are pretty
    //    sure that's a bug on the user's side.
    if bin_path.is_file() {
      return Ok(bin_path);
    }
  }

  let err = if let Some(name) = bin_name.to_str() {
    format!("Extension {} not found", name).into()
  } else {
    borrow::Cow::from("Extension not found")
  };
  Err(io::Error::new(io::ErrorKind::NotFound, err).into())
}

/// Run an extension.
pub fn extension(ctx: &mut Context<'_>, args: Vec<ffi::OsString>) -> anyhow::Result<()> {
  // Note that while `Command` would actually honor PATH by itself, we
  // do not want that behavior because it would circumvent the execution
  // context we use for testing. As such, we need to do our own search.
  let mut args = args.into_iter();
  let ext_name = args.next().context("No extension specified")?;
  let path_var = ctx.path.as_ref().context("PATH variable not present")?;
  let ext_path = resolve_extension(path_var, &ext_name)?;

  // Note that theoretically we could just exec the extension and be
  // done. However, the problem with that approach is that it makes
  // testing extension support much more nasty, because the test process
  // would be overwritten in the process, requiring us to essentially
  // fork & exec nitrocli beforehand -- which is much more involved from
  // a cargo test context.
  let mut cmd = process::Command::new(&ext_path);

  if let Ok(device_info) = find_device(&ctx.config) {
    let _ = cmd.env(crate::NITROCLI_RESOLVED_USB_PATH, device_info.path);
  }

  if let Some(model) = ctx.config.model {
    let _ = cmd.env(crate::NITROCLI_MODEL, model.to_string());
  }

  if let Some(usb_path) = &ctx.config.usb_path {
    let _ = cmd.env(crate::NITROCLI_USB_PATH, usb_path);
  }

  // TODO: We may want to take this path from the command execution
  //       context.
  let binary = env::current_exe().context("Failed to retrieve path to nitrocli binary")?;
  let serial_numbers = ctx
    .config
    .serial_numbers
    .iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>()
    .join(",");

  let out = cmd
    .env(crate::NITROCLI_BINARY, binary)
    .env(crate::NITROCLI_VERBOSITY, ctx.config.verbosity.to_string())
    .env(crate::NITROCLI_NO_CACHE, ctx.config.no_cache.to_string())
    .env(crate::NITROCLI_SERIAL_NUMBERS, serial_numbers)
    .args(args)
    .output()
    .with_context(|| format!("Failed to execute extension {}", ext_path.display()))?;
  ctx.stdout.write_all(&out.stdout)?;
  ctx.stderr.write_all(&out.stderr)?;

  if out.status.success() {
    Ok(())
  } else if let Some(rc) = out.status.code() {
    Err(anyhow::Error::new(crate::DirectExitError(rc)))
  } else {
    Err(anyhow::Error::new(crate::DirectExitError(1)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prepare_secret_ascii() {
    let result = prepare_ascii_secret("12345678901234567890");
    assert_eq!(
      "3132333435363738393031323334353637383930".to_string(),
      result.unwrap()
    );
  }

  #[test]
  fn prepare_secret_non_ascii() {
    let result = prepare_ascii_secret("Österreich");
    assert!(result.is_err());
  }

  #[test]
  fn prepare_secret_base32() {
    let result = prepare_base32_secret("gezdgnbvgy3tqojqgezdgnbvgy3tqojq").unwrap();
    assert_eq!(
      "3132333435363738393031323334353637383930".to_string(),
      result
    );
    let result2 = prepare_base32_secret("gezd gnbv     gy3t qojq gezd gnbv gy3t qojq").unwrap();
    assert_eq!(result, result2);
  }

  #[test]
  fn hex_string() {
    assert_eq!(format_bytes(b" "), "20");
    assert_eq!(format_bytes(b"  "), "2020");
    assert_eq!(format_bytes(b"\n\n"), "0a0a");
  }
}
