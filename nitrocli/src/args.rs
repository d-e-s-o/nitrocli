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

/// Wraps a writer and buffers its output.
///
/// This implementation is similar to `io::BufWriter`, but:
/// - The inner writer is only written to if `flush` is called.
/// - The buffer may grow infinitely large.
struct BufWriter<'w, W: io::Write + ?Sized> {
  buf: Vec<u8>,
  inner: &'w mut W,
}

impl<'w, W: io::Write + ?Sized> BufWriter<'w, W> {
  pub fn new(inner: &'w mut W) -> Self {
    BufWriter {
      buf: Vec::with_capacity(128),
      inner,
    }
  }
}

impl<'w, W: io::Write + ?Sized> io::Write for BufWriter<'w, W> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    self.buf.extend_from_slice(buf);
    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    self.inner.write_all(&self.buf)?;
    self.buf.clear();
    self.inner.flush()
  }
}

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
Enum! {Command, [
  Config => ("config", config),
  Encrypted => ("encrypted", encrypted),
  Hidden => ("hidden", hidden),
  Lock => ("lock", lock),
  Otp => ("otp", otp),
  Pin => ("pin", pin),
  Pws => ("pws", pws),
  Reset => ("reset", reset),
  Status => ("status", status),
  Unencrypted => ("unencrypted", unencrypted),
]}

