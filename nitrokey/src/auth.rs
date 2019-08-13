// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::ops;
use std::os::raw::c_char;
use std::os::raw::c_int;

use nitrokey_sys;

use crate::config::{Config, RawConfig};
use crate::device::{Device, DeviceWrapper, Pro, Storage};
use crate::error::Error;
use crate::otp::{ConfigureOtp, GenerateOtp, OtpMode, OtpSlotData, RawOtpSlotData};
use crate::util::{generate_password, get_command_result, get_cstring, result_from_string};

static TEMPORARY_PASSWORD_LENGTH: usize = 25;

/// Provides methods to authenticate as a user or as an admin using a PIN.  The authenticated
/// methods will consume the current device instance.  On success, they return the authenticated
/// device.  Otherwise, they return the current unauthenticated device and the error code.
pub trait Authenticate {
    /// Performs user authentication.  This method consumes the device.  If successful, an
    /// authenticated device is returned.  Otherwise, the current unauthenticated device and the
    /// error are returned.
    ///
    /// This method generates a random temporary password that is used for all operations that
    /// require user access.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided user password contains a null byte
    /// - [`RngError`][] if the generation of the temporary password failed
    /// - [`WrongPassword`][] if the provided user password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Authenticate, DeviceWrapper, User};
    /// # use nitrokey::Error;
    ///
    /// fn perform_user_task(device: &User<DeviceWrapper>) {}
    /// fn perform_other_task(device: &DeviceWrapper) {}
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// let device = match device.authenticate_user("123456") {
    ///     Ok(user) => {
    ///         perform_user_task(&user);
    ///         user.device()
    ///     },
    ///     Err((device, err)) => {
    ///         eprintln!("Could not authenticate as user: {}", err);
    ///         device
    ///     },
    /// };
    /// perform_other_task(&device);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`RngError`]: enum.CommandError.html#variant.RngError
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    fn authenticate_user(self, password: &str) -> Result<User<Self>, (Self, Error)>
    where
        Self: Device + Sized;

    /// Performs admin authentication.  This method consumes the device.  If successful, an
    /// authenticated device is returned.  Otherwise, the current unauthenticated device and the
    /// error are returned.
    ///
    /// This method generates a random temporary password that is used for all operations that
    /// require admin access.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided admin password contains a null byte
    /// - [`RngError`][] if the generation of the temporary password failed
    /// - [`WrongPassword`][] if the provided admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Authenticate, Admin, DeviceWrapper};
    /// # use nitrokey::Error;
    ///
    /// fn perform_admin_task(device: &Admin<DeviceWrapper>) {}
    /// fn perform_other_task(device: &DeviceWrapper) {}
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// let device = match device.authenticate_admin("123456") {
    ///     Ok(admin) => {
    ///         perform_admin_task(&admin);
    ///         admin.device()
    ///     },
    ///     Err((device, err)) => {
    ///         eprintln!("Could not authenticate as admin: {}", err);
    ///         device
    ///     },
    /// };
    /// perform_other_task(&device);
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`RngError`]: enum.CommandError.html#variant.RngError
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    fn authenticate_admin(self, password: &str) -> Result<Admin<Self>, (Self, Error)>
    where
        Self: Device + Sized;
}

trait AuthenticatedDevice<T> {
    fn new(device: T, temp_password: Vec<u8>) -> Self;

    fn temp_password_ptr(&self) -> *const c_char;
}

/// A Nitrokey device with user authentication.
///
/// To obtain an instance of this struct, use the [`authenticate_user`][] method from the
/// [`Authenticate`][] trait.  To get back to an unauthenticated device, use the [`device`][]
/// method.
///
/// [`Authenticate`]: trait.Authenticate.html
/// [`authenticate_admin`]: trait.Authenticate.html#method.authenticate_admin
/// [`device`]: #method.device
#[derive(Debug)]
pub struct User<T: Device> {
    device: T,
    temp_password: Vec<u8>,
}

/// A Nitrokey device with admin authentication.
///
/// To obtain an instance of this struct, use the [`authenticate_admin`][] method from the
/// [`Authenticate`][] trait.  To get back to an unauthenticated device, use the [`device`][]
/// method.
///
/// [`Authenticate`]: trait.Authenticate.html
/// [`authenticate_admin`]: trait.Authenticate.html#method.authenticate_admin
/// [`device`]: #method.device
#[derive(Debug)]
pub struct Admin<T: Device> {
    device: T,
    temp_password: Vec<u8>,
}

