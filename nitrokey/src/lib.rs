// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

//! Provides access to a Nitrokey device using the native libnitrokey API.
//!
//! # Usage
//!
//! Operations on the Nitrokey require different authentication levels.  Some operations can be
//! performed without authentication, some require user access, and some require admin access.
//! This is modelled using the types [`User`][] and [`Admin`][].
//!
//! You can only connect to one Nitrokey at a time.  Use the global [`take`][] function to obtain
//! an reference to the [`Manager`][] singleton that keeps track of the connections.  Then use the
//! [`connect`][] method to connect to any Nitrokey device.  The method will return a
//! [`DeviceWrapper`][] that abstracts over the supported Nitrokey devices.  You can also use
//! [`connect_model`][], [`connect_pro`][] or [`connect_storage`][] to connect to a specific
//! device.
//!
//! To get a list of all connected Nitrokey devices, use the [`list_devices`][] function.  You can
//! then connect to one of the connected devices using the [`connect_path`][] function of the
//! `Manager` struct.
//!
//! You can call [`authenticate_user`][] or [`authenticate_admin`][] to get an authenticated device
//! that can perform operations that require authentication.  You can use [`device`][] to go back
//! to the unauthenticated device.
//!
//! This makes sure that you can only execute a command if you have the required access rights.
//! Otherwise, your code will not compile.  The only exception are the methods to generate one-time
//! passwords – [`get_hotp_code`][] and [`get_totp_code`][].  Depending on the stick configuration,
//! these operations are available without authentication or with user authentication.
//!
//! # Background operations
//!
//! Some commands may start background operations.  During such an operation, every new command
//! will cause a [`WrongCrc`][] error.  To check whether a background operation is currently
//! running, use the [`get_operation_status`][] method.
//!
//! Background operations are only available on the Nitrokey Storage.  Currently,
//! [`fill_sd_card`][] is the only command that triggers a background operation.
//!
//! # Examples
//!
//! Connect to any Nitrokey and print its serial number:
//!
//! ```no_run
//! use nitrokey::Device;
//! # use nitrokey::Error;
//!
//! # fn try_main() -> Result<(), Error> {
//! let mut manager = nitrokey::take()?;
//! let device = manager.connect()?;
//! println!("{}", device.get_serial_number()?);
//! #     Ok(())
//! # }
//! ```
//!
//! Configure an HOTP slot:
//!
//! ```no_run
//! use nitrokey::{Authenticate, ConfigureOtp, OtpMode, OtpSlotData};
//! # use nitrokey::Error;
//!
//! # fn try_main() -> Result<(), Error> {
//! let mut manager = nitrokey::take()?;
//! let device = manager.connect()?;
//! let slot_data = OtpSlotData::new(1, "test", "01234567890123456689", OtpMode::SixDigits);
//! match device.authenticate_admin("12345678") {
//!     Ok(mut admin) => {
//!         match admin.write_hotp_slot(slot_data, 0) {
//!             Ok(()) => println!("Successfully wrote slot."),
//!             Err(err) => eprintln!("Could not write slot: {}", err),
//!         }
//!     },
//!     Err((_, err)) => eprintln!("Could not authenticate as admin: {}", err),
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! Generate an HOTP one-time password:
//!
//! ```no_run
//! use nitrokey::{Device, GenerateOtp};
//! # use nitrokey::Error;
//!
//! # fn try_main() -> Result<(), Error> {
//! let mut manager = nitrokey::take()?;
//! let mut device = manager.connect()?;
//! match device.get_hotp_code(1) {
//!     Ok(code) => println!("Generated HOTP code: {}", code),
//!     Err(err) => eprintln!("Could not generate HOTP code: {}", err),
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! [`authenticate_admin`]: trait.Authenticate.html#method.authenticate_admin
//! [`authenticate_user`]: trait.Authenticate.html#method.authenticate_user
//! [`take`]: fn.take.html
//! [`connect`]: struct.Manager.html#method.connect
//! [`connect_model`]: struct.Manager.html#method.connect_model
//! [`connect_path`]: struct.Manager.html#method.connect_path
//! [`connect_pro`]: struct.Manager.html#method.connect_pro
//! [`connect_storage`]: struct.Manager.html#method.connect_storage
//! [`fill_sd_card`]: struct.Storage.html#method.fill_sd_card
//! [`get_operation_status`]: struct.Storage.html#method.get_operation_status
//! [`list_devices`]: fn.list_devices.html
//! [`manager`]: trait.Device.html#method.manager
//! [`device`]: struct.User.html#method.device
//! [`get_hotp_code`]: trait.GenerateOtp.html#method.get_hotp_code
//! [`get_totp_code`]: trait.GenerateOtp.html#method.get_totp_code
//! [`Admin`]: struct.Admin.html
//! [`DeviceWrapper`]: enum.DeviceWrapper.html
//! [`User`]: struct.User.html
//! [`WrongCrc`]: enum.CommandError.html#variant.WrongCrc

