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
  Config,
  Otp,
  Status,
  Storage,
}

impl Command {
  /// Execute this command with the given arguments.
  pub fn execute(&self, args: Vec<String>) -> Result<()> {
    match *self {
      Command::Clear => clear(args),
      Command::Config => config(args),
      Command::Otp => otp(args),
      Command::Status => status(args),
      Command::Storage => storage(args),
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
        Command::Config => "config",
        Command::Otp => "otp",
        Command::Status => "status",
        Command::Storage => "storage",
      }
    )
  }
}

impl str::FromStr for Command {
  type Err = ();

  fn from_str(s: &str) -> result::Result<Self, Self::Err> {
    match s {
      "clear" => Ok(Command::Clear),
      "config" => Ok(Command::Config),
      "otp" => Ok(Command::Otp),
      "status" => Ok(Command::Status),
      "storage" => Ok(Command::Storage),
      _ => Err(()),
    }
  }
}

#[derive(Debug)]
enum ConfigCommand {
  Get,
  Set,
}

impl ConfigCommand {
  fn execute(&self, args: Vec<String>) -> Result<()> {
    match *self {
      ConfigCommand::Get => config_get(args),
      ConfigCommand::Set => config_set(args),
    }
  }
}

impl fmt::Display for ConfigCommand {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        ConfigCommand::Get => "get",
        ConfigCommand::Set => "set",
      }
    )
  }
}

impl str::FromStr for ConfigCommand {
  type Err = ();

  fn from_str(s: &str) -> result::Result<Self, Self::Err> {
    match s {
      "get" => Ok(ConfigCommand::Get),
      "set" => Ok(ConfigCommand::Set),
      _ => Err(()),
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub enum ConfigOption<T> {
  Enable(T),
  Disable,
  Ignore,
}

impl<T> ConfigOption<T> {
  fn try_from(disable: bool, value: Option<T>, name: &'static str) -> Result<Self> {
    if disable {
      if value.is_some() {
        Err(Error::Error(format!(
          "--{name} and --no-{name} are mutually exclusive",
          name = name
        )))
      } else {
        Ok(ConfigOption::Disable)
      }
    } else {
      match value {
        Some(value) => Ok(ConfigOption::Enable(value)),
        None => Ok(ConfigOption::Ignore),
      }
    }
  }

  pub fn or(self, default: Option<T>) -> Option<T> {
    match self {
      ConfigOption::Enable(value) => Some(value),
      ConfigOption::Disable => None,
      ConfigOption::Ignore => default,
    }
  }
}

#[derive(Debug)]
enum OtpCommand {
  Clear,
  Get,
  Set,
  Status,
}

impl OtpCommand {
  fn execute(&self, args: Vec<String>) -> Result<()> {
    match *self {
      OtpCommand::Clear => otp_clear(args),
      OtpCommand::Get => otp_get(args),
      OtpCommand::Set => otp_set(args),
      OtpCommand::Status => otp_status(args),
    }
  }
}

impl fmt::Display for OtpCommand {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        OtpCommand::Clear => "clear",
        OtpCommand::Get => "get",
        OtpCommand::Set => "set",
        OtpCommand::Status => "status",
      }
    )
  }
}

impl str::FromStr for OtpCommand {
  type Err = ();

  fn from_str(s: &str) -> result::Result<Self, Self::Err> {
    match s {
      "clear" => Ok(OtpCommand::Clear),
      "get" => Ok(OtpCommand::Get),
      "set" => Ok(OtpCommand::Set),
      "status" => Ok(OtpCommand::Status),
      _ => Err(()),
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub enum OtpAlgorithm {
  Hotp,
  Totp,
}

impl fmt::Display for OtpAlgorithm {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        OtpAlgorithm::Hotp => "hotp",
        OtpAlgorithm::Totp => "totp",
      }
    )
  }
}

impl str::FromStr for OtpAlgorithm {
  type Err = ();

  fn from_str(s: &str) -> result::Result<Self, Self::Err> {
    match s {
      "hotp" => Ok(OtpAlgorithm::Hotp),
      "totp" => Ok(OtpAlgorithm::Totp),
      _ => Err(()),
    }
  }
}

#[derive(Clone, Copy, Debug)]
enum OtpMode {
  SixDigits,
  EightDigits,
}

impl fmt::Display for OtpMode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        OtpMode::SixDigits => "6",
        OtpMode::EightDigits => "8",
      }
    )
  }
}

impl str::FromStr for OtpMode {
  type Err = ();

  fn from_str(s: &str) -> result::Result<Self, Self::Err> {
    match s {
      "6" => Ok(OtpMode::SixDigits),
      "8" => Ok(OtpMode::EightDigits),
      _ => Err(()),
    }
  }
}

