nitrocli
========

**nitrocli** is a program that provides a command line interface for
certain commands on the [Nitrokey Storage][nitrokey] device.

The following commands are currently supported:
- open: Open the encrypted volume. The user PIN needs to be entered.
- close: Close the encrypted volume.
- status: Report status information about the Nitrokey.
- clear: Remove the user PIN from gpg-agent's cache.

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
parameter, e.g.:
```bash
# Open the nitrokey's encrypted volume.
$ nitrocli open

$ nitrocli status
Status:
  SD card ID:        0xdeadbeef
  firmware version:  0.47
  firmware:          unlocked
  storage keys:      created
  user retry count:  3
  admin retry count: 3
  volumes:
    unencrypted:     active
    encrypted:       active
    hidden:          inactive

# Close it again.
$ nitrocli close
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
If you are using [Gentoo Linux](https://www.gentoo.org/), there is an
[ebuild](https://github.com/d-e-s-o/nitrocli-ebuild) available that can
be used directly.

[nitrokey]: https://www.nitrokey.com/news/2016/nitrokey-storage-available
[nitrocli-cratesio]: https://crates.io/crates/nitrocli
