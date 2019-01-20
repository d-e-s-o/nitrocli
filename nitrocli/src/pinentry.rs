// pinentry.rs

// *************************************************************************
// * Copyright (C) 2017-2019 Daniel Mueller (deso@posteo.net)              *
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

use std::process;
use std::str;

use crate::error::Error;

/// PIN type requested from pinentry.
///
/// The available PIN types correspond to the PIN types used by the Nitrokey devices:  user and
/// admin.
Enum! {PinType, [
  Admin => "admin",
  User => "user"
]}

#[derive(Debug)]
pub struct PinEntry {
  pin_type: PinType,
  model: nitrokey::Model,
  serial: String,
}

impl PinEntry {
  pub fn from<D>(pin_type: PinType, device: &D) -> crate::Result<Self>
  where
    D: nitrokey::Device,
  {
    let model = device.get_model();
    let serial = device.get_serial_number()?;
    Ok(Self {
      pin_type,
      model,
      serial,
    })
  }

  fn cache_id(&self) -> String {
    let model = self.model.to_string().to_lowercase();
    let suffix = format!("{}:{}", model, self.serial);

    match self.pin_type {
      PinType::Admin => format!("nitrocli:admin:{}", suffix),
      PinType::User => format!("nitrocli:user:{}", suffix),
    }
  }

  fn prompt(&self) -> &'static str {
    match self.pin_type {
      PinType::Admin => "Admin PIN",
      PinType::User => "User PIN",
    }
  }

  fn description(&self, mode: Mode) -> String {
    format!(
      "{} for\rNitrokey {} {}",
      match self.pin_type {
        PinType::Admin => match mode {
          Mode::Choose => "Please enter a new admin PIN",
          Mode::Confirm => "Please confirm the new admin PIN",
          Mode::Query => "Please enter the admin PIN",
        },
        PinType::User => match mode {
          Mode::Choose => "Please enter a new user PIN",
          Mode::Confirm => "Please confirm the new user PIN",
          Mode::Query => "Please enter the user PIN",
        },
      },
      self.model,
      self.serial,
    )
  }

  pub fn pin_type(&self) -> PinType {
    self.pin_type
  }
}

/// PIN entry mode for pinentry.
///
/// This enum describes the context of the pinentry query, for example
/// prompting for the current PIN or requesting a new PIN.  The mode may
/// affect the pinentry description and whether a quality bar is shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
  /// Let the user choose a new PIN.
  Choose,
  /// Let the user confirm the previously chosen PIN.
  Confirm,
  /// Query an existing PIN.
  Query,
}

impl Mode {
  fn show_quality_bar(self) -> bool {
    self == Mode::Choose
  }
}

fn parse_pinentry_pin<R>(response: R) -> Result<String, Error>
where
  R: AsRef<str>,
{
  let string = response.as_ref();
  let lines: Vec<&str> = string.lines().collect();

  // We expect the response to be of the form:
  // > D passphrase
  // > OK
  // or potentially:
  // > ERR 83886179 Operation cancelled <Pinentry>
  if lines.len() == 2 && lines[1] == "OK" && lines[0].starts_with("D ") {
    // We got the only valid answer we accept.
    let (_, pass) = lines[0].split_at(2);
    return Ok(pass.to_string());
  }

  // Check if we are dealing with a special "ERR " line and report that
  // specially.
  if !lines.is_empty() && lines[0].starts_with("ERR ") {
    let (_, error) = lines[0].split_at(4);
    return Err(Error::from(error));
  }
  Err(Error::Error(format!("Unexpected response: {}", string)))
}