Enum! {ConfigCommand, [
  Get => ("get", config_get),
  Set => ("set", config_set),
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

Enum! {OtpCommand, [
  Clear => ("clear", otp_clear),
  Get => ("get", otp_get),
  Set => ("set", otp_set),
  Status => ("status", otp_status),
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

Enum! {PinCommand, [
  Clear => ("clear", pin_clear),
  Set => ("set", pin_set),
  Unblock => ("unblock", pin_unblock),
]}

Enum! {PwsCommand, [
  Clear => ("clear", pws_clear),
  Get => ("get", pws_get),
  Set => ("set", pws_set),
  Status => ("status", pws_status),
]}

fn parse(
  ctx: &mut impl Stdio,
  parser: argparse::ArgumentParser<'_>,
  args: Vec<String>,
) -> Result<()> {
  let (stdout, stderr) = ctx.stdio();
  let result = parser
    .parse(args, stdout, stderr)
    .map_err(Error::ArgparseError);
  drop(parser);
  result
}

/// Inquire the status of the Nitrokey.
fn status(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Prints the status of the connected Nitrokey device");
  parse(ctx, parser, args)?;

  commands::status(ctx)
}

/// Perform a factory reset.
fn reset(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Performs a factory reset");
  parse(ctx, parser, args)?;

  commands::reset(ctx)
}

Enum! {UnencryptedCommand, [
  Set => ("set", unencrypted_set),
]}

Enum! {UnencryptedVolumeMode, [
  ReadWrite => "read-write",
  ReadOnly => "read-only",
]}

/// Execute an unencrypted subcommand.
fn unencrypted(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut subcommand = UnencryptedCommand::Set;
  let help = cmd_help!(subcommand);
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Interacts with the device's unencrypted volume");
  let _ =
    parser
      .refer(&mut subcommand)
      .required()
      .add_argument("subcommand", argparse::Store, &help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(ctx, parser, args)?;

  subargs.insert(
    0,
    format!(
      "{} {} {}",
      crate::NITROCLI,
      Command::Unencrypted,
      subcommand,
    ),
  );
  subcommand.execute(ctx, subargs)
}

/// Change the configuration of the unencrypted volume.
fn unencrypted_set(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut mode = UnencryptedVolumeMode::ReadWrite;
  let help = format!("The mode to change to ({})", fmt_enum!(mode));
  let mut parser = argparse::ArgumentParser::new();
  parser
    .set_description("Changes the configuration of the unencrypted volume on a Nitrokey Storage");
  let _ = parser
    .refer(&mut mode)
    .required()
    .add_argument("type", argparse::Store, &help);
  parse(ctx, parser, args)?;

  commands::unencrypted_set(ctx, mode)
}

Enum! {EncryptedCommand, [
  Close => ("close", encrypted_close),
  Open => ("open", encrypted_open),
]}

/// Execute an encrypted subcommand.
fn encrypted(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut subcommand = EncryptedCommand::Open;
  let help = cmd_help!(subcommand);
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Interacts with the device's encrypted volume");
  let _ =
    parser
      .refer(&mut subcommand)
      .required()
      .add_argument("subcommand", argparse::Store, &help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(ctx, parser, args)?;

  subargs.insert(
    0,
    format!("{} {} {}", crate::NITROCLI, Command::Encrypted, subcommand),
  );
  subcommand.execute(ctx, subargs)
}

/// Open the encrypted volume on the Nitrokey.
fn encrypted_open(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Opens the encrypted volume on a Nitrokey Storage");
  parse(ctx, parser, args)?;

  commands::encrypted_open(ctx)
}

/// Close the previously opened encrypted volume.
fn encrypted_close(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Closes the encrypted volume on a Nitrokey Storage");
  parse(ctx, parser, args)?;

  commands::encrypted_close(ctx)
}

Enum! {HiddenCommand, [
  Close => ("close", hidden_close),
  Create => ("create", hidden_create),
  Open => ("open", hidden_open),
]}

/// Execute a hidden subcommand.
fn hidden(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut subcommand = HiddenCommand::Open;
  let help = cmd_help!(subcommand);
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Interacts with the device's hidden volume");
  let _ =
    parser
      .refer(&mut subcommand)
      .required()
      .add_argument("subcommand", argparse::Store, &help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(ctx, parser, args)?;

  subargs.insert(
    0,
    format!("{} {} {}", crate::NITROCLI, Command::Hidden, subcommand),
  );
  subcommand.execute(ctx, subargs)
}

fn hidden_create(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut start: u8 = 0;
  let mut end: u8 = 0;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Creates a hidden volume on a Nitrokey Storage");
  let _ = parser.refer(&mut slot).required().add_argument(
    "slot",
    argparse::Store,
    "The hidden volume slot to use",
  );
  let _ = parser.refer(&mut start).required().add_argument(
    "start",
    argparse::Store,
    "The start location of the hidden volume as percentage of the \
     encrypted volume's size (0-99)",
  );
  let _ = parser.refer(&mut end).required().add_argument(
    "end",
    argparse::Store,
    "The end location of the hidden volume as percentage of the \
     encrypted volume's size (1-100)",
  );
  parse(ctx, parser, args)?;

  commands::hidden_create(ctx, slot, start, end)
}

fn hidden_open(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Opens a hidden volume on a Nitrokey Storage");
  parse(ctx, parser, args)?;

  commands::hidden_open(ctx)
}

fn hidden_close(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Closes the hidden volume on a Nitrokey Storage");
  parse(ctx, parser, args)?;

  commands::hidden_close(ctx)
}

/// Execute a config subcommand.
fn config(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut subcommand = ConfigCommand::Get;
  let help = cmd_help!(subcommand);
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Reads or writes the device configuration");
  let _ =
    parser
      .refer(&mut subcommand)
      .required()
      .add_argument("subcommand", argparse::Store, &help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(ctx, parser, args)?;

  subargs.insert(
    0,
    format!("{} {} {}", crate::NITROCLI, Command::Config, subcommand),
  );
  subcommand.execute(ctx, subargs)
}

/// Read the Nitrokey configuration.
fn config_get(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Prints the Nitrokey configuration");
  parse(ctx, parser, args)?;

  commands::config_get(ctx)
}

/// Write the Nitrokey configuration.
fn config_set(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
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
  parse(ctx, parser, args)?;

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
  commands::config_set(ctx, numlock, capslock, scrollock, otp_pin)
}

/// Lock the Nitrokey.
fn lock(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Locks the connected Nitrokey device");
  parse(ctx, parser, args)?;

  commands::lock(ctx)
}

/// Execute an OTP subcommand.
fn otp(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut subcommand = OtpCommand::Get;
  let help = cmd_help!(subcommand);
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Accesses one-time passwords");
  let _ =
    parser
      .refer(&mut subcommand)
      .required()
      .add_argument("subcommand", argparse::Store, &help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(ctx, parser, args)?;

  subargs.insert(
    0,
    format!("{} {} {}", crate::NITROCLI, Command::Otp, subcommand),
  );
  subcommand.execute(ctx, subargs)
}

/// Generate a one-time password on the Nitrokey device.
fn otp_get(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut algorithm = OtpAlgorithm::Totp;
  let help = format!(
    "The OTP algorithm to use ({}, default: {})",
    fmt_enum!(algorithm),
    algorithm
  );
  let mut time: Option<u64> = None;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Generates a one-time password");
  let _ =
    parser
      .refer(&mut slot)
      .required()
      .add_argument("slot", argparse::Store, "The OTP slot to use");
  let _ = parser
    .refer(&mut algorithm)
    .add_option(&["-a", "--algorithm"], argparse::Store, &help);
  let _ = parser.refer(&mut time).add_option(
    &["-t", "--time"],
    argparse::StoreOption,
    "The time to use for TOTP generation (Unix timestamp, default: system time)",
  );
  parse(ctx, parser, args)?;

  commands::otp_get(ctx, slot, algorithm, time)
}

/// Configure a one-time password slot on the Nitrokey device.
pub fn otp_set(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut algorithm = OtpAlgorithm::Totp;
  let algo_help = format!(
    "The OTP algorithm to use ({}, default: {})",
    fmt_enum!(algorithm),
    algorithm
  );
  let mut name = "".to_owned();
  let mut secret = "".to_owned();
  let mut digits = OtpMode::SixDigits;
  let mut counter: u64 = 0;
  let mut time_window: u16 = 30;
  let mut secret_format: Option<OtpSecretFormat> = None;
  let fmt_help = format!(
    "The format of the secret ({})",
    fmt_enum!(OtpSecretFormat::all_variants())
  );
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Configures a one-time password slot");
  let _ =
    parser
      .refer(&mut slot)
      .required()
      .add_argument("slot", argparse::Store, "The OTP slot to use");
  let _ =
    parser
      .refer(&mut algorithm)
      .add_option(&["-a", "--algorithm"], argparse::Store, &algo_help);
  let _ = parser.refer(&mut name).required().add_argument(
    "name",
    argparse::Store,
    "The name of the slot",
  );
  let _ = parser.refer(&mut secret).required().add_argument(
    "secret",
    argparse::Store,
    "The secret to store on the slot as a hexadecimal string (unless overwritten by --format)",
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
  let _ = parser.refer(&mut secret_format).add_option(
    &["-f", "--format"],
    argparse::StoreOption,
    &fmt_help,
  );
  parse(ctx, parser, args)?;

  let secret_format = secret_format.unwrap_or(OtpSecretFormat::Hex);

  let data = nitrokey::OtpSlotData {
    number: slot,
    name,
    secret,
    mode: nitrokey::OtpMode::from(digits),
    use_enter: false,
    token_id: None,
  };
  commands::otp_set(ctx, data, algorithm, counter, time_window, secret_format)
}

/// Clear an OTP slot.
fn otp_clear(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut algorithm = OtpAlgorithm::Totp;
  let help = format!(
    "The OTP algorithm to use ({}, default: {})",
    fmt_enum!(algorithm),
    algorithm
  );
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Clears a one-time password slot");
  let _ = parser.refer(&mut slot).required().add_argument(
    "slot",
    argparse::Store,
    "The OTP slot to clear",
  );
  let _ = parser
    .refer(&mut algorithm)
    .add_option(&["-a", "--algorithm"], argparse::Store, &help);
  parse(ctx, parser, args)?;

  commands::otp_clear(ctx, slot, algorithm)
}

/// Print the status of the OTP slots.
fn otp_status(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut all = false;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Prints the status of the OTP slots");
  let _ = parser.refer(&mut all).add_option(
    &["-a", "--all"],
    argparse::StoreTrue,
    "Show slots that are not programmed",
  );
  parse(ctx, parser, args)?;

  commands::otp_status(ctx, all)
}

/// Execute a PIN subcommand.
fn pin(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut subcommand = PinCommand::Clear;
  let help = cmd_help!(subcommand);
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Manages the Nitrokey PINs");
  let _ =
    parser
      .refer(&mut subcommand)
      .required()
      .add_argument("subcommand", argparse::Store, &help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(ctx, parser, args)?;

  subargs.insert(
    0,
    format!("{} {} {}", crate::NITROCLI, Command::Pin, subcommand),
  );
  subcommand.execute(ctx, subargs)
}

/// Clear the PIN as cached by various other commands.
fn pin_clear(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Clears the cached PINs");
  parse(ctx, parser, args)?;

  commands::pin_clear(ctx)
}

/// Change a PIN.
fn pin_set(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut pintype = pinentry::PinType::User;
  let help = format!("The PIN type to change ({})", fmt_enum!(pintype));
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Changes a PIN");
  let _ = parser
    .refer(&mut pintype)
    .required()
    .add_argument("type", argparse::Store, &help);
  parse(ctx, parser, args)?;

  commands::pin_set(ctx, pintype)
}

/// Unblock and reset the user PIN.
fn pin_unblock(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Unblocks and resets the user PIN");
  parse(ctx, parser, args)?;

  commands::pin_unblock(ctx)
}

/// Execute a PWS subcommand.
fn pws(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut subcommand = PwsCommand::Get;
  let mut subargs = vec![];
  let help = cmd_help!(subcommand);
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Accesses the password safe");
  let _ =
    parser
      .refer(&mut subcommand)
      .required()
      .add_argument("subcommand", argparse::Store, &help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the subcommand",
  );
  parser.stop_on_first_argument(true);
  parse(ctx, parser, args)?;

  subargs.insert(
    0,
    format!("{} {} {}", crate::NITROCLI, Command::Pws, subcommand),
  );
  subcommand.execute(ctx, subargs)
}

/// Access a slot of the password safe on the Nitrokey.
fn pws_get(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut name = false;
  let mut login = false;
  let mut password = false;
  let mut quiet = false;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Reads a password safe slot");
  let _ = parser.refer(&mut slot).required().add_argument(
    "slot",
    argparse::Store,
    "The PWS slot to read",
  );
  let _ = parser.refer(&mut name).add_option(
    &["-n", "--name"],
    argparse::StoreTrue,
    "Show the name stored on the slot",
  );
  let _ = parser.refer(&mut login).add_option(
    &["-l", "--login"],
    argparse::StoreTrue,
    "Show the login stored on the slot",
  );
  let _ = parser.refer(&mut password).add_option(
    &["-p", "--password"],
    argparse::StoreTrue,
    "Show the password stored on the slot",
  );
  let _ = parser.refer(&mut quiet).add_option(
    &["-q", "--quiet"],
    argparse::StoreTrue,
    "Print the stored data without description",
  );
  parse(ctx, parser, args)?;

  commands::pws_get(ctx, slot, name, login, password, quiet)
}

/// Set a slot of the password safe on the Nitrokey.
fn pws_set(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut name = String::new();
  let mut login = String::new();
  let mut password = String::new();
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Writes a password safe slot");
  let _ = parser.refer(&mut slot).required().add_argument(
    "slot",
    argparse::Store,
    "The PWS slot to write",
  );
  let _ = parser.refer(&mut name).required().add_argument(
    "name",
    argparse::Store,
    "The name to store on the slot",
  );
  let _ = parser.refer(&mut login).required().add_argument(
    "login",
    argparse::Store,
    "The login to store on the slot",
  );
  let _ = parser.refer(&mut password).required().add_argument(
    "password",
    argparse::Store,
    "The password to store on the slot",
  );
  parse(ctx, parser, args)?;

  commands::pws_set(ctx, slot, &name, &login, &password)
}

/// Clear a PWS slot.
fn pws_clear(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut slot: u8 = 0;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Clears a password safe slot");
  let _ = parser.refer(&mut slot).required().add_argument(
    "slot",
    argparse::Store,
    "The PWS slot to clear",
  );
  parse(ctx, parser, args)?;

  commands::pws_clear(ctx, slot)
}

/// Print the status of the PWS slots.
fn pws_status(ctx: &mut ExecCtx<'_>, args: Vec<String>) -> Result<()> {
  let mut all = false;
  let mut parser = argparse::ArgumentParser::new();
  parser.set_description("Prints the status of the PWS slots");
  let _ = parser.refer(&mut all).add_option(
    &["-a", "--all"],
    argparse::StoreTrue,
    "Show slots that are not programmed",
  );
  parse(ctx, parser, args)?;

  commands::pws_status(ctx, all)
}

/// Parse the command-line arguments and execute the selected command.
pub(crate) fn handle_arguments(ctx: &mut RunCtx<'_>, args: Vec<String>) -> Result<()> {
  use std::io::Write;

  let mut version = false;
  let mut model: Option<DeviceModel> = None;
  let model_help = format!(
    "Select the device model to connect to ({})",
    fmt_enum!(DeviceModel::all_variants())
  );
  let mut verbosity = 0;
  let mut command = Command::Status;
  let cmd_help = cmd_help!(command);
  let mut subargs = vec![];
  let mut parser = argparse::ArgumentParser::new();
  let _ = parser.refer(&mut version).add_option(
    &["-V", "--version"],
    argparse::StoreTrue,
    "Print version information and exit",
  );
  let _ = parser.refer(&mut verbosity).add_option(
    &["-v", "--verbose"],
    argparse::IncrBy::<u64>(1),
    "Increase the log level (can be supplied multiple times)",
  );
  let _ =
    parser
      .refer(&mut model)
      .add_option(&["-m", "--model"], argparse::StoreOption, &model_help);
  parser.set_description("Provides access to a Nitrokey device");
  let _ = parser
    .refer(&mut command)
    .required()
    .add_argument("command", argparse::Store, &cmd_help);
  let _ = parser.refer(&mut subargs).add_argument(
    "arguments",
    argparse::List,
    "The arguments for the command",
  );
  parser.stop_on_first_argument(true);

  let mut stdout_buf = BufWriter::new(ctx.stdout);
  let mut stderr_buf = BufWriter::new(ctx.stderr);
  let mut stdio_buf = (&mut stdout_buf, &mut stderr_buf);
  let result = parse(&mut stdio_buf, parser, args);

  if version {
    println!(ctx, "{} {}", crate::NITROCLI, env!("CARGO_PKG_VERSION"))?;
    Ok(())
  } else {
    stdout_buf.flush()?;
    stderr_buf.flush()?;

    result?;
    subargs.insert(0, format!("{} {}", crate::NITROCLI, command));

    let mut ctx = ExecCtx {
      model,
      stdout: ctx.stdout,
      stderr: ctx.stderr,
      admin_pin: ctx.admin_pin.take(),
      user_pin: ctx.user_pin.take(),
      new_admin_pin: ctx.new_admin_pin.take(),
      new_user_pin: ctx.new_user_pin.take(),
      password: ctx.password.take(),
      no_cache: ctx.no_cache,
      verbosity,
    };
    command.execute(&mut ctx, subargs)
  }
}
