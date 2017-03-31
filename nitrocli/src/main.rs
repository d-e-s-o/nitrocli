// main.rs

// *************************************************************************
// * Copyright (C) 2017 Daniel Mueller (deso@posteo.net)                   *
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
use std::process;
use std::result;
use std::thread;
use std::time;

type Result<T> = result::Result<T, Error>;
type NitroFunc = Fn(&mut libhid::Handle) -> Result<()>;


const SEND_RECV_DELAY_MS: u64 = 200;


/// Send a HID feature report to the device represented by the given handle.
fn send<P>(handle: &mut libhid::Handle, report: &nitrokey::Report<P>) -> Result<()>
  where P: AsRef<[u8]>,
{
  handle.feature().send_to(0, report.as_ref())?;
  return Ok(());
}


/// Receive a HID feature report from the device represented by the given handle.
fn receive<P>(handle: &mut libhid::Handle) -> Result<nitrokey::Report<P>>
  where P: AsRef<[u8]> + Default,
{
  let mut report = nitrokey::Report::<P>::new();
  handle.feature().get_from(0, report.as_mut())?;
  return Ok(report);
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
  return Err(Error::Error("Nitrokey device not found".to_string()));
}


/// Open the encrypted volume on the nitrokey.
fn open() -> Result<()> {
  return nitrokey_do(&|handle| {
    let passphrase = pinentry::inquire_passphrase()?;
    let payload = nitrokey::EnableEncryptedVolumeCommand::new(&passphrase);
    let report = nitrokey::Report::from(payload);

    send(handle, &report)?;
    // We need to give the stick some time to handle the command. If we
    // don't, we might just receive stale data from before.
    thread::sleep(time::Duration::from_millis(SEND_RECV_DELAY_MS));

    receive::<nitrokey::EmptyPayload>(handle)?;
    return Ok(());
  });
}


/// Close the previously opened encrypted volume.
fn close() -> Result<()> {
  return nitrokey_do(&|handle| {
    let payload = nitrokey::DisableEncryptedVolumeCommand::new();
    let report = nitrokey::Report::from(payload);

    send(handle, &report)?;
    return Ok(());
  });
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

  commands!(&argv[1], [open, close]);
}

fn main() {
  process::exit(run());
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wrong_crc() {
    nitrokey_do(&|handle| {
      let payload = nitrokey::DeviceStatusCommand::new();
      let mut report = nitrokey::Report::from(payload);

      // We want to verify that we get the correct result (i.e., a
      // report of the CRC mismatch) repeatedly.
      for _ in 0..10 {
        report.crc += 1;
        send(handle, &report).unwrap();
        thread::sleep(time::Duration::from_millis(SEND_RECV_DELAY_MS));

        let new_report = receive::<nitrokey::EmptyPayload>(handle).unwrap();
        assert!(new_report.is_valid());

        let response: &nitrokey::Response<nitrokey::DeviceStatusResponse> = new_report.data
                                                                                      .as_ref();
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
    nitrokey_do(&|handle| {
      let payload = nitrokey::DeviceStatusCommand::new();
      let report = nitrokey::Report::from(payload);

      send(handle, &report).unwrap();
      thread::sleep(time::Duration::from_millis(SEND_RECV_DELAY_MS));

      let new_report = receive::<nitrokey::EmptyPayload>(handle).unwrap();
      assert!(new_report.is_valid());

      let response: &nitrokey::Response<nitrokey::DeviceStatusResponse> = new_report.data.as_ref();

      assert!(response.device_status == nitrokey::StorageStatus::Idle ||
              response.device_status == nitrokey::StorageStatus::Okay);
      assert_eq!(response.data.magic, nitrokey::MAGIC_NUMBER_STICK20_CONFIG);
      return Ok(());
    })
      .unwrap();
  }
}