/// Inquire a PIN of the given type from the user.
///
/// This function inquires a PIN of the given type from the user or
/// returns the cached pin, if available.  If an error message is set,
/// it is displayed in the pin dialog.  The mode describes the context
/// of the pinentry dialog.  It is used to choose an appropriate
/// description and to decide whether a quality bar is shown in the
/// dialog.
pub fn inquire_pin(
  pin_entry: &PinEntry,
  mode: Mode,
  error_msg: Option<&str>,
) -> Result<String, Error> {
  let cache_id = pin_entry.cache_id();
  let error_msg = error_msg
    .map(|msg| msg.replace(" ", "+"))
    .unwrap_or_else(|| String::from("+"));
  let prompt = pin_entry.prompt().replace(" ", "+");
  let description = pin_entry.description(mode).replace(" ", "+");

  let args = vec![cache_id, error_msg, prompt, description].join(" ");
  let mut command = "GET_PASSPHRASE --data ".to_string();
  if mode.show_quality_bar() {
    command += "--qualitybar ";
  }
  command += &args;
  // We could also use the --data parameter here to have a more direct
  // representation of the pin but the resulting response was
  // considered more difficult to parse overall. It appears an error
  // reported for the GET_PASSPHRASE command does not actually cause
  // gpg-connect-agent to exit with a non-zero error code, we have to
  // evaluate the output to determine success/failure.
  let output = process::Command::new("gpg-connect-agent")
    .arg(command)
    .arg("/bye")
    .output()?;
  parse_pinentry_pin(str::from_utf8(&output.stdout)?)
}

fn check_pin(pin_type: PinType, pin: &str) -> crate::Result<()> {
  let minimum_length = match pin_type {
    PinType::Admin => 8,
    PinType::User => 6,
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

pub fn choose_pin(pin_entry: &PinEntry) -> crate::Result<String> {
  clear_pin(pin_entry)?;
  let new_pin = inquire_pin(pin_entry, Mode::Choose, None)?;
  clear_pin(pin_entry)?;
  check_pin(pin_entry.pin_type(), &new_pin)?;

  let confirm_pin = inquire_pin(pin_entry, Mode::Confirm, None)?;
  clear_pin(pin_entry)?;

  if new_pin != confirm_pin {
    Err(Error::from("Entered PINs do not match"))
  } else {
    Ok(new_pin)
  }
}

fn parse_pinentry_response<R>(response: R) -> Result<(), Error>
where
  R: AsRef<str>,
{
  let string = response.as_ref();
  let lines = string.lines().collect::<Vec<_>>();

  if lines.len() == 1 && lines[0] == "OK" {
    // We got the only valid answer we accept.
    return Ok(());
  }
  Err(Error::Error(format!("Unexpected response: {}", string)))
}

/// Clear the cached pin represented by the given entry.
pub fn clear_pin(pin_entry: &PinEntry) -> Result<(), Error> {
  let command = format!("CLEAR_PASSPHRASE {}", pin_entry.cache_id());
  let output = process::Command::new("gpg-connect-agent")
    .arg(command)
    .arg("/bye")
    .output()?;

  parse_pinentry_response(str::from_utf8(&output.stdout)?)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_pinentry_pin_good() {
    let response = "D passphrase\nOK\n";
    let expected = "passphrase";

    assert_eq!(parse_pinentry_pin(response).unwrap(), expected)
  }

  #[test]
  fn parse_pinentry_pin_error() {
    let error = "83886179 Operation cancelled";
    let response = "ERR ".to_string() + error + "\n";
    let expected = error;

    let error = parse_pinentry_pin(response.to_string());

    if let Error::Error(ref e) = error.err().unwrap() {
      assert_eq!(e, &expected);
    } else {
      panic!("Unexpected result");
    }
  }

  #[test]
  fn parse_pinentry_pin_unexpected() {
    let response = "foobar\n";
    let expected = format!("Unexpected response: {}", response);
    let error = parse_pinentry_pin(response);

    if let Error::Error(ref e) = error.err().unwrap() {
      assert_eq!(e, &expected);
    } else {
      panic!("Unexpected result");
    }
  }

  #[test]
  fn parse_pinentry_response_ok() {
    assert!(parse_pinentry_response("OK\n").is_ok())
  }

  #[test]
  fn parse_pinentry_response_ok_no_newline() {
    assert!(parse_pinentry_response("OK").is_ok())
  }

  #[test]
  fn parse_pinentry_response_unexpected() {
    let response = "ERR 42";
    let expected = format!("Unexpected response: {}", response);
    let error = parse_pinentry_response(response);

    if let Error::Error(ref e) = error.err().unwrap() {
      assert_eq!(e, &expected);
    } else {
      panic!("Unexpected result");
    }
  }
}
