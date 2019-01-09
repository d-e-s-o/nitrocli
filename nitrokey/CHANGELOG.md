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
