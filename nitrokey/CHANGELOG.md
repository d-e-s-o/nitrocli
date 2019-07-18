<!---
Copyright (C) 2019 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# Unreleased
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