fn authenticate<D, A, T>(device: D, password: &str, callback: T) -> Result<A, (D, Error)>
where
    D: Device,
    A: AuthenticatedDevice<D>,
    T: Fn(*const c_char, *const c_char) -> c_int,
{
    let temp_password = match generate_password(TEMPORARY_PASSWORD_LENGTH) {
        Ok(temp_password) => temp_password,
        Err(err) => return Err((device, err)),
    };
    let password = match get_cstring(password) {
        Ok(password) => password,
        Err(err) => return Err((device, err)),
    };
    let password_ptr = password.as_ptr();
    let temp_password_ptr = temp_password.as_ptr() as *const c_char;
    match callback(password_ptr, temp_password_ptr) {
        0 => Ok(A::new(device, temp_password)),
        rv => Err((device, Error::from(rv))),
    }
}

fn authenticate_user_wrapper<T, C>(
    device: T,
    constructor: C,
    password: &str,
) -> Result<User<DeviceWrapper>, (DeviceWrapper, Error)>
where
    T: Device,
    C: Fn(T) -> DeviceWrapper,
{
    let result = device.authenticate_user(password);
    match result {
        Ok(user) => Ok(User::new(constructor(user.device), user.temp_password)),
        Err((device, err)) => Err((constructor(device), err)),
    }
}

fn authenticate_admin_wrapper<T, C>(
    device: T,
    constructor: C,
    password: &str,
) -> Result<Admin<DeviceWrapper>, (DeviceWrapper, Error)>
where
    T: Device,
    C: Fn(T) -> DeviceWrapper,
{
    let result = device.authenticate_admin(password);
    match result {
        Ok(user) => Ok(Admin::new(constructor(user.device), user.temp_password)),
        Err((device, err)) => Err((constructor(device), err)),
    }
}

impl<T: Device> User<T> {
    /// Forgets the user authentication and returns an unauthenticated device.  This method
    /// consumes the authenticated device.  It does not perform any actual commands on the
    /// Nitrokey.
    pub fn device(self) -> T {
        self.device
    }
}

impl<T: Device> ops::Deref for User<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl<T: Device> ops::DerefMut for User<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.device
    }
}

impl<T: Device> GenerateOtp for User<T> {
    fn get_hotp_code(&mut self, slot: u8) -> Result<String, Error> {
        result_from_string(unsafe {
            nitrokey_sys::NK_get_hotp_code_PIN(slot, self.temp_password_ptr())
        })
    }

    fn get_totp_code(&self, slot: u8) -> Result<String, Error> {
        result_from_string(unsafe {
            nitrokey_sys::NK_get_totp_code_PIN(slot, 0, 0, 0, self.temp_password_ptr())
        })
    }
}

impl<T: Device> AuthenticatedDevice<T> for User<T> {
    fn new(device: T, temp_password: Vec<u8>) -> Self {
        User {
            device,
            temp_password,
        }
    }

    fn temp_password_ptr(&self) -> *const c_char {
        self.temp_password.as_ptr() as *const c_char
    }
}

impl<T: Device> ops::Deref for Admin<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl<T: Device> ops::DerefMut for Admin<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.device
    }
}

impl<T: Device> Admin<T> {
    /// Forgets the user authentication and returns an unauthenticated device.  This method
    /// consumes the authenticated device.  It does not perform any actual commands on the
    /// Nitrokey.
    pub fn device(self) -> T {
        self.device
    }

    /// Writes the given configuration to the Nitrokey device.
    ///
    /// # Errors
    ///
    /// - [`InvalidSlot`][] if the provided numlock, capslock or scrolllock slot is larger than two
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::{Authenticate, Config};
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
    /// let config = Config::new(None, None, None, false);
    /// match device.authenticate_admin("12345678") {
    ///     Ok(mut admin) => {
    ///         admin.write_config(config);
    ///         ()
    ///     },
    ///     Err((_, err)) => eprintln!("Could not authenticate as admin: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidSlot`]: enum.LibraryError.html#variant.InvalidSlot
    pub fn write_config(&mut self, config: Config) -> Result<(), Error> {
        let raw_config = RawConfig::try_from(config)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_write_config(
                raw_config.numlock,
                raw_config.capslock,
                raw_config.scrollock,
                raw_config.user_password,
                false,
                self.temp_password_ptr(),
            )
        })
    }
}

