// Copyright (C) 2019-2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: MIT

use std::convert::TryFrom as _;
use std::fmt;
use std::ops;

use nitrokey_sys;

use crate::device::{Device, FirmwareVersion, Model, Status};
use crate::error::{CommandError, Error};
use crate::otp::GenerateOtp;
use crate::util::{get_command_result, get_cstring, get_last_error};

/// A Nitrokey Storage device without user or admin authentication.
///
/// Use the [`connect`][] method to obtain an instance wrapper or the [`connect_storage`] method to
/// directly obtain an instance.  If you want to execute a command that requires user or admin
/// authentication, use [`authenticate_admin`][] or [`authenticate_user`][].
///
/// # Examples
///
/// Authentication with error handling:
///
/// ```no_run
/// use nitrokey::{Authenticate, User, Storage};
/// # use nitrokey::Error;
///
/// fn perform_user_task<'a>(device: &User<'a, Storage<'a>>) {}
/// fn perform_other_task(device: &Storage) {}
///
/// # fn try_main() -> Result<(), Error> {
/// let mut manager = nitrokey::take()?;
/// let device = manager.connect_storage()?;
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
/// [`connect`]: struct.Manager.html#method.connect
/// [`connect_storage`]: struct.Manager.html#method.connect_storage
#[derive(Debug)]
pub struct Storage<'a> {
    manager: Option<&'a mut crate::Manager>,
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

/// The progress of a background operation on the Nitrokey.
///
/// Some commands may start a background operation during which no other commands can be executed.
/// This enum stores the status of a background operation:  Ongoing with a relative progress (up to
/// 100), or idle, i. e. no background operation has been started or the last one has been
/// finished.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OperationStatus {
    /// A background operation with its progress value (less than or equal to 100).
    Ongoing(u8),
    /// No backgrund operation.
    Idle,
}

