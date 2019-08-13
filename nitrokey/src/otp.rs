// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::ffi::CString;

use nitrokey_sys;

use crate::error::Error;
use crate::util::{get_command_result, get_cstring, result_from_string};

/// Modes for one-time password generation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OtpMode {
    /// Generate one-time passwords with six digits.
    SixDigits,
    /// Generate one-time passwords with eight digits.
    EightDigits,
}

/// Provides methods to configure and erase OTP slots on a Nitrokey device.
pub trait ConfigureOtp {
    /// Configure an HOTP slot with the given data and set the HOTP counter to the given value
    /// (default 0).
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    /// - [`InvalidString`][] if the provided token ID contains a null byte
    /// - [`NoName`][] if the provided name is empty
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Authenticate, ConfigureOtp, OtpMode, OtpSlotData};
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// let slot_data = OtpSlotData::new(1, "test", "01234567890123456689", OtpMode::SixDigits);
    /// match device.authenticate_admin("12345678") {
    ///     Ok(mut admin) => {
    ///         match admin.write_hotp_slot(slot_data, 0) {
    ///             Ok(()) => println!("Successfully wrote slot."),
    ///             Err(err) => eprintln!("Could not write slot: {}", err),
    ///         }
    ///     },
    ///     Err((_, err)) => eprintln!("Could not authenticate as admin: {}", err),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`NoName`]: enum.CommandError.html#variant.NoName
    fn write_hotp_slot(&mut self, data: OtpSlotData, counter: u64) -> Result<(), Error>;

    /// Configure a TOTP slot with the given data and set the TOTP time window to the given value
    /// (default 30).
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    /// - [`InvalidString`][] if the provided token ID contains a null byte
    /// - [`NoName`][] if the provided name is empty
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Authenticate, ConfigureOtp, OtpMode, OtpSlotData};
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// let slot_data = OtpSlotData::new(1, "test", "01234567890123456689", OtpMode::EightDigits);
    /// match device.authenticate_admin("12345678") {
    ///     Ok(mut admin) => {
    ///         match admin.write_totp_slot(slot_data, 30) {
    ///             Ok(()) => println!("Successfully wrote slot."),
    ///             Err(err) => eprintln!("Could not write slot: {}", err),
    ///         }
    ///     },
    ///     Err((_, err)) => eprintln!("Could not authenticate as admin: {}", err),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`NoName`]: enum.CommandError.html#variant.NoName
    fn write_totp_slot(&mut self, data: OtpSlotData, time_window: u16) -> Result<(), Error>;

    /// Erases an HOTP slot.
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Authenticate, ConfigureOtp};
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// match device.authenticate_admin("12345678") {
    ///     Ok(mut admin) => {
    ///         match admin.erase_hotp_slot(1) {
    ///             Ok(()) => println!("Successfully erased slot."),
    ///             Err(err) => eprintln!("Could not erase slot: {}", err),
    ///         }
    ///     },
    ///     Err((_, err)) => eprintln!("Could not authenticate as admin: {}", err),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    fn erase_hotp_slot(&mut self, slot: u8) -> Result<(), Error>;

    /// Erases a TOTP slot.
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Authenticate, ConfigureOtp};
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// match device.authenticate_admin("12345678") {
    ///     Ok(mut admin) => {
    ///         match admin.erase_totp_slot(1) {
    ///             Ok(()) => println!("Successfully erased slot."),
    ///             Err(err) => eprintln!("Could not erase slot: {}", err),
    ///         }
    ///     },
    ///     Err((_, err)) => eprintln!("Could not authenticate as admin: {}", err),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    fn erase_totp_slot(&mut self, slot: u8) -> Result<(), Error>;
}

/// Provides methods to generate OTP codes and to query OTP slots on a Nitrokey
/// device.
pub trait GenerateOtp {
    /// Sets the time on the Nitrokey.
    ///
    /// `time` is the number of seconds since January 1st, 1970 (Unix timestamp).  Unless `force`
    /// is set to `true`, this command fails if the timestamp on the device is larger than the
    /// given timestamp or if it is zero.
    ///
    /// The time is used for TOTP generation (see [`get_totp_code`][]).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time;
    /// use nitrokey::GenerateOtp;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::connect()?;
    /// let time = time::SystemTime::now().duration_since(time::UNIX_EPOCH);
    /// match time {
    ///     Ok(time) => device.set_time(time.as_secs(), false)?,
    ///     Err(_) => eprintln!("The system time is before the Unix epoch!"),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// - [`Timestamp`][] if the time could not be set
    ///
    /// [`get_totp_code`]: #method.get_totp_code
    /// [`Timestamp`]: enum.CommandError.html#variant.Timestamp
    fn set_time(&mut self, time: u64, force: bool) -> Result<(), Error> {
        let result = if force {
            unsafe { nitrokey_sys::NK_totp_set_time(time) }
        } else {
            unsafe { nitrokey_sys::NK_totp_set_time_soft(time) }
        };
        get_command_result(result)
    }

    /// Returns the name of the given HOTP slot.
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    /// - [`SlotNotProgrammed`][] if the given slot is not configured
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{CommandError, Error, GenerateOtp};
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// match device.get_hotp_slot_name(1) {
    ///     Ok(name) => println!("HOTP slot 1: {}", name),
    ///     Err(Error::CommandError(CommandError::SlotNotProgrammed)) => eprintln!("HOTP slot 1 not programmed"),
    ///     Err(err) => eprintln!("Could not get slot name: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    /// [`SlotNotProgrammed`]: enum.CommandError.html#variant.SlotNotProgrammed
    fn get_hotp_slot_name(&self, slot: u8) -> Result<String, Error> {
        result_from_string(unsafe { nitrokey_sys::NK_get_hotp_slot_name(slot) })
    }

