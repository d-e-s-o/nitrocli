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
fn get_error(msg: &str, err: nitrokey::CommandError) -> Error {
  Error::Error(format!("{}: {:?}", msg, err))
}

/// Set `libnitrokey`'s log level based on the execution context's verbosity.
fn set_log_level(ctx: &args::ExecCtx) {
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
fn get_device(ctx: &args::ExecCtx) -> Result<nitrokey::DeviceWrapper> {
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
  .map_err(|_| Error::Error("Nitrokey device not found".to_string()))
}

/// Connect to a Nitrokey Storage device and return it.
fn get_storage_device(ctx: &args::ExecCtx) -> Result<nitrokey::Storage> {
  set_log_level(ctx);

  if let Some(model) = ctx.model {
    if model != args::DeviceModel::Storage {
      return Err(Error::Error(
        "This command is only available on the Nitrokey Storage".to_string(),
      ));
    }
  }

  nitrokey::Storage::connect().or_else(|_| {
    Err(Error::Error(
      "Nitrokey Storage device not found".to_string(),
    ))
  })
}

/// Open the password safe on the given device.
fn get_password_safe(device: &dyn Device) -> Result<nitrokey::PasswordSafe<'_>> {
  try_with_passphrase_and_data(
    pinentry::PinType::User,
    "Could not access the password safe",
    (),
    |_, passphrase| {
      device
        .get_password_safe(passphrase)
        .map_err(|err| ((), err))
    },
  )
  .map_err(|(_, err)| err)
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
  try_with_passphrase_and_data(pin_type, msg, device, op).map_err(|(_device, err)| err)
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
    |device, passphrase| device.authenticate_user(passphrase),
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
    |device, passphrase| device.authenticate_admin(passphrase),
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

/// Try to execute the given function with a passphrase queried using pinentry.
///
/// This function will query the passphrase of the given type from the
/// user using pinentry.  It will then execute the given function.  If
/// this function returns a result, the result will be passed it on.  If
/// it returns a `CommandError::WrongPassword`, the user will be asked
/// again to enter the passphrase.  Otherwise, this function returns an
/// error containing the given error message.  The user will have at
/// most three tries to get the passphrase right.
///
/// The data argument can be used to pass on data between the tries.  At
/// the first try, this function will call `op` with `data`.  At the
/// second or third try, it will call `op` with the data returned by the
/// previous call to `op`.
fn try_with_passphrase_and_data<D, F, R>(
  pin: pinentry::PinType,
  msg: &'static str,
  data: D,
  op: F,
) -> result::Result<R, (D, Error)>
where
  F: Fn(D, &str) -> result::Result<R, (D, nitrokey::CommandError)>,
{
  let mut data = data;
  let mut retry = 3;
  let mut error_msg = None;
  loop {
    let passphrase = match pinentry::inquire_passphrase(pin, pinentry::Mode::Query, error_msg) {
      Ok(passphrase) => passphrase,
      Err(err) => return Err((data, err)),
    };
    let passphrase = match String::from_utf8(passphrase) {
      Ok(passphrase) => passphrase,
      Err(err) => return Err((data, Error::from(err))),
    };
    match op(data, &passphrase) {
      Ok(result) => return Ok(result),
      Err((new_data, err)) => match err {
        nitrokey::CommandError::WrongPassword => {
          if let Err(err) = pinentry::clear_passphrase(pin) {
            return Err((new_data, err));
          }
          retry -= 1;

          if retry > 0 {
            error_msg = Some("Wrong password, please reenter");
            data = new_data;
            continue;
          }
          let error = format!("{}: Wrong password", msg);
          return Err((new_data, Error::Error(error)));
        }
        err => return Err((new_data, get_error(msg, err))),
      },
    };
  }
}

/// Try to execute the given function with a passphrase queried using pinentry.
///
/// This function behaves exactly as `try_with_passphrase_and_data`, but
/// it refrains from passing any data to it.
fn try_with_passphrase<F>(pin: pinentry::PinType, msg: &'static str, op: F) -> Result<()>
where
  F: Fn(&str) -> result::Result<(), nitrokey::CommandError>,
{
  try_with_passphrase_and_data(pin, msg, (), |data, passphrase| {
    op(passphrase).map_err(|err| (data, err))
  })
  .map_err(|(_data, err)| err)
}

/// Query and pretty print the status that is common to all Nitrokey devices.
fn print_status(model: &'static str, device: &nitrokey::DeviceWrapper) -> Result<()> {
  let serial_number = device
    .get_serial_number()
    .map_err(|err| get_error("Could not query the serial number", err))?;
  println!(
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
  );
  Ok(())
}

/// Inquire the status of the nitrokey.
pub fn status(ctx: &args::ExecCtx) -> Result<()> {
  let device = get_device(ctx)?;
  let model = match device {
    nitrokey::DeviceWrapper::Pro(_) => "Pro",
    nitrokey::DeviceWrapper::Storage(_) => "Storage",
  };
  print_status(model, &device)
}

/// Open the encrypted volume on the nitrokey.
pub fn storage_open(ctx: &args::ExecCtx) -> Result<()> {
  let device = get_storage_device(ctx)?;
  try_with_passphrase(
    pinentry::PinType::User,
    "Opening encrypted volume failed",
    |passphrase| device.enable_encrypted_volume(&passphrase),
  )
}

/// Close the previously opened encrypted volume.
pub fn storage_close(ctx: &args::ExecCtx) -> Result<()> {
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
fn print_storage_status(status: &nitrokey::StorageStatus) {
  println!(
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
  );
}

/// Connect to and pretty print the status of a Nitrokey Storage.
pub fn storage_status(ctx: &args::ExecCtx) -> Result<()> {
  let device = get_storage_device(ctx)?;
  let status = device
    .get_status()
    .map_err(|err| get_error("Getting Storage status failed", err))?;

  print_storage_status(&status);
  Ok(())
}

/// Return a String representation of the given Option.
fn format_option<T: fmt::Display>(option: Option<T>) -> String {
  match option {
    Some(value) => format!("{}", value),
    None => "not set".to_string(),
  }
}

/// Read the Nitrokey configuration.
pub fn config_get(ctx: &args::ExecCtx) -> Result<()> {
  let config = get_device(ctx)?
    .get_config()
    .map_err(|err| get_error("Could not get configuration", err))?;
  println!(
    r#"Config:
  numlock binding:          {nl}
  capslock binding:         {cl}
  scrollock binding:        {sl}
  require user PIN for OTP: {otp}"#,
    nl = format_option(config.numlock),
    cl = format_option(config.capslock),
    sl = format_option(config.scrollock),
    otp = config.user_password,
  );
  Ok(())
}

/// Write the Nitrokey configuration.
pub fn config_set(
  ctx: &args::ExecCtx,
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
pub fn lock(ctx: &args::ExecCtx) -> Result<()> {
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
    .or_else(|_| {
      Err(Error::Error(
        "Current system time is before the Unix epoch".to_string(),
      ))
    })
    .map(|duration| duration.as_secs())
}

/// Generate a one-time password on the Nitrokey device.
pub fn otp_get(
  ctx: &args::ExecCtx,
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
  println!("{}", otp);
  Ok(())
}

/// Prepare an ASCII secret string for libnitrokey.
///
/// libnitrokey expects secrets as hexadecimal strings.  This function transforms an ASCII string
/// into a hexadecimal string or returns an error if the given string contains non-ASCII
/// characters.
fn prepare_secret(secret: &str) -> Result<String> {
  if secret.is_ascii() {
    Ok(
      secret
        .as_bytes()
        .iter()
        .map(|c| format!("{:x}", c))
        .collect::<Vec<String>>()
        .join(""),
    )
  } else {
    Err(Error::Error(
      "The given secret is not an ASCII string despite --ascii being set".to_string(),
    ))
  }
}

/// Configure a one-time password slot on the Nitrokey device.
pub fn otp_set(
  ctx: &args::ExecCtx,
  data: nitrokey::OtpSlotData,
  algorithm: args::OtpAlgorithm,
  counter: u64,
  time_window: u16,
  ascii: bool,
) -> Result<()> {
  let secret = if ascii {
    prepare_secret(&data.secret)?
  } else {
    data.secret
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
pub fn otp_clear(ctx: &args::ExecCtx, slot: u8, algorithm: args::OtpAlgorithm) -> Result<()> {
  let device = authenticate_admin(get_device(ctx)?)?;
  match algorithm {
    args::OtpAlgorithm::Hotp => device.erase_hotp_slot(slot),
    args::OtpAlgorithm::Totp => device.erase_totp_slot(slot),
  }
  .map_err(|err| get_error("Could not clear OTP slot", err))?;
  Ok(())
}

fn print_otp_status(
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
        return Err(Error::Error(
          "Integer overflow when iterating OTP slots".to_string(),
        ))
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
    println!("{}\t{}\t{}", algorithm, slot - 1, name);
  }
}

/// Print the status of the OTP slots.
pub fn otp_status(ctx: &args::ExecCtx, all: bool) -> Result<()> {
  let device = get_device(ctx)?;
  println!("alg\tslot\tname");
  print_otp_status(args::OtpAlgorithm::Hotp, &device, all)?;
  print_otp_status(args::OtpAlgorithm::Totp, &device, all)?;
  Ok(())
}

/// Clear the PIN stored by various operations.
pub fn pin_clear() -> Result<()> {
  pinentry::clear_passphrase(pinentry::PinType::Admin)?;
  pinentry::clear_passphrase(pinentry::PinType::User)?;
  Ok(())
}

fn check_pin(pintype: pinentry::PinType, pin: &str) -> Result<()> {
  let minimum_length = match pintype {
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

fn choose_pin(pintype: pinentry::PinType) -> Result<String> {
  pinentry::clear_passphrase(pintype)?;
  let new_pin = pinentry::inquire_passphrase(pintype, pinentry::Mode::Choose, None)?;
  pinentry::clear_passphrase(pintype)?;
  let new_pin = String::from_utf8(new_pin)?;
  check_pin(pintype, &new_pin)?;

  let confirm_pin = pinentry::inquire_passphrase(pintype, pinentry::Mode::Confirm, None)?;
  pinentry::clear_passphrase(pintype)?;
  let confirm_pin = String::from_utf8(confirm_pin)?;

  if new_pin != confirm_pin {
    Err(Error::Error("Entered PINs do not match".to_string()))
  } else {
    Ok(new_pin)
  }
}

/// Change a PIN.
pub fn pin_set(ctx: &args::ExecCtx, pintype: pinentry::PinType) -> Result<()> {
  let device = get_device(ctx)?;
  let new_pin = choose_pin(pintype)?;
  try_with_passphrase(
    pintype,
    "Could not change the PIN",
    |current_pin| match pintype {
      pinentry::PinType::Admin => device.change_admin_pin(&current_pin, &new_pin),
      pinentry::PinType::User => device.change_user_pin(&current_pin, &new_pin),
    },
  )
}

/// Unblock and reset the user PIN.
pub fn pin_unblock(ctx: &args::ExecCtx) -> Result<()> {
  let device = get_device(ctx)?;
  let user_pin = choose_pin(pinentry::PinType::User)?;
  try_with_passphrase(
    pinentry::PinType::Admin,
    "Could not unblock the user PIN",
    |admin_pin| device.unlock_user_pin(&admin_pin, &user_pin),
  )
}

fn print_pws_data(
  description: &'static str,
  result: result::Result<String, nitrokey::CommandError>,
  quiet: bool,
) -> Result<()> {
  let value = result.map_err(|err| get_error("Could not access PWS slot", err))?;
  if quiet {
    println!("{}", value);
  } else {
    println!("{} {}", description, value);
  }
  Ok(())
}

/// Read a PWS slot.
pub fn pws_get(
  ctx: &args::ExecCtx,
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
    print_pws_data("name:    ", pws.get_slot_name(slot), quiet)?;
  }
  if show_all || show_login {
    print_pws_data("login:   ", pws.get_slot_login(slot), quiet)?;
  }
  if show_all || show_password {
    print_pws_data("password:", pws.get_slot_password(slot), quiet)?;
  }
  Ok(())
}

/// Write a PWS slot.
pub fn pws_set(
  ctx: &args::ExecCtx,
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
pub fn pws_clear(ctx: &args::ExecCtx, slot: u8) -> Result<()> {
  let device = get_device(ctx)?;
  let pws = get_password_safe(&device)?;
  pws
    .erase_slot(slot)
    .map_err(|err| get_error("Could not clear PWS slot", err))
}

fn print_pws_slot(pws: &nitrokey::PasswordSafe<'_>, slot: usize, programmed: bool) -> Result<()> {
  if slot > u8::MAX as usize {
    return Err(Error::Error("Invalid PWS slot number".to_string()));
  }
  let slot = slot as u8;
  let name = if programmed {
    pws
      .get_slot_name(slot)
      .map_err(|err| get_error("Could not read PWS slot", err))?
  } else {
    "[not programmed]".to_string()
  };
  println!("{}\t{}", slot, name);
  Ok(())
}

/// Print the status of all PWS slots.
pub fn pws_status(ctx: &args::ExecCtx, all: bool) -> Result<()> {
  let device = get_device(ctx)?;
  let pws = get_password_safe(&device)?;
  let slots = pws
    .get_slot_status()
    .map_err(|err| get_error("Could not read PWS slot status", err))?;
  println!("slot\tname");
  for (i, &value) in slots
    .into_iter()
    .enumerate()
    .filter(|(_, &value)| all || value)
  {
    print_pws_slot(&pws, i, value)?;
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prepare_secret_ascii() {
    let result = prepare_secret("12345678901234567890");
    assert_eq!(
      "3132333435363738393031323334353637383930".to_string(),
      result.unwrap()
    );
  }

  #[test]
  fn prepare_secret_non_ascii() {
    let result = prepare_secret("Österreich");
    assert!(result.is_err());
  }
}