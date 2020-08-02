// commands.rs

// *************************************************************************
// * Copyright (C) 2018-2020 Daniel Mueller (deso@posteo.net)              *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

use std::fmt;
use std::mem;
use std::result;
use std::thread;
use std::time;
use std::u8;

use libc::sync;

use nitrokey::ConfigureOtp;
use nitrokey::Device;
use nitrokey::GenerateOtp;
use nitrokey::GetPasswordSafe;

use crate::args;
use crate::error;
use crate::error::Error;
use crate::pinentry;
use crate::ExecCtx;
use crate::Result;

/// Create an `error::Error` with an error message of the format `msg: err`.
fn get_error(msg: &'static str, err: nitrokey::Error) -> Error {
  Error::NitrokeyError(Some(msg), err)
}

/// Set `libnitrokey`'s log level based on the execution context's verbosity.
fn set_log_level(ctx: &mut ExecCtx<'_>) {
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

/// Connect to any Nitrokey device and do something with it.
fn with_device<F>(ctx: &mut ExecCtx<'_>, op: F) -> Result<()>
where
  F: FnOnce(&mut ExecCtx<'_>, nitrokey::DeviceWrapper<'_>) -> Result<()>,
{
  let mut manager = nitrokey::take()?;
  set_log_level(ctx);

  let device = match ctx.config.model {
    Some(model) => manager.connect_model(model.into()).map_err(|_| {
      let error = format!("Nitrokey {} device not found", model.as_user_facing_str());
      Error::Error(error)
    })?,
    None => manager
      .connect()
      .map_err(|_| Error::from("Nitrokey device not found"))?,
  };

  op(ctx, device)
}

/// Connect to a Nitrokey Storage device and do something with it.
fn with_storage_device<F>(ctx: &mut ExecCtx<'_>, op: F) -> Result<()>
where
  F: FnOnce(&mut ExecCtx<'_>, nitrokey::Storage<'_>) -> Result<()>,
{
  let mut manager = nitrokey::take()?;
  set_log_level(ctx);

  if let Some(model) = ctx.config.model {
    if model != args::DeviceModel::Storage {
      return Err(Error::from(
        "This command is only available on the Nitrokey Storage",
      ));
    }
  }

  let device = manager
    .connect_storage()
    .map_err(|_| Error::from("Nitrokey Storage device not found"))?;
  op(ctx, device)
}

/// Connect to any Nitrokey device, retrieve a password safe handle, and
/// do something with it.
fn with_password_safe<F>(ctx: &mut ExecCtx<'_>, mut op: F) -> Result<()>
where
  F: FnMut(&mut ExecCtx<'_>, nitrokey::PasswordSafe<'_, '_>) -> Result<()>,
{
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::User, &device)?;
    try_with_pin_and_data(
      ctx,
      &pin_entry,
      "Could not access the password safe",
      (),
      move |ctx, _, pin| {
        let pws = device
          .get_password_safe(pin)
          .map_err(|err| ((), Error::from(err)))?;

        op(ctx, pws).map_err(|err| ((), err))
      },
    )
  })?;
  Ok(())
}

/// Authenticate the given device using the given PIN type and operation.
///
/// If an error occurs, the error message `msg` is used.
fn authenticate<'mgr, D, A, F>(
  ctx: &mut ExecCtx<'_>,
  device: D,
  pin_type: args::PinType,
  msg: &'static str,
  op: F,
) -> Result<A>
where
  D: Device<'mgr>,
  F: FnMut(&mut ExecCtx<'_>, D, &str) -> result::Result<A, (D, nitrokey::Error)>,
{
  let pin_entry = pinentry::PinEntry::from(pin_type, &device)?;

  try_with_pin_and_data(ctx, &pin_entry, msg, device, op)
}

/// Authenticate the given device with the user PIN.
fn authenticate_user<'mgr, T>(ctx: &mut ExecCtx<'_>, device: T) -> Result<nitrokey::User<'mgr, T>>
where
  T: Device<'mgr>,
{
  authenticate(
    ctx,
    device,
    args::PinType::User,
    "Could not authenticate as user",
    |_ctx, device, pin| device.authenticate_user(pin),
  )
}

/// Authenticate the given device with the admin PIN.
fn authenticate_admin<'mgr, T>(ctx: &mut ExecCtx<'_>, device: T) -> Result<nitrokey::Admin<'mgr, T>>
where
  T: Device<'mgr>,
{
  authenticate(
    ctx,
    device,
    args::PinType::Admin,
    "Could not authenticate as admin",
    |_ctx, device, pin| device.authenticate_admin(pin),
  )
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
fn try_with_pin_and_data_with_pinentry<D, F, R, E>(
  ctx: &mut ExecCtx<'_>,
  pin_entry: &pinentry::PinEntry,
  msg: &'static str,
  data: D,
  mut op: F,
) -> Result<R>
where
  F: FnMut(&mut ExecCtx<'_>, D, &str) -> result::Result<R, (D, E)>,
  E: error::TryInto<nitrokey::Error>,
{
  let mut data = data;
  let mut retry = 3;
  let mut error_msg = None;
  loop {
    let pin = pinentry::inquire(ctx, pin_entry, pinentry::Mode::Query, error_msg)?;
    match op(ctx, data, &pin) {
      Ok(result) => return Ok(result),
      Err((new_data, err)) => match err.try_into() {
        Ok(err) => match err {
          nitrokey::Error::CommandError(nitrokey::CommandError::WrongPassword) => {
            pinentry::clear(pin_entry)?;
            retry -= 1;

            if retry > 0 {
              error_msg = Some("Wrong password, please reenter");
              data = new_data;
              continue;
            }
            return Err(get_error(msg, err));
          }
          err => return Err(get_error(msg, err)),
        },
        Err(err) => return Err(err),
      },
    };
  }
}

/// Try to execute the given function with a PIN.
fn try_with_pin_and_data<D, F, R, E>(
  ctx: &mut ExecCtx<'_>,
  pin_entry: &pinentry::PinEntry,
  msg: &'static str,
  data: D,
  mut op: F,
) -> Result<R>
where
  F: FnMut(&mut ExecCtx<'_>, D, &str) -> result::Result<R, (D, E)>,
  E: Into<Error> + error::TryInto<nitrokey::Error>,
{
  let pin = match pin_entry.pin_type() {
    // Ideally we would not clone here, but that would require us to
    // restrict op to work with an immutable ExecCtx, which is not
    // possible given that some clients print data.
    args::PinType::Admin => ctx.admin_pin.clone(),
    args::PinType::User => ctx.user_pin.clone(),
  };

  if let Some(pin) = pin {
    let pin = pin.to_str().ok_or_else(|| {
      Error::Error(format!(
        "{}: Failed to read PIN due to invalid Unicode data",
        msg
      ))
    })?;
    op(ctx, data, &pin).map_err(|(_, err)| err.into())
  } else {
    try_with_pin_and_data_with_pinentry(ctx, pin_entry, msg, data, op)
  }
}

/// Try to execute the given function with a pin queried using pinentry.
///
/// This function behaves exactly as `try_with_pin_and_data`, but
/// it refrains from passing any data to it.
fn try_with_pin<F, E>(
  ctx: &mut ExecCtx<'_>,
  pin_entry: &pinentry::PinEntry,
  msg: &'static str,
  mut op: F,
) -> Result<()>
where
  F: FnMut(&str) -> result::Result<(), E>,
  E: Into<Error> + error::TryInto<nitrokey::Error>,
{
  try_with_pin_and_data(ctx, pin_entry, msg, (), |_ctx, data, pin| {
    op(pin).map_err(|err| (data, err))
  })
}

/// Pretty print the status of a Nitrokey Storage.
fn print_storage_status(ctx: &mut ExecCtx<'_>, status: &nitrokey::StorageStatus) -> Result<()> {
  println!(
    ctx,
    r#"  Storage:
    SD card ID:        {id:#x}
    firmware:          {fw}
    storage keys:      {sk}
    volumes:
      unencrypted:     {vu}
      encrypted:       {ve}
      hidden:          {vh}"#,
    id = status.serial_number_sd_card,
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

/// Query and pretty print the status that is common to all Nitrokey devices.
fn print_status(
  ctx: &mut ExecCtx<'_>,
  model: &'static str,
  device: &nitrokey::DeviceWrapper<'_>,
) -> Result<()> {
  let serial_number = device
    .get_serial_number()
    .map_err(|err| get_error("Could not query the serial number", err))?;

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
    fwv = device.get_firmware_version()?,
    urc = device.get_user_retry_count()?,
    arc = device.get_admin_retry_count()?,
  )?;

  if let nitrokey::DeviceWrapper::Storage(device) = device {
    let status = device
      .get_storage_status()
      .map_err(|err| get_error("Getting Storage status failed", err))?;

    print_storage_status(ctx, &status)
  } else {
    Ok(())
  }
}

/// Inquire the status of the nitrokey.
pub fn status(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_device(ctx, |ctx, device| {
    let model = match device {
      nitrokey::DeviceWrapper::Pro(_) => "Pro",
      nitrokey::DeviceWrapper::Storage(_) => "Storage",
    };
    print_status(ctx, model, &device)
  })
}

/// List the attached Nitrokey devices.
pub fn list(ctx: &mut ExecCtx<'_>, no_connect: bool) -> Result<()> {
  set_log_level(ctx);

  let device_infos = nitrokey::list_devices()?;
  if device_infos.is_empty() {
    println!(ctx, "No Nitrokey device connected")?;
  } else {
    println!(ctx, "device path\tmodel\tserial number")?;
    let mut manager = nitrokey::take()?;

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
            let device = manager.connect_path(device_info.path.clone())?;
            device.get_serial_number()?.to_string()
          }
        }
      };

      println!(ctx, "{}\t{}\t{}", device_info.path, model, serial_number)?;
    }
  }

  Ok(())
}

/// Perform a factory reset.
pub fn reset(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;

    // To force the user to enter the admin PIN before performing a
    // factory reset, we clear the pinentry cache for the admin PIN.
    pinentry::clear(&pin_entry)?;

    try_with_pin(ctx, &pin_entry, "Factory reset failed", |pin| {
      device.factory_reset(&pin)?;
      // Work around for a timing issue between factory_reset and
      // build_aes_key, see
      // https://github.com/Nitrokey/nitrokey-storage-firmware/issues/80
      thread::sleep(time::Duration::from_secs(3));
      // Another work around for spurious WrongPassword returns of
      // build_aes_key after a factory reset on Pro devices.
      // https://github.com/Nitrokey/nitrokey-pro-firmware/issues/57
      let _ = device.get_user_retry_count();
      device.build_aes_key(nitrokey::DEFAULT_ADMIN_PIN)
    })
  })
}