#![warn(missing_docs, rust_2018_compatibility, rust_2018_idioms, unused)]

#[macro_use(lazy_static)]
extern crate lazy_static;

mod auth;
mod config;
mod device;
mod error;
mod otp;
mod pws;
mod util;

use std::convert::TryInto as _;
use std::fmt;
use std::marker;
use std::ptr::NonNull;
use std::sync;

use nitrokey_sys;

pub use crate::auth::{Admin, Authenticate, User};
pub use crate::config::Config;
pub use crate::device::{
    Device, DeviceInfo, DeviceWrapper, Model, OperationStatus, Pro, SdCardData, Status, Storage,
    StorageProductionInfo, StorageStatus, VolumeMode, VolumeStatus,
};
pub use crate::error::{CommandError, CommunicationError, Error, LibraryError};
pub use crate::otp::{ConfigureOtp, GenerateOtp, OtpMode, OtpSlotData};
pub use crate::pws::{GetPasswordSafe, PasswordSafe, SLOT_COUNT};
pub use crate::util::LogLevel;

use crate::util::{get_cstring, get_last_result};

/// The default admin PIN for all Nitrokey devices.
pub const DEFAULT_ADMIN_PIN: &str = "12345678";
/// The default user PIN for all Nitrokey devices.
pub const DEFAULT_USER_PIN: &str = "123456";

lazy_static! {
    static ref MANAGER: sync::Mutex<Manager> = sync::Mutex::new(Manager::new());
}

/// A version of the libnitrokey library.
///
/// Use the [`get_library_version`](fn.get_library_version.html) function to query the library
/// version.
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    /// The Git library version as a string.
    ///
    /// The library version is the output of `git describe --always` at compile time, for example
    /// `v3.3` or `v3.4.1`.  If the library has not been built from a release, the version string
    /// contains the number of commits since the last release and the hash of the current commit, for
    /// example `v3.3-19-gaee920b`.  If the library has not been built from a Git checkout, this
    /// string may be empty.
    pub git: String,
    /// The major library version.
    pub major: u32,
    /// The minor library version.
    pub minor: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.git.is_empty() {
            write!(f, "v{}.{}", self.major, self.minor)
        } else {
            f.write_str(&self.git)
        }
    }
}

