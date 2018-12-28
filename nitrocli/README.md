[![pipeline](https://gitlab.com/d-e-s-o/nitrocli/badges/master/pipeline.svg)](https://gitlab.com/d-e-s-o/nitrocli/commits/master)
[![crates.io](https://img.shields.io/crates/v/nitrocli.svg)](https://crates.io/crates/nitrocli)
[![rustc](https://img.shields.io/badge/rustc-1.31+-blue.svg)](https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html)

nitrocli
========

- [Changelog](CHANGELOG.md)

**nitrocli** is a program that provides a command line interface for
certain commands on the [Nitrokey Storage][nitrokey-storage] device.

The following commands are currently supported:
- status: Report status information about the Nitrokey.
- clear: Remove the user and admin PIN from gpg-agent's cache.
- storage: Work with the Nitrokey's storage.
  - open: Open the encrypted volume. The user PIN needs to be entered.
  - close: Close the encrypted volume.
- otp: Access one-time passwords (OTP).
  - get: Generate a one-time password.
  - set: Set an OTP slot.
  - status: List all OTP slots.
  - clear: Delete an OTP slot.

### *Note:*
----------------------------------------------------------------------
> **nitrocli** requires the Nitrokey Storage to be running **firmware
> version 0.47** or higher. Versions before that reported incorrect
> checksums which will cause the program to indicate data retrieval
> errors, causing commands to fail.
----------------------------------------------------------------------


Usage
-----

Usage is as simple as providing the name of the respective command as a
parameter (note that some commands are organized through subcommands,
which are required as well), e.g.:
```bash
# Open the nitrokey's encrypted volume.
$ nitrocli storage open

$ nitrocli status
Status:
  model:             Storage
  serial number:     0x00053141
  firmware version:  0.47
  user retry count:  3
  admin retry count: 3

  SD card ID:        0x05dcad1d
  firmware:          unlocked
  storage keys:      created
  volumes:
    unencrypted:     active
    encrypted:       active
    hidden:          inactive

# Close it again.
$ nitrocli storage close
```


Installation
------------

The following dependencies are required:
- **hidapi**: In order to provide USB access this library is used.
- **GnuPG**: The `gpg-connect-agent` program allows the user to enter
             PINs.

#### From Source
In order to compile the program the `hid` crate needs to be available
which allows to access the nitrokey as a USB HID device. This crate and
its dependencies are contained in the form of subrepos in compatible and
tested versions. Cargo is required to build the program.

The build is as simple as running:
```bash
$ cargo build --release
```

It is recommended that the resulting executable be installed in a
directory accessible via the `PATH` environment variable.

#### From Crates.io
**nitrocli** is [published][nitrocli-cratesio] on crates.io. If an
installation from the checked-out source code is not desired, a
quick-and-dirty local installation can happen via:
```bash
$ cargo install nitrocli --root=$PWD/nitrocli
```

#### Via Packages
Packages are available for:
- Arch Linux: [`nitrocli`](https://aur.archlinux.org/packages/nitrocli/) in the
  Arch User Repository
- Gentoo Linux: [`app-crypt/nitrocli`](https://github.com/d-e-s-o/nitrocli-ebuild)
  ebuild


Acknowledgments
---------------

Robin Krahl ([@robinkrahl](https://github.com/robinkrahl)) has been
a crucial help for the development of **nitrocli**.

The [Nitrokey UG][nitrokey-ug] has generously provided the necessary
hardware for developing and testing the program.


[nitrokey-ug]: https://www.nitrokey.com
[nitrokey-storage]: https://www.nitrokey.com/news/2016/nitrokey-storage-available
[nitrocli-cratesio]: https://crates.io/crates/nitrocli