/// Change the configuration of the unencrypted volume.
pub fn unencrypted_set(ctx: &mut ExecCtx<'_>, mode: args::UnencryptedVolumeMode) -> Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;
    let mode = match mode {
      args::UnencryptedVolumeMode::ReadWrite => nitrokey::VolumeMode::ReadWrite,
      args::UnencryptedVolumeMode::ReadOnly => nitrokey::VolumeMode::ReadOnly,
    };

    // The unencrypted volume may reconnect, so be sure to flush caches to
    // disk.
    unsafe { sync() };

    try_with_pin(
      ctx,
      &pin_entry,
      "Changing unencrypted volume mode failed",
      |pin| device.set_unencrypted_volume_mode(&pin, mode),
    )
  })
}

/// Open the encrypted volume on the Nitrokey.
pub fn encrypted_open(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::User, &device)?;

    // We may forcefully close a hidden volume, if active, so be sure to
    // flush caches to disk.
    unsafe { sync() };

    try_with_pin(ctx, &pin_entry, "Opening encrypted volume failed", |pin| {
      device.enable_encrypted_volume(&pin)
    })
  })
}

/// Close the previously opened encrypted volume.
pub fn encrypted_close(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_storage_device(ctx, |_ctx, mut device| {
    // Flush all filesystem caches to disk. We are mostly interested in
    // making sure that the encrypted volume on the Nitrokey we are
    // about to close is not closed while not all data was written to
    // it.
    unsafe { sync() };

    device
      .disable_encrypted_volume()
      .map_err(|err| get_error("Closing encrypted volume failed", err))
  })
}

