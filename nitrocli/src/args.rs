// args.rs

// *************************************************************************
// * Copyright (C) 2018-2019 Daniel Mueller (deso@posteo.net)              *
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
pub struct Args {
  /// Increases the log level (can be supplied multiple times)
  #[structopt(short, long, parse(from_occurrences))]
  verbose: u8,
  /// Selects the device model to connect to
  #[structopt(short, long)]
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
  Config(ConfigArgs) => |ctx, args: ConfigArgs| args.subcmd.execute(ctx),
  Encrypted(EncryptedArgs) => |ctx, args: EncryptedArgs| args.subcmd.execute(ctx),
  Hidden(HiddenArgs) => |ctx, args: HiddenArgs| args.subcmd.execute(ctx),
  Lock(LockArgs) => |ctx, _| commands::lock(ctx),
  Otp(OtpArgs) => |ctx, args: OtpArgs| args.subcmd.execute(ctx),
  Pin(PinArgs) => |ctx, args: PinArgs| args.subcmd.execute(ctx),
  Pws(PwsArgs) => |ctx, args: PwsArgs| args.subcmd.execute(ctx),
  Reset(ResetArgs) => |ctx, _| commands::reset(ctx),
  Status(StatusArgs) => |ctx, _| commands::status(ctx),
  Unencrypted(UnencryptedArgs) => |ctx, args: UnencryptedArgs| args.subcmd.execute(ctx),
]}

/// Reads or writes the device configuration
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct ConfigArgs {
  #[structopt(subcommand)]
  subcmd: ConfigCommand,
}

/// Prints the Nitrokey configuration
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct ConfigGetArgs {}

/// Changes the Nitrokey configuration
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct ConfigSetArgs {
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

/// Interacts with the device's encrypted volume
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct EncryptedArgs {
  #[structopt(subcommand)]
  subcmd: EncryptedCommand,
}

/// Closes the encrypted volume on a Nitrokey Storage
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct EncryptedCloseArgs {}

/// Opens the encrypted volume on a Nitrokey Storage
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct EncryptedOpenArgs {}

/// Interacts with the device's hidden volume
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct HiddenArgs {
  #[structopt(subcommand)]
  subcmd: HiddenCommand,
}

/// Closes the hidden volume on a Nitrokey Storage
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct HiddenCloseArgs {}

/// Creates a hidden volume on a Nitrokey Storage
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct HiddenCreateArgs {
  /// The hidden volume slot to use
  slot: u8,
  /// The start location of the hidden volume as a percentage of the encrypted volume's size (0-99)
  start: u8,
  /// The end location of the hidden volume as a percentage of the encrypted volume's size (1-100)
  end: u8,
}

/// Opens the hidden volume on a Nitrokey Storage
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct HiddenOpenArgs {}

/// Locks the connected Nitrokey device
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct LockArgs {}

/// Accesses one-time passwords
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpArgs {
  #[structopt(subcommand)]
  subcmd: OtpCommand,
}

/// Clears a one-time password slot
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpClearArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = "totp")]
  algorithm: OtpAlgorithm,
  /// The OTP slot to clear
  slot: u8,
}

/// Generates a one-time password
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpGetArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = "totp")]
  algorithm: OtpAlgorithm,
  /// The time to use for TOTP generation (Unix timestamp) [default: system time]
  #[structopt(short, long)]
  time: Option<u64>,
  /// The OTP slot to use
  slot: u8,
}

/// Configures a one-time password slot
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpSetArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = "totp")]
  algorithm: OtpAlgorithm,
  /// The number of digits to use for the one-time password
  #[structopt(short, long, default_value = "6")]
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

/// Prints the status of the one-time password slots
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpStatusArgs {
  /// Shows slots that are not programmed
  #[structopt(short, long)]
  all: bool,
}

/// Manages the Nitrokey PINs
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PinArgs {
  #[structopt(subcommand)]
  subcmd: PinCommand,
}

/// Clears the cached PINs
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PinClearArgs {}