impl<T: Device> ConfigureOtp for Admin<T> {
    fn write_hotp_slot(&mut self, data: OtpSlotData, counter: u64) -> Result<(), Error> {
        let raw_data = RawOtpSlotData::new(data)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_write_hotp_slot(
                raw_data.number,
                raw_data.name.as_ptr(),
                raw_data.secret.as_ptr(),
                counter,
                raw_data.mode == OtpMode::EightDigits,
                raw_data.use_enter,
                raw_data.use_token_id,
                raw_data.token_id.as_ptr(),
                self.temp_password_ptr(),
            )
        })
    }

    fn write_totp_slot(&mut self, data: OtpSlotData, time_window: u16) -> Result<(), Error> {
        let raw_data = RawOtpSlotData::new(data)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_write_totp_slot(
                raw_data.number,
                raw_data.name.as_ptr(),
                raw_data.secret.as_ptr(),
                time_window,
                raw_data.mode == OtpMode::EightDigits,
                raw_data.use_enter,
                raw_data.use_token_id,
                raw_data.token_id.as_ptr(),
                self.temp_password_ptr(),
            )
        })
    }

    fn erase_hotp_slot(&mut self, slot: u8) -> Result<(), Error> {
        get_command_result(unsafe {
            nitrokey_sys::NK_erase_hotp_slot(slot, self.temp_password_ptr())
        })
    }

    fn erase_totp_slot(&mut self, slot: u8) -> Result<(), Error> {
        get_command_result(unsafe {
            nitrokey_sys::NK_erase_totp_slot(slot, self.temp_password_ptr())
        })
    }
}

impl<T: Device> AuthenticatedDevice<T> for Admin<T> {
    fn new(device: T, temp_password: Vec<u8>) -> Self {
        Admin {
            device,
            temp_password,
        }
    }

    fn temp_password_ptr(&self) -> *const c_char {
        self.temp_password.as_ptr() as *const c_char
    }
}

impl Authenticate for DeviceWrapper {
    fn authenticate_user(self, password: &str) -> Result<User<Self>, (Self, Error)> {
        match self {
            DeviceWrapper::Storage(storage) => {
                authenticate_user_wrapper(storage, DeviceWrapper::Storage, password)
            }
            DeviceWrapper::Pro(pro) => authenticate_user_wrapper(pro, DeviceWrapper::Pro, password),
        }
    }

    fn authenticate_admin(self, password: &str) -> Result<Admin<Self>, (Self, Error)> {
        match self {
            DeviceWrapper::Storage(storage) => {
                authenticate_admin_wrapper(storage, DeviceWrapper::Storage, password)
            }
            DeviceWrapper::Pro(pro) => {
                authenticate_admin_wrapper(pro, DeviceWrapper::Pro, password)
            }
        }
    }
}

impl Authenticate for Pro {
    fn authenticate_user(self, password: &str) -> Result<User<Self>, (Self, Error)> {
        authenticate(self, password, |password_ptr, temp_password_ptr| unsafe {
            nitrokey_sys::NK_user_authenticate(password_ptr, temp_password_ptr)
        })
    }

    fn authenticate_admin(self, password: &str) -> Result<Admin<Self>, (Self, Error)> {
        authenticate(self, password, |password_ptr, temp_password_ptr| unsafe {
            nitrokey_sys::NK_first_authenticate(password_ptr, temp_password_ptr)
        })
    }
}

impl Authenticate for Storage {
    fn authenticate_user(self, password: &str) -> Result<User<Self>, (Self, Error)> {
        authenticate(self, password, |password_ptr, temp_password_ptr| unsafe {
            nitrokey_sys::NK_user_authenticate(password_ptr, temp_password_ptr)
        })
    }

    fn authenticate_admin(self, password: &str) -> Result<Admin<Self>, (Self, Error)> {
        authenticate(self, password, |password_ptr, temp_password_ptr| unsafe {
            nitrokey_sys::NK_first_authenticate(password_ptr, temp_password_ptr)
        })
    }
}
