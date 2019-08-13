// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::fmt;
use std::marker;

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

/// The access mode of a volume on the Nitrokey Storage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VolumeMode {
    /// A read-only volume.
    ReadOnly,
    /// A read-write volume.
    ReadWrite,
}

impl fmt::Display for VolumeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match *self {
            VolumeMode::ReadOnly => "read-only",
            VolumeMode::ReadWrite => "read-write",
        })
    }
}

/// A wrapper for a Nitrokey device of unknown type.
///
/// Use the function [`connect`][] to obtain a wrapped instance.  The wrapper implements all traits
/// that are shared between all Nitrokey devices so that the shared functionality can be used
/// without knowing the type of the underlying device.  If you want to use functionality that is
/// not available for all devices, you have to extract the device.
///
/// # Examples
///
/// Authentication with error handling:
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
/// Device-specific commands:
///
/// ```no_run
/// use nitrokey::{DeviceWrapper, Storage};
/// # use nitrokey::Error;
///
/// fn perform_common_task(device: &DeviceWrapper) {}
/// fn perform_storage_task(device: &Storage) {}
///
/// # fn try_main() -> Result<(), Error> {
/// let device = nitrokey::connect()?;
/// perform_common_task(&device);
/// match device {
///     DeviceWrapper::Storage(storage) => perform_storage_task(&storage),
///     _ => (),
/// };
/// #     Ok(())
/// # }
/// ```
///
/// [`connect`]: fn.connect.html
#[derive(Debug)]
pub enum DeviceWrapper {
    /// A Nitrokey Storage device.
    Storage(Storage),
    /// A Nitrokey Pro device.
    Pro(Pro),
}

/// A Nitrokey Pro device without user or admin authentication.
///
/// Use the global function [`connect`][] to obtain an instance wrapper or the method
/// [`connect`][`Pro::connect`] to directly obtain an instance.  If you want to execute a command
/// that requires user or admin authentication, use [`authenticate_admin`][] or
/// [`authenticate_user`][].
///
/// # Examples
///
/// Authentication with error handling:
///
/// ```no_run
/// use nitrokey::{Authenticate, User, Pro};
/// # use nitrokey::Error;
///
/// fn perform_user_task(device: &User<Pro>) {}
/// fn perform_other_task(device: &Pro) {}
///
/// # fn try_main() -> Result<(), Error> {
/// let device = nitrokey::Pro::connect()?;
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
/// [`authenticate_admin`]: trait.Authenticate.html#method.authenticate_admin
/// [`authenticate_user`]: trait.Authenticate.html#method.authenticate_user
/// [`connect`]: fn.connect.html
/// [`Pro::connect`]: #method.connect
#[derive(Debug)]
pub struct Pro {
    // make sure that users cannot directly instantiate this type
    #[doc(hidden)]
    marker: marker::PhantomData<()>,
}

/// A Nitrokey Storage device without user or admin authentication.
///
/// Use the global function [`connect`][] to obtain an instance wrapper or the method
/// [`connect`][`Storage::connect`] to directly obtain an instance.  If you want to execute a
/// command that requires user or admin authentication, use [`authenticate_admin`][] or
/// [`authenticate_user`][].
///
/// # Examples
///
/// Authentication with error handling:
///
/// ```no_run
/// use nitrokey::{Authenticate, User, Storage};
/// # use nitrokey::Error;
///
/// fn perform_user_task(device: &User<Storage>) {}
/// fn perform_other_task(device: &Storage) {}
///
/// # fn try_main() -> Result<(), Error> {
/// let device = nitrokey::Storage::connect()?;
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
/// [`authenticate_admin`]: trait.Authenticate.html#method.authenticate_admin
/// [`authenticate_user`]: trait.Authenticate.html#method.authenticate_user
/// [`connect`]: fn.connect.html
/// [`Storage::connect`]: #method.connect
#[derive(Debug)]
pub struct Storage {
    // make sure that users cannot directly instantiate this type
    #[doc(hidden)]
    marker: marker::PhantomData<()>,
}

/// The status of a volume on a Nitrokey Storage device.
#[derive(Debug)]
pub struct VolumeStatus {
    /// Indicates whether the volume is read-only.
    pub read_only: bool,
    /// Indicates whether the volume is active.
    pub active: bool,
}

