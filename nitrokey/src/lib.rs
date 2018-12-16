//! Provides access to a Nitrokey device using the native libnitrokey API.
//!
//! # Usage
//!
//! Operations on the Nitrokey require different authentication levels.  Some operations can be
//! performed without authentication, some require user access, and some require admin access.
//! This is modelled using the types [`User`][] and [`Admin`][].
//!
//! Use [`connect`][] to connect to any Nitrokey device.  The method will return a
//! [`DeviceWrapper`][] that abstracts over the supported Nitrokey devices.  You can also use
//! [`Pro::connect`][] or [`Storage::connect`][] to connect to a specific device.
//!
//! You can then use [`authenticate_user`][] or [`authenticate_admin`][] to get an authenticated
//! device that can perform operations that require authentication.  You can use [`device`][] to go
//! back to the unauthenticated device.
//!
//! This makes sure that you can only execute a command if you have the required access rights.
//! Otherwise, your code will not compile.  The only exception are the methods to generate one-time
//! passwords – [`get_hotp_code`][] and [`get_totp_code`][].  Depending on the stick configuration,
//! these operations are available without authentication or with user authentication.
//!
//! # Examples
//!
//! Connect to any Nitrokey and print its serial number:
//!
//! ```no_run
//! use nitrokey::Device;
//! # use nitrokey::CommandError;
//!
//! # fn try_main() -> Result<(), CommandError> {
//! let device = nitrokey::connect()?;
//! println!("{}", device.get_serial_number()?);
//! #     Ok(())
//! # }
//! ```
//!
//! Configure an HOTP slot:
//!
//! ```no_run
//! use nitrokey::{Authenticate, ConfigureOtp, OtpMode, OtpSlotData};
//! # use nitrokey::CommandError;
//!
//! # fn try_main() -> Result<(), (CommandError)> {
//! let device = nitrokey::connect()?;
//! let slot_data = OtpSlotData::new(1, "test", "01234567890123456689", OtpMode::SixDigits);
//! match device.authenticate_admin("12345678") {
//!     Ok(admin) => {
//!         match admin.write_hotp_slot(slot_data, 0) {
//!             Ok(()) => println!("Successfully wrote slot."),
//!             Err(err) => println!("Could not write slot: {}", err),
//!         }
//!     },
//!     Err((_, err)) => println!("Could not authenticate as admin: {}", err),
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! Generate an HOTP one-time password:
//!
//! ```no_run
//! use nitrokey::{Device, GenerateOtp};
//! # use nitrokey::CommandError;
//!
//! # fn try_main() -> Result<(), (CommandError)> {
//! let device = nitrokey::connect()?;
//! match device.get_hotp_code(1) {
//!     Ok(code) => println!("Generated HOTP code: {}", code),
//!     Err(err) => println!("Could not generate HOTP code: {}", err),
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! [`authenticate_admin`]: trait.Authenticate.html#method.authenticate_admin
//! [`authenticate_user`]: trait.Authenticate.html#method.authenticate_user
//! [`connect`]: fn.connect.html
//! [`Pro::connect`]: struct.Pro.html#fn.connect.html
//! [`Storage::connect`]: struct.Storage.html#fn.connect.html
//! [`device`]: struct.User.html#method.device
//! [`get_hotp_code`]: trait.GenerateOtp.html#method.get_hotp_code
//! [`get_totp_code`]: trait.GenerateOtp.html#method.get_totp_code
//! [`Admin`]: struct.Admin.html
//! [`DeviceWrapper`]: enum.DeviceWrapper.html
//! [`User`]: struct.User.html

extern crate libc;
extern crate nitrokey_sys;
extern crate rand;

mod auth;
mod config;
mod device;
mod otp;
mod pws;
#[cfg(test)]
mod tests;
mod util;

pub use auth::{Admin, Authenticate, User};
pub use config::Config;
pub use device::{connect, Device, DeviceWrapper, Pro, Storage, StorageStatus, VolumeStatus};
pub use otp::{ConfigureOtp, GenerateOtp, OtpMode, OtpSlotData};
pub use pws::{GetPasswordSafe, PasswordSafe, SLOT_COUNT};
pub use util::{CommandError, LogLevel};

/// Enables or disables debug output.  Calling this method with `true` is equivalent to setting the
/// log level to `Debug`; calling it with `false` is equivalent to the log level `Error` (see
/// [`set_log_level`][]).
///
/// If debug output is enabled, detailed information about the communication with the Nitrokey
/// device is printed to the standard output.
///
/// [`set_log_level`]: fn.set_log_level.html
pub fn set_debug(state: bool) {
    unsafe {
        nitrokey_sys::NK_set_debug(state);
    }
}

/// Sets the log level for libnitrokey.  All log messages are written to the standard error stream.
/// Setting the log level enables all log messages on the same or on a higher log level.
pub fn set_log_level(level: LogLevel) {
    unsafe {
        nitrokey_sys::NK_set_debug_level(level.into());
    }
}
