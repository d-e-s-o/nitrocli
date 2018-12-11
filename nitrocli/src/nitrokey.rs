// nitrokey.rs

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

use std::mem;

use crate::crc32::crc;


// The Nitrokey Storage vendor ID.
pub const VID: u16 = 0x20A0;
// The Nitrokey Storage product ID.
pub const PID: u16 = 0x4109;

// Magic number identifying a storage response.
pub const MAGIC_NUMBER_STICK20_CONFIG: u16 = 0x3318;

// Flags indicating whether the respective volume is active or not.
pub const VOLUME_ACTIVE_UNENCRYPTED: u8 = 0b001;
pub const VOLUME_ACTIVE_ENCRYPTED: u8 = 0b010;
pub const VOLUME_ACTIVE_HIDDEN: u8 = 0b100;


#[derive(Debug)]
#[derive(PartialEq)]
#[repr(u8)]
pub enum Command {
  // Retrieve the device status.
  GetDeviceStatus = 0x2E,
}


/// A report is the entity we send to the Nitrokey Storage HID.
///
/// A report is always 64 bytes in size. The last four bytes comprise a
/// CRC of the actual payload. Note that when sending or receiving a
/// report it usually is preceded by a one byte report ID. This report
/// ID is zero here and not represented in the actual report object in
/// our design.
#[repr(packed)]
pub struct Report<Payload>
  where Payload: AsRef<[u8]>,
{
  // The actual payload data. A report may encapsulate a command to send
  // to the stick or a response to receive from it.
  pub data: Payload,
  pub crc: u32,
}


impl<P> Report<P>
  where P: AsRef<[u8]> + Default,
{
  pub fn new() -> Report<P> {
    Report {
      data: P::default(),
      crc: 0,
    }
  }

  pub fn is_valid(&self) -> bool {
    self.crc == crc(self.data.as_ref())
  }
}


impl<P> AsRef<[u8]> for Report<P>
  where P: AsRef<[u8]>,
{
  fn as_ref(&self) -> &[u8] {
    unsafe { mem::transmute::<&Report<P>, &[u8; 64]>(self) }
  }
}


impl<P> From<P> for Report<P>
  where P: AsRef<[u8]>,
{
  fn from(payload: P) -> Report<P> {
    let crc = crc(payload.as_ref());
    Report {
      data: payload,
      crc: crc,
    }
  }
}


impl<P> AsMut<[u8]> for Report<P>
  where P: AsRef<[u8]>,
{
  fn as_mut(&mut self) -> &mut [u8] {
    unsafe { mem::transmute::<&mut Report<P>, &mut [u8; 64]>(self) }
  }
}


pub struct EmptyPayload {
  pub data: [u8; 60],
}

impl Default for EmptyPayload {
  fn default() -> EmptyPayload {
    EmptyPayload {
      data: [0u8; 60],
    }
  }
}

impl AsRef<[u8]> for EmptyPayload {
  fn as_ref(&self) -> &[u8] {
    unsafe { mem::transmute::<&EmptyPayload, &[u8; 60]>(self) }
  }
}

impl<P> AsRef<Response<P>> for EmptyPayload {
  fn as_ref(&self) -> &Response<P> {
    unsafe { mem::transmute::<&EmptyPayload, &Response<P>>(self) }
  }
}


macro_rules! defaultCommandType {
  ( $name:ident ) => {
    #[allow(dead_code)]
    #[repr(packed)]
    pub struct $name {
      command: Command,
      padding: [u8; 59],
    }
  }
}

macro_rules! defaultCommandNew {
  ( $name:ident, $command:ident ) => {
    impl $name {
      pub fn new() -> $name {
        $name{
          command: Command::$command,
          padding: [0; 59],
        }
      }
    }
  }
}

macro_rules! defaultPayloadAsRef {
  ( $name:ty ) => {
    impl AsRef<[u8]> for $name {
      fn as_ref(&self) -> &[u8] {
        unsafe { mem::transmute::<&$name, &[u8; 60]>(self) }
      }
    }
  }
}

macro_rules! defaultCommand {
  ( $name:ident, $command:ident ) => {
    defaultCommandType!($name);
    defaultCommandNew!($name, $command);
    defaultPayloadAsRef!($name);
  }
}


defaultCommand!(DeviceStatusCommand, GetDeviceStatus);


#[allow(dead_code)]
#[derive(Debug)]
#[derive(PartialEq)]
#[repr(u8)]
pub enum CommandStatus {
  Okay = 0,
  WrongCrc = 1,
  WrongSlot = 2,
  SlotNotProgrammed = 3,
  WrongPassword = 4,
  NotAuthorized = 5,
  TimestampWarning = 6,
  NoNameError = 7,
}


#[allow(dead_code)]
#[derive(Copy)]
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
#[repr(u8)]
pub enum StorageStatus {
  Idle = 0,
  Okay = 1,
  Busy = 2,
  WrongPassword = 3,
  BusyProgressbar = 4,
  PasswordMatrixReady = 5,
  NoUserPasswordUnlock = 6,
  SmartcardError = 7,
  SecurityBitActive = 8,
}


#[repr(packed)]
pub struct Response<Payload> {
  pub device_status: StorageStatus,
  pub command: Command,
  pub command_crc: u32,
  pub command_status: CommandStatus,
  pub data: Payload,
}

impl<P> AsRef<[u8]> for Response<P> {
  fn as_ref(&self) -> &[u8] {
    unsafe { mem::transmute::<&Response<P>, &[u8; 60]>(self) }
  }
}


#[repr(packed)]
pub struct DeviceStatusResponse {
  pub padding0: [u8; 22],
  pub magic: u16,
  pub unencrypted_volume_read_only: u8,
  pub encrypted_volume_read_only: u8,
  pub version_major: u8,
  pub version_minor: u8,
  pub version_build: u8,
  pub version_internal: u8,
  pub hidden_volume_read_only: u8,
  pub firmware_locked: u8,
  pub new_sdcard_found: u8,
  pub sdcard_fill_with_random: u8,
  pub active_sdcard_id: u32,
  pub volume_active: u8,
  pub new_smartcard_found: u8,
  pub user_password_retry_count: u8,
  pub admin_password_retry_count: u8,
  pub active_smartcard_id: u32,
  pub storage_keys_missing: u8,
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn encrypted_volume_report() {
    let password = "test42".to_string().into_bytes();
    let report = EnableEncryptedVolumeCommand::new(&password);
    let expected = ['t' as u8, 'e' as u8, 's' as u8, 't' as u8, '4' as u8, '2' as u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
    assert_eq!(report.password, expected);
  }

  #[test]
  #[cfg(debug)]
  #[should_panic(expected = "assertion failed")]
  fn overly_long_password() {
    let password = "012345678912345678901".to_string().into_bytes();
    EnableEncryptedVolumeCommand::new(&password);
  }

  #[test]
  fn report_crc() {
    let password = "passphrase".to_string().into_bytes();
    let payload = EnableEncryptedVolumeCommand::new(&password);
    let report = Report::from(payload);

    // The expected checksum was computed using the original
    // functionality.
    assert_eq!(report.crc, 0xeeb583c);
  }
}
