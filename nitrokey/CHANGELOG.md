<!---
Copyright (C) 2019-2020 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: CC0-1.0
-->

# v0.5.1 (2020-01-15)
- Fix serial number formatting for Nitrokey Pro devices with firmware 0.8 or
  older in the `list_devices` function.

# v0.5.0 (2020-01-14)
- List these libnitrokey functions as unsupported:
  - `NK_change_firmware_password_pro`
  - `NK_connect_with_ID`
  - `NK_enable_firmware_update_pro`
  - `NK_list_devices_by_cpuID`
  - `NK_send_startup`
- Implement connection by path:
  - Add the `Error::UnsupportedDeviceError` variant.
  - Add the `DeviceInfo` struct.
  - Add the `list_devices` function.
  - Add the `connect_path` function to the `Manager` struct.
- Add the `get_status` function to the `Device` trait.
- Rename `Status::get_status` to `get_storage_status`.
- Add the `get_sd_card_usage` function to the `Storage` struct.
- Add the `OperationStatus` enum and the `get_operation_status` function for
  the `Storage` struct.
- Add the `fill_sd_card` function to the `Storage` struct.

# v0.4.0 (2020-01-02)
- Remove the `test-pro` and `test-storage` features.
- Implement `Display` for `Version`.
- Introduce `DEFAULT_ADMIN_PIN` and `DEFAULT_USER_PIN` constants.
- Refactor the error handling code:
  - Implement `std::error::Error` for `CommandError`.
  - Add the `Error` enum.
  - Add the `LibraryError` enum and move the library error variants from
    `CommandError` to `LibraryError`.
  - Add the `CommunicationError` enum and move the communication error variants
    from `CommandError` to `CommunicationError`.
  - Return `Error` instead of `CommandError` in all public functions.
  - Move the `CommandError::RngError` variant to `Error::RandError` and the
    `CommandError::Unknown` variant to `Error::UnknownError`.
  - Return `CommunicationError::NotConnected` instead of
    `CommandError::Undefined` from the connect functions.
  - Remove the `CommandError::Undefined` variant.
- Add a private `PhantomData` field to `Pro` and `Storage` to make direct
  instantiation impossible.
- Refactor and clean up internal code:
  - Prefer using the `Into` trait over numeric casting.
  - Add `Pro::new` and `Storage::new` functions.
- Implement `From<Pro>` and `From<Storage>` for `DeviceWrapper`.
- Add `Error::Utf8Error` variant.
  - Return `Result<Version>` instead of `Version` from `get_library_version`.
  - Return `Error::Utf8Error` if libnitrokey returns an invalid UTF-8 string.
- Implement `From<(T: Device, Error)>` for `Error`.
- Fix timing issues with the `totp_no_pin` and `totp_pin` test cases.
- Always return a `Result` in functions that communicate with a device.
- Combine `get_{major,minor}_firmware_version` into `get_firmware_version`.
- Add `set_encrypted_volume_mode` to `Storage`.
- Use mutability to represent changes to the device status:
  - Implement `DerefMut` for `User<T>` and `Admin<T>`.
  - Add `device_mut` method to `DeviceWrapper`.
  - Require a mutable `Device` reference if a method changes the device state.
- Update dependencies:
  - `nitrokey-sys` to 3.5
  - `nitrokey-test` to 0.3
  - `rand_core` to 0.5
  - `rand_os` to 0.2
- Add `nitrokey-test-state` dependency in version 0.1.
- Refactor connection management:
  - Add `ConcurrentAccessError` and `PoisonError` `Error` variants.
  - Add the `Manager` struct that manages connections to Nitrokey devices.
  - Remove `connect`, `connect_model`, `Pro::connect` and `Storage::connect`.
  - Add the `into_manager` function to the `Device` trait.
  - Add the `force_take` function that ignores a `PoisonError` when accessing
    the manager instance.
- Internally refactor the `device` module into submodules.

# v0.3.5 (2019-12-16)
- Update the nitrokey-sys dependency version specification to ~3.4.

# v0.3.4 (2019-01-20)
- Fix authentication methods that assumed that `char` is signed.

# v0.3.3 (2019-01-16)
- Add the `get_production_info` and `clear_new_sd_card_warning` methods to the
  `Storage` struct.
- Use `rand_os` instead of `rand` for random data creation.
  - (Re-)add `CommandError::RngError` variant.
- Account for the possibility that an empty string returned by libnitrokey can
  not only indicate an error but also be a valid return value.
- Make test cases more robust and avoid side effects on other test cases.

# v0.3.2 (2019-01-12)
- Make three additional error codes known: `CommandError::StringTooLong`,
  `CommandError::InvalidHexString` and `CommandError::TargetBufferTooSmall`.
- Add the `get_library_version` function to query the libnitrokey version.
- Add the `wink` method to the `Storage` struct.
- Add the `set_unencrypted_volume_mode` to set the access mode of the
  unencrypted volume.
- Add the `export_firmware` method to the `Storage` struct.

# v0.3.1 (2019-01-07)
- Use `nitrokey-test` to select and execute the unit tests.
- Add support for the hidden volumes on a Nitrokey Storage
  (`enable_hidden_volume`, `disable_hidden_volume` and `create_hidden_volume`
  methods for the `Storage` struct).
- Add the `connect_model` function to connect to a specific model using an enum
  variant.

# v0.3.0 (2019-01-04)
- Add a `force` argument to `ConfigureOtp::set_time`.
- Remove the obsolete `CommandError::RngError`.
- Add `CommandError::Undefined` to represent errors without further
  information (e. g. a method returned `NULL` unexpectedly).
- Add error code to `CommandError::Unknown`.
- Add the `Storage::change_update_pin` method that changes the firmware update
  PIN.
- Add the `Device::factory_reset` method that performs a factory reset.
- Add the `Device::build_aes_key` method that builds a new AES key on the Nitrokey.
- Add the `Storage::enable_firmware_update` method that puts the Nitrokey
  Storage in update mode so that the firmware can be updated.

# v0.2.3 (2018-12-31)

- Dummy release to fix an issue with the crates.io tarball.

# v0.2.2 (2018-12-30)

- Update to Rust edition 2018.
- Remove the `test-no-device` feature.
- Update the rand dependency to version 0.6.
- Add function `Device::get_model` that returns the connected model.
- Derive the `Copy` and `Clone` traits for the enums `CommandError`, `LogLevel`
  and `OtpMode`

# v0.2.1 (2018-12-10)

- Re-export `device::{StorageStatus, VolumeStatus}` in `lib.rs`.

# v0.2.0 (2018-12-10)

- Update to libnitrokey v3.4.1.
- Major refactoring of the existing code structure.
- Add support for most of the Nitrokey Pro features and some of the Nitrokey
  Storage features. See the `TODO.md` file for more details about the missing
  functionality.

# v0.1.1 (2018-05-21)

- Update the `nitrokey-sys` dependency to version 3.3.0.  Now `libnitrokey`
  is built from source and `bindgen` is no longer a build dependency.
- Add `get_minor_firmware_version` to `Device`.
- Use `NK_login_enum` instead of `NK_login` in `Device::connect`.

# v0.1.0 (2018-05-19)

- Initial release
