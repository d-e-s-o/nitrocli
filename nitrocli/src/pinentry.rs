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

use std::fmt;
use std::process;
use std::str;

use crate::error::Error;

/// PIN type requested from pinentry.
///
/// The available PIN types correspond to the PIN types used by the Nitrokey devices:  user and
/// admin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PinType {
  /// The admin PIN.
  Admin,
  /// The user PIN.
  User,
}

impl fmt::Display for PinType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        PinType::Admin => "admin",
        PinType::User => "user",
      }
    )
  }
}

impl str::FromStr for PinType {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "admin" => Ok(PinType::Admin),
      "user" => Ok(PinType::User),
      _ => Err(()),
    }
  }
}

impl PinType {
  fn cache_id(self) -> &'static str {
    match self {
      PinType::Admin => "nitrocli:admin",
      PinType::User => "nitrocli:user",
    }
  }

  fn prompt(self) -> &'static str {
    match self {
      PinType::Admin => "Admin PIN",
      PinType::User => "User PIN",
    }
  }

  fn description(self, mode: Mode) -> &'static str {
    match self {
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
    }
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
  pin_type: PinType,
  mode: Mode,
  error_msg: Option<&str>,
) -> Result<String, Error> {
  let cache_id = pin_type.cache_id();
  let error_msg = error_msg
    .map(|msg| msg.replace(" ", "+"))
    .unwrap_or_else(|| String::from("+"));
  let prompt = pin_type.prompt().replace(" ", "+");
  let description = pin_type.description(mode).replace(" ", "+");

  let args = vec![cache_id, &error_msg, &prompt, &description].join(" ");
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

/// Clear the cached pin of the given type.
pub fn clear_pin(pin_type: PinType) -> Result<(), Error> {
  let command = "CLEAR_PASSPHRASE ".to_string() + pin_type.cache_id();
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
