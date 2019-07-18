<!---
Copyright (C) 2019 Robin Krahl <robin.krahl@ireas.org>
SPDX-License-Identifier: MIT
-->

# nitrokey-rs

A libnitrokey wrapper for Rust providing access to Nitrokey devices.

[Documentation][]

## Compatibility

The required [`libnitrokey`][] version is built from source.  The host system
must provide `libhidapi-libusb0` (Linux) or `libhidapi` (non-Linux) in the
default library search path.  Depending on your system, you might also have to
install the [Nitrokey udev rules][].

Currently, this crate provides access to the common features of the Nitrokey
Pro and the Nitrokey Storage:  general configuration, OTP generation and the
password safe.  Basic support for the secure storage on the Nitrokey Storage is
available but still under development.

### Unsupported Functions

The following functions provided by `libnitrokey` are deliberately not
supported by `nitrokey-rs`:

- `NK_get_device_model`.  We know which model we connected to, so we can
  provide this information without calling `libnitrokey`.
- `NK_get_time`.  This method is useless as it will always cause a timestamp
  error on the device (see [pull request #114][] for `libnitrokey` for details).
- `NK_get_status`.  This method only provides a string representation of
  data that can be accessed by other methods (firmware version, serial number,
  configuration).
- `NK_get_status_storage_as_string`.  This method only provides an incomplete
  string representation of the data returned by `NK_get_status_storage`.
- `NK_is_AES_supported`.  This method is no longer needed for Nitrokey devices
  with a recent firmware version.
- `NK_set_unencrypted_volume_rorw_pin_type_user`,
  `NK_set_unencrypted_read_only`, `NK_set_unencrypted_read_write`,
  `NK_set_encrypted_read_only` and `NK_set_encrypted_read_write`.  These
  methods are only relevant for older firmware versions (pre-v0.51).  As the
  Nitrokey Storage firmware can be updated easily, we do not support these
  outdated versions.

## Tests

This crate has tests for different scenarios:  Some tests require that no
Nitrokey device is connected, others require a Nitrokey Storage or a Nitrokey
Pro.  We use the [`nitrokey-test`][] crate to select the test cases.  You can
just run `cargo test` to auto-detect connected Nitrokey devices and to run the
appropriate tests.  If you want to manually select the tests, set the
`NITROKEY_TEST_GROUP` environment variable to `nodev` (no device connected),
`pro` (Nitrokey Pro connected) or `storage` (Nitrokey Storage connected).

Note that the tests assume that the device’s passwords are the factory defaults
(admin PIN `12345678`, user PIN `123456`, update password `12345678`) and that
an AES key has been built.  Some tests will overwrite the data stored on the
Nitrokey device or perform a factory reset.  Never execute the tests if you
unless yout want to destroy all data on all connected Nitrokey devices!

## Acknowledgments

Thanks to Nitrokey UG for providing a Nitrokey Storage to support the
development of this crate.  Thanks to Daniel Mueller for contributions to
`nitrokey-rs` and for the `nitrokey-test` crate.

## Contact

For bug reports, patches, feature requests or other messages, please send a
mail to [nitrokey-rs-dev@ireas.org][].

## License

This project is licensed under the [MIT License][].  `libnitrokey` is licensed
under the [LGPL-3.0][].

`nitrokey-rs` complies with [version 2.0 of the REUSE practices][reuse].

[Documentation]: https://docs.rs/nitrokey
[Nitrokey udev rules]: https://www.nitrokey.com/documentation/frequently-asked-questions-faq#openpgp-card-not-available
[`libnitrokey`]: https://github.com/nitrokey/libnitrokey
[`nitrokey-test`]: https://github.com/d-e-s-o/nitrokey-test
[nitrokey-rs-dev@ireas.org]: mailto:nitrokey-rs-dev@ireas.org
[pull request #114]: https://github.com/Nitrokey/libnitrokey/pull/114
[MIT license]: https://opensource.org/licenses/MIT
[LGPL-3.0]: https://opensource.org/licenses/lgpl-3.0.html
[reuse]: https://reuse.software/practices/2.0/
