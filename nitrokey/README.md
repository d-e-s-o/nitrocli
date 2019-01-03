# nitrokey-rs

A libnitrokey wrapper for Rust providing access to Nitrokey devices.

[Documentation][]

## Compatibility

The required [`libnitrokey`][] version is built from source.  The host system
must provide `libhidapi-libusb0` in the default library search path.

Currently, this crate provides access to the common features of the Nitrokey
Pro and the Nitrokey Storage:  general configuration, OTP generation and the
password safe.  Basic support for the secure storage on the Nitrokey Storage is
available but still under development.

### Unsupported Functions

The following functions provided by `libnitrokey` are deliberately not
supported by `nitrokey-rs`:

- `NK_get_time()`.  This method is useless as it will always cause a timestamp
  error on the device (see [pull request #114][] for `libnitrokey` for details).
- `NK_get_status()`.  This method only provides a string representation of
  data that can be accessed by other methods (firmware version, serial number,
  configuration).
- `NK_get_status_storage_as_string()`.  This method only provides an incomplete
  string representation of the data returned by `NK_get_status_storage`.

## Tests

This crate has three test suites that can be selected using features.  One test
suite assumes that no Nitrokey device is connected. It is run if no other test
suite is selected.  The two other test suites require a Nitrokey Pro (feature
`test-pro`) or a Nitrokey Storage (feature `test-storage`) to be connected.

Use the `--features` option for Cargo to select one of the test suites.  You
should select more than one of the test suites at the same time.  Note that the
test suites that require a Nitrokey device assume that the device’s passwords
are the factory defaults (admin password `12345678` and user password
`123456`).  Running the test suite with a device with different passwords might
lock your device!  Also note that the test suite might delete or overwrite data
on all connected devices.

As the tests currently are not synchronized, you have to make sure that they
are not executed in parallel.  To do so, pass the option `--test-threads 1` to
the test executable.

In conclusion, you can use these commands to run the test suites:

```
$ cargo test
$ cargo test --features test-pro -- --test-threads 1
$ cargo test --features test-storage -- --test-threads 1
```

The `totp_no_pin` and `totp_pin` tests can occasionally fail due to bad timing.

## Acknowledgments

Thanks to Nitrokey UG for providing a Nitrokey Storage to support the
development of this crate.

## Contact

For bug reports, patches, feature requests or other messages, please send a
mail to [nitrokey-rs-dev@ireas.org][].

## License

This project is licensed under the [MIT License][].  `libnitrokey` is licensed
under the [LGPL-3.0][].

[Documentation]: https://docs.rs/nitrokey
[`libnitrokey`]: https://github.com/nitrokey/libnitrokey
[nitrokey-rs-dev@ireas.org]: mailto:nitrokey-rs-dev@ireas.org
[pull request #114]: https://github.com/Nitrokey/libnitrokey/pull/114
[MIT license]: https://opensource.org/licenses/MIT
[LGPL-3.0]: https://opensource.org/licenses/lgpl-3.0.html
