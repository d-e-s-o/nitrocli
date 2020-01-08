// args.rs

// *************************************************************************
// * Copyright (C) 2018-2020 Daniel Mueller (deso@posteo.net)              *
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

use std::ffi;
use std::io;
use std::result;
use std::str;

use crate::commands;
use crate::error::Error;
use crate::pinentry;
use crate::RunCtx;

type Result<T> = result::Result<T, Error>;

trait Stdio {
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write);
}

impl<'io> Stdio for RunCtx<'io> {
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write) {
    (self.stdout, self.stderr)
  }
}

impl<W> Stdio for (&mut W, &mut W)
where
  W: io::Write,
{
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write) {
    (self.0, self.1)
  }
}

/// A command execution context that captures additional data pertaining
/// the command execution.
pub struct ExecCtx<'io> {
  pub model: Option<DeviceModel>,
  pub stdout: &'io mut dyn io::Write,
  pub stderr: &'io mut dyn io::Write,
  pub admin_pin: Option<ffi::OsString>,
  pub user_pin: Option<ffi::OsString>,
  pub new_admin_pin: Option<ffi::OsString>,
  pub new_user_pin: Option<ffi::OsString>,
  pub password: Option<ffi::OsString>,
  pub no_cache: bool,
  pub verbosity: u64,
}

impl<'io> Stdio for ExecCtx<'io> {
  fn stdio(&mut self) -> (&mut dyn io::Write, &mut dyn io::Write) {
    (self.stdout, self.stderr)
  }
}

/// Provides access to a Nitrokey device
#[derive(structopt::StructOpt)]
#[structopt(name = "nitrocli")]
struct Args {
  /// Increases the log level (can be supplied multiple times)
  #[structopt(short, long, parse(from_occurrences))]
  verbose: u8,
  /// Selects the device model to connect to
  #[structopt(short, long, possible_values = &DeviceModel::all_str())]
  model: Option<DeviceModel>,
  #[structopt(subcommand)]
  cmd: Command,
}

/// The available Nitrokey models.
#[allow(unused_doc_comments)]
Enum! {DeviceModel, [
  Pro => "pro",
  Storage => "storage",
]}

impl DeviceModel {
  pub fn as_user_facing_str(&self) -> &str {
    match self {
      DeviceModel::Pro => "Pro",
      DeviceModel::Storage => "Storage",
    }
  }
}

impl From<DeviceModel> for nitrokey::Model {
  fn from(model: DeviceModel) -> nitrokey::Model {
    match model {
      DeviceModel::Pro => nitrokey::Model::Pro,
      DeviceModel::Storage => nitrokey::Model::Storage,
    }
  }
}