/// Information about the SD card in a Storage device.
#[derive(Debug)]
pub struct SdCardData {
    /// The serial number of the SD card.
    pub serial_number: u32,
    /// The size of the SD card in GB.
    pub size: u8,
    /// The year the card was manufactured, e. g. 17 for 2017.
    pub manufacturing_year: u8,
    /// The month the card was manufactured.
    pub manufacturing_month: u8,
    /// The OEM ID.
    pub oem: u16,
    /// The manufacturer ID.
    pub manufacturer: u8,
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

/// Production information for a Storage device.
#[derive(Debug)]
pub struct StorageProductionInfo {
    /// The firmware version.
    pub firmware_version: FirmwareVersion,
    /// The internal firmware version.
    pub firmware_version_internal: u8,
    /// The serial number of the CPU.
    pub serial_number_cpu: u32,
    /// Information about the SD card.
    pub sd_card: SdCardData,
}

/// The status of a Nitrokey Storage device.
#[derive(Debug)]
pub struct StorageStatus {
    /// The status of the unencrypted volume.
    pub unencrypted_volume: VolumeStatus,
    /// The status of the encrypted volume.
    pub encrypted_volume: VolumeStatus,
    /// The status of the hidden volume.
    pub hidden_volume: VolumeStatus,
    /// The firmware version.
    pub firmware_version: FirmwareVersion,
    /// Indicates whether the firmware is locked.
    pub firmware_locked: bool,
    /// The serial number of the SD card in the Storage stick.
    pub serial_number_sd_card: u32,
    /// The serial number of the smart card in the Storage stick.
    pub serial_number_smart_card: u32,
    /// The number of remaining login attempts for the user PIN.
    pub user_retry_count: u8,
    /// The number of remaining login attempts for the admin PIN.
    pub admin_retry_count: u8,
    /// Indicates whether a new SD card was found.
    pub new_sd_card_found: bool,
    /// Indicates whether the SD card is filled with random characters.
    pub filled_with_random: bool,
    /// Indicates whether the stick has been initialized by generating
    /// the AES keys.
    pub stick_initialized: bool,
}

/// A Nitrokey device.
///
/// This trait provides the commands that can be executed without authentication and that are
/// present on all supported Nitrokey devices.
pub trait Device: Authenticate + GetPasswordSafe + GenerateOtp + fmt::Debug {
    /// Returns the model of the connected Nitrokey device.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::Device;
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::connect()?;
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
    /// let device = nitrokey::connect()?;
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
    /// let device = nitrokey::connect()?;
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
    /// let device = nitrokey::connect()?;
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
    /// let device = nitrokey::connect()?;
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
        let max = i32::from(u8::max_value());
        if major < 0 || minor < 0 || major > max || minor > max {
            return Err(Error::UnexpectedError);
        }
        Ok(FirmwareVersion {
            major: major as u8,
            minor: minor as u8,
        })
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
    /// let device = nitrokey::connect()?;
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
    /// let mut device = nitrokey::connect()?;
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
    /// let mut device = nitrokey::connect()?;
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
    /// let mut device = nitrokey::connect()?;
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
    /// let mut device = nitrokey::connect()?;
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
    /// let mut device = nitrokey::connect()?;
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
    /// let mut device = nitrokey::connect()?;
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

/// Connects to a Nitrokey device.  This method can be used to connect to any connected device,
/// both a Nitrokey Pro and a Nitrokey Storage.
///
/// # Errors
///
/// - [`NotConnected`][] if no Nitrokey device is connected
///
/// # Example
///
/// ```
/// use nitrokey::DeviceWrapper;
///
/// fn do_something(device: DeviceWrapper) {}
///
/// match nitrokey::connect() {
///     Ok(device) => do_something(device),
///     Err(err) => eprintln!("Could not connect to a Nitrokey: {}", err),
/// }
/// ```
///
/// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
pub fn connect() -> Result<DeviceWrapper, Error> {
    if unsafe { nitrokey_sys::NK_login_auto() } == 1 {
        match get_connected_device() {
            Some(wrapper) => Ok(wrapper),
            None => Err(CommunicationError::NotConnected.into()),
        }
    } else {
        Err(CommunicationError::NotConnected.into())
    }
}

/// Connects to a Nitrokey device of the given model.
///
/// # Errors
///
/// - [`NotConnected`][] if no Nitrokey device of the given model is connected
///
/// # Example
///
/// ```
/// use nitrokey::DeviceWrapper;
/// use nitrokey::Model;
///
/// fn do_something(device: DeviceWrapper) {}
///
/// match nitrokey::connect_model(Model::Pro) {
///     Ok(device) => do_something(device),
///     Err(err) => eprintln!("Could not connect to a Nitrokey Pro: {}", err),
/// }
/// ```
///
/// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
pub fn connect_model(model: Model) -> Result<DeviceWrapper, Error> {
    if connect_enum(model) {
        Ok(create_device_wrapper(model))
    } else {
        Err(CommunicationError::NotConnected.into())
    }
}

fn get_connected_model() -> Option<Model> {
    match unsafe { nitrokey_sys::NK_get_device_model() } {
        nitrokey_sys::NK_device_model_NK_PRO => Some(Model::Pro),
        nitrokey_sys::NK_device_model_NK_STORAGE => Some(Model::Storage),
        _ => None,
    }
}

fn create_device_wrapper(model: Model) -> DeviceWrapper {
    match model {
        Model::Pro => Pro::new().into(),
        Model::Storage => Storage::new().into(),
    }
}

fn get_connected_device() -> Option<DeviceWrapper> {
    get_connected_model().map(create_device_wrapper)
}

fn connect_enum(model: Model) -> bool {
    let model = match model {
        Model::Storage => nitrokey_sys::NK_device_model_NK_STORAGE,
        Model::Pro => nitrokey_sys::NK_device_model_NK_PRO,
    };
    unsafe { nitrokey_sys::NK_login_enum(model) == 1 }
}

impl DeviceWrapper {
    fn device(&self) -> &dyn Device {
        match *self {
            DeviceWrapper::Storage(ref storage) => storage,
            DeviceWrapper::Pro(ref pro) => pro,
        }
    }

