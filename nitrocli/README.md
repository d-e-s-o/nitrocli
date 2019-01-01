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
- lock: Lock the Nitrokey.
- config: Access the Nitrokey's configuration
  - get: Read the current configuration.
  - set: Change the configuration.
- storage: Work with the Nitrokey's storage.
  - open: Open the encrypted volume. The user PIN needs to be entered.
  - close: Close the encrypted volume.
  - status: Print information about the Nitrokey's storage.
- otp: Access one-time passwords (OTP).
  - get: Generate a one-time password.
  - set: Set an OTP slot.
  - status: List all OTP slots.
  - clear: Delete an OTP slot.
- pws: Access the password safe (PWS).
  - get: Query the data on a PWS slot.
  - set: Set the data on a PWS slot.
  - status: List all PWS slots.
  - clear: Delete a PWS slot.
- pin: Manage the Nitrokey’s PINs.
  - clear: Remove the user and admin PIN from gpg-agent's cache.
  - set: Change the admin or the user PIN.
  - unblock: Unblock and reset the user PIN.


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

$ nitrocli storage status
Status:
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

More examples, a more detailed explanation of the purpose, the potential
subcommands, as well as the parameters of each command are provided in
the [`man` page](doc/nitrocli.1.pdf).


Installation
------------

In addition to Rust itself and Cargo, its package management tool, the
following dependencies are required:
- **hidapi**: In order to provide USB access this library is used.
- **GnuPG**: The `gpg-connect-agent` program allows the user to enter
             PINs.

#### Via Packages
Packages are available for:
- Arch Linux: [`nitrocli`](https://aur.archlinux.org/packages/nitrocli/) in the
  Arch User Repository
- Gentoo Linux: [`app-crypt/nitrocli`](https://github.com/d-e-s-o/nitrocli-ebuild)
  ebuild

#### From Crates.io
**nitrocli** is [published][nitrocli-cratesio] on crates.io and can
directly be installed from there:
```bash
$ cargo install nitrocli --root=$PWD/nitrocli
```

#### From Source
After cloning the repository and changing into the `nitrocli` subfolder,
the build is as simple as running:
```bash
$ cargo build --release
```

It is recommended that the resulting executable be installed in a
directory accessible via the `PATH` environment variable.


Contributing
------------

Contributions are generally welcome. Please follow the guidelines
outlined in [CONTRIBUTING.md](CONTRIBUTING.md).


Acknowledgments
---------------

Robin Krahl ([@robinkrahl](https://github.com/robinkrahl)) has been
a crucial help for the development of **nitrocli**.

The [Nitrokey UG][nitrokey-ug] has generously provided the necessary
hardware for developing and testing the program.


License
-------
**nitrocli** is made available under the terms of the
[GPLv3](gplv3-tldr).

See the [LICENSE] file that accompanies this distribution for the full
text of the license.


[nitrokey-ug]: https://www.nitrokey.com
[nitrokey-storage]: https://www.nitrokey.com/news/2016/nitrokey-storage-available
[nitrocli-cratesio]: https://crates.io/crates/nitrocli
[gplv3-tldr]: https://tldrlegal.com/license/gnu-general-public-license-v3-(gpl-3)
