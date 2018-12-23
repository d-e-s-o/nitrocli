// args.rs

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

use std::fmt;
use std::io;
use std::result;
use std::str;

use crate::commands;
use crate::error::Error;

type Result<T> = result::Result<T, Error>;

/// A top-level command for nitrocli.
#[derive(Debug)]
pub enum Command {
  Clear,
  Close,
  Open,
  Otp,
  Status,
}

impl Command {
  /// Execute this command with the given arguments.
  pub fn execute(&self, args: Vec<String>) -> Result<()> {
    match *self {
      Command::Clear => clear(args),
      Command::Close => close(args),
      Command::Open => open(args),
      Command::Otp => otp(args),
      Command::Status => status(args),
    }
  }
}

impl fmt::Display for Command {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        Command::Clear => "clear",
        Command::Close => "close",
        Command::Open => "open",
        Command::Otp => "otp",
        Command::Status => "status",
      }
    )
  }
}

impl str::FromStr for Command {
  type Err = ();

  fn from_str(s: &str) -> result::Result<Self, Self::Err> {
    match s {
      "clear" => Ok(Command::Clear),
      "close" => Ok(Command::Close),
      "open" => Ok(Command::Open),
      "otp" => Ok(Command::Otp),
      "status" => Ok(Command::Status),
      _ => Err(()),
    }
  }
}

#[derive(Debug)]
enum OtpCommand {
  Get,
}

impl OtpCommand {
  fn execute(&self, _args: Vec<String>) -> Result<()> {
    match *self {
      OtpCommand::Get => Err(Error::Error("Not implementend".to_string())),
    }
  }
}

impl fmt::Display for OtpCommand {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        OtpCommand::Get => "get",
      }
    )
  }
}

impl str::FromStr for OtpCommand {
  type Err = ();

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    match s {
      "get" => Ok(OtpCommand::Get),
      _ => Err(()),
    }
  }
}

fn parse(parser: &argparse::ArgumentParser<'_>, args: Vec<String>) -> Result<()> {
  if let Err(err) = parser.parse(args, &mut io::stdout(), &mut io::stderr()) {
    Err(Error::ArgparseError(err))
  } else {
    Ok(())
  }
}

/// Inquire the status of the nitrokey.
fn status(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Print the status of the connected Nitrokey Storage");
  parse(&parser, args)?;

  commands::status()
}

/// Open the encrypted volume on the nitrokey.
fn open(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Opens the encrypted volume on a Nitrokey Storage");
  parse(&parser, args)?;

  commands::open()
}

/// Close the previously opened encrypted volume.
fn close(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Closes the encrypted volume on a Nitrokey Storage");
  parse(&parser, args)?;

  commands::close()
}

/// Clear the PIN stored when opening the nitrokey's encrypted volume.
fn clear(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Clears the cached passphrases");
  parse(&parser, args)?;

  commands::clear()
}

/// Execute an OTP subcommand.
fn otp(args: Vec<String>) -> Result<()> {
  let mut subcommand = OtpCommand::Get;
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Accesses one-time passwords");
  let _ = parser.refer(&mut subcommand).required().add_argument(
    "subcommand",
    argparse::Store,
    "the subcommand to execute (get)",
  );
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "the arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(&parser, args)?;
  drop(parser);

  subargs.insert(0, format!("nitrocli otp {}", subcommand));
  subcommand.execute(subargs)
}

/// Parse the command-line arguments and return the selected command and
/// the remaining arguments for the command.
fn parse_arguments(args: Vec<String>) -> Result<(Command, Vec<String>)> {
  let mut command = Command::Status;
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Provides access to a Nitrokey device");
  let _ = parser.refer(&mut command).required().add_argument(
    "command",
    argparse::Store,
    "the command to execute (clear|close|open|otp|status)",
  );
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the command",
  );
  parser.stop_on_first_argument(true);
  parse(&parser, args)?;
  drop(parser);

  subargs.insert(0, format!("nitrocli {}", command));
  Ok((command, subargs))
}

/// Parse the command-line arguments and execute the selected command.
pub fn handle_arguments(args: Vec<String>) -> Result<()> {
  let (command, args) = parse_arguments(args)?;
  command.execute(args)
}
