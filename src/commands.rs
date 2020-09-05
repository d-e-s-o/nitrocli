// commands.rs

// Copyright (C) 2018-2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::convert::TryFrom as _;
use std::fmt;
use std::mem;
use std::thread;
use std::time;
use std::u8;

use anyhow::Context as _;

use libc::sync;

use nitrokey::ConfigureOtp;
use nitrokey::Device;
use nitrokey::GenerateOtp;
use nitrokey::GetPasswordSafe;

use crate::args;
use crate::output;
use crate::output::Output as _;
use crate::pinentry;
use crate::Context;

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

/// Connect to any Nitrokey device and do something with it.
fn with_device<F>(ctx: &mut Context<'_>, op: F) -> anyhow::Result<()>
where
  F: FnOnce(&mut Context<'_>, nitrokey::DeviceWrapper<'_>) -> anyhow::Result<()>,
{
  let mut manager =
    nitrokey::take().context("Failed to acquire access to Nitrokey device manager")?;

  set_log_level(ctx);

  let device = match ctx.config.model {
    Some(model) => manager.connect_model(model.into()).with_context(|| {
      anyhow::anyhow!("Nitrokey {} device not found", model.as_user_facing_str())
    })?,
    None => manager.connect().context("Nitrokey device not found")?,
  };

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
  }

  let device = manager
    .connect_storage()
    .context("Nitrokey Storage device not found")?;
  op(ctx, device)
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
      .ok_or_else(|| anyhow::anyhow!("Failed to read PIN: Invalid Unicode data found"))?;
    op(ctx, data, &pin).map_err(|(_, err)| err)
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

#[derive(serde::Serialize)]
struct Status {
  model: args::DeviceModel,
  serial_number: String,
  firmware_version: String,
  user_retry_count: u8,
  admin_retry_count: u8,
  storage_status: Option<StorageStatus>,
}

#[derive(serde::Serialize)]
struct StorageStatus {
  card_id: u32,
  firmware_locked: bool,
  stick_initialized: bool,
  unencrypted_volume: VolumeStatus,
  encrypted_volume: VolumeStatus,
  hidden_volume: VolumeStatus,
}

#[derive(serde::Serialize)]
struct VolumeStatus {
  read_only: bool,
  active: bool,
}

impl From<&Status> for output::TextObject {
  fn from(status: &Status) -> output::TextObject {
    let mut o = output::TextObject::new("Status");
    o.push_line("model", status.model.to_string());
    o.push_line("serial number", &status.serial_number);
    o.push_line("firmware version", &status.firmware_version);
    o.push_line("user retry count", status.user_retry_count.to_string());
    o.push_line("admin retry count", status.admin_retry_count.to_string());

    if let Some(storage_status) = &status.storage_status {
      o.push_object(storage_status.into());
    }

    o
  }
}

impl fmt::Display for Status {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    output::TextObject::from(self).fmt(f)
  }
}

impl From<&StorageStatus> for output::TextObject {
  fn from(status: &StorageStatus) -> output::TextObject {
    let mut o = output::TextObject::new("Storage");
    o.push_line("SD card ID", format!("{:#x}", status.card_id));
    o.push_line(
      "firmware",
      if status.firmware_locked {
        "locked"
      } else {
        "unlocked"
      },
    );
    o.push_line(
      "storage keys",
      if status.stick_initialized {
        "created"
      } else {
        "not created "
      },
    );
    o.push_line("volumes", "");
    o.push_line("  unencrypted", status.unencrypted_volume.to_string());
    o.push_line("  encrypted", status.encrypted_volume.to_string());
    o.push_line("  hidden", status.hidden_volume.to_string());
    o
  }
}

impl From<nitrokey::StorageStatus> for StorageStatus {
  fn from(status: nitrokey::StorageStatus) -> StorageStatus {
    StorageStatus {
      card_id: status.serial_number_sd_card,
      firmware_locked: status.firmware_locked,
      stick_initialized: status.stick_initialized,
      unencrypted_volume: status.unencrypted_volume.into(),
      encrypted_volume: status.encrypted_volume.into(),
      hidden_volume: status.hidden_volume.into(),
    }
  }
}

impl From<nitrokey::VolumeStatus> for VolumeStatus {
  fn from(status: nitrokey::VolumeStatus) -> VolumeStatus {
    VolumeStatus {
      read_only: status.read_only,
      active: status.active,
    }
  }
}

impl fmt::Display for VolumeStatus {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(if self.active {
      if self.read_only {
        "read-only"
      } else {
        "active"
      }
    } else {
      "inactive"
    })
  }
}