impl From<OtpMode> for nitrokey::OtpMode {
  fn from(mode: OtpMode) -> Self {
    match mode {
      OtpMode::SixDigits => nitrokey::OtpMode::SixDigits,
      OtpMode::EightDigits => nitrokey::OtpMode::EightDigits,
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
  parser.set_description("Print the status of the connected Nitrokey device");
  parse(&parser, args)?;

  commands::status()
}

#[derive(Debug)]
enum StorageCommand {
  Close,
  Open,
}

impl StorageCommand {
  fn execute(&self, args: Vec<String>) -> Result<()> {
    match *self {
      StorageCommand::Close => storage_close(args),
      StorageCommand::Open => storage_open(args),
    }
  }
}

impl fmt::Display for StorageCommand {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match *self {
        StorageCommand::Close => "close",
        StorageCommand::Open => "open",
      }
    )
  }
}

impl str::FromStr for StorageCommand {
  type Err = ();

  fn from_str(s: &str) -> result::Result<Self, Self::Err> {
    match s {
      "close" => Ok(StorageCommand::Close),
      "open" => Ok(StorageCommand::Open),
      _ => Err(()),
    }
  }
}

/// Execute a storage subcommand.
fn storage(args: Vec<String>) -> Result<()> {
  let mut subcommand = StorageCommand::Open;
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Interacts with the device's storage");
  let _ = parser.refer(&mut subcommand).required().add_argument(
    "subcommand",
    argparse::Store,
    "The subcommand to execute (open|close)",
  );
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(&parser, args)?;
  drop(parser);

  subargs.insert(0, format!("nitrocli storage {}", subcommand));
  subcommand.execute(subargs)
}

/// Open the encrypted volume on the nitrokey.
fn storage_open(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Opens the encrypted volume on a Nitrokey Storage");
  parse(&parser, args)?;

  commands::storage_open()
}

/// Close the previously opened encrypted volume.
fn storage_close(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Closes the encrypted volume on a Nitrokey Storage");
  parse(&parser, args)?;

  commands::storage_close()
}

/// Clear the PIN as cached by various other commands.
fn clear(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Clears the cached passphrases");
  parse(&parser, args)?;

  commands::clear()
}

/// Execute a config subcommand.
fn config(args: Vec<String>) -> Result<()> {
  let mut subcommand = ConfigCommand::Get;
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Reads or writes the device configuration");
  let _ = parser.refer(&mut subcommand).required().add_argument(
    "subcommand",
    argparse::Store,
    "The subcommand to execute (get|set)",
  );
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(&parser, args)?;
  drop(parser);

  subargs.insert(0, format!("nitrocli config {}", subcommand));
  subcommand.execute(subargs)
}

/// Read the Nitrokey configuration.
fn config_get(args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Prints the Nitrokey configuration");
  parse(&parser, args)?;

  commands::config_get()
}

/// Write the Nitrokey configuration.
fn config_set(args: Vec<String>) -> Result<()> {
  let mut numlock = None;
  let mut no_numlock = false;
  let mut capslock = None;
  let mut no_capslock = false;
  let mut scrollock = None;
  let mut no_scrollock = false;
  let mut otp_pin = false;
  let mut no_otp_pin = false;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Changes the Nitrokey configuration");
  let _ = parser.refer(&mut numlock).add_option(
    &["-n", "--numlock"],
    argparse::StoreOption,
    "Set the numlock option to the given HOTP slot",
  );
  let _ = parser.refer(&mut no_numlock).add_option(
    &["-N", "--no-numlock"],
    argparse::StoreTrue,
    "Unset the numlock option",
  );
  let _ = parser.refer(&mut capslock).add_option(
    &["-c", "--capslock"],
    argparse::StoreOption,
    "Set the capslock option to the given HOTP slot",
  );
  let _ = parser.refer(&mut no_capslock).add_option(
    &["-C", "--no-capslock"],
    argparse::StoreTrue,
    "Unset the capslock option",
  );
  let _ = parser.refer(&mut scrollock).add_option(
    &["-s", "--scrollock"],
    argparse::StoreOption,
    "Set the scrollock option to the given HOTP slot",
  );
  let _ = parser.refer(&mut no_scrollock).add_option(
    &["-S", "--no-scrollock"],
    argparse::StoreTrue,
    "Unset the scrollock option",
  );
  let _ = parser.refer(&mut otp_pin).add_option(
    &["-o", "--otp-pin"],
    argparse::StoreTrue,
    "Require the user PIN to generate one-time passwords",
  );
  let _ = parser.refer(&mut no_otp_pin).add_option(
    &["-O", "--no-otp-pin"],
    argparse::StoreTrue,
    "Allow one-time password generation without PIN",
  );
  parse(&parser, args)?;
  drop(parser);

  let numlock = ConfigOption::try_from(no_numlock, numlock, "numlock")?;
  let capslock = ConfigOption::try_from(no_capslock, capslock, "capslock")?;
  let scrollock = ConfigOption::try_from(no_scrollock, scrollock, "scrollock")?;
  let otp_pin = if otp_pin {
    Some(true)
  } else if no_otp_pin {
    Some(false)
  } else {
    None
  };
  commands::config_set(numlock, capslock, scrollock, otp_pin)
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
    "The subcommand to execute (clear|get|set|status)",
  );
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(&parser, args)?;
  drop(parser);

  subargs.insert(0, format!("nitrocli otp {}", subcommand));
  subcommand.execute(subargs)
}