/// Create a hidden volume.
pub fn hidden_create(ctx: &mut ExecCtx<'_>, slot: u8, start: u8, end: u8) -> Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pwd_entry = pinentry::PwdEntry::from(&device)?;
    let pwd = if let Some(pwd) = &ctx.password {
      pwd
        .to_str()
        .ok_or_else(|| Error::from("Failed to read password: invalid Unicode data found"))
        .map(ToOwned::to_owned)
    } else {
      pinentry::choose(ctx, &pwd_entry)
    }?;

    device
      .create_hidden_volume(slot, start, end, &pwd)
      .map_err(|err| get_error("Creating hidden volume failed", err))
  })
}

/// Open a hidden volume.
pub fn hidden_open(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_storage_device(ctx, |ctx, mut device| {
    let pwd_entry = pinentry::PwdEntry::from(&device)?;
    let pwd = if let Some(pwd) = &ctx.password {
      pwd
        .to_str()
        .ok_or_else(|| Error::from("Failed to read password: invalid Unicode data found"))
        .map(ToOwned::to_owned)
    } else {
      pinentry::inquire(ctx, &pwd_entry, pinentry::Mode::Query, None)
    }?;

    // We may forcefully close an encrypted volume, if active, so be sure
    // to flush caches to disk.
    unsafe { sync() };

    device
      .enable_hidden_volume(&pwd)
      .map_err(|err| get_error("Opening hidden volume failed", err))
  })
}