/// Inquire the status of the nitrokey.
pub fn status(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |ctx, device| {
    let status = Status {
      model: device.get_model().into(),
      serial_number: device
        .get_serial_number()
        .context("Could not query the serial number")?
        .to_string(),
      firmware_version: device
        .get_firmware_version()
        .context("Failed to retrieve firmware version")?
        .to_string(),
      user_retry_count: device
        .get_user_retry_count()
        .context("Failed to retrieve user retry count")?,
      admin_retry_count: device
        .get_admin_retry_count()
        .context("Failed to retrieve admin retry count")?,
      storage_status: if let nitrokey::DeviceWrapper::Storage(device) = device {
        Some(
          device
            .get_storage_status()
            .context("Failed to retrieve storage status")?
            .into(),
        )
      } else {
        None
      },
    };
    output::Value::new("status", status).print(ctx)
  })
}

#[derive(serde::Serialize)]
struct DeviceInfo {
  path: String,
  model: Option<args::DeviceModel>,
  serial_number: Option<String>,
}

impl DeviceInfo {
  fn new(
    device_info: nitrokey::DeviceInfo,
    serial_number: Option<nitrokey::SerialNumber>,
  ) -> DeviceInfo {
    DeviceInfo {
      path: device_info.path,
      model: device_info.model.map(From::from),
      serial_number: serial_number.as_ref().map(ToString::to_string),
    }
  }
}

impl output::TableItem for DeviceInfo {
  fn headers() -> Vec<&'static str> {
    vec!["device path", "model", "serial number"]
  }

  fn values(&self) -> Vec<String> {
    vec![
      self.path.clone(),
      self
        .model
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_owned()),
      self
        .serial_number
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "N/A".to_owned()),
    ]
  }
}

/// List the attached Nitrokey devices.
pub fn list(ctx: &mut Context<'_>, no_connect: bool) -> anyhow::Result<()> {
  set_log_level(ctx);

  let mut table = output::Table::new("devices", "No Nitrokey device connected");
  let device_infos =
    nitrokey::list_devices().context("Failed to list connected Nitrokey devices")?;

  if !device_infos.is_empty() {
    let mut manager =
      nitrokey::take().context("Failed to acquire access to Nitrokey device manager")?;

    for device_info in device_infos {
      let serial_number = match device_info.serial_number {
        Some(serial_number) => Some(serial_number),
        None => {
          // Storage devices do not have the serial number present in
          // the device information. We have to connect to them to
          // retrieve the information.
          if no_connect {
            None
          } else {
            let device = manager
              .connect_path(device_info.path.clone())
              .context("Failed to connect to Nitrokey")?;
            Some(
              device
                .get_serial_number()
                .context("Failed to retrieve device serial number")?,
            )
          }
        }
      };

      table.push(DeviceInfo::new(device_info, serial_number));
    }
  }

  table.print(ctx)
}

