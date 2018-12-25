// commands.rs

// *************************************************************************
// * Copyright (C) 2018 Daniel Mueller (deso@posteo.net)                   *
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

use std::result;

use crate::error::Error;
use crate::pinentry;
use crate::Result;

const PIN_TYPE: pinentry::PinType = pinentry::PinType::User;

/// Create an `error::Error` with an error message of the format `msg: err`.
fn get_error(msg: &str, err: &nitrokey::CommandError) -> Error {
  Error::Error(format!("{}: {:?}", msg, err))
}

/// Connect to a Nitrokey Storage device and return it.
fn get_storage_device() -> Result<nitrokey::Storage> {
  nitrokey::Storage::connect()
    .or_else(|_| Err(Error::Error("Nitrokey device not found".to_string())))
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
  let mut error_msg: Option<&str> = None;
  loop {
    let passphrase = match pinentry::inquire_passphrase(pin, error_msg) {
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
        err => return Err((new_data, get_error(msg, &err))),
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

/// Pretty print the response of a status command.
fn print_status(status: &nitrokey::StorageStatus) {
  // We omit displaying information about the smartcard here as this
  // program really is only about the SD card portion of the device.
  println!(
    r#"Status:
  SD card ID:        {id:#x}
  firmware version:  {fwv0}.{fwv1}
  firmware:          {fw}
  storage keys:      {sk}
  user retry count:  {urc}
  admin retry count: {arc}
  volumes:
    unencrypted:     {vu}
    encrypted:       {ve}
    hidden:          {vh}"#,
    id = status.serial_number_sd_card,
    fwv0 = status.firmware_version_major,
    fwv1 = status.firmware_version_minor,
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
    urc = status.user_retry_count,
    arc = status.admin_retry_count,
    vu = get_volume_status(&status.unencrypted_volume),
    ve = get_volume_status(&status.encrypted_volume),
    vh = get_volume_status(&status.hidden_volume),
  );
}

/// Inquire the status of the nitrokey.
pub fn status() -> Result<()> {
  let status = get_storage_device()?
    .get_status()
    .map_err(|err| get_error("Getting Storage status failed", &err))?;

  print_status(&status);
  Ok(())
}

/// Open the encrypted volume on the nitrokey.
pub fn open() -> Result<()> {
  let device = get_storage_device()?;
  try_with_passphrase(
    pinentry::PinType::User,
    "Opening encrypted volume failed",
    |passphrase| device.enable_encrypted_volume(&passphrase),
  )
}

#[link(name = "c")]
extern "C" {
  fn sync();
}

/// Close the previously opened encrypted volume.
pub fn close() -> Result<()> {
  // Flush all filesystem caches to disk. We are mostly interested in
  // making sure that the encrypted volume on the nitrokey we are
  // about to close is not closed while not all data was written to
  // it.
  unsafe { sync() };

  get_storage_device()?
    .disable_encrypted_volume()
    .map_err(|err| get_error("Closing encrypted volume failed", &err))
}

/// Clear the PIN stored when opening the nitrokey's encrypted volume.
pub fn clear() -> Result<()> {
  pinentry::clear_passphrase(PIN_TYPE)
}