    /// Returns the name of the given TOTP slot.
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    /// - [`SlotNotProgrammed`][] if the given slot is not configured
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{CommandError, Error, GenerateOtp};
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// match device.get_totp_slot_name(1) {
    ///     Ok(name) => println!("TOTP slot 1: {}", name),
    ///     Err(Error::CommandError(CommandError::SlotNotProgrammed)) => eprintln!("TOTP slot 1 not programmed"),
    ///     Err(err) => eprintln!("Could not get slot name: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    /// [`SlotNotProgrammed`]: enum.CommandError.html#variant.SlotNotProgrammed
    fn get_totp_slot_name(&self, slot: u8) -> Result<String, Error> {
        result_from_string(unsafe { nitrokey_sys::NK_get_totp_slot_name(slot) })
    }

    /// Generates an HOTP code on the given slot.  This operation may require user authorization,
    /// depending on the device configuration (see [`get_config`][]).
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    /// - [`NotAuthorized`][] if OTP generation requires user authentication
    /// - [`SlotNotProgrammed`][] if the given slot is not configured
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::GenerateOtp;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::connect()?;
    /// let code = device.get_hotp_code(1)?;
    /// println!("Generated HOTP code on slot 1: {}", code);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`get_config`]: trait.Device.html#method.get_config
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    /// [`NotAuthorized`]: enum.CommandError.html#variant.NotAuthorized
    /// [`SlotNotProgrammed`]: enum.CommandError.html#variant.SlotNotProgrammed
    fn get_hotp_code(&mut self, slot: u8) -> Result<String, Error> {
        result_from_string(unsafe { nitrokey_sys::NK_get_hotp_code(slot) })
    }

    /// Generates a TOTP code on the given slot.  This operation may require user authorization,
    /// depending on the device configuration (see [`get_config`][]).
    ///
    /// To make sure that the Nitrokey’s time is in sync, consider calling [`set_time`][] before
    /// calling this method.
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if there is no slot with the given number
    /// - [`NotAuthorized`][] if OTP generation requires user authentication
    /// - [`SlotNotProgrammed`][] if the given slot is not configured
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time;
    /// use nitrokey::GenerateOtp;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::connect()?;
    /// let time = time::SystemTime::now().duration_since(time::UNIX_EPOCH);
    /// match time {
    ///     Ok(time) => {
    ///         device.set_time(time.as_secs(), false)?;
    ///         let code = device.get_totp_code(1)?;
    ///         println!("Generated TOTP code on slot 1: {}", code);
    ///     },
    ///     Err(_) => eprintln!("Timestamps before 1970-01-01 are not supported!"),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`set_time`]: #method.set_time
    /// [`get_config`]: trait.Device.html#method.get_config
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    /// [`NotAuthorized`]: enum.CommandError.html#variant.NotAuthorized
    /// [`SlotNotProgrammed`]: enum.CommandError.html#variant.SlotNotProgrammed
    fn get_totp_code(&self, slot: u8) -> Result<String, Error> {
        result_from_string(unsafe { nitrokey_sys::NK_get_totp_code(slot, 0, 0, 0) })
    }
}

/// The configuration for an OTP slot.
#[derive(Debug)]
pub struct OtpSlotData {
    /// The number of the slot – must be less than three for HOTP and less than 15 for TOTP.
    pub number: u8,
    /// The name of the slot – must not be empty.
    pub name: String,
    /// The secret for the slot.
    pub secret: String,
    /// The OTP generation mode.
    pub mode: OtpMode,
    /// If true, press the enter key after sending an OTP code using double-pressed
    /// numlock, capslock or scrolllock.
    pub use_enter: bool,
    /// Set the token ID, see [OATH Token Identifier Specification][tokspec], section “Class A”.
    ///
    /// [tokspec]: https://openauthentication.org/token-specs/
    pub token_id: Option<String>,
}

#[derive(Debug)]
pub struct RawOtpSlotData {
    pub number: u8,
    pub name: CString,
    pub secret: CString,
    pub mode: OtpMode,
    pub use_enter: bool,
    pub use_token_id: bool,
    pub token_id: CString,
}

impl OtpSlotData {
    /// Constructs a new instance of this struct.
    pub fn new<S: Into<String>, T: Into<String>>(
        number: u8,
        name: S,
        secret: T,
        mode: OtpMode,
    ) -> OtpSlotData {
        OtpSlotData {
            number,
            name: name.into(),
            secret: secret.into(),
            mode,
            use_enter: false,
            token_id: None,
        }
    }

    /// Enables pressing the enter key after sending an OTP code using double-pressed numlock,
    /// capslock or scrollock.
    pub fn use_enter(mut self) -> OtpSlotData {
        self.use_enter = true;
        self
    }

    /// Sets the token ID, see [OATH Token Identifier Specification][tokspec], section “Class A”.
    ///
    /// [tokspec]: https://openauthentication.org/token-specs/
    pub fn token_id<S: Into<String>>(mut self, id: S) -> OtpSlotData {
        self.token_id = Some(id.into());
        self
    }
}

impl RawOtpSlotData {
    pub fn new(data: OtpSlotData) -> Result<RawOtpSlotData, Error> {
        let name = get_cstring(data.name)?;
        let secret = get_cstring(data.secret)?;
        let use_token_id = data.token_id.is_some();
        let token_id = get_cstring(data.token_id.unwrap_or_else(String::new))?;

        Ok(RawOtpSlotData {
            number: data.number,
            name,
            secret,
            mode: data.mode,
            use_enter: data.use_enter,
            use_token_id,
            token_id,
        })
    }
}