/// A manager for connections to Nitrokey devices.
///
/// Currently, libnitrokey only provides access to one Nitrokey device at the same time.  This
/// manager struct makes sure that `nitrokey-rs` does not try to connect to two devices at the same
/// time.
///
/// To obtain a reference to an instance of this manager, use the [`take`][] function.  Use one of
/// the connect methods – [`connect`][], [`connect_model`][], [`connect_pro`][] or
/// [`connect_storage`][] – to retrieve a [`Device`][] instance.
///
/// # Examples
///
/// Connect to a single device:
///
/// ```no_run
/// use nitrokey::Device;
/// # use nitrokey::Error;
///
/// # fn try_main() -> Result<(), Error> {
/// let mut manager = nitrokey::take()?;
/// let device = manager.connect()?;
/// println!("{}", device.get_serial_number()?);
/// #     Ok(())
/// # }
/// ```
///
/// Connect to a Pro and a Storage device:
///
/// ```no_run
/// use nitrokey::{Device, Model};
/// # use nitrokey::Error;
///
/// # fn try_main() -> Result<(), Error> {
/// let mut manager = nitrokey::take()?;
/// let device = manager.connect_model(Model::Pro)?;
/// println!("Pro: {}", device.get_serial_number()?);
/// drop(device);
/// let device = manager.connect_model(Model::Storage)?;
/// println!("Storage: {}", device.get_serial_number()?);
/// #     Ok(())
/// # }
/// ```
///
/// [`connect`]: #method.connect
/// [`connect_model`]: #method.connect_model
/// [`connect_pro`]: #method.connect_pro
/// [`connect_storage`]: #method.connect_storage
/// [`manager`]: trait.Device.html#method.manager
/// [`take`]: fn.take.html
/// [`Device`]: trait.Device.html
#[derive(Debug)]
pub struct Manager {
    marker: marker::PhantomData<()>,
}

impl Manager {
    fn new() -> Self {
        Manager {
            marker: marker::PhantomData,
        }
    }

    /// Connects to a Nitrokey device.
    ///
    /// This method can be used to connect to any connected device, both a Nitrokey Pro and a
    /// Nitrokey Storage.
    ///
    /// # Errors
    ///
    /// - [`NotConnected`][] if no Nitrokey device is connected
    /// - [`UnsupportedModelError`][] if the Nitrokey device is not supported by this crate
    ///
    /// # Example
    ///
    /// ```
    /// use nitrokey::DeviceWrapper;
    ///
    /// fn do_something(device: DeviceWrapper) {}
    ///
    /// let mut manager = nitrokey::take()?;
    /// match manager.connect() {
    ///     Ok(device) => do_something(device),
    ///     Err(err) => println!("Could not connect to a Nitrokey: {}", err),
    /// }
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    ///
    /// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
    /// [`UnsupportedModelError`]: enum.Error.html#variant.UnsupportedModelError
    pub fn connect(&mut self) -> Result<DeviceWrapper<'_>, Error> {
        if unsafe { nitrokey_sys::NK_login_auto() } == 1 {
            device::get_connected_device(self)
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
    /// match nitrokey::take()?.connect_model(Model::Pro) {
    ///     Ok(device) => do_something(device),
    ///     Err(err) => println!("Could not connect to a Nitrokey Pro: {}", err),
    /// }
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    ///
    /// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
    pub fn connect_model(&mut self, model: Model) -> Result<DeviceWrapper<'_>, Error> {
        if device::connect_enum(model) {
            Ok(device::create_device_wrapper(self, model))
        } else {
            Err(CommunicationError::NotConnected.into())
        }
    }