    fn device_mut(&mut self) -> &mut dyn Device {
        match *self {
            DeviceWrapper::Storage(ref mut storage) => storage,
            DeviceWrapper::Pro(ref mut pro) => pro,
        }
    }
}

impl From<Pro> for DeviceWrapper {
    fn from(device: Pro) -> Self {
        DeviceWrapper::Pro(device)
    }
}

impl From<Storage> for DeviceWrapper {
    fn from(device: Storage) -> Self {
        DeviceWrapper::Storage(device)
    }
}

impl GenerateOtp for DeviceWrapper {
    fn get_hotp_slot_name(&self, slot: u8) -> Result<String, Error> {
        self.device().get_hotp_slot_name(slot)
    }

    fn get_totp_slot_name(&self, slot: u8) -> Result<String, Error> {
        self.device().get_totp_slot_name(slot)
    }

    fn get_hotp_code(&mut self, slot: u8) -> Result<String, Error> {
        self.device_mut().get_hotp_code(slot)
    }

    fn get_totp_code(&self, slot: u8) -> Result<String, Error> {
        self.device().get_totp_code(slot)
    }
}

impl Device for DeviceWrapper {
    fn get_model(&self) -> Model {
        match *self {
            DeviceWrapper::Pro(_) => Model::Pro,
            DeviceWrapper::Storage(_) => Model::Storage,
        }
    }
}

impl Pro {
    /// Connects to a Nitrokey Pro.
    ///
    /// # Errors
    ///
    /// - [`NotConnected`][] if no Nitrokey device of the given model is connected
    ///
    /// # Example
    ///
    /// ```
    /// use nitrokey::Pro;
    ///
    /// fn use_pro(device: Pro) {}
    ///
    /// match nitrokey::Pro::connect() {
    ///     Ok(device) => use_pro(device),
    ///     Err(err) => eprintln!("Could not connect to the Nitrokey Pro: {}", err),
    /// }
    /// ```
    ///
    /// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
    pub fn connect() -> Result<Pro, Error> {
        // TODO: maybe Option instead of Result?
        if connect_enum(Model::Pro) {
            Ok(Pro::new())
        } else {
            Err(CommunicationError::NotConnected.into())
        }
    }