/// Close a previously opened hidden volume.
pub fn hidden_close(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_storage_device(ctx, |_ctx, mut device| {
    unsafe { sync() };

    device
      .disable_hidden_volume()
      .map_err(|err| get_error("Closing hidden volume failed", err))
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
pub fn config_get(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_device(ctx, |ctx, device| {
    let config = device
      .get_config()
      .map_err(|err| get_error("Could not get configuration", err))?;
    println!(
      ctx,
      r#"Config:
  numlock binding:          {nl}
  capslock binding:         {cl}
  scrollock binding:        {sl}
  require user PIN for OTP: {otp}"#,
      nl = format_option(config.numlock),
      cl = format_option(config.capslock),
      sl = format_option(config.scrollock),
      otp = config.user_password,
    )?;
    Ok(())
  })
}

/// Write the Nitrokey configuration.
pub fn config_set(ctx: &mut ExecCtx<'_>, args: args::ConfigSetArgs) -> Result<()> {
  let numlock = args::ConfigOption::try_from(args.no_numlock, args.numlock, "numlock")?;
  let capslock = args::ConfigOption::try_from(args.no_capslock, args.capslock, "capslock")?;
  let scrollock = args::ConfigOption::try_from(args.no_scrollock, args.scrollock, "scrollock")?;
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
      .map_err(|err| get_error("Could not get configuration", err))?;
    let config = nitrokey::Config {
      numlock: numlock.or(config.numlock),
      capslock: capslock.or(config.capslock),
      scrollock: scrollock.or(config.scrollock),
      user_password: otp_pin.unwrap_or(config.user_password),
    };
    device
      .write_config(config)
      .map_err(|err| get_error("Could not set configuration", err))
  })
}

/// Lock the Nitrokey device.
pub fn lock(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_device(ctx, |_ctx, mut device| {
    device
      .lock()
      .map_err(|err| get_error("Could not lock the device", err))
  })
}

fn get_otp<T>(slot: u8, algorithm: args::OtpAlgorithm, device: &mut T) -> Result<String>
where
  T: GenerateOtp,
{
  match algorithm {
    args::OtpAlgorithm::Hotp => device.get_hotp_code(slot),
    args::OtpAlgorithm::Totp => device.get_totp_code(slot),
  }
  .map_err(|err| get_error("Could not generate OTP", err))
}

fn get_unix_timestamp() -> Result<u64> {
  time::SystemTime::now()
    .duration_since(time::UNIX_EPOCH)
    .map_err(|_| Error::from("Current system time is before the Unix epoch"))
    .map(|duration| duration.as_secs())
}

/// Generate a one-time password on the Nitrokey device.
pub fn otp_get(
  ctx: &mut ExecCtx<'_>,
  slot: u8,
  algorithm: args::OtpAlgorithm,
  time: Option<u64>,
) -> Result<()> {
  with_device(ctx, |ctx, mut device| {
    if algorithm == args::OtpAlgorithm::Totp {
      device
        .set_time(
          match time {
            Some(time) => time,
            None => get_unix_timestamp()?,
          },
          true,
        )
        .map_err(|err| get_error("Could not set time", err))?;
    }
    let config = device
      .get_config()
      .map_err(|err| get_error("Could not get device configuration", err))?;
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
fn prepare_ascii_secret(secret: &str) -> Result<String> {
  if secret.is_ascii() {
    Ok(format_bytes(&secret.as_bytes()))
  } else {
    Err(Error::from(
      "The given secret is not an ASCII string despite --format ascii being set",
    ))
  }
}

/// Prepare a base32 secret string for libnitrokey.
fn prepare_base32_secret(secret: &str) -> Result<String> {
  base32::decode(base32::Alphabet::RFC4648 { padding: false }, secret)
    .map(|vec| format_bytes(&vec))
    .ok_or_else(|| Error::from("Could not parse base32 secret"))
}

/// Configure a one-time password slot on the Nitrokey device.
pub fn otp_set(ctx: &mut ExecCtx<'_>, mut args: args::OtpSetArgs) -> Result<()> {
  let mut data = nitrokey::OtpSlotData {
    number: args.slot,
    name: mem::take(&mut args.name),
    secret: mem::take(&mut args.secret),
    mode: args.digits.into(),
    use_enter: false,
    token_id: None,
  };

  with_device(ctx, |ctx, device| {
    let secret = match args.format {
      args::OtpSecretFormat::Ascii => prepare_ascii_secret(&data.secret)?,
      args::OtpSecretFormat::Base32 => prepare_base32_secret(&data.secret)?,
      args::OtpSecretFormat::Hex => {
        // We need to ensure to provide a string with an even number of
        // characters in it, just because that's what libnitrokey
        // expects. So prepend a '0' if that is not the case.
        // TODO: This code can be removed once upstream issue #164
        //       (https://github.com/Nitrokey/libnitrokey/issues/164) is
        //       addressed.
        if data.secret.len() % 2 != 0 {
          data.secret.insert(0, '0')
        }
        data.secret
      }
    };
    let data = nitrokey::OtpSlotData { secret, ..data };
    let mut device = authenticate_admin(ctx, device)?;
    match args.algorithm {
      args::OtpAlgorithm::Hotp => device.write_hotp_slot(data, args.counter),
      args::OtpAlgorithm::Totp => device.write_totp_slot(data, args.time_window),
    }
    .map_err(|err| get_error("Could not write OTP slot", err))?;
    Ok(())
  })
}

/// Clear an OTP slot.
pub fn otp_clear(ctx: &mut ExecCtx<'_>, slot: u8, algorithm: args::OtpAlgorithm) -> Result<()> {
  with_device(ctx, |ctx, device| {
    let mut device = authenticate_admin(ctx, device)?;
    match algorithm {
      args::OtpAlgorithm::Hotp => device.erase_hotp_slot(slot),
      args::OtpAlgorithm::Totp => device.erase_totp_slot(slot),
    }
    .map_err(|err| get_error("Could not clear OTP slot", err))?;
    Ok(())
  })
}

fn print_otp_status(
  ctx: &mut ExecCtx<'_>,
  algorithm: args::OtpAlgorithm,
  device: &nitrokey::DeviceWrapper<'_>,
  all: bool,
) -> Result<()> {
  let mut slot: u8 = 0;
  loop {
    let result = match algorithm {
      args::OtpAlgorithm::Hotp => device.get_hotp_slot_name(slot),
      args::OtpAlgorithm::Totp => device.get_totp_slot_name(slot),
    };
    slot = match slot.checked_add(1) {
      Some(slot) => slot,
      None => {
        return Err(Error::from("Integer overflow when iterating OTP slots"));
      }
    };
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
      Err(err) => return Err(get_error("Could not check OTP slot", err)),
    };
    println!(ctx, "{}\t{}\t{}", algorithm, slot - 1, name)?;
  }
}

/// Print the status of the OTP slots.
pub fn otp_status(ctx: &mut ExecCtx<'_>, all: bool) -> Result<()> {
  with_device(ctx, |ctx, device| {
    println!(ctx, "alg\tslot\tname")?;
    print_otp_status(ctx, args::OtpAlgorithm::Hotp, &device, all)?;
    print_otp_status(ctx, args::OtpAlgorithm::Totp, &device, all)?;
    Ok(())
  })
}

/// Clear the PIN stored by various operations.
pub fn pin_clear(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_device(ctx, |_ctx, device| {
    pinentry::clear(&pinentry::PinEntry::from(args::PinType::Admin, &device)?)?;
    pinentry::clear(&pinentry::PinEntry::from(args::PinType::User, &device)?)?;
    Ok(())
  })
}

/// Choose a PIN of the given type.
///
/// If the user has set the respective environment variable for the
/// given PIN type, it will be used.
fn choose_pin(ctx: &mut ExecCtx<'_>, pin_entry: &pinentry::PinEntry, new: bool) -> Result<String> {
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
      .ok_or_else(|| Error::from("Failed to read PIN: invalid Unicode data found"))
      .map(ToOwned::to_owned)
  } else {
    pinentry::choose(ctx, pin_entry)
  }
}

/// Change a PIN.
pub fn pin_set(ctx: &mut ExecCtx<'_>, pin_type: args::PinType) -> Result<()> {
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(pin_type, &device)?;
    let new_pin = choose_pin(ctx, &pin_entry, true)?;

    try_with_pin(
      ctx,
      &pin_entry,
      "Could not change the PIN",
      |current_pin| match pin_type {
        args::PinType::Admin => device.change_admin_pin(&current_pin, &new_pin),
        args::PinType::User => device.change_user_pin(&current_pin, &new_pin),
      },
    )?;

    // We just changed the PIN but confirmed the action with the old PIN,
    // which may have caused it to be cached. Since it no longer applies,
    // make sure to evict the corresponding entry from the cache.
    pinentry::clear(&pin_entry)
  })
}