    /// Connects to a Nitrokey device at the given USB path.
    ///
    /// To get a list of all connected Nitrokey devices, use the [`list_devices`][] function.  The
    /// [`DeviceInfo`][] structs returned by that function contain the USB path in the `path`
    /// field.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if the USB path contains a null byte
    /// - [`NotConnected`][] if no Nitrokey device can be found at the given USB path
    /// - [`UnsupportedModelError`][] if the model of the Nitrokey device at the given USB path is
    ///   not supported by this crate
    ///
    /// # Example
    ///
    /// ```
    /// use nitrokey::DeviceWrapper;
    ///
    /// fn use_device(device: DeviceWrapper) {}
    ///
    /// let mut manager = nitrokey::take()?;
    /// let devices = nitrokey::list_devices()?;
    /// for device in devices {
    ///     let device = manager.connect_path(device.path)?;
    ///     use_device(device);
    /// }
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    ///
    /// [`list_devices`]: fn.list_devices.html
    /// [`DeviceInfo`]: struct.DeviceInfo.html
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
    /// [`UnsupportedModelError`]: enum.Error.html#variant.UnsupportedModelError
    pub fn connect_path<S: Into<Vec<u8>>>(&mut self, path: S) -> Result<DeviceWrapper<'_>, Error> {
        let path = get_cstring(path)?;
        if unsafe { nitrokey_sys::NK_connect_with_path(path.as_ptr()) } == 1 {
            device::get_connected_device(self)
        } else {
            Err(CommunicationError::NotConnected.into())
        }
    }

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
    /// match nitrokey::take()?.connect_pro() {
    ///     Ok(device) => use_pro(device),
    ///     Err(err) => println!("Could not connect to the Nitrokey Pro: {}", err),
    /// }
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    ///
    /// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
    pub fn connect_pro(&mut self) -> Result<Pro<'_>, Error> {
        if device::connect_enum(device::Model::Pro) {
            Ok(device::Pro::new(self))
        } else {
            Err(CommunicationError::NotConnected.into())
        }
    }

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
    /// match nitrokey::take()?.connect_storage() {
    ///     Ok(device) => use_storage(device),
    ///     Err(err) => println!("Could not connect to the Nitrokey Storage: {}", err),
    /// }
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    ///
    /// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
    pub fn connect_storage(&mut self) -> Result<Storage<'_>, Error> {
        if device::connect_enum(Model::Storage) {
            Ok(Storage::new(self))
        } else {
            Err(CommunicationError::NotConnected.into())
        }
    }
}

/// Take an instance of the connection manager, blocking until an instance is available.
///
/// There may only be one [`Manager`][] instance at the same time.  If there already is an
/// instance, this method blocks.  If you want a non-blocking version, use [`take`][].
///
/// # Errors
///
/// - [`PoisonError`][] if the lock is poisoned
///
/// [`take`]: fn.take.html
/// [`PoisonError`]: struct.Error.html#variant.PoisonError
/// [`Manager`]: struct.Manager.html
pub fn take_blocking() -> Result<sync::MutexGuard<'static, Manager>, Error> {
    MANAGER.lock().map_err(Into::into)
}

/// Try to take an instance of the connection manager.
///
/// There may only be one [`Manager`][] instance at the same time.  If there already is an
/// instance, a [`ConcurrentAccessError`][] is returned.  If you want a blocking version, use
/// [`take_blocking`][].  If you want to access the manager instance even if the cache is poisoned,
/// use [`force_take`][].
///
/// # Errors
///
/// - [`ConcurrentAccessError`][] if the token for the `Manager` instance cannot be locked
/// - [`PoisonError`][] if the lock is poisoned
///
/// [`take_blocking`]: fn.take_blocking.html
/// [`force_take`]: fn.force_take.html
/// [`ConcurrentAccessError`]: struct.Error.html#variant.ConcurrentAccessError
/// [`PoisonError`]: struct.Error.html#variant.PoisonError
/// [`Manager`]: struct.Manager.html
pub fn take() -> Result<sync::MutexGuard<'static, Manager>, Error> {
    MANAGER.try_lock().map_err(Into::into)
}

/// Try to take an instance of the connection manager, ignoring a poisoned cache.
///
/// There may only be one [`Manager`][] instance at the same time.  If there already is an
/// instance, a [`ConcurrentAccessError`][] is returned.  If you want a blocking version, use
/// [`take_blocking`][].
///
/// If a thread has previously panicked while accessing the manager instance, the cache is
/// poisoned.  The default implementation, [`take`][], returns a [`PoisonError`][] on subsequent
/// calls.  This implementation ignores the poisoned cache and returns the manager instance.
///
/// # Errors
///
/// - [`ConcurrentAccessError`][] if the token for the `Manager` instance cannot be locked
///
/// [`take`]: fn.take.html
/// [`take_blocking`]: fn.take_blocking.html
/// [`ConcurrentAccessError`]: struct.Error.html#variant.ConcurrentAccessError
/// [`Manager`]: struct.Manager.html
pub fn force_take() -> Result<sync::MutexGuard<'static, Manager>, Error> {
    match take() {
        Ok(guard) => Ok(guard),
        Err(err) => match err {
            Error::PoisonError(err) => Ok(err.into_inner()),
            err => Err(err),
        },
    }
}

