// main.rs

// *************************************************************************
// * Copyright (C) 2017-2018 Daniel Mueller (deso@posteo.net)              *
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


#![deny(missing_docs)]

//! Nitrocli is a program providing a command line interface to certain
//! commands of the Nitrokey Storage device.

extern crate hid as libhid;


mod crc32;
mod error;
mod nitrokey;
mod pinentry;

use error::Error;
use std::mem;
use std::process;
use std::result;
use std::thread;
use std::time;

type Result<T> = result::Result<T, Error>;
type NitroFunc = Fn(&mut libhid::Handle) -> Result<()>;


const PIN_TYPE: pinentry::PinType = pinentry::PinType::User;
const SEND_TRY_COUNT: i8 = 3;
const RECV_TRY_COUNT: i8 = 40;
const SEND_RECV_DELAY_MS: u64 = 200;


/// Send a HID feature report to the device represented by the given handle.
fn send<P>(handle: &mut libhid::Handle, report: &nitrokey::Report<P>) -> Result<()>
  where P: AsRef<[u8]>,
{
  let mut retry = SEND_TRY_COUNT;
  loop {
    let result = handle.feature().send_to(0, report.as_ref());
    retry -= 1;

    match result {
      Ok(_) => {
        return Ok(());
      },
      Err(err) => {
        if retry > 0 {
          thread::sleep(time::Duration::from_millis(SEND_RECV_DELAY_MS));
          continue;
        } else {
          return Err(Error::HidError(err));
        }
      },
    }
  }
}


/// Receive a HID feature report from the device represented by the given handle.
fn receive<P>(handle: &mut libhid::Handle) -> Result<nitrokey::Report<P>>
  where P: AsRef<[u8]> + Default,
{
  let mut retry = RECV_TRY_COUNT;
  loop {
    let mut report = nitrokey::Report::<P>::new();
    let result = handle.feature().get_from(0, report.as_mut());

    retry -= 1;

    match result {
      Ok(size) => {
        if size < mem::size_of_val(&report) {
          if retry > 0 {
            continue;
          } else {
            return Err(Error::Error("Failed to receive complete report".to_string()));
          }
        }

        if !report.is_valid() {
          if retry > 0 {
            continue;
          } else {
            return Err(Error::Error("Failed to receive report: CRC mismatch".to_string()));
          }
        }
        return Ok(report);
      },

      Err(err) => {
        if retry > 0 {
          thread::sleep(time::Duration::from_millis(SEND_RECV_DELAY_MS));
          continue;
        } else {
          return Err(Error::HidError(err));
        }
      },
    }
  }
}


/// Transmit a HID feature report to the nitrokey and receive a response.
fn transmit<PS, PR>(handle: &mut libhid::Handle,
                    report: &nitrokey::Report<PS>)
                    -> Result<nitrokey::Report<PR>>
  where PS: AsRef<[u8]>,
        PR: AsRef<[u8]> + Default,
{
  send(handle, report)?;

  // We need to give the stick some time to handle the command. If we
  // don't, we might just receive stale data from before.
  thread::sleep(time::Duration::from_millis(SEND_RECV_DELAY_MS));

  receive::<PR>(handle)
}


/// Find and open the nitrokey device and execute a function on it.
fn nitrokey_do(function: &NitroFunc) -> Result<()> {
  let hid = libhid::init()?;
  // The Manager::find method is plain stupid as it still returns an
  // iterable. Using it does not help in more concise error handling.
  for device in hid.devices() {
    if device.vendor_id() == nitrokey::VID && device.product_id() == nitrokey::PID {
      return function(&mut device.open()?);
    }
  }
  Err(Error::Error("Nitrokey device not found".to_string()))
}