/// Unblock and reset the user PIN.
pub fn pin_unblock(ctx: &mut ExecCtx<'_>) -> Result<()> {
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::User, &device)?;
    let user_pin = choose_pin(ctx, &pin_entry, false)?;
    let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;

    try_with_pin(
      ctx,
      &pin_entry,
      "Could not unblock the user PIN",
      |admin_pin| device.unlock_user_pin(&admin_pin, &user_pin),
    )
  })
}

fn print_pws_data(
  ctx: &mut ExecCtx<'_>,
  description: &'static str,
  result: result::Result<String, nitrokey::Error>,
  quiet: bool,
) -> Result<()> {
  let value = result.map_err(|err| get_error("Could not access PWS slot", err))?;
  if quiet {
    println!(ctx, "{}", value)?;
  } else {
    println!(ctx, "{} {}", description, value)?;
  }
  Ok(())
}

fn check_slot(pws: &nitrokey::PasswordSafe<'_, '_>, slot: u8) -> Result<()> {
  if slot >= nitrokey::SLOT_COUNT {
    return Err(nitrokey::Error::from(nitrokey::LibraryError::InvalidSlot).into());
  }
  let status = pws
    .get_slot_status()
    .map_err(|err| get_error("Could not read PWS slot status", err))?;
  if status[slot as usize] {
    Ok(())
  } else {
    Err(get_error(
      "Could not access PWS slot",
      nitrokey::CommandError::SlotNotProgrammed.into(),
    ))
  }
}

