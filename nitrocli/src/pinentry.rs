// pinentry.rs

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

use error::Error;
use std::process;


const CACHE_ID: &str = "nitrocli:user";


fn parse_pinentry_passphrase(response: Vec<u8>) -> Result<Vec<u8>, Error> {
  let string = String::from_utf8(response)?;
  let lines: Vec<&str> = string.lines().collect();

  // We expect the response to be of the form:
  // > D passphrase
  // > OK
  // or potentially:
  // > ERR 83886179 Operation cancelled <Pinentry>
  if lines.len() == 2 && lines[1] == "OK" && lines[0].starts_with("D ") {
    // We got the only valid answer we accept.
    let (_, pass) = lines[0].split_at(2);
    return Ok(pass.to_string().into_bytes());
  }

  // Check if we are dealing with a special "ERR " line and report that
  // specially.
  if lines.len() >= 1 && lines[0].starts_with("ERR ") {
    let (_, error) = lines[0].split_at(4);
    return Err(Error::Error(error.to_string()));
  }
  Err(Error::Error("Unexpected response: ".to_string() + &string))
}


pub fn inquire_passphrase() -> Result<Vec<u8>, Error> {
  const PINENTRY_ERROR_MSG: &str = "+";
  const PINENTRY_PROMPT: &str = "PIN";
  const PINENTRY_DESCR: &str = "Please+enter+user+PIN";

  let args = vec![CACHE_ID, PINENTRY_ERROR_MSG, PINENTRY_PROMPT, PINENTRY_DESCR].join(" ");
  let command = "GET_PASSPHRASE --data ".to_string() + &args;
  // We could also use the --data parameter here to have a more direct
  // representation of the passphrase but the resulting response was
  // considered more difficult to parse overall. It appears an error
  // reported for the GET_PASSPHRASE command does not actually cause
  // gpg-connect-agent to exit with a non-zero error code, we have to
  // evaluate the output to determine success/failure.
  let output = process::Command::new("gpg-connect-agent").arg(command)
    .arg("/bye")
    .output()?;
  parse_pinentry_passphrase(output.stdout)
}


fn parse_pinentry_response(response: Vec<u8>) -> Result<(), Error> {
  let string = String::from_utf8(response)?;
  let lines: Vec<&str> = string.lines().collect();

  if lines.len() == 1 && lines[0] == "OK" {
    // We got the only valid answer we accept.
    return Ok(());
  }
  Err(Error::Error("Unexpected response: ".to_string() + &string))
}


/// Clear the cached passphrase.
pub fn clear_passphrase() -> Result<(), Error> {
  let command = "CLEAR_PASSPHRASE ".to_string() + CACHE_ID;
  let output = process::Command::new("gpg-connect-agent").arg(command)
    .arg("/bye")
    .output()?;

  parse_pinentry_response(output.stdout)
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_pinentry_passphrase_good() {
    let response = "D passphrase\nOK\n".to_string().into_bytes();
    let expected = "passphrase".to_string().into_bytes();

    assert_eq!(parse_pinentry_passphrase(response).unwrap(), expected)
  }

  #[test]
  fn parse_pinentry_passphrase_error() {
    let error = "83886179 Operation cancelled";
    let response = "ERR ".to_string() + error + "\n";
    let expected = error;

    let error = parse_pinentry_passphrase(response.to_string().into_bytes());

    if let Error::Error(ref e) = error.err().unwrap() {
      assert_eq!(e, &expected);
    } else {
      panic!("Unexpected result");
    }
  }

  #[test]
  fn parse_pinentry_passphrase_unexpected() {
    let response = "foobar\n";
    let expected = "Unexpected response: ".to_string() + response;

    let error = parse_pinentry_passphrase(response.to_string().into_bytes());

    if let Error::Error(ref e) = error.err().unwrap() {
      assert_eq!(e, &expected);
    } else {
      panic!("Unexpected result");
    }
  }

  #[test]
  fn parse_pinentry_response_ok() {
    let response = "OK\n".to_string().into_bytes();
    assert!(parse_pinentry_response(response).is_ok())
  }

  #[test]
  fn parse_pinentry_response_ok_no_newline() {
    let response = "OK".to_string().into_bytes();
    assert!(parse_pinentry_response(response).is_ok())
  }

  #[test]
  fn parse_pinentry_response_unexpected() {
    let response = "ERR 42";
    let expected = "Unexpected response: ".to_string() + response;

    let error = parse_pinentry_response(response.to_string().into_bytes());

    if let Error::Error(ref e) = error.err().unwrap() {
      assert_eq!(e, &expected);
    } else {
      panic!("Unexpected result");
    }
  }
}