/// Generate a one-time password on the Nitrokey device.
fn otp_get(args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut algorithm = OtpAlgorithm::Totp;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Generates a one-time password");
  let _ =
    parser
      .refer(&mut slot)
      .required()
      .add_argument("slot", argparse::Store, "The OTP slot to use");
  let _ = parser.refer(&mut algorithm).add_option(
    &["-a", "--algorithm"],
    argparse::Store,
    "The OTP algorithm to use (hotp|totp)",
  );
  parse(&parser, args)?;
  drop(parser);

  commands::otp_get(slot, algorithm)
}

/// Configure a one-time password slot on the Nitrokey device.
pub fn otp_set(args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut algorithm = OtpAlgorithm::Totp;
  let mut name = "".to_owned();
  let mut secret = "".to_owned();
  let mut digits = OtpMode::SixDigits;
  let mut counter: u64 = 0;
  let mut time_window: u16 = 30;
  let mut ascii = false;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Configures a one-time password slot");
  let _ =
    parser
      .refer(&mut slot)
      .required()
      .add_argument("slot", argparse::Store, "The OTP slot to use");
  let _ = parser.refer(&mut algorithm).add_option(
    &["-a", "--algorithm"],
    argparse::Store,
    "The OTP algorithm to use (hotp or totp, default: totp",
  );
  let _ = parser.refer(&mut name).required().add_argument(
    "name",
    argparse::Store,
    "The name of the slot",
  );
  let _ = parser.refer(&mut secret).required().add_argument(
    "secret",
    argparse::Store,
    "The secret to store on the slot as a hexadecimal string (unless --ascii is set)",
  );
  let _ = parser.refer(&mut digits).add_option(
    &["-d", "--digits"],
    argparse::Store,
    "The number of digits to use for the one-time password (6 or 8, default: 6)",
  );
  let _ = parser.refer(&mut counter).add_option(
    &["-c", "--counter"],
    argparse::Store,
    "The counter value for HOTP (default: 0)",
  );
  let _ = parser.refer(&mut time_window).add_option(
    &["-t", "--time-window"],
    argparse::Store,
    "The time window for TOTP (default: 30)",
  );
  let _ = parser.refer(&mut ascii).add_option(
    &["--ascii"],
    argparse::StoreTrue,
    "Interpret the given secret as an ASCII string of the secret",
  );
  parse(&parser, args)?;
  drop(parser);

  let data = nitrokey::OtpSlotData {
    number: slot,
    name,
    secret,
    mode: nitrokey::OtpMode::from(digits),
    use_enter: false,
    token_id: None,
  };
  commands::otp_set(data, algorithm, counter, time_window, ascii)
}

/// Clear an OTP slot.
fn otp_clear(args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut algorithm = OtpAlgorithm::Totp;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Clears a one-time password slot");
  let _ = parser.refer(&mut slot).required().add_argument(
    "slot",
    argparse::Store,
    "The OTP slot to clear",
  );
  let _ = parser.refer(&mut algorithm).add_option(
    &["-a", "--algorithm"],
    argparse::Store,
    "The OTP algorithm to use (hotp|totp)",
  );
  parse(&parser, args)?;
  drop(parser);

  commands::otp_clear(slot, algorithm)
}

/// Print the status of the OTP slots.
fn otp_status(args: Vec<String>) -> Result<()> {
  let mut all = false;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Prints the status of the OTP slots");
  let _ = parser.refer(&mut all).add_option(
    &["-a", "--all"],
    argparse::StoreTrue,
    "Show slots that are not programmed",
  );
  parse(&parser, args)?;
  drop(parser);

  commands::otp_status(all)
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
    "The command to execute (clear|config|otp|status|storage)",
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