    fn new() -> Pro {
        Pro {
            marker: marker::PhantomData,
        }
    }
}

impl Drop for Pro {
    fn drop(&mut self) {
        unsafe {
            nitrokey_sys::NK_logout();
        }
    }
}

impl Device for Pro {
    fn get_model(&self) -> Model {
        Model::Pro
    }
}

impl GenerateOtp for Pro {}

impl Storage {
    /// Connects to a Nitrokey Storage.
    ///
    /// # Errors
    ///
    /// - [`NotConnected`][] if no Nitrokey device of the given model is connected
    ///
    /// # Example
    ///
    /// ```
    /// use nitrokey::Storage;
    ///
    /// fn use_storage(device: Storage) {}
    ///
    /// match nitrokey::Storage::connect() {
    ///     Ok(device) => use_storage(device),
    ///     Err(err) => eprintln!("Could not connect to the Nitrokey Storage: {}", err),
    /// }
    /// ```
    ///
    /// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
    pub fn connect() -> Result<Storage, Error> {
        // TODO: maybe Option instead of Result?
        if connect_enum(Model::Storage) {
            Ok(Storage::new())
        } else {
            Err(CommunicationError::NotConnected.into())
        }
    }

    fn new() -> Storage {
        Storage {
            marker: marker::PhantomData,
        }
    }