/// Pretty print the response of a status command.
fn print_status(response: &nitrokey::DeviceStatusResponse) {
  println!("Status:");
  // We omit displaying information about the smartcard here as this
  // program really is only about the SD card portion of the device.
  println!("  SD card ID:        {:#x}", response.active_sdcard_id);
  println!("  firmware version:  {}.{}",
           response.version_major,
           response.version_minor);
  println!("  firmware:          {}",
           if response.firmware_locked != 0 {
             "locked".to_string()
           } else {
             "unlocked".to_string()
           });
  println!("  storage keys:      {}",
           if response.storage_keys_missing == 0 {
             "created".to_string()
           } else {
             "not created".to_string()
           });
  println!("  user retry count:  {}",
           response.user_password_retry_count);
  println!("  admin retry count: {}",
           response.admin_password_retry_count);
  println!("  volumes:");
  println!("    unencrypted:     {}",
           if response.volume_active & nitrokey::VOLUME_ACTIVE_UNENCRYPTED == 0 {
             "inactive"
           } else if response.unencrypted_volume_read_only != 0 {
             "read-only"
           } else {
             "active"
           });
  println!("    encrypted:       {}",
           if response.volume_active & nitrokey::VOLUME_ACTIVE_ENCRYPTED == 0 {
             "inactive"
           } else if response.encrypted_volume_read_only != 0 {
             "read-only"
           } else {
             "active"
           });
  println!("    hidden:          {}",
           if response.volume_active & nitrokey::VOLUME_ACTIVE_HIDDEN == 0 {
             "inactive"
           } else if response.hidden_volume_read_only != 0 {
             "read-only"
           } else {
             "active"
           });
}


/// Inquire the status of the nitrokey.
fn status() -> Result<()> {
  type Response = nitrokey::Response<nitrokey::DeviceStatusResponse>;

  nitrokey_do(&|handle| {
    let payload = nitrokey::DeviceStatusCommand::new();
    let report = nitrokey::Report::from(payload);

    let report = transmit::<_, nitrokey::EmptyPayload>(handle, &report)?;
    let response = &AsRef::<Response>::as_ref(&report.data).data;

    // TODO: We should probably check the success of the command as
    //       well.
    if response.magic != nitrokey::MAGIC_NUMBER_STICK20_CONFIG {
      let error = format!("Status response contains invalid magic: {:#x} \
                           (expected: {:#x})",
                          response.magic,
                          nitrokey::MAGIC_NUMBER_STICK20_CONFIG);
      return Err(Error::Error(error.to_string()));
    }

    print_status(response);
    Ok(())
  })
}


/// Poll the nitrokey until it reports to no longer be busy.
fn wait(handle: &mut libhid::Handle) -> Result<nitrokey::StorageStatus> {
  type Response = nitrokey::Response<nitrokey::StorageResponse>;

  loop {
    thread::sleep(time::Duration::from_millis(SEND_RECV_DELAY_MS));

    let report = receive::<nitrokey::EmptyPayload>(handle)?;
    let response = AsRef::<Response>::as_ref(&report.data);
    let status = response.data.storage_status;

    if status != nitrokey::StorageStatus::Busy &&
       status != nitrokey::StorageStatus::BusyProgressbar {
      return Ok(status);
    }
  }
}


/// Open the encrypted volume on the nitrokey.
fn open() -> Result<()> {
  type Response = nitrokey::Response<nitrokey::StorageResponse>;

  nitrokey_do(&|handle| {
    let mut retry = 3;
    let mut error_msg: Option<&str> = None;
    loop {
      let passphrase = pinentry::inquire_passphrase(PIN_TYPE, error_msg)?;
      let payload = nitrokey::EnableEncryptedVolumeCommand::new(&passphrase);
      let report = nitrokey::Report::from(payload);

      let report = transmit::<_, nitrokey::EmptyPayload>(handle, &report)?;
      let response = AsRef::<Response>::as_ref(&report.data);
      let mut status = response.data.storage_status;

      if status == nitrokey::StorageStatus::WrongPassword {
        pinentry::clear_passphrase(PIN_TYPE)?;
        retry -= 1;

        if retry > 0 {
          error_msg = Some("Wrong password, please reenter");
          continue;
        }
        let error = "Opening encrypted volume failed: Wrong password";
        return Err(Error::Error(error.to_string()));
      }
      if status == nitrokey::StorageStatus::Busy ||
         status == nitrokey::StorageStatus::BusyProgressbar {
        status = wait(handle)?;
      }
      if status != nitrokey::StorageStatus::Okay && status != nitrokey::StorageStatus::Idle {
        let status = format!("{:?}", status);
        let error = format!("Opening encrypted volume failed: {}", status);
        return Err(Error::Error(error));
      }
      return Ok(());
    }
  })
}