/// Perform a factory reset.
pub fn reset(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |ctx, mut device| {
    let pin_entry = pinentry::PinEntry::from(args::PinType::Admin, &device)?;

    // To force the user to enter the admin PIN before performing a
    // factory reset, we clear the pinentry cache for the admin PIN.
    pinentry::clear(&pin_entry).context("Failed to clear cached secret")?;

    try_with_pin(ctx, &pin_entry, |pin| {
      device
        .factory_reset(&pin)
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
        .set_unencrypted_volume_mode(&pin, mode)
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
        .enable_encrypted_volume(&pin)
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
        .ok_or_else(|| anyhow::anyhow!("Failed to read password: invalid unicode data found"))
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
        .ok_or_else(|| anyhow::anyhow!("Failed to read password: Invalid Unicode data found"))
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

#[derive(serde::Serialize)]
struct Config {
  numlock: Option<u8>,
  capslock: Option<u8>,
  scrolllock: Option<u8>,
  user_password: bool,
}

impl From<&Config> for output::TextObject {
  fn from(config: &Config) -> output::TextObject {
    let mut o = output::TextObject::new("Config");
    o.push_line("numlock binding", format_option(config.numlock));
    o.push_line("capslock binding", format_option(config.capslock));
    o.push_line("scrolllock binding", format_option(config.scrolllock));
    o.push_line("require user PIN for OTP", config.user_password.to_string());
    o
  }
}

impl From<nitrokey::Config> for Config {
  fn from(config: nitrokey::Config) -> Config {
    Config {
      numlock: config.numlock,
      capslock: config.capslock,
      scrolllock: config.scrollock,
      user_password: config.user_password,
    }
  }
}

impl fmt::Display for Config {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    output::TextObject::from(self).fmt(f)
  }
}

/// Read the Nitrokey configuration.
pub fn config_get(ctx: &mut Context<'_>) -> anyhow::Result<()> {
  with_device(ctx, |ctx, device| {
    let config: Config = device
      .get_config()
      .context("Failed to get configuration")?
      .into();
    output::Value::new("config", config).print(ctx)
  })
}

/// Write the Nitrokey configuration.
pub fn config_set(ctx: &mut Context<'_>, args: args::ConfigSetArgs) -> anyhow::Result<()> {
  let numlock = args::ConfigOption::try_from(args.no_numlock, args.numlock, "numlock")
    .context("Failed to apply numlock configuration")?;
  let capslock = args::ConfigOption::try_from(args.no_capslock, args.capslock, "capslock")
    .context("Failed to apply capslock configuration")?;
  let scrollock = args::ConfigOption::try_from(args.no_scrollock, args.scrollock, "scrollock")
    .context("Failed to apply scrollock configuration")?;
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
      numlock: numlock.or(config.numlock),
      capslock: capslock.or(config.capslock),
      scrollock: scrollock.or(config.scrollock),
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
    output::Value::new("code", otp).print(ctx)
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
    Ok(format_bytes(&secret.as_bytes()))
  } else {
    anyhow::bail!("The given secret is not an ASCII string as expected")
  }
}

/// Prepare a base32 secret string for libnitrokey.
fn prepare_base32_secret(secret: &str) -> anyhow::Result<String> {
  base32::decode(base32::Alphabet::RFC4648 { padding: false }, secret)
    .map(|vec| format_bytes(&vec))
    .ok_or_else(|| anyhow::anyhow!("Failed to parse base32 secret"))
}

/// Configure a one-time password slot on the Nitrokey device.
pub fn otp_set(ctx: &mut Context<'_>, mut args: args::OtpSetArgs) -> anyhow::Result<()> {
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

#[derive(serde::Serialize)]
struct OtpSlot {
  algorithm: args::OtpAlgorithm,
  slot: u8,
  name: Option<String>,
}

impl OtpSlot {
  fn new(algorithm: args::OtpAlgorithm, slot: u8, name: Option<String>) -> OtpSlot {
    OtpSlot {
      algorithm,
      slot,
      name,
    }
  }
}

impl output::TableItem for OtpSlot {
  fn headers() -> Vec<&'static str> {
    vec!["algorithm", "slot", "name"]
  }

  fn values(&self) -> Vec<String> {
    vec![
      self.algorithm.to_string(),
      self.slot.to_string(),
      self
        .name
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "[not programmed]".to_owned()),
    ]
  }
}

fn get_otp_slots(
  algorithm: args::OtpAlgorithm,
  device: &nitrokey::DeviceWrapper<'_>,
  all: bool,
) -> anyhow::Result<Vec<OtpSlot>> {
  let mut vec = Vec::new();
  let mut slot: u8 = 0;
  loop {
    let result = match algorithm {
      args::OtpAlgorithm::Hotp => device.get_hotp_slot_name(slot),
      args::OtpAlgorithm::Totp => device.get_totp_slot_name(slot),
    };
    slot = slot
      .checked_add(1)
      .ok_or_else(|| anyhow::anyhow!("Encountered integer overflow when iterating OTP slots"))?;
    let name = match result {
      Ok(name) => Some(name),
      Err(nitrokey::Error::LibraryError(nitrokey::LibraryError::InvalidSlot)) => break,
      Err(nitrokey::Error::CommandError(nitrokey::CommandError::SlotNotProgrammed)) => {
        if all {
          None
        } else {
          continue;
        }
      }
      Err(err) => return Err(err).context("Failed to check OTP slot"),
    };
    vec.push(OtpSlot::new(algorithm, slot - 1, name));
  }
  Ok(vec)
}

/// Print the status of the OTP slots.
pub fn otp_status(ctx: &mut Context<'_>, all: bool) -> anyhow::Result<()> {
  with_device(ctx, |ctx, device| {
    let mut table = output::Table::new("otp-slots", "No OTP slots programmed.");
    table.append(&mut get_otp_slots(args::OtpAlgorithm::Hotp, &device, all)?);
    table.append(&mut get_otp_slots(args::OtpAlgorithm::Totp, &device, all)?);
    table.print(ctx)
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
      .ok_or_else(|| anyhow::anyhow!("Failed to read PIN: Invalid Unicode data found"))
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
        .change_admin_pin(&current_pin, &new_pin)
        .context("Failed to change admin PIN"),
      args::PinType::User => device
        .change_user_pin(&current_pin, &new_pin)
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
        .unlock_user_pin(&admin_pin, &user_pin)
        .context("Failed to unblock user PIN")
    })
  })
}

