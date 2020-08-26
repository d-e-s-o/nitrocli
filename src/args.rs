// args.rs

// Copyright (C) 2020 The Nitrocli Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi;

/// Provides access to a Nitrokey device
#[derive(Debug, structopt::StructOpt)]
#[structopt(name = "nitrocli")]
pub struct Args {
  /// Increases the log level (can be supplied multiple times)
  #[structopt(short, long, global = true, parse(from_occurrences))]
  pub verbose: u8,
  /// Selects the device model to connect to
  #[structopt(short, long, global = true, possible_values = &DeviceModel::all_str())]
  pub model: Option<DeviceModel>,
  /// Sets the serial number of the device to connect to. Can be set
  /// multiple times to allow multiple serial numbers
  // TODO: Add short options (avoid collisions).
  #[structopt(
    long = "serial-number",
    global = true,
    multiple = true,
    number_of_values = 1
  )]
  pub serial_numbers: Vec<nitrokey::SerialNumber>,
  /// Sets the USB path of the device to connect to
  #[structopt(long, global = true)]
  pub usb_path: Option<String>,
  /// Disables the cache for all secrets.
  #[structopt(long, global = true)]
  pub no_cache: bool,
  #[structopt(subcommand)]
  pub cmd: Command,
}

Enum! {
  /// The available Nitrokey models.
  DeviceModel, [
    Pro => "pro",
    Storage => "storage",
  ]
}

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

impl<'de> serde::Deserialize<'de> for DeviceModel {
  fn deserialize<D>(deserializer: D) -> Result<DeviceModel, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    use serde::de::Error as _;
    use std::str::FromStr as _;

    let s = String::deserialize(deserializer)?;
    DeviceModel::from_str(&s).map_err(D::Error::custom)
  }
}