#[link(name = "c")]
extern "C" {
  fn sync();
}

/// Close the previously opened encrypted volume.
fn close() -> Result<()> {
  type Response = nitrokey::Response<nitrokey::StorageResponse>;

  nitrokey_do(&|handle| {
    // Flush all filesystem caches to disk. We are mostly interested in
    // making sure that the encrypted volume on the nitrokey we are
    // about to close is not closed while not all data was written to
    // it.
    unsafe { sync() };

    let payload = nitrokey::DisableEncryptedVolumeCommand::new();
    let report = nitrokey::Report::from(payload);

    let report = transmit::<_, nitrokey::EmptyPayload>(handle, &report)?;
    let response = AsRef::<Response>::as_ref(&report.data);
    let mut status = response.data.storage_status;

    if status == nitrokey::StorageStatus::Busy ||
       status == nitrokey::StorageStatus::BusyProgressbar {
      status = wait(handle)?;
    }
    if status != nitrokey::StorageStatus::Okay && status != nitrokey::StorageStatus::Idle {
      let status = format!("{:?}", status);
      let error = format!("Closing encrypted volume failed: {}", status);
      return Err(Error::Error(error));
    }
    Ok(())
  })
}


/// Clear the PIN stored when opening the nitrokey's encrypted volume.
fn clear() -> Result<()> {
  pinentry::clear_passphrase(PIN_TYPE)
}


// A macro for generating a match of the different supported commands.
// Each supplied command is converted into a string and matched against.
macro_rules! commands {
  ( $str:expr, [ $( $command:expr), *] ) => {
    match &*$str.to_string() {
      $(
        stringify!($command) => {
          if let Err(err) = $command() {
            println!("{}", err);
            return 1
          }
          return 0
        },
      )*
      x => {
        println!("Invalid command: {}", x);
        println!("Available commands: {}", stringify!( $($command)* ));
        return 1
      },
    }
  }
}

fn run() -> i32 {
  let argv: Vec<String> = std::env::args().collect();
  if argv.len() != 2 {
    println!("Usage: {} <command>", argv[0]);
    return 1;
  }

  commands!(&argv[1], [status, open, close, clear]);
}

fn main() {
  process::exit(run());
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wrong_crc() {
    type Response = nitrokey::Response<nitrokey::DeviceStatusResponse>;

    nitrokey_do(&|handle| {
      let payload = nitrokey::DeviceStatusCommand::new();
      let mut report = nitrokey::Report::from(payload);

      // We want to verify that we get the correct result (i.e., a
      // report of the CRC mismatch) repeatedly.
      for _ in 0..10 {
        report.crc += 1;

        let new_report = transmit::<_, nitrokey::EmptyPayload>(handle, &report)?;
        let response = AsRef::<Response>::as_ref(&new_report.data);

        assert_eq!(response.command, nitrokey::Command::GetDeviceStatus);
        assert_eq!(response.command_crc, report.crc);
        assert_eq!(response.command_status, nitrokey::CommandStatus::WrongCrc);
      }
      return Ok(());
    })
      .unwrap();
  }

  #[test]
  fn device_status() {
    type Response = nitrokey::Response<nitrokey::DeviceStatusResponse>;

    nitrokey_do(&|handle| {
      let payload = nitrokey::DeviceStatusCommand::new();
      let report = nitrokey::Report::from(payload);

      let report = transmit::<_, nitrokey::EmptyPayload>(handle, &report)?;
      let response = AsRef::<Response>::as_ref(&report.data);

      assert!(response.device_status == nitrokey::StorageStatus::Idle ||
              response.device_status == nitrokey::StorageStatus::Okay);
      assert_eq!(response.data.magic, nitrokey::MAGIC_NUMBER_STICK20_CONFIG);
      return Ok(());
    })
      .unwrap();
  }
}
