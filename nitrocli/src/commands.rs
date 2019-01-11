// commands.rs

// *************************************************************************
// * Copyright (C) 2018-2019 Daniel Mueller (deso@posteo.net)              *
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
use std::result;
use std::time;
use std::u8;

use libc::sync;

use nitrokey::ConfigureOtp;
use nitrokey::Device;
use nitrokey::GenerateOtp;

use crate::args;
use crate::error::Error;
use crate::pinentry;
use crate::Result;

/// Create an `error::Error` with an error message of the format `msg: err`.
fn get_error(msg: &'static str, err: nitrokey::CommandError) -> Error {
  Error::CommandError(Some(msg), err)
}

/// Set `libnitrokey`'s log level based on the execution context's verbosity.
fn set_log_level(ctx: &mut args::ExecCtx<'_>) {
  let log_lvl = match ctx.verbosity {
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

/// Connect to any Nitrokey device and return it.
fn get_device(ctx: &mut args::ExecCtx<'_>) -> Result<nitrokey::DeviceWrapper> {
  set_log_level(ctx);

  match ctx.model {
    Some(model) => match model {
      args::DeviceModel::Pro => nitrokey::Pro::connect().map(nitrokey::DeviceWrapper::Pro),
      args::DeviceModel::Storage => {
        nitrokey::Storage::connect().map(nitrokey::DeviceWrapper::Storage)
      }
    },
    None => nitrokey::connect(),
  }
  .map_err(|_| Error::from("Nitrokey device not found"))
}

/// Connect to a Nitrokey Storage device and return it.
fn get_storage_device(ctx: &mut args::ExecCtx<'_>) -> Result<nitrokey::Storage> {
  set_log_level(ctx);

  if let Some(model) = ctx.model {
    if model != args::DeviceModel::Storage {
      return Err(Error::from(
        "This command is only available on the Nitrokey Storage",
      ));
    }
  }

  nitrokey::Storage::connect().or_else(|_| Err(Error::from("Nitrokey Storage device not found")))
}

/// Open the password safe on the given device.
fn get_password_safe(device: &dyn Device) -> Result<nitrokey::PasswordSafe<'_>> {
  try_with_pin_and_data(
    pinentry::PinType::User,
    "Could not access the password safe",
    (),
    |_, pin| device.get_password_safe(pin).map_err(|err| ((), err)),
  )
}

/// Authenticate the given device using the given PIN type and operation.
///
/// If an error occurs, the error message `msg` is used.
fn authenticate<D, A, F>(
  device: D,
  pin_type: pinentry::PinType,
  msg: &'static str,
  op: F,
) -> Result<A>
where
  D: Device,
  F: Fn(D, &str) -> result::Result<A, (D, nitrokey::CommandError)>,
{
  try_with_pin_and_data(pin_type, msg, device, op)
}

/// Authenticate the given device with the user PIN.
fn authenticate_user<T>(device: T) -> Result<nitrokey::User<T>>
where
  T: Device,
{
  authenticate(
    device,
    pinentry::PinType::User,
    "Could not authenticate as user",
    |device, pin| device.authenticate_user(pin),
  )
}

/// Authenticate the given device with the admin PIN.
fn authenticate_admin<T>(device: T) -> Result<nitrokey::Admin<T>>
where
  T: Device,
{
  authenticate(
    device,
    pinentry::PinType::Admin,
    "Could not authenticate as admin",
    |device, pin| device.authenticate_admin(pin),
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
/// function returns a result, the result will be passed it on.  If it
/// returns a `CommandError::WrongPassword`, the user will be asked
/// again to enter the pin.  Otherwise, this function returns an error
/// containing the given error message.  The user will have at most
/// three tries to get the pin right.
///
/// The data argument can be used to pass on data between the tries.  At
/// the first try, this function will call `op` with `data`.  At the
/// second or third try, it will call `op` with the data returned by the
/// previous call to `op`.
fn try_with_pin_and_data<D, F, R>(
  pin_type: pinentry::PinType,
  msg: &'static str,
  data: D,
  op: F,
) -> Result<R>
where
  F: Fn(D, &str) -> result::Result<R, (D, nitrokey::CommandError)>,
{
  let mut data = data;
  let mut retry = 3;
  let mut error_msg = None;
  loop {
    let pin = pinentry::inquire_pin(pin_type, pinentry::Mode::Query, error_msg)?;
    match op(data, &pin) {
      Ok(result) => return Ok(result),
      Err((new_data, err)) => match err {
        nitrokey::CommandError::WrongPassword => {
          pinentry::clear_pin(pin_type)?;
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
    };
  }
}

/// Try to execute the given function with a pin queried using pinentry.
///
/// This function behaves exactly as `try_with_pin_and_data`, but
/// it refrains from passing any data to it.
fn try_with_pin<F>(pin_type: pinentry::PinType, msg: &'static str, op: F) -> Result<()>
where
  F: Fn(&str) -> result::Result<(), nitrokey::CommandError>,
{
  try_with_pin_and_data(pin_type, msg, (), |data, pin| {
    op(pin).map_err(|err| (data, err))
  })
}

/// Query and pretty print the status that is common to all Nitrokey devices.
fn print_status(
  ctx: &mut args::ExecCtx<'_>,
  model: &'static str,
  device: &nitrokey::DeviceWrapper,
) -> Result<()> {
  let serial_number = device
    .get_serial_number()
    .map_err(|err| get_error("Could not query the serial number", err))?;
  println!(
    ctx,
    r#"Status:
  model:             {model}
  serial number:     0x{id}
  firmware version:  {fwv0}.{fwv1}
  user retry count:  {urc}
  admin retry count: {arc}"#,
    model = model,
    id = serial_number,
    fwv0 = device.get_major_firmware_version(),
    fwv1 = device.get_minor_firmware_version(),
    urc = device.get_user_retry_count(),
    arc = device.get_admin_retry_count(),
  )?;
  Ok(())
}

/// Inquire the status of the nitrokey.
pub fn status(ctx: &mut args::ExecCtx<'_>) -> Result<()> {
  let device = get_device(ctx)?;
  let model = match device {
    nitrokey::DeviceWrapper::Pro(_) => "Pro",
    nitrokey::DeviceWrapper::Storage(_) => "Storage",
  };
  print_status(ctx, model, &device)
}

/// Open the encrypted volume on the nitrokey.
pub fn storage_open(ctx: &mut args::ExecCtx<'_>) -> Result<()> {
  let device = get_storage_device(ctx)?;
  try_with_pin(
    pinentry::PinType::User,
    "Opening encrypted volume failed",
    |pin| device.enable_encrypted_volume(&pin),
  )
}

/// Close the previously opened encrypted volume.
pub fn storage_close(ctx: &mut args::ExecCtx<'_>) -> Result<()> {
  // Flush all filesystem caches to disk. We are mostly interested in
  // making sure that the encrypted volume on the nitrokey we are
  // about to close is not closed while not all data was written to
  // it.
  unsafe { sync() };

  get_storage_device(ctx)?
    .disable_encrypted_volume()
    .map_err(|err| get_error("Closing encrypted volume failed", err))
}

/// Pretty print the status of a Nitrokey Storage.
fn print_storage_status(
  ctx: &mut args::ExecCtx<'_>,
  status: &nitrokey::StorageStatus,
) -> Result<()> {
  println!(
    ctx,
    r#"Status:
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

/// Connect to and pretty print the status of a Nitrokey Storage.
pub fn storage_status(ctx: &mut args::ExecCtx<'_>) -> Result<()> {
  let device = get_storage_device(ctx)?;
  let status = device
    .get_status()
    .map_err(|err| get_error("Getting Storage status failed", err))?;

  print_storage_status(ctx, &status)
}

/// Return a String representation of the given Option.
fn format_option<T: fmt::Display>(option: Option<T>) -> String {
  match option {
    Some(value) => format!("{}", value),
    None => "not set".to_string(),
  }
}

/// Read the Nitrokey configuration.
pub fn config_get(ctx: &mut args::ExecCtx<'_>) -> Result<()> {
  let config = get_device(ctx)?
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
}

/// Write the Nitrokey configuration.
pub fn config_set(
  ctx: &mut args::ExecCtx<'_>,
  numlock: args::ConfigOption<u8>,
  capslock: args::ConfigOption<u8>,
  scrollock: args::ConfigOption<u8>,
  user_password: Option<bool>,
) -> Result<()> {
  let device = authenticate_admin(get_device(ctx)?)?;
  let config = device
    .get_config()
    .map_err(|err| get_error("Could not get configuration", err))?;
  let config = nitrokey::Config {
    numlock: numlock.or(config.numlock),
    capslock: capslock.or(config.capslock),
    scrollock: scrollock.or(config.scrollock),
    user_password: user_password.unwrap_or(config.user_password),
  };
  device
    .write_config(config)
    .map_err(|err| get_error("Could not set configuration", err))
}

/// Lock the Nitrokey device.
pub fn lock(ctx: &mut args::ExecCtx<'_>) -> Result<()> {
  get_device(ctx)?
    .lock()
    .map_err(|err| get_error("Getting Storage status failed", err))
}

fn get_otp<T: GenerateOtp>(slot: u8, algorithm: args::OtpAlgorithm, device: &T) -> Result<String> {
  match algorithm {
    args::OtpAlgorithm::Hotp => device.get_hotp_code(slot),
    args::OtpAlgorithm::Totp => device.get_totp_code(slot),
  }
  .map_err(|err| get_error("Could not generate OTP", err))
}

fn get_unix_timestamp() -> Result<u64> {
  time::SystemTime::now()
    .duration_since(time::UNIX_EPOCH)
    .or_else(|_| Err(Error::from("Current system time is before the Unix epoch")))
    .map(|duration| duration.as_secs())
}

/// Generate a one-time password on the Nitrokey device.
pub fn otp_get(
  ctx: &mut args::ExecCtx<'_>,
  slot: u8,
  algorithm: args::OtpAlgorithm,
  time: Option<u64>,
) -> Result<()> {
  let device = get_device(ctx)?;
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
    let user = authenticate_user(device)?;
    get_otp(slot, algorithm, &user)
  } else {
    get_otp(slot, algorithm, &device)
  }?;
  println!(ctx, "{}", otp)?;
  Ok(())
}

/// Format a byte vector as a hex string.
fn format_bytes(bytes: &[u8]) -> String {
  bytes
    .iter()
    .map(|c| format!("{:x}", c))
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
pub fn otp_set(
  ctx: &mut args::ExecCtx<'_>,
  data: nitrokey::OtpSlotData,
  algorithm: args::OtpAlgorithm,
  counter: u64,
  time_window: u16,
  secret_format: args::OtpSecretFormat,
) -> Result<()> {
  let secret = match secret_format {
    args::OtpSecretFormat::Ascii => prepare_ascii_secret(&data.secret)?,
    args::OtpSecretFormat::Base32 => prepare_base32_secret(&data.secret)?,
    args::OtpSecretFormat::Hex => data.secret,
  };
  let data = nitrokey::OtpSlotData { secret, ..data };
  let device = authenticate_admin(get_device(ctx)?)?;
  match algorithm {
    args::OtpAlgorithm::Hotp => device.write_hotp_slot(data, counter),
    args::OtpAlgorithm::Totp => device.write_totp_slot(data, time_window),
  }
  .map_err(|err| get_error("Could not write OTP slot", err))?;
  Ok(())
}

/// Clear an OTP slot.
pub fn otp_clear(
  ctx: &mut args::ExecCtx<'_>,
  slot: u8,
  algorithm: args::OtpAlgorithm,
) -> Result<()> {
  let device = authenticate_admin(get_device(ctx)?)?;
  match algorithm {
    args::OtpAlgorithm::Hotp => device.erase_hotp_slot(slot),
    args::OtpAlgorithm::Totp => device.erase_totp_slot(slot),
  }
  .map_err(|err| get_error("Could not clear OTP slot", err))?;
  Ok(())
}

fn print_otp_status(
  ctx: &mut args::ExecCtx<'_>,
  algorithm: args::OtpAlgorithm,
  device: &nitrokey::DeviceWrapper,
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
      Err(nitrokey::CommandError::InvalidSlot) => return Ok(()),
      Err(nitrokey::CommandError::SlotNotProgrammed) => {
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
pub fn otp_status(ctx: &mut args::ExecCtx<'_>, all: bool) -> Result<()> {
  let device = get_device(ctx)?;
  println!(ctx, "alg\tslot\tname")?;
  print_otp_status(ctx, args::OtpAlgorithm::Hotp, &device, all)?;
  print_otp_status(ctx, args::OtpAlgorithm::Totp, &device, all)?;
  Ok(())
}

/// Clear the PIN stored by various operations.
pub fn pin_clear() -> Result<()> {
  pinentry::clear_pin(pinentry::PinType::Admin)?;
  pinentry::clear_pin(pinentry::PinType::User)?;
  Ok(())
}

fn check_pin(pin_type: pinentry::PinType, pin: &str) -> Result<()> {
  let minimum_length = match pin_type {
    pinentry::PinType::Admin => 8,
    pinentry::PinType::User => 6,
  };
  if pin.len() < minimum_length {
    Err(Error::Error(format!(
      "The PIN must be at least {} characters long",
      minimum_length
    )))
  } else {
    Ok(())
  }
}

fn choose_pin(pin_type: pinentry::PinType) -> Result<String> {
  pinentry::clear_pin(pin_type)?;
  let new_pin = pinentry::inquire_pin(pin_type, pinentry::Mode::Choose, None)?;
  pinentry::clear_pin(pin_type)?;
  check_pin(pin_type, &new_pin)?;

  let confirm_pin = pinentry::inquire_pin(pin_type, pinentry::Mode::Confirm, None)?;
  pinentry::clear_pin(pin_type)?;

  if new_pin != confirm_pin {
    Err(Error::from("Entered PINs do not match"))
  } else {
    Ok(new_pin)
  }
}

/// Change a PIN.
pub fn pin_set(ctx: &mut args::ExecCtx<'_>, pin_type: pinentry::PinType) -> Result<()> {
  let device = get_device(ctx)?;
  let new_pin = choose_pin(pin_type)?;
  try_with_pin(
    pin_type,
    "Could not change the PIN",
    |current_pin| match pin_type {
      pinentry::PinType::Admin => device.change_admin_pin(&current_pin, &new_pin),
      pinentry::PinType::User => device.change_user_pin(&current_pin, &new_pin),
    },
  )
}

/// Unblock and reset the user PIN.
pub fn pin_unblock(ctx: &mut args::ExecCtx<'_>) -> Result<()> {
  let device = get_device(ctx)?;
  let user_pin = choose_pin(pinentry::PinType::User)?;
  try_with_pin(
    pinentry::PinType::Admin,
    "Could not unblock the user PIN",
    |admin_pin| device.unlock_user_pin(&admin_pin, &user_pin),
  )
}

fn print_pws_data(
  ctx: &mut args::ExecCtx<'_>,
  description: &'static str,
  result: result::Result<String, nitrokey::CommandError>,
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

/// Read a PWS slot.
pub fn pws_get(
  ctx: &mut args::ExecCtx<'_>,
  slot: u8,
  show_name: bool,
  show_login: bool,
  show_password: bool,
  quiet: bool,
) -> Result<()> {
  let device = get_device(ctx)?;
  let pws = get_password_safe(&device)?;
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
}

/// Write a PWS slot.
pub fn pws_set(
  ctx: &mut args::ExecCtx<'_>,
  slot: u8,
  name: &str,
  login: &str,
  password: &str,
) -> Result<()> {
  let device = get_device(ctx)?;
  let pws = get_password_safe(&device)?;
  pws
    .write_slot(slot, name, login, password)
    .map_err(|err| get_error("Could not write PWS slot", err))
}

/// Clear a PWS slot.
pub fn pws_clear(ctx: &mut args::ExecCtx<'_>, slot: u8) -> Result<()> {
  let device = get_device(ctx)?;
  let pws = get_password_safe(&device)?;
  pws
    .erase_slot(slot)
    .map_err(|err| get_error("Could not clear PWS slot", err))
}

fn print_pws_slot(
  ctx: &mut args::ExecCtx<'_>,
  pws: &nitrokey::PasswordSafe<'_>,
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
pub fn pws_status(ctx: &mut args::ExecCtx<'_>, all: bool) -> Result<()> {
  let device = get_device(ctx)?;
  let pws = get_password_safe(&device)?;
  let slots = pws
    .get_slot_status()
    .map_err(|err| get_error("Could not read PWS slot status", err))?;
  println!(ctx, "slot\tname")?;
  for (i, &value) in slots
    .into_iter()
    .enumerate()
    .filter(|(_, &value)| all || value)
  {
    print_pws_slot(ctx, &pws, i, value)?;
  }
  Ok(())
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
}