Command! {
  /// A top-level command for nitrocli.
  Command, [
    /// Reads or writes the device configuration
    Config(ConfigArgs) => |ctx, args: ConfigArgs| args.subcmd.execute(ctx),
    /// Interacts with the device's encrypted volume
    Encrypted(EncryptedArgs) => |ctx, args: EncryptedArgs| args.subcmd.execute(ctx),
    /// Fills the SD card with random data
    Fill(FillArgs) => |ctx, args: FillArgs| crate::commands::fill(ctx, args.attach),
    /// Interacts with the device's hidden volume
    Hidden(HiddenArgs) => |ctx, args: HiddenArgs| args.subcmd.execute(ctx),
    /// Lists the attached Nitrokey devices
    List(ListArgs) => |ctx, args: ListArgs| crate::commands::list(ctx, args.no_connect),
    /// Locks the connected Nitrokey device
    Lock => crate::commands::lock,
    /// Accesses one-time passwords
    Otp(OtpArgs) => |ctx, args: OtpArgs| args.subcmd.execute(ctx),
    /// Manages the Nitrokey PINs
    Pin(PinArgs) => |ctx, args: PinArgs| args.subcmd.execute(ctx),
    /// Accesses the password safe
    Pws(PwsArgs) => |ctx, args: PwsArgs| args.subcmd.execute(ctx),
    /// Performs a factory reset
    Reset => crate::commands::reset,
    /// Prints the status of the connected Nitrokey device
    Status => crate::commands::status,
    /// Interacts with the device's unencrypted volume
    Unencrypted(UnencryptedArgs) => |ctx, args: UnencryptedArgs| args.subcmd.execute(ctx),
    /// An extension and its arguments.
    #[structopt(external_subcommand)]
    Extension(Vec<ffi::OsString>) => crate::commands::extension,
  ]
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct ConfigArgs {
  #[structopt(subcommand)]
  subcmd: ConfigCommand,
}

Command! {ConfigCommand, [
  /// Prints the Nitrokey configuration
  Get => crate::commands::config_get,
  /// Changes the Nitrokey configuration
  Set(ConfigSetArgs) => crate::commands::config_set,
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct ConfigSetArgs {
  /// Sets the numlock option to the given HOTP slot
  #[structopt(short = "n", long)]
  pub numlock: Option<u8>,
  /// Unsets the numlock option
  #[structopt(short = "N", long, conflicts_with("numlock"))]
  pub no_numlock: bool,
  /// Sets the capslock option to the given HOTP slot
  #[structopt(short = "c", long)]
  pub capslock: Option<u8>,
  /// Unsets the capslock option
  #[structopt(short = "C", long, conflicts_with("capslock"))]
  pub no_capslock: bool,
  /// Sets the scrollock option to the given HOTP slot
  #[structopt(short = "s", long)]
  pub scrollock: Option<u8>,
  /// Unsets the scrollock option
  #[structopt(short = "S", long, conflicts_with("scrollock"))]
  pub no_scrollock: bool,
  /// Requires the user PIN to generate one-time passwords
  #[structopt(short = "o", long)]
  pub otp_pin: bool,
  /// Allows one-time password generation without PIN
  #[structopt(short = "O", long, conflicts_with("otp-pin"))]
  pub no_otp_pin: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum ConfigOption<T> {
  Enable(T),
  Disable,
  Ignore,
}

impl<T> ConfigOption<T> {
  pub fn try_from(disable: bool, value: Option<T>, name: &'static str) -> anyhow::Result<Self> {
    if disable {
      anyhow::ensure!(
        value.is_none(),
        "--{name} and --no-{name} are mutually exclusive",
        name = name
      );
      Ok(ConfigOption::Disable)
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

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct EncryptedArgs {
  #[structopt(subcommand)]
  subcmd: EncryptedCommand,
}

Command! {EncryptedCommand, [
  /// Closes the encrypted volume on a Nitrokey Storage
  Close => crate::commands::encrypted_close,
  /// Opens the encrypted volume on a Nitrokey Storage
  Open => crate::commands::encrypted_open,
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct FillArgs {
  /// Checks if a fill operation is already running and show its progress instead of starting a new
  /// operation.
  #[structopt(short, long)]
  attach: bool,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct HiddenArgs {
  #[structopt(subcommand)]
  subcmd: HiddenCommand,
}

Command! {HiddenCommand, [
  /// Closes the hidden volume on a Nitrokey Storage
  Close => crate::commands::hidden_close,
  /// Creates a hidden volume on a Nitrokey Storage
  Create(HiddenCreateArgs) => |ctx, args: HiddenCreateArgs| {
    crate::commands::hidden_create(ctx, args.slot, args.start, args.end)
  },
  /// Opens the hidden volume on a Nitrokey Storage
  Open => crate::commands::hidden_open,
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct HiddenCreateArgs {
  /// The hidden volume slot to use
  pub slot: u8,
  /// The start location of the hidden volume as a percentage of the encrypted volume's size (0-99)
  pub start: u8,
  /// The end location of the hidden volume as a percentage of the encrypted volume's size (1-100)
  pub end: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct ListArgs {
  /// Only print the information that is available without connecting to a device
  #[structopt(short, long)]
  pub no_connect: bool,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpArgs {
  #[structopt(subcommand)]
  subcmd: OtpCommand,
}

Command! {OtpCommand, [
  /// Clears a one-time password slot
  Clear(OtpClearArgs) => |ctx, args: OtpClearArgs| {
    crate::commands::otp_clear(ctx, args.slot, args.algorithm)
  },
  /// Generates a one-time password
  Get(OtpGetArgs) => |ctx, args: OtpGetArgs| {
    crate::commands::otp_get(ctx, args.slot, args.algorithm, args.time)
  },
  /// Configures a one-time password slot
  Set(OtpSetArgs) => crate::commands::otp_set,
  /// Prints the status of the one-time password slots
  Status(OtpStatusArgs) => |ctx, args: OtpStatusArgs| crate::commands::otp_status(ctx, args.all),
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpClearArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = OtpAlgorithm::Totp.as_ref(),
              possible_values = &OtpAlgorithm::all_str())]
  pub algorithm: OtpAlgorithm,
  /// The OTP slot to clear
  pub slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpGetArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = OtpAlgorithm::Totp.as_ref(),
              possible_values = &OtpAlgorithm::all_str())]
  pub algorithm: OtpAlgorithm,
  /// The time to use for TOTP generation (Unix timestamp) [default: system time]
  #[structopt(short, long)]
  pub time: Option<u64>,
  /// The OTP slot to use
  pub slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpSetArgs {
  /// The OTP algorithm to use
  #[structopt(short, long, default_value = OtpAlgorithm::Totp.as_ref(),
              possible_values = &OtpAlgorithm::all_str())]
  pub algorithm: OtpAlgorithm,
  /// The number of digits to use for the one-time password
  #[structopt(short, long, default_value = OtpMode::SixDigits.as_ref(),
              possible_values = &OtpMode::all_str())]
  pub digits: OtpMode,
  /// The counter value for HOTP
  #[structopt(short, long, default_value = "0")]
  pub counter: u64,
  /// The time window for TOTP
  #[structopt(short, long, default_value = "30")]
  pub time_window: u16,
  /// The format of the secret
  #[structopt(short, long, default_value = OtpSecretFormat::Base32.as_ref(),
              possible_values = &OtpSecretFormat::all_str())]
  pub format: OtpSecretFormat,
  /// The OTP slot to use
  pub slot: u8,
  /// The name of the slot
  pub name: String,
  /// The secret to store on the slot as a hexadecimal string (or in the format set with the
  /// --format option)
  pub secret: String,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct OtpStatusArgs {
  /// Shows slots that are not programmed
  #[structopt(short, long)]
  pub all: bool,
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

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PinArgs {
  #[structopt(subcommand)]
  subcmd: PinCommand,
}

Command! {PinCommand, [
  /// Clears the cached PINs
  Clear => crate::commands::pin_clear,
  /// Changes a PIN
  Set(PinSetArgs) => |ctx, args: PinSetArgs| crate::commands::pin_set(ctx, args.pintype),
  /// Unblocks and resets the user PIN
  Unblock => crate::commands::pin_unblock,
]}

Enum! {
  /// PIN type requested from pinentry.
  ///
  /// The available PIN types correspond to the PIN types used by the
  /// Nitrokey devices: user and admin.
  PinType, [
    Admin => "admin",
    User => "user",
  ]
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PinSetArgs {
  /// The PIN type to change
  #[structopt(name = "type", possible_values = &PinType::all_str())]
  pub pintype: PinType,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsArgs {
  #[structopt(subcommand)]
  subcmd: PwsCommand,
}

Command! {PwsCommand, [
  /// Clears a password safe slot
  Clear(PwsClearArgs) => |ctx, args: PwsClearArgs| crate::commands::pws_clear(ctx, args.slot),
  /// Reads a password safe slot
  Get(PwsGetArgs) => |ctx, args: PwsGetArgs| {
    crate::commands::pws_get(ctx, args.slot, args.name, args.login, args.password, args.quiet)
  },
  /// Writes a password safe slot
  Set(PwsSetArgs) => |ctx, args: PwsSetArgs| {
    crate::commands::pws_set(ctx, args.slot, &args.name, &args.login, &args.password)
  },
  /// Prints the status of the password safe slots
  Status(PwsStatusArgs) => |ctx, args: PwsStatusArgs| crate::commands::pws_status(ctx, args.all),
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsClearArgs {
  /// The PWS slot to clear
  pub slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsGetArgs {
  /// Shows the name stored on the slot
  #[structopt(short, long)]
  pub name: bool,
  /// Shows the login stored on the slot
  #[structopt(short, long)]
  pub login: bool,
  /// Shows the password stored on the slot
  #[structopt(short, long)]
  pub password: bool,
  /// Prints the stored data without description
  #[structopt(short, long)]
  pub quiet: bool,
  /// The PWS slot to read
  pub slot: u8,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsSetArgs {
  /// The PWS slot to write
  pub slot: u8,
  /// The name to store on the slot
  pub name: String,
  /// The login to store on the slot
  pub login: String,
  /// The password to store on the slot
  pub password: String,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct PwsStatusArgs {
  /// Shows slots that are not programmed
  #[structopt(short, long)]
  pub all: bool,
}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct UnencryptedArgs {
  #[structopt(subcommand)]
  subcmd: UnencryptedCommand,
}

Command! {UnencryptedCommand, [
  /// Changes the configuration of the unencrypted volume on a Nitrokey Storage
  Set(UnencryptedSetArgs) => |ctx, args: UnencryptedSetArgs| {
    crate::commands::unencrypted_set(ctx, args.mode)
  },
]}

#[derive(Debug, PartialEq, structopt::StructOpt)]
pub struct UnencryptedSetArgs {
  /// The mode to change to
  #[structopt(name = "type", possible_values = &UnencryptedVolumeMode::all_str())]
  pub mode: UnencryptedVolumeMode,
}

Enum! {UnencryptedVolumeMode, [
  ReadWrite => "read-write",
  ReadOnly => "read-only",
]}