/// Read a PWS slot.
pub fn pws_get(
  ctx: &mut ExecCtx<'_>,
  slot: u8,
  show_name: bool,
  show_login: bool,
  show_password: bool,
  quiet: bool,
) -> Result<()> {
  with_password_safe(ctx, |ctx, pws| {
    check_slot(&pws, slot)?;

    let show_all = !show_name && !show_login && !show_password;
    if show_all || show_name {
      print_pws_data(ctx, "name:    ", pws.get_slot_name(slot), quiet)?;
    }
    if show_all || show_login {
      print_pws_data(ctx, "login:   ", pws.get_slot_login(slot), quiet)?;
    }
    if show_all || show_password {
      print_pws_data(ctx, "password:", pws.get_slot_password(slot), quiet)?;
    }
    Ok(())
  })
}

/// Write a PWS slot.
pub fn pws_set(
  ctx: &mut ExecCtx<'_>,
  slot: u8,
  name: &str,
  login: &str,
  password: &str,
) -> Result<()> {
  with_password_safe(ctx, |_ctx, mut pws| {
    pws
      .write_slot(slot, name, login, password)
      .map_err(|err| get_error("Could not write PWS slot", err))
  })
}

/// Clear a PWS slot.
pub fn pws_clear(ctx: &mut ExecCtx<'_>, slot: u8) -> Result<()> {
  with_password_safe(ctx, |_ctx, mut pws| {
    pws
      .erase_slot(slot)
      .map_err(|err| get_error("Could not clear PWS slot", err))
  })
}

