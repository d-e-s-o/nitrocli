// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

mod pro;
mod storage;
mod wrapper;

use std::fmt;

use libc;
use nitrokey_sys;

use crate::auth::Authenticate;
use crate::config::{Config, RawConfig};
use crate::error::{CommunicationError, Error};
use crate::otp::GenerateOtp;
use crate::pws::GetPasswordSafe;
use crate::util::{
    get_command_result, get_cstring, get_last_error, result_from_string, result_or_error,
};

pub use pro::Pro;
pub use storage::{
    SdCardData, Storage, StorageProductionInfo, StorageStatus, VolumeMode, VolumeStatus,
};
pub use wrapper::DeviceWrapper;

/// Available Nitrokey models.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Model {
    /// The Nitrokey Storage.
    Storage,
    /// The Nitrokey Pro.
    Pro,
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            Model::Pro => "Pro",
            Model::Storage => "Storage",
        })
    }
}

/// A firmware version for a Nitrokey device.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirmwareVersion {
    /// The major firmware version, e. g. 0 in v0.40.
    pub major: u8,
    /// The minor firmware version, e. g. 40 in v0.40.
    pub minor: u8,
}

impl fmt::Display for FirmwareVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}.{}", self.major, self.minor)
    }
}

/// A Nitrokey device.
///
/// This trait provides the commands that can be executed without authentication and that are
/// present on all supported Nitrokey devices.
pub trait Device<'a>: Authenticate<'a> + GetPasswordSafe<'a> + GenerateOtp + fmt::Debug {
    /// Returns the [`Manager`][] instance that has been used to connect to this device.
    ///
    /// # Example
    ///
    /// ```
    /// use nitrokey::{Device, DeviceWrapper};
    ///
    /// fn do_something(device: DeviceWrapper) {
    ///     // reconnect to any device
    ///     let manager = device.into_manager();
    ///     let device = manager.connect();
    ///     // do something with the device
    ///     // ...
    /// }
    ///
    /// match nitrokey::take()?.connect() {
    ///     Ok(device) => do_something(device),
    ///     Err(err) => println!("Could not connect to a Nitrokey: {}", err),
    /// }
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    fn into_manager(self) -> &'a mut crate::Manager;

    /// Returns the model of the connected Nitrokey device.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect()?;
    /// println!("Connected to a Nitrokey {}", device.get_model());
    /// #    Ok(())
    /// # }
    fn get_model(&self) -> Model;

    /// Returns the serial number of the Nitrokey device.  The serial number is the string
    /// representation of a hex number.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect()?;
    /// match device.get_serial_number() {
    ///     Ok(number) => println!("serial no: {}", number),
    ///     Err(err) => eprintln!("Could not get serial number: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    fn get_serial_number(&self) -> Result<String, Error> {
        result_from_string(unsafe { nitrokey_sys::NK_device_serial_number() })
    }

    /// Returns the number of remaining authentication attempts for the user.  The total number of
    /// available attempts is three.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect()?;
    /// let count = device.get_user_retry_count();
    /// match device.get_user_retry_count() {
    ///     Ok(count) => println!("{} remaining authentication attempts (user)", count),
    ///     Err(err) => eprintln!("Could not get user retry count: {}", err),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    fn get_user_retry_count(&self) -> Result<u8, Error> {
        result_or_error(unsafe { nitrokey_sys::NK_get_user_retry_count() })
    }

    /// Returns the number of remaining authentication attempts for the admin.  The total number of
    /// available attempts is three.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect()?;
    /// let count = device.get_admin_retry_count();
    /// match device.get_admin_retry_count() {
    ///     Ok(count) => println!("{} remaining authentication attempts (admin)", count),
    ///     Err(err) => eprintln!("Could not get admin retry count: {}", err),
    /// }
    /// #     Ok(())
    /// # }
    /// ```
    fn get_admin_retry_count(&self) -> Result<u8, Error> {
        result_or_error(unsafe { nitrokey_sys::NK_get_admin_retry_count() })
    }

    /// Returns the firmware version.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect()?;
    /// match device.get_firmware_version() {
    ///     Ok(version) => println!("Firmware version: {}", version),
    ///     Err(err) => eprintln!("Could not access firmware version: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    fn get_firmware_version(&self) -> Result<FirmwareVersion, Error> {
        let major = result_or_error(unsafe { nitrokey_sys::NK_get_major_firmware_version() })?;
        let minor = result_or_error(unsafe { nitrokey_sys::NK_get_minor_firmware_version() })?;
        Ok(FirmwareVersion { major, minor })
    }

    /// Returns the current configuration of the Nitrokey device.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect()?;
    /// let config = device.get_config()?;
    /// println!("numlock binding:          {:?}", config.numlock);
    /// println!("capslock binding:         {:?}", config.capslock);
    /// println!("scrollock binding:        {:?}", config.scrollock);
    /// println!("require password for OTP: {:?}", config.user_password);
    /// #     Ok(())
    /// # }
    /// ```
    fn get_config(&self) -> Result<Config, Error> {
        let config_ptr = unsafe { nitrokey_sys::NK_read_config() };
        if config_ptr.is_null() {
            return Err(get_last_error());
        }
        let config_array_ptr = config_ptr as *const [u8; 5];
        let raw_config = unsafe { RawConfig::from(*config_array_ptr) };
        unsafe { libc::free(config_ptr as *mut libc::c_void) };
        Ok(raw_config.into())
    }

    /// Changes the administrator PIN.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the current admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect()?;
    /// match device.change_admin_pin("12345678", "12345679") {
    ///     Ok(()) => println!("Updated admin PIN."),
    ///     Err(err) => eprintln!("Failed to update admin PIN: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    fn change_admin_pin(&mut self, current: &str, new: &str) -> Result<(), Error> {
        let current_string = get_cstring(current)?;
        let new_string = get_cstring(new)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_change_admin_PIN(current_string.as_ptr(), new_string.as_ptr())
        })
    }

    /// Changes the user PIN.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the current user password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect()?;
    /// match device.change_user_pin("123456", "123457") {
    ///     Ok(()) => println!("Updated admin PIN."),
    ///     Err(err) => eprintln!("Failed to update admin PIN: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    fn change_user_pin(&mut self, current: &str, new: &str) -> Result<(), Error> {
        let current_string = get_cstring(current)?;
        let new_string = get_cstring(new)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_change_user_PIN(current_string.as_ptr(), new_string.as_ptr())
        })
    }

    /// Unlocks the user PIN after three failed login attempts and sets it to the given value.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect()?;
    /// match device.unlock_user_pin("12345678", "123456") {
    ///     Ok(()) => println!("Unlocked user PIN."),
    ///     Err(err) => eprintln!("Failed to unlock user PIN: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    fn unlock_user_pin(&mut self, admin_pin: &str, user_pin: &str) -> Result<(), Error> {
        let admin_pin_string = get_cstring(admin_pin)?;
        let user_pin_string = get_cstring(user_pin)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_unlock_user_password(
                admin_pin_string.as_ptr(),
                user_pin_string.as_ptr(),
            )
        })
    }

    /// Locks the Nitrokey device.
    ///
    /// This disables the password store if it has been unlocked.  On the Nitrokey Storage, this
    /// also disables the volumes if they have been enabled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect()?;
    /// match device.lock() {
    ///     Ok(()) => println!("Locked the Nitrokey device."),
    ///     Err(err) => eprintln!("Could not lock the Nitrokey device: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    fn lock(&mut self) -> Result<(), Error> {
        get_command_result(unsafe { nitrokey_sys::NK_lock_device() })
    }

    /// Performs a factory reset on the Nitrokey device.
    ///
    /// This commands performs a factory reset on the smart card (like the factory reset via `gpg
    /// --card-edit`) and then clears the flash memory (password safe, one-time passwords etc.).
    /// After a factory reset, [`build_aes_key`][] has to be called before the password safe or the
    /// encrypted volume can be used.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided password contains a null byte
    /// - [`WrongPassword`][] if the admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect()?;
    /// match device.factory_reset("12345678") {
    ///     Ok(()) => println!("Performed a factory reset."),
    ///     Err(err) => eprintln!("Could not perform a factory reset: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`build_aes_key`]: #method.build_aes_key
    fn factory_reset(&mut self, admin_pin: &str) -> Result<(), Error> {
        let admin_pin_string = get_cstring(admin_pin)?;
        get_command_result(unsafe { nitrokey_sys::NK_factory_reset(admin_pin_string.as_ptr()) })
    }

    /// Builds a new AES key on the Nitrokey.
    ///
    /// The AES key is used to encrypt the password safe and the encrypted volume.  You may need
    /// to call this method after a factory reset, either using [`factory_reset`][] or using `gpg
    /// --card-edit`.  You can also use it to destroy the data stored in the password safe or on
    /// the encrypted volume.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided password contains a null byte
    /// - [`WrongPassword`][] if the admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect()?;
    /// match device.build_aes_key("12345678") {
    ///     Ok(()) => println!("New AES keys have been built."),
    ///     Err(err) => eprintln!("Could not build new AES keys: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`factory_reset`]: #method.factory_reset
    fn build_aes_key(&mut self, admin_pin: &str) -> Result<(), Error> {
        let admin_pin_string = get_cstring(admin_pin)?;
        get_command_result(unsafe { nitrokey_sys::NK_build_aes_key(admin_pin_string.as_ptr()) })
    }
}

fn get_connected_model() -> Option<Model> {
    match unsafe { nitrokey_sys::NK_get_device_model() } {
        nitrokey_sys::NK_device_model_NK_PRO => Some(Model::Pro),
        nitrokey_sys::NK_device_model_NK_STORAGE => Some(Model::Storage),
        _ => None,
    }
}

pub(crate) fn create_device_wrapper(
    manager: &mut crate::Manager,
    model: Model,
) -> DeviceWrapper<'_> {
    match model {
        Model::Pro => Pro::new(manager).into(),
        Model::Storage => Storage::new(manager).into(),
    }
}

pub(crate) fn get_connected_device(
    manager: &mut crate::Manager,
) -> Result<DeviceWrapper<'_>, Error> {
    match get_connected_model() {
        Some(model) => Ok(create_device_wrapper(manager, model)),
        None => Err(CommunicationError::NotConnected.into()),
    }
}

pub(crate) fn connect_enum(model: Model) -> bool {
    let model = match model {
        Model::Storage => nitrokey_sys::NK_device_model_NK_STORAGE,
        Model::Pro => nitrokey_sys::NK_device_model_NK_PRO,
    };
    unsafe { nitrokey_sys::NK_login_enum(model) == 1 }
}