    /// Changes the update PIN.
    ///
    /// The update PIN is used to enable firmware updates.  Unlike the user and the admin PIN, the
    /// update PIN is not managed by the OpenPGP smart card but by the Nitrokey firmware.  There is
    /// no retry counter as with the other PIN types.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the current update password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// match device.change_update_pin("12345678", "87654321") {
    ///     Ok(()) => println!("Updated update PIN."),
    ///     Err(err) => eprintln!("Failed to update update PIN: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn change_update_pin(&mut self, current: &str, new: &str) -> Result<(), Error> {
        let current_string = get_cstring(current)?;
        let new_string = get_cstring(new)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_change_update_password(current_string.as_ptr(), new_string.as_ptr())
        })
    }

    /// Enables the firmware update mode.
    ///
    /// During firmware update mode, the Nitrokey can no longer be accessed using HID commands.
    /// To resume normal operation, run `dfu-programmer at32uc3a3256s launch`.  In order to enter
    /// the firmware update mode, you need the update password that can be changed using the
    /// [`change_update_pin`][] method.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the current update password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// match device.enable_firmware_update("12345678") {
    ///     Ok(()) => println!("Nitrokey entered update mode."),
    ///     Err(err) => eprintln!("Could not enter update mode: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn enable_firmware_update(&mut self, update_pin: &str) -> Result<(), Error> {
        let update_pin_string = get_cstring(update_pin)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_enable_firmware_update(update_pin_string.as_ptr())
        })
    }

    /// Enables the encrypted storage volume.
    ///
    /// Once the encrypted volume is enabled, it is presented to the operating system as a block
    /// device.  The API does not provide any information on the name or path of this block device.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided password contains a null byte
    /// - [`WrongPassword`][] if the provided user password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// match device.enable_encrypted_volume("123456") {
    ///     Ok(()) => println!("Enabled the encrypted volume."),
    ///     Err(err) => eprintln!("Could not enable the encrypted volume: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn enable_encrypted_volume(&mut self, user_pin: &str) -> Result<(), Error> {
        let user_pin = get_cstring(user_pin)?;
        get_command_result(unsafe { nitrokey_sys::NK_unlock_encrypted_volume(user_pin.as_ptr()) })
    }

    /// Disables the encrypted storage volume.
    ///
    /// Once the volume is disabled, it can be no longer accessed as a block device.  If the
    /// encrypted volume has not been enabled, this method still returns a success.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// fn use_volume() {}
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// match device.enable_encrypted_volume("123456") {
    ///     Ok(()) => {
    ///         println!("Enabled the encrypted volume.");
    ///         use_volume();
    ///         match device.disable_encrypted_volume() {
    ///             Ok(()) => println!("Disabled the encrypted volume."),
    ///             Err(err) => {
    ///                 eprintln!("Could not disable the encrypted volume: {}", err);
    ///             },
    ///         };
    ///     },
    ///     Err(err) => eprintln!("Could not enable the encrypted volume: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    pub fn disable_encrypted_volume(&mut self) -> Result<(), Error> {
        get_command_result(unsafe { nitrokey_sys::NK_lock_encrypted_volume() })
    }

    /// Enables a hidden storage volume.
    ///
    /// This function will only succeed if the encrypted storage ([`enable_encrypted_volume`][]) or
    /// another hidden volume has been enabled previously.  Once the hidden volume is enabled, it
    /// is presented to the operating system as a block device and any previously opened encrypted
    /// or hidden volumes are closed.  The API does not provide any information on the name or path
    /// of this block device.
    ///
    /// Note that the encrypted and the hidden volumes operate on the same storage area, so using
    /// both at the same time might lead to data loss.
    ///
    /// The hidden volume to unlock is selected based on the provided password.
    ///
    /// # Errors
    ///
    /// - [`AesDecryptionFailed`][] if the encrypted storage has not been opened before calling
    ///   this method or the AES key has not been built
    /// - [`InvalidString`][] if the provided password contains a null byte
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// device.enable_encrypted_volume("123445")?;
    /// match device.enable_hidden_volume("hidden-pw") {
    ///     Ok(()) => println!("Enabled a hidden volume."),
    ///     Err(err) => eprintln!("Could not enable the hidden volume: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`enable_encrypted_volume`]: #method.enable_encrypted_volume
    /// [`AesDecryptionFailed`]: enum.CommandError.html#variant.AesDecryptionFailed
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    pub fn enable_hidden_volume(&mut self, volume_password: &str) -> Result<(), Error> {
        let volume_password = get_cstring(volume_password)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_unlock_hidden_volume(volume_password.as_ptr())
        })
    }

    /// Disables a hidden storage volume.
    ///
    /// Once the volume is disabled, it can be no longer accessed as a block device.  If no hidden
    /// volume has been enabled, this method still returns a success.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// fn use_volume() {}
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// device.enable_encrypted_volume("123445")?;
    /// match device.enable_hidden_volume("hidden-pw") {
    ///     Ok(()) => {
    ///         println!("Enabled the hidden volume.");
    ///         use_volume();
    ///         match device.disable_hidden_volume() {
    ///             Ok(()) => println!("Disabled the hidden volume."),
    ///             Err(err) => {
    ///                 eprintln!("Could not disable the hidden volume: {}", err);
    ///             },
    ///         };
    ///     },
    ///     Err(err) => eprintln!("Could not enable the hidden volume: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    pub fn disable_hidden_volume(&mut self) -> Result<(), Error> {
        get_command_result(unsafe { nitrokey_sys::NK_lock_hidden_volume() })
    }

    /// Creates a hidden volume.
    ///
    /// The volume is crated in the given slot and in the given range of the available memory,
    /// where `start` is the start position as a percentage of the available memory, and `end` is
    /// the end position as a percentage of the available memory.  The volume will be protected by
    /// the given password.
    ///
    /// Note that the encrypted and the hidden volumes operate on the same storage area, so using
    /// both at the same time might lead to data loss.
    ///
    /// According to the libnitrokey documentation, this function only works if the encrypted
    /// storage has been opened.
    ///
    /// # Errors
    ///
    /// - [`AesDecryptionFailed`][] if the encrypted storage has not been opened before calling
    ///   this method or the AES key has not been built
    /// - [`InvalidString`][] if the provided password contains a null byte
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// device.enable_encrypted_volume("123445")?;
    /// device.create_hidden_volume(0, 0, 100, "hidden-pw")?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`AesDecryptionFailed`]: enum.CommandError.html#variant.AesDecryptionFailed
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    pub fn create_hidden_volume(
        &mut self,
        slot: u8,
        start: u8,
        end: u8,
        password: &str,
    ) -> Result<(), Error> {
        let password = get_cstring(password)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_create_hidden_volume(slot, start, end, password.as_ptr())
        })
    }

    /// Sets the access mode of the unencrypted volume.
    ///
    /// This command will reconnect the unencrypted volume so buffers should be flushed before
    /// calling it.  Since firmware version v0.51, this command requires the admin PIN.  Older
    /// firmware versions are not supported.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided password contains a null byte
    /// - [`WrongPassword`][] if the provided admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    /// use nitrokey::VolumeMode;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// match device.set_unencrypted_volume_mode("12345678", VolumeMode::ReadWrite) {
    ///     Ok(()) => println!("Set the unencrypted volume to read-write mode."),
    ///     Err(err) => eprintln!("Could not set the unencrypted volume to read-write mode: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn set_unencrypted_volume_mode(
        &mut self,
        admin_pin: &str,
        mode: VolumeMode,
    ) -> Result<(), Error> {
        let admin_pin = get_cstring(admin_pin)?;
        let result = match mode {
            VolumeMode::ReadOnly => unsafe {
                nitrokey_sys::NK_set_unencrypted_read_only_admin(admin_pin.as_ptr())
            },
            VolumeMode::ReadWrite => unsafe {
                nitrokey_sys::NK_set_unencrypted_read_write_admin(admin_pin.as_ptr())
            },
        };
        get_command_result(result)
    }

    /// Sets the access mode of the encrypted volume.
    ///
    /// This command will reconnect the encrypted volume so buffers should be flushed before
    /// calling it.  It is only available in firmware version 0.49.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided password contains a null byte
    /// - [`WrongPassword`][] if the provided admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    /// use nitrokey::VolumeMode;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// match device.set_encrypted_volume_mode("12345678", VolumeMode::ReadWrite) {
    ///     Ok(()) => println!("Set the encrypted volume to read-write mode."),
    ///     Err(err) => eprintln!("Could not set the encrypted volume to read-write mode: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn set_encrypted_volume_mode(
        &mut self,
        admin_pin: &str,
        mode: VolumeMode,
    ) -> Result<(), Error> {
        let admin_pin = get_cstring(admin_pin)?;
        let result = match mode {
            VolumeMode::ReadOnly => unsafe {
                nitrokey_sys::NK_set_encrypted_read_only(admin_pin.as_ptr())
            },
            VolumeMode::ReadWrite => unsafe {
                nitrokey_sys::NK_set_encrypted_read_write(admin_pin.as_ptr())
            },
        };
        get_command_result(result)
    }

    /// Returns the status of the connected storage device.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// fn use_volume() {}
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::Storage::connect()?;
    /// match device.get_status() {
    ///     Ok(status) => {
    ///         println!("SD card ID: {:#x}", status.serial_number_sd_card);
    ///     },
    ///     Err(err) => eprintln!("Could not get Storage status: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    pub fn get_status(&self) -> Result<StorageStatus, Error> {
        let mut raw_status = nitrokey_sys::NK_storage_status {
            unencrypted_volume_read_only: false,
            unencrypted_volume_active: false,
            encrypted_volume_read_only: false,
            encrypted_volume_active: false,
            hidden_volume_read_only: false,
            hidden_volume_active: false,
            firmware_version_major: 0,
            firmware_version_minor: 0,
            firmware_locked: false,
            serial_number_sd_card: 0,
            serial_number_smart_card: 0,
            user_retry_count: 0,
            admin_retry_count: 0,
            new_sd_card_found: false,
            filled_with_random: false,
            stick_initialized: false,
        };
        let raw_result = unsafe { nitrokey_sys::NK_get_status_storage(&mut raw_status) };
        get_command_result(raw_result).map(|_| StorageStatus::from(raw_status))
    }

    /// Returns the production information for the connected storage device.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// fn use_volume() {}
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let device = nitrokey::Storage::connect()?;
    /// match device.get_production_info() {
    ///     Ok(data) => {
    ///         println!("SD card ID:   {:#x}", data.sd_card.serial_number);
    ///         println!("SD card size: {} GB", data.sd_card.size);
    ///     },
    ///     Err(err) => eprintln!("Could not get Storage production info: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    pub fn get_production_info(&self) -> Result<StorageProductionInfo, Error> {
        let mut raw_data = nitrokey_sys::NK_storage_ProductionTest {
            FirmwareVersion_au8: [0, 2],
            FirmwareVersionInternal_u8: 0,
            SD_Card_Size_u8: 0,
            CPU_CardID_u32: 0,
            SmartCardID_u32: 0,
            SD_CardID_u32: 0,
            SC_UserPwRetryCount: 0,
            SC_AdminPwRetryCount: 0,
            SD_Card_ManufacturingYear_u8: 0,
            SD_Card_ManufacturingMonth_u8: 0,
            SD_Card_OEM_u16: 0,
            SD_WriteSpeed_u16: 0,
            SD_Card_Manufacturer_u8: 0,
        };
        let raw_result = unsafe { nitrokey_sys::NK_get_storage_production_info(&mut raw_data) };
        get_command_result(raw_result).map(|_| StorageProductionInfo::from(raw_data))
    }

    /// Clears the warning for a new SD card.
    ///
    /// The Storage status contains a field for a new SD card warning.  After a factory reset, the
    /// field is set to true.  After filling the SD card with random data, it is set to false.
    /// This method can be used to set it to false without filling the SD card with random data.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the provided password contains a null byte
    /// - [`WrongPassword`][] if the provided admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nitrokey::Error;
    ///
    /// # fn try_main() -> Result<(), Error> {
    /// let mut device = nitrokey::Storage::connect()?;
    /// match device.clear_new_sd_card_warning("12345678") {
    ///     Ok(()) => println!("Cleared the new SD card warning."),
    ///     Err(err) => eprintln!("Could not set the clear the new SD card warning: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn clear_new_sd_card_warning(&mut self, admin_pin: &str) -> Result<(), Error> {
        let admin_pin = get_cstring(admin_pin)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_clear_new_sd_card_warning(admin_pin.as_ptr())
        })
    }

    /// Blinks the red and green LED alternatively and infinitely until the device is reconnected.
    pub fn wink(&mut self) -> Result<(), Error> {
        get_command_result(unsafe { nitrokey_sys::NK_wink() })
    }

    /// Exports the firmware to the unencrypted volume.
    ///
    /// This command requires the admin PIN.  The unencrypted volume must be in read-write mode
    /// when this command is executed.  Otherwise, it will still return `Ok` but not write the
    /// firmware.
    ///
    /// This command unmounts the unencrypted volume if it has been mounted, so all buffers should
    /// be flushed.  The firmware is written to the `firmware.bin` file on the unencrypted volume.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the admin password is wrong
    ///
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn export_firmware(&mut self, admin_pin: &str) -> Result<(), Error> {
        let admin_pin_string = get_cstring(admin_pin)?;
        get_command_result(unsafe { nitrokey_sys::NK_export_firmware(admin_pin_string.as_ptr()) })
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        unsafe {
            nitrokey_sys::NK_logout();
        }
    }
}