/// A top-level command for nitrocli.
#[allow(unused_doc_comments)]
Command! {Command, [
  /// Reads or writes the device configuration
  Config(ConfigArgs) => |ctx, args: ConfigArgs| args.subcmd.execute(ctx),
  /// Interacts with the device's encrypted volume
  Encrypted(EncryptedArgs) => |ctx, args: EncryptedArgs| args.subcmd.execute(ctx),
  /// Interacts with the device's hidden volume
  Hidden(HiddenArgs) => |ctx, args: HiddenArgs| args.subcmd.execute(ctx),
  /// Locks the connected Nitrokey device
  Lock => commands::lock,
  /// Accesses one-time passwords
  Otp(OtpArgs) => |ctx, args: OtpArgs| args.subcmd.execute(ctx),
  /// Manages the Nitrokey PINs
  Pin(PinArgs) => |ctx, args: PinArgs| args.subcmd.execute(ctx),
  /// Accesses the password safe
  Pws(PwsArgs) => |ctx, args: PwsArgs| args.subcmd.execute(ctx),
  /// Performs a factory reset
  Reset => commands::reset,
  /// Prints the status of the connected Nitrokey device
  Status => commands::status,
  /// Interacts with the device's unencrypted volume
  Unencrypted(UnencryptedArgs) => |ctx, args: UnencryptedArgs| args.subcmd.execute(ctx),
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct ConfigArgs {
  #[structopt(subcommand)]
  subcmd: ConfigCommand,
}

Command! {ConfigCommand, [
  /// Prints the Nitrokey configuration
  Get => commands::config_get,
  /// Changes the Nitrokey configuration
  Set(ConfigSetArgs) => config_set,
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct ConfigSetArgs {
  /// Sets the numlock option to the given HOTP slot
  #[structopt(short = "n", long)]
  numlock: Option<u8>,
  /// Unsets the numlock option
  #[structopt(short = "N", long, conflicts_with("numlock"))]
  no_numlock: bool,
  /// Sets the capslock option to the given HOTP slot
  #[structopt(short = "c", long)]
  capslock: Option<u8>,
  /// Unsets the capslock option
  #[structopt(short = "C", long, conflicts_with("capslock"))]
  no_capslock: bool,
  /// Sets the scrollock option to the given HOTP slot
  #[structopt(short = "s", long)]
  scrollock: Option<u8>,
  /// Unsets the scrollock option
  #[structopt(short = "S", long, conflicts_with("scrollock"))]
  no_scrollock: bool,
  /// Requires the user PIN to generate one-time passwords
  #[structopt(short = "o", long)]
  otp_pin: bool,
  /// Allows one-time password generation without PIN
  #[structopt(short = "O", long, conflicts_with("otp_pin"))]
  no_otp_pin: bool,
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

fn config_set(ctx: &mut ExecCtx<'_>, args: ConfigSetArgs) -> Result<()> {
  let numlock = ConfigOption::try_from(args.no_numlock, args.numlock, "numlock")?;
  let capslock = ConfigOption::try_from(args.no_capslock, args.capslock, "capslock")?;
  let scrollock = ConfigOption::try_from(args.no_scrollock, args.scrollock, "scrollock")?;
  let otp_pin = if args.otp_pin {
    Some(true)
  } else if args.no_otp_pin {
    Some(false)
  } else {
    None
  };
  commands::config_set(ctx, numlock, capslock, scrollock, otp_pin)
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct EncryptedArgs {
  #[structopt(subcommand)]
  subcmd: EncryptedCommand,
}

Command! {EncryptedCommand, [
  /// Closes the encrypted volume on a Nitrokey Storage
  Close => commands::encrypted_close,
  /// Opens the encrypted volume on a Nitrokey Storage
  Open => commands::encrypted_open,
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct HiddenArgs {
  #[structopt(subcommand)]
  subcmd: HiddenCommand,
}

Command! {HiddenCommand, [
  /// Closes the hidden volume on a Nitrokey Storage
  Close => commands::hidden_close,
  /// Creates a hidden volume on a Nitrokey Storage
  Create(HiddenCreateArgs) => |ctx, args: HiddenCreateArgs| {
    commands::hidden_create(ctx, args.slot, args.start, args.end)
  },
  /// Opens the hidden volume on a Nitrokey Storage
  Open => commands::hidden_open,
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct HiddenCreateArgs {
  /// The hidden volume slot to use
  slot: u8,
  /// The start location of the hidden volume as a percentage of the encrypted volume's size (0-99)
  start: u8,
  /// The end location of the hidden volume as a percentage of the encrypted volume's size (1-100)
  end: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct OtpArgs {
  #[structopt(subcommand)]
  subcmd: OtpCommand,
}

Command! {OtpCommand, [
  /// Clears a one-time password slot
  Clear(OtpClearArgs) => |ctx, args: OtpClearArgs| {
    commands::otp_clear(ctx, args.slot, args.algorithm)
  },
  /// Generates a one-time password
  Get(OtpGetArgs) => |ctx, args: OtpGetArgs| {
    commands::otp_get(ctx, args.slot, args.algorithm, args.time)
  },
  /// Configures a one-time password slot
  Set(OtpSetArgs) => otp_set,
  /// Prints the status of the one-time password slots
  Status(OtpStatusArgs) => |ctx, args: OtpStatusArgs| commands::otp_status(ctx, args.all),
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct OtpClearArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = "totp", possible_values = &OtpAlgorithm::all_str())]
  algorithm: OtpAlgorithm,
  /// The OTP slot to clear
  slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct OtpGetArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = "totp", possible_values = &OtpAlgorithm::all_str())]
  algorithm: OtpAlgorithm,
  /// The time to use for TOTP generation (Unix timestamp) [default: system time]
  #[structopt(short, long)]
  time: Option<u64>,
  /// The OTP slot to use
  slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct OtpSetArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = "totp", possible_values = &OtpAlgorithm::all_str())]
  algorithm: OtpAlgorithm,
  /// The number of digits to use for the one-time password
  #[structopt(short, long, default_value = "6", possible_values = &OtpMode::all_str())]
  digits: OtpMode,
  /// The counter value for HOTP
  #[structopt(short, long, default_value = "0")]
  counter: u64,
  /// The time window for TOTP
  #[structopt(short, long, default_value = "30")]
  time_window: u16,
  /// The format of the secret
  #[structopt(short, long, default_value = "hex")]
  format: OtpSecretFormat,
  /// The OTP slot to use
  slot: u8,
  /// The name of the slot
  name: String,
  /// The secret to store on the slot as a hexadecimal string (or in the format set with the
  /// --format option)
  secret: String,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct OtpStatusArgs {
  /// Shows slots that are not programmed
  #[structopt(short, long)]
  all: bool,
}

Enum! {OtpAlgorithm, [
  Hotp => "hotp",
  Totp => "totp",
]}

Enum! {OtpMode, [
  SixDigits => "6",
  EightDigits => "8",
]}

impl From<OtpMode> for nitrokey::OtpMode {
  fn from(mode: OtpMode) -> Self {
    match mode {
      OtpMode::SixDigits => nitrokey::OtpMode::SixDigits,
      OtpMode::EightDigits => nitrokey::OtpMode::EightDigits,
    }
  }
}

Enum! {OtpSecretFormat, [
  Ascii => "ascii",
  Base32 => "base32",
  Hex => "hex",
]}

fn otp_set(ctx: &mut ExecCtx<'_>, args: OtpSetArgs) -> Result<()> {
  let data = nitrokey::OtpSlotData {
    number: args.slot,
    name: args.name,
    secret: args.secret,
    mode: args.digits.into(),
    use_enter: false,
    token_id: None,
  };
  commands::otp_set(
    ctx,
    data,
    args.algorithm,
    args.counter,
    args.time_window,
    args.format,
  )
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct PinArgs {
  #[structopt(subcommand)]
  subcmd: PinCommand,
}

Command! {PinCommand, [
  /// Clears the cached PINs
  Clear => commands::pin_clear,
  /// Changes a PIN
  Set(PinSetArgs) => |ctx, args: PinSetArgs| commands::pin_set(ctx, args.pintype),
  /// Unblocks and resets the user PIN
  Unblock => commands::pin_unblock,
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct PinSetArgs {
  /// The PIN type to change
  #[structopt(name = "type", possible_values = &pinentry::PinType::all_str())]
  pintype: pinentry::PinType,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct PwsArgs {
  #[structopt(subcommand)]
  subcmd: PwsCommand,
}

Command! {PwsCommand, [
  /// Clears a password safe slot
  Clear(PwsClearArgs) => |ctx, args: PwsClearArgs| commands::pws_clear(ctx, args.slot),
  /// Reads a password safe slot
  Get(PwsGetArgs) => |ctx, args: PwsGetArgs| {
    commands::pws_get(ctx, args.slot, args.name, args.login, args.password, args.quiet)
  },
  /// Writes a password safe slot
  Set(PwsSetArgs) => |ctx, args: PwsSetArgs| {
    commands::pws_set(ctx, args.slot, &args.name, &args.login, &args.password)
  },
  /// Prints the status of the password safe slots
  Status(PwsStatusArgs) => |ctx, args: PwsStatusArgs| commands::pws_status(ctx, args.all),
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct PwsClearArgs {
  /// The PWS slot to clear
  slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct PwsGetArgs {
  /// Shows the name stored on the slot
  #[structopt(short, long)]
  name: bool,
  /// Shows the login stored on the slot
  #[structopt(short, long)]
  login: bool,
  /// Shows the password stored on the slot
  #[structopt(short, long)]
  password: bool,
  /// Prints the stored data without description
  #[structopt(short, long)]
  quiet: bool,
  /// The PWS slot to read
  slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct PwsSetArgs {
  /// The PWS slot to write
  slot: u8,
  /// The name to store on the slot
  name: String,
  /// The login to store on the slot
  login: String,
  /// The password to store on the slot
  password: String,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct PwsStatusArgs {
  /// Shows slots that are not programmed
  #[structopt(short, long)]
  all: bool,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct UnencryptedArgs {
  #[structopt(subcommand)]
  subcmd: UnencryptedCommand,
}

Command! {UnencryptedCommand, [
  /// Changes the configuration of the unencrypted volume on a Nitrokey Storage
  Set(UnencryptedSetArgs) => |ctx, args: UnencryptedSetArgs| {
    commands::unencrypted_set(ctx, args.mode)
  },
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
struct UnencryptedSetArgs {
  /// The mode to change to
  #[structopt(name = "type", possible_values = &UnencryptedVolumeMode::all_str())]
  mode: UnencryptedVolumeMode,
}

Enum! {UnencryptedVolumeMode, [
  ReadWrite => "read-write",
  ReadOnly => "read-only",
]}

/// Parse the command-line arguments and execute the selected command.
pub(crate) fn handle_arguments(ctx: &mut RunCtx<'_>, args: Vec<String>) -> Result<()> {
  use structopt::StructOpt;

  match Args::from_iter_safe(args.iter()) {
    Ok(args) => {
      let mut ctx = ExecCtx {
        model: args.model,
        stdout: ctx.stdout,
        stderr: ctx.stderr,
        admin_pin: ctx.admin_pin.take(),
        user_pin: ctx.user_pin.take(),
        new_admin_pin: ctx.new_admin_pin.take(),
        new_user_pin: ctx.new_user_pin.take(),
        password: ctx.password.take(),
        no_cache: ctx.no_cache,
        verbosity: args.verbose.into(),
      };
      args.cmd.execute(&mut ctx)
    }
    Err(err) => {
      if err.use_stderr() {
        Err(err.into())
      } else {
        println!(ctx, "{}", err.message)?;
        Ok(())
      }
    }
  }
}