/// Changes a PIN
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PinSetArgs {
  /// The PIN type to change
  #[structopt(name = "type")]
  pintype: pinentry::PinType,
}

/// Unblocks and resets the user PIN
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PinUnblockArgs {}

/// Accesses the password safe
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsArgs {
  #[structopt(subcommand)]
  subcmd: PwsCommand,
}

/// Clears a password safe slot
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsClearArgs {
  /// The PWS slot to clear
  slot: u8,
}

/// Reads a password safe slot
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsGetArgs {
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

/// Writes a password safe slot
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsSetArgs {
  /// The PWS slot to write
  slot: u8,
  /// The name to store on the slot
  name: String,
  /// The login to store on the slot
  login: String,
  /// The password to store on the slot
  password: String,
}

/// Prints the status of the password safe slots
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsStatusArgs {
  /// Shows slots that are not programmed
  #[structopt(short, long)]
  all: bool,
}

/// Performs a factory reset
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct ResetArgs {}

/// Prints the status of the connected Nitrokey device
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct StatusArgs {}

/// Interacts with the device's unencrypted volume
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct UnencryptedArgs {
  #[structopt(subcommand)]
  subcmd: UnencryptedCommand,
}

/// Changes the configuration of the unencrypted volume on a Nitrokey Storage
#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct UnencryptedSetArgs {
  /// The mode to change to
  #[structopt(name = "type")]
  mode: UnencryptedVolumeMode,
}

Command! {ConfigCommand, [
  Get(ConfigGetArgs) => |ctx, _| commands::config_get(ctx),
  Set(ConfigSetArgs) => config_set,
]}

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

Command! {OtpCommand, [
  Clear(OtpClearArgs) => |ctx, args: OtpClearArgs| commands::otp_clear(ctx, args.slot, args.algorithm),
  Get(OtpGetArgs) => |ctx, args: OtpGetArgs| commands::otp_get(ctx, args.slot, args.algorithm, args.time),
  Set(OtpSetArgs) => otp_set,
  Status(OtpStatusArgs) => |ctx, args: OtpStatusArgs| commands::otp_status(ctx, args.all),
]}

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

Command! {PinCommand, [
  Clear(PinClearArgs) => |ctx, _| commands::pin_clear(ctx),
  Set(PinSetArgs) => |ctx, args: PinSetArgs| commands::pin_set(ctx, args.pintype),
  Unblock(PinUnblockArgs) => |ctx, _| commands::pin_unblock(ctx),
]}

Command! {PwsCommand, [
  Clear(PwsClearArgs) => |ctx, args: PwsClearArgs| commands::pws_clear(ctx, args.slot),
  Get(PwsGetArgs) => |ctx, args: PwsGetArgs| commands::pws_get(ctx, args.slot, args.name, args.login, args.password, args.quiet),
  Set(PwsSetArgs) => |ctx, args: PwsSetArgs| commands::pws_set(ctx, args.slot, &args.name, &args.login, &args.password),
  Status(PwsStatusArgs) => |ctx, args: PwsStatusArgs| commands::pws_status(ctx, args.all),
]}

Command! {UnencryptedCommand, [
  Set(UnencryptedSetArgs) => |ctx, args: UnencryptedSetArgs| commands::unencrypted_set(ctx, args.mode),
]}

Enum! {UnencryptedVolumeMode, [
  ReadWrite => "read-write",
  ReadOnly => "read-only",
]}

Command! {EncryptedCommand, [
  Close(EncryptedCloseArgs) => |ctx, _| commands::encrypted_close(ctx),
  Open(EncryptedOpenArgs) => |ctx, _| commands::encrypted_open(ctx),
]}

Command! {HiddenCommand, [
  Close(HiddenCloseArgs) => |ctx, _| commands::hidden_close(ctx),
  Create(HiddenCreateArgs) => |ctx, args: HiddenCreateArgs| commands::hidden_create(ctx, args.slot, args.start, args.end),
  Open(HiddenOpenArgs) => |ctx, _| commands::hidden_open(ctx),
]}

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