fn print_pws_slot(
  ctx: &mut ExecCtx<'_>,
  pws: &nitrokey::PasswordSafe<'_, '_>,
  slot: usize,
  programmed: bool,
) -> Result<()> {
  if slot > u8::MAX as usize {
    return Err(Error::from("Invalid PWS slot number"));
  }
  let slot = slot as u8;
  let name = if programmed {
    pws
      .get_slot_name(slot)
      .map_err(|err| get_error("Could not read PWS slot", err))?
  } else {
    "[not programmed]".to_string()
  };
  println!(ctx, "{}\t{}", slot, name)?;
  Ok(())
}

/// Print the status of all PWS slots.
pub fn pws_status(ctx: &mut ExecCtx<'_>, all: bool) -> Result<()> {
  with_password_safe(ctx, |ctx, pws| {
    let slots = pws
      .get_slot_status()
      .map_err(|err| get_error("Could not read PWS slot status", err))?;
    println!(ctx, "slot\tname")?;
    for (i, &value) in slots.iter().enumerate().filter(|(_, &value)| all || value) {
      print_pws_slot(ctx, &pws, i, value)?;
    }
    Ok(())
  })
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
  fn hex_string() {
    assert_eq!(format_bytes(&[b' ']), "20");
    assert_eq!(format_bytes(&[b' ', b' ']), "2020");
    assert_eq!(format_bytes(&[b'\n', b'\n']), "0a0a");
  }
}