/// List all connected Nitrokey devices.
///
/// This functions returns a vector with [`DeviceInfo`][] structs that contain information about
/// all connected Nitrokey devices.  It will even list unsupported models, although you cannot
/// connect to them.  To connect to a supported model, call the [`connect_path`][] function.
///
/// # Errors
///
/// - [`NotConnected`][] if a Nitrokey device has been disconnected during enumeration
/// - [`Utf8Error`][] if the USB path or the serial number returned by libnitrokey are invalid
///   UTF-8 strings
///
/// # Example
///
/// ```
/// let devices = nitrokey::list_devices()?;
/// if devices.is_empty() {
///     println!("No connected Nitrokey devices found.");
/// } else {
///     println!("model\tpath\tserial number");
///     for device in devices {
///         match device.model {
///             Some(model) => print!("{}", model),
///             None => print!("unsupported"),
///         }
///         print!("\t{}\t", device.path);
///         match device.serial_number {
///             Some(serial_number) => println!("{}", serial_number),
///             None => println!("unknown"),
///         }
///     }
/// }
/// # Ok::<(), nitrokey::Error>(())
/// ```
///
/// [`connect_path`]: struct.Manager.html#fn.connect_path
/// [`DeviceInfo`]: struct.DeviceInfo.html
/// [`NotConnected`]: enum.CommunicationError.html#variant.NotConnected
/// [`Utf8Error`]: enum.Error.html#variant.Utf8Error
pub fn list_devices() -> Result<Vec<DeviceInfo>, Error> {
    let ptr = NonNull::new(unsafe { nitrokey_sys::NK_list_devices() });
    match ptr {
        Some(mut ptr) => {
            let mut vec: Vec<DeviceInfo> = Vec::new();
            push_device_info(&mut vec, unsafe { ptr.as_ref() })?;
            unsafe {
                nitrokey_sys::NK_free_device_info(ptr.as_mut());
            }
            Ok(vec)
        }
        None => get_last_result().map(|_| Vec::new()),
    }
}

fn push_device_info(
    vec: &mut Vec<DeviceInfo>,
    info: &nitrokey_sys::NK_device_info,
) -> Result<(), Error> {
    vec.push(info.try_into()?);
    if let Some(ptr) = NonNull::new(info.next) {
        push_device_info(vec, unsafe { ptr.as_ref() })?;
    }
    Ok(())
}

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

/// Returns the libnitrokey library version.
///
/// # Errors
///
/// - [`Utf8Error`][] if libnitrokey returned an invalid UTF-8 string
///
/// # Example
///
/// ```
/// let version = nitrokey::get_library_version()?;
/// println!("Using libnitrokey {}", version.git);
/// # Ok::<(), nitrokey::Error>(())
/// ```
///
/// [`Utf8Error`]: enum.Error.html#variant.Utf8Error
pub fn get_library_version() -> Result<Version, Error> {
    // NK_get_library_version returns a static string, so we don’t have to free the pointer.
    let git = unsafe { nitrokey_sys::NK_get_library_version() };
    let git = if git.is_null() {
        String::new()
    } else {
        util::owned_str_from_ptr(git)?
    };
    let major = unsafe { nitrokey_sys::NK_get_major_library_version() };
    let minor = unsafe { nitrokey_sys::NK_get_minor_library_version() };
    Ok(Version { git, major, minor })
}