impl<'a> Storage<'a> {
    pub(crate) fn new(manager: &'a mut crate::Manager) -> Storage<'a> {
        Storage {
            manager: Some(manager),
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect_storage()?;
    /// match device.get_storage_status() {
    ///     Ok(status) => {
    ///         println!("SD card ID: {:#x}", status.serial_number_sd_card);
    ///     },
    ///     Err(err) => eprintln!("Could not get Storage status: {}", err),
    /// };
    /// #     Ok(())
    /// # }
    /// ```
    pub fn get_storage_status(&self) -> Result<StorageStatus, Error> {
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
    /// let mut manager = nitrokey::take()?;
    /// let device = manager.connect_storage()?;
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
    /// let mut manager = nitrokey::take()?;
    /// let mut device = manager.connect_storage()?;
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

    /// Returns a range of the SD card that has not been used to during this power cycle.
    ///
    /// The Nitrokey Storage tracks read and write access to the SD card during a power cycle.
    /// This method returns a range of the SD card that has not been accessed during this power
    /// cycle.  The range is relative to the total size of the SD card, so both values are less
    /// than or equal to 100.  This can be used as a guideline when creating a hidden volume.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mut manager = nitrokey::take()?;
    /// let storage = manager.connect_storage()?;
    /// let usage = storage.get_sd_card_usage()?;
    /// println!("SD card usage: {}..{}", usage.start, usage.end);
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    pub fn get_sd_card_usage(&self) -> Result<ops::Range<u8>, Error> {
        let mut usage_data = nitrokey_sys::NK_SD_usage_data {
            write_level_min: 0,
            write_level_max: 0,
        };
        let result = unsafe { nitrokey_sys::NK_get_SD_usage_data(&mut usage_data) };
        match get_command_result(result) {
            Ok(_) => {
                if usage_data.write_level_min > usage_data.write_level_max
                    || usage_data.write_level_max > 100
                {
                    Err(Error::UnexpectedError)
                } else {
                    Ok(ops::Range {
                        start: usage_data.write_level_min,
                        end: usage_data.write_level_max,
                    })
                }
            }
            Err(err) => Err(err),
        }
    }

    /// Blinks the red and green LED alternatively and infinitely until the device is reconnected.
    pub fn wink(&mut self) -> Result<(), Error> {
        get_command_result(unsafe { nitrokey_sys::NK_wink() })
    }

    /// Returns the status of an ongoing background operation on the Nitrokey Storage.
    ///
    /// Some commands may start a background operation during which no other commands can be
    /// executed.  This method can be used to check whether such an operation is ongoing.
    ///
    /// Currently, this is only used by the [`fill_sd_card`][] method.
    ///
    /// [`fill_sd_card`]: #method.fill_sd_card
    pub fn get_operation_status(&self) -> Result<OperationStatus, Error> {
        let status = unsafe { nitrokey_sys::NK_get_progress_bar_value() };
        match status {
            0..=100 => u8::try_from(status)
                .map(OperationStatus::Ongoing)
                .map_err(|_| Error::UnexpectedError),
            -1 => Ok(OperationStatus::Idle),
            -2 => Err(get_last_error()),
            _ => Err(Error::UnexpectedError),
        }
    }

    /// Overwrites the SD card with random data.
    ///
    /// Ths method starts a background operation that overwrites the SD card with random data.
    /// While this operation is ongoing, no other commands can be executed.  Use the
    /// [`get_operation_status`][] function to check the progress of the operation.
    ///
    /// # Errors
    ///
    /// - [`InvalidString`][] if one of the provided passwords contains a null byte
    /// - [`WrongPassword`][] if the admin password is wrong
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nitrokey::OperationStatus;
    ///
    /// let mut manager = nitrokey::take()?;
    /// let mut storage = manager.connect_storage()?;
    /// storage.fill_sd_card("12345678")?;
    /// loop {
    ///     match storage.get_operation_status()? {
    ///         OperationStatus::Ongoing(progress) => println!("{}/100", progress),
    ///         OperationStatus::Idle => {
    ///             println!("Done!");
    ///             break;
    ///         }
    ///     }
    /// }
    /// # Ok::<(), nitrokey::Error>(())
    /// ```
    ///
    /// [`get_operation_status`]: #method.get_operation_status
    /// [`InvalidString`]: enum.LibraryError.html#variant.InvalidString
    /// [`WrongPassword`]: enum.CommandError.html#variant.WrongPassword
    pub fn fill_sd_card(&mut self, admin_pin: &str) -> Result<(), Error> {
        let admin_pin_string = get_cstring(admin_pin)?;
        get_command_result(unsafe {
            nitrokey_sys::NK_fill_SD_card_with_random_data(admin_pin_string.as_ptr())
        })
        .or_else(|err| match err {
            // libnitrokey’s C API returns a LongOperationInProgressException with the same error
            // code as the WrongCrc command error, so we cannot distinguish them.
            Error::CommandError(CommandError::WrongCrc) => Ok(()),
            err => Err(err),
        })
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

impl<'a> Drop for Storage<'a> {
    fn drop(&mut self) {
        unsafe {
            nitrokey_sys::NK_logout();
        }
    }
}

impl<'a> Device<'a> for Storage<'a> {
    fn into_manager(mut self) -> &'a mut crate::Manager {
        self.manager.take().unwrap()
    }

    fn get_model(&self) -> Model {
        Model::Storage
    }

    fn get_status(&self) -> Result<Status, Error> {
        // Currently, the GET_STATUS command does not report the correct firmware version and
        // serial number on the Nitrokey Storage, see [0].  Until this is fixed in libnitrokey, we
        // have to manually execute the GET_DEVICE_STATUS command (get_storage_status) and complete
        // the missing data, see [1].
        // [0] https://github.com/Nitrokey/nitrokey-storage-firmware/issues/96
        // [1] https://github.com/Nitrokey/libnitrokey/issues/166

        let mut raw_status = nitrokey_sys::NK_status {
            firmware_version_major: 0,
            firmware_version_minor: 0,
            serial_number_smart_card: 0,
            config_numlock: 0,
            config_capslock: 0,
            config_scrolllock: 0,
            otp_user_password: false,
        };
        get_command_result(unsafe { nitrokey_sys::NK_get_status(&mut raw_status) })?;
        let mut status = Status::from(raw_status);

        let storage_status = self.get_storage_status()?;
        status.firmware_version = storage_status.firmware_version;
        status.serial_number = storage_status.serial_number_smart_card;

        Ok(status)
    }
}

impl<'a> GenerateOtp for Storage<'a> {}

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
