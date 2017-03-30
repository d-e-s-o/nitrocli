// nitrokey.rs

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

use crc32::crc;
use std::cmp;
use std::mem;


// The Nitrokey Storage vendor ID.
pub const VID: u16 = 0x20A0;
// The Nitrokey Storage product ID.
pub const PID: u16 = 0x4109;


#[derive(Debug)]
#[derive(PartialEq)]
#[repr(u8)]
pub enum Command {
  // The command to enable the encrypted volume.
  EnableEncryptedVolume = 0x20,
  // The command to disable the encrypted volume.
  DisableEncryptedVolume = 0x21,
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


impl<P> AsRef<[u8]> for Report<P>
  where P: AsRef<[u8]>,
{
  fn as_ref(&self) -> &[u8] {
    unsafe { return mem::transmute::<&Report<P>, &[u8; 64]>(self) };
  }
}


impl<P> From<P> for Report<P>
  where P: AsRef<[u8]>,
{
  fn from(payload: P) -> Report<P> {
    let crc = crc(payload.as_ref());
    return Report {
      data: payload,
      crc: crc,
    };
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
        return $name{
          command: Command::$command,
          padding: [0; 59],
        };
      }
    }
  }
}

macro_rules! defaultPayloadAsRef {
  ( $name:ty ) => {
    impl AsRef<[u8]> for $name {
      fn as_ref(&self) -> &[u8] {
        unsafe {
          return mem::transmute::<&$name, &[u8; 60]>(self)
        };
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


#[allow(dead_code)]
#[repr(packed)]
pub struct EnableEncryptedVolumeCommand {
  command: Command,
  // The kind of password. Unconditionally 'P' because the User PIN is
  // used to enable the encrypted volume.
  kind: u8,
  // The password has a maximum length of twenty characters.
  password: [u8; 20],
  padding: [u8; 38],
}


impl EnableEncryptedVolumeCommand {
  pub fn new(password: &Vec<u8>) -> EnableEncryptedVolumeCommand {
    let mut report = EnableEncryptedVolumeCommand {
      command: Command::EnableEncryptedVolume,
      kind: 'P' as u8,
      password: [0; 20],
      padding: [0; 38],
    };

    debug_assert!(password.len() <= report.password.len());

    let len = cmp::min(report.password.len(), password.len());
    report.password[..len].copy_from_slice(&password[..len]);
    return report;
  }
}

defaultPayloadAsRef!(EnableEncryptedVolumeCommand);

defaultCommand!(DisableEncryptedVolumeCommand, DisableEncryptedVolume);


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
