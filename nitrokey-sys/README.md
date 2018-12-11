# nitrokey-sys-rs

Low-level Rust bindings for `libnitrokey`, providing access to Nitrokey
devices.

```toml
[dependencies]
nitrokey-sys = "3.4.1"
```

The version of this crate corresponds to the wrapped `libnitrokey` version.
This crate contains a copy of the `libnitrokey` library, builds it from source
and links it statically.  The host system must provide its dependencies in the
library search path:

- `libhidapi-libusb0`

## Contact

For bug reports, patches, feature requests or other messages, please send a
mail to [nitrokey-rs-dev@ireas.org][].

## License

This project as well as `libnitrokey` are licensed under the [LGPL-3.0][].

[`libnitrokey`]: https://github.com/nitrokey/libnitrokey
[nitrokey-rs-dev@ireas.org]: mailto:nitrokey-rs-dev@ireas.org
[LGPL-3.0]: https://opensource.org/licenses/lgpl-3.0.html