#[derive(serde::Serialize)]
struct PwsSlotData {
  name: Option<String>,
  login: Option<String>,
  password: Option<String>,
}

impl From<&PwsSlotData> for output::TextObject {
  fn from(data: &PwsSlotData) -> output::TextObject {
    let mut o = output::TextObject::new("PWS slot");
    if let Some(name) = &data.name {
      o.push_line("name", name);
    }
    if let Some(login) = &data.login {
      o.push_line("login", login);
    }
    if let Some(password) = &data.password {
      o.push_line("password", password);
    }
    o
  }
}

impl fmt::Display for PwsSlotData {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    output::TextObject::from(self).fmt(f)
  }
}

fn check_slot(pws: &nitrokey::PasswordSafe<'_, '_>, slot: u8) -> anyhow::Result<()> {
  if slot >= nitrokey::SLOT_COUNT {
    anyhow::bail!("Slot {} is not valid", slot);
  }
  let status = pws
    .get_slot_status()
    .context("Failed to read PWS slot status")?;
  if status[slot as usize] {
    Ok(())
  } else {
    anyhow::bail!("Slot {} is not programmed", slot)
  }
}

/// Read a PWS slot.
pub fn pws_get(
  ctx: &mut Context<'_>,
  slot: u8,
  show_name: bool,
  show_login: bool,
  show_password: bool,
) -> anyhow::Result<()> {
  with_password_safe(ctx, |ctx, pws| {
    check_slot(&pws, slot).context("Failed to access PWS slot")?;

    let show_all = !show_name && !show_login && !show_password;
    let name = if show_all || show_name {
      Some(
        pws
          .get_slot_name(slot)
          .context("Failed to access PWS slot")?,
      )
    } else {
      None
    };
    let login = if show_all || show_login {
      Some(
        pws
          .get_slot_login(slot)
          .context("Failed to access PWS slot")?,
      )
    } else {
      None
    };
    let password = if show_all || show_password {
      Some(
        pws
          .get_slot_password(slot)
          .context("Failed to access PWS slot")?,
      )
    } else {
      None
    };

    output::Value::new(
      "pws-slot",
      PwsSlotData {
        name,
        login,
        password,
      },
    )
    .print(ctx)
  })
}

/// Write a PWS slot.
pub fn pws_set(
  ctx: &mut Context<'_>,
  slot: u8,
  name: &str,
  login: &str,
  password: &str,
) -> anyhow::Result<()> {
  with_password_safe(ctx, |_ctx, mut pws| {
    pws
      .write_slot(slot, name, login, password)
      .context("Failed to write PWS slot")
  })
}

/// Clear a PWS slot.
pub fn pws_clear(ctx: &mut Context<'_>, slot: u8) -> anyhow::Result<()> {
  with_password_safe(ctx, |_ctx, mut pws| {
    pws.erase_slot(slot).context("Failed to clear PWS slot")
  })
}

#[derive(serde::Serialize)]
struct PwsSlot {
  slot: u8,
  name: Option<String>,
}

impl output::TableItem for PwsSlot {
  fn headers() -> Vec<&'static str> {
    vec!["slot", "name"]
  }

  fn values(&self) -> Vec<String> {
    vec![
      self.slot.to_string(),
      self
        .name
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "[not programmed]".to_owned()),
    ]
  }
}

fn get_pws_slot(
  pws: &nitrokey::PasswordSafe<'_, '_>,
  slot: usize,
  programmed: bool,
) -> anyhow::Result<PwsSlot> {
  let slot = u8::try_from(slot).map_err(|_| anyhow::anyhow!("Invalid PWS slot number"))?;
  let name = if programmed {
    Some(
      pws
        .get_slot_name(slot)
        .context("Failed to read PWS slot name")?,
    )
  } else {
    None
  };
  Ok(PwsSlot { slot, name })
}

/// Print the status of all PWS slots.
pub fn pws_status(ctx: &mut Context<'_>, all: bool) -> anyhow::Result<()> {
  with_password_safe(ctx, |ctx, pws| {
    let mut table = output::Table::new("otp-slots", "No PWS slots programmed");
    let slots = pws
      .get_slot_status()
      .context("Failed to read PWS slot status")?;
    for (i, &value) in slots.iter().enumerate().filter(|(_, &value)| all || value) {
      table.push(get_pws_slot(&pws, i, value)?);
    }
    table.print(ctx)
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
