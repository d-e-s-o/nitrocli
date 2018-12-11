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
