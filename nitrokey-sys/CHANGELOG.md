# v3.5.0 (2019-07-04)
- Mark deprecated functions using the `deprecated` attribute.
- Update to libnitrokey 3.5, causing all following changes.
- New constant `NK_PWS_SLOT_COUNT`.
- New structures:
  - `NK_device_info`
  - `NK_status`
  - `NK_SD_usage_data`
  - `ReadSlot_t`
- New functions:
  - `NK_get_SD_usage_data`
  - `NK_get_status`
  - `NK_get_status_as_string`
  - `NK_list_devices`
  - `NK_free_device_info`
  - `NK_connect_with_path`
  - `NK_enable_firmware_update_pro`
  - `NK_change_firmware_password_pro`
  - `NK_read_HOTP_slot`
- Deprecated functions:
  - `NK_status`
- Changed the return type for `NK_get_major_firmware_version` and
  `NK_get_minor_firmware_version` to `u8`.
- Changed `NK_get_progress_bar_value` to return -2 instead of 0 if an error
  occurs.

# v3.4.3 (2019-10-12)
- Link directly against `libnitrokey` if the `USE_SYSTEM_LIBNITROKEY`
  environment variable is set.

# v3.4.2 (2019-01-01)
- Use the -std=c++14 compiler flag.
- Change the build script to link to `-lhidapi` on non-Linux operating systems
  (while still using `-lhidapi-libusb` on Linux).
- Decouple the libnitrokey and nitrokey-sys-rs versions.

# v3.4.1 (2018-12-10)

- Update to libnitrokey 3.4.1.  There are no changes affecting this crate.

# v3.4.0 (2018-12-10)

- Update to libnitrokey 3.4, causing all following changes.
- New constant `NK_device_model_NK_DISCONNECTED` in the `NK_device_model`
  enumeration.
- New structures:
    - `NK_storage_ProductionTest`
    - `NK_storage_status`
- New functions:
    - `NK_get_device_model`
    - `NK_get_library_version`
    - `NK_get_major_library_version`
    - `NK_get_minor_libray_version`
    - `NK_get_status_storage`
    - `NK_get_storage_production_info`
    - `NK_totp_set_time_soft`
    - `NK_wink`
- The function `NK_totp_get_time` is now deprecated.  If applicable,
  `NK_totp_set_time_soft` should be used instead.  See the [upstream pull
  request #114][] for details.
- Strings are now returned as mutable instead of constant pointers.

# v3.3.0 (2018-05-21)

- Change the crate license to LGPL 3.0.
- Adapt the crate version number according to the bundled `libnitrokey`
  version.
- Include a copy of `libnitrokey`.
- Compile `libnitrokey` from source.
- Generate the `bindgen` bindings statically and add them to the repository.

# v0.1.0 (2018-05-19)

- Initial release.

[upstream pull request #114]: https://github.com/Nitrokey/libnitrokey/pull/114