impl Device for Storage {
    fn get_model(&self) -> Model {
        Model::Storage
    }
}

impl GenerateOtp for Storage {}

impl From<nitrokey_sys::NK_storage_ProductionTest> for StorageProductionInfo {
    fn from(data: nitrokey_sys::NK_storage_ProductionTest) -> Self {
        Self {
            firmware_version: FirmwareVersion {
                major: data.FirmwareVersion_au8[0],
                minor: data.FirmwareVersion_au8[1],
            },
            firmware_version_internal: data.FirmwareVersionInternal_u8,
            serial_number_cpu: data.CPU_CardID_u32,
            sd_card: SdCardData {
                serial_number: data.SD_CardID_u32,
                size: data.SD_Card_Size_u8,
                manufacturing_year: data.SD_Card_ManufacturingYear_u8,
                manufacturing_month: data.SD_Card_ManufacturingMonth_u8,
                oem: data.SD_Card_OEM_u16,
                manufacturer: data.SD_Card_Manufacturer_u8,
            },
        }
    }
}

impl From<nitrokey_sys::NK_storage_status> for StorageStatus {
    fn from(status: nitrokey_sys::NK_storage_status) -> Self {
        StorageStatus {
            unencrypted_volume: VolumeStatus {
                read_only: status.unencrypted_volume_read_only,
                active: status.unencrypted_volume_active,
            },
            encrypted_volume: VolumeStatus {
                read_only: status.encrypted_volume_read_only,
                active: status.encrypted_volume_active,
            },
            hidden_volume: VolumeStatus {
                read_only: status.hidden_volume_read_only,
                active: status.hidden_volume_active,
            },
            firmware_version: FirmwareVersion {
                major: status.firmware_version_major,
                minor: status.firmware_version_minor,
            },
            firmware_locked: status.firmware_locked,
            serial_number_sd_card: status.serial_number_sd_card,
            serial_number_smart_card: status.serial_number_smart_card,
            user_retry_count: status.user_retry_count,
            admin_retry_count: status.admin_retry_count,
            new_sd_card_found: status.new_sd_card_found,
            filled_with_random: status.filled_with_random,
            stick_initialized: status.stick_initialized,
        }
    }
}
