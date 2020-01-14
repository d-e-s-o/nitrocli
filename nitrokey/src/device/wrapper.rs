// Copyright (C) 2018-2019 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use crate::device::{Device, Model, Pro, Status, Storage};
use crate::error::Error;
use crate::otp::GenerateOtp;

/// A wrapper for a Nitrokey device of unknown type.
///
/// Use the [`connect`][] method to obtain a wrapped instance.  The wrapper implements all traits
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
/// fn perform_user_task<'a>(device: &User<'a, DeviceWrapper<'a>>) {}
/// fn perform_other_task(device: &DeviceWrapper) {}
///
/// # fn try_main() -> Result<(), Error> {
/// let mut manager = nitrokey::take()?;
/// let device = manager.connect()?;
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
/// let mut manager = nitrokey::take()?;
/// let device = manager.connect()?;
/// perform_common_task(&device);
/// match device {
///     DeviceWrapper::Storage(storage) => perform_storage_task(&storage),
///     _ => (),
/// };
/// #     Ok(())
/// # }
/// ```
///
/// [`connect`]: struct.Manager.html#method.connect
#[derive(Debug)]
pub enum DeviceWrapper<'a> {
    /// A Nitrokey Storage device.
    Storage(Storage<'a>),
    /// A Nitrokey Pro device.
    Pro(Pro<'a>),
}

impl<'a> DeviceWrapper<'a> {
    fn device(&self) -> &dyn Device<'a> {
        match *self {
            DeviceWrapper::Storage(ref storage) => storage,
            DeviceWrapper::Pro(ref pro) => pro,
        }
    }

    fn device_mut(&mut self) -> &mut dyn Device<'a> {
        match *self {
            DeviceWrapper::Storage(ref mut storage) => storage,
            DeviceWrapper::Pro(ref mut pro) => pro,
        }
    }
}

impl<'a> From<Pro<'a>> for DeviceWrapper<'a> {
    fn from(device: Pro<'a>) -> Self {
        DeviceWrapper::Pro(device)
    }
}

impl<'a> From<Storage<'a>> for DeviceWrapper<'a> {
    fn from(device: Storage<'a>) -> Self {
        DeviceWrapper::Storage(device)
    }
}

impl<'a> GenerateOtp for DeviceWrapper<'a> {
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

impl<'a> Device<'a> for DeviceWrapper<'a> {
    fn into_manager(self) -> &'a mut crate::Manager {
        match self {
            DeviceWrapper::Pro(dev) => dev.into_manager(),
            DeviceWrapper::Storage(dev) => dev.into_manager(),
        }
    }

    fn get_model(&self) -> Model {
        match *self {
            DeviceWrapper::Pro(_) => Model::Pro,
            DeviceWrapper::Storage(_) => Model::Storage,
        }
    }

    fn get_status(&self) -> Result<Status, Error> {
        match self {
            DeviceWrapper::Pro(dev) => dev.get_status(),
            DeviceWrapper::Storage(dev) => dev.get_status(),
        }
    }
}
