Unreleased
----------
- Added support for user provided extensions through lookup via the
  `PATH` environment variable
- Added support for configuration files
  - Added support for configuration files that can be used to set
    default values for some arguments
  - Added `toml` dependency in version `0.5.6`
  - Added `serde` dependency in version `1.0.114`
  - Added `envy` dependency in version `0.4.1`
  - Added `merge` dependency in version `0.1.0`
  - Added `directories` dependency in version `3.0.1`
- Reworked connection handling for multiple attached Nitrokey devices:
  - Fail if multiple attached devices match the filter options (or no filter
    options are set)
  - Added `--serial-number` option that restricts the serial number of the
    device to connect to
  - Added `--usb-path` option that restricts the USB path of the device to
    connect to
- Added the `fill` command that fills the SD card of a Nitrokey Storage device
  with random data
  - Added the `termion` dependency in version `1.5.5`
- Added SD card usage information to the output of the `status` command for
  Storage devices
- Bumped `structopt` dependency to `0.3.17`


0.3.4
-----
- Changed default OTP format from `hex` to `base32`
- Improved error reporting format and fidelity
  - Added `anyhow` dependency in version `1.0.32`
- Reworked environment variables:
  - Added the `NITROCLI_MODEL` and `NITROCLI_VERBOSITY` variables that
    set the defaults for the `--model` and `--verbose` options
  - Changed the handling of the `NITROCLI_NO_CACHE` variable to check
    the value of the variable instead of only the presence
- Declared public API to be the man page
- Adjusted license & copyright headers to comply with REUSE 3.0
  - Added CI stage checking compliance
- Updated minimum required Rust version to `1.42.0`
- Bumped `nitrokey` dependency to `0.7.1`
- Bumped `proc-macro2` dependency to `1.0.19`
- Bumped `syn` dependency to `1.0.36`


0.3.3
-----
- Added bash completion support via `shell-complete` utility program
- Updated minimum required Rust version to `1.40.0`
- Converted `Cargo.lock` to new lock file format
- Bumped `libc` dependency to `0.2.69`
- Bumped `structopt` dependency to `0.3.13`
- Bumped various transitive dependencies to most recent versions


0.3.2
-----
- Added the `list` command that lists all attached Nitrokey devices
- Reworked argument handling:
  - Added `structopt` dependency in version `0.3.7`
  - Replaced `argparse` with `structopt`
  - Removed `argparse` dependency
  - Made the `--verbose` and `--model` options global
- Removed vendored dependencies and moved source code into repository
  root
- Bumped `nitrokey` dependency to `0.6.0`
- Bumped `quote` dependency to `1.0.3`
- Bumped `syn` dependency to `1.0.14`


0.3.1
-----
- Added note about interaction with GnuPG to `README` file
- Bumped `nitrokey` dependency to `0.4.0`
  - Bumped `nitrokey-sys` dependency to `3.5.0`
  - Added `lazy_static` dependency in version `1.4.0`
  - Added `cfg-if` dependency in version `0.1.10`
  - Added `getrandom` dependency in version `0.1.13`


0.3.0
-----
- Added `unencrypted` command with `set` subcommand for changing the
  unencrypted volume's read-write mode
- Changed `storage hidden` subcommand to `hidden` top-level command
- Renamed `storage` command to `encrypted`
- Removed `storage status` subcommand
  - Moved its output into `status` command
- Removed previously deprecated `--ascii` option from `otp set` command
- Fixed wrong hexadecimal conversion used in `otp set` command
- Bumped `nitrokey` dependency to `0.3.5`
- Bumped `libc` dependency to `0.2.66`
- Bumped `cc` dependency to `1.0.48`


0.2.4
-----
- Added the `reset` command to perform a factory reset
- Added the `-V`/`--version` option to print the program's version
- Check the status of a PWS slot before accessing it in `pws get`
- Added `NITROCLI_NO_CACHE` environment variable to bypass caching of
  secrets
- Clear cached PIN entry as part of `pin set` command to prevent
  spurious authentication failures
- Bumped `libc` dependency to `0.2.57`
- Bumped `cc` dependency to `1.0.37`


0.2.3
-----
- Added the `storage hidden` subcommand for working with hidden volumes
- Store cached PINs on a per-device basis to better support multi-device
  scenarios
- Further decreased binary size by using system allocator
- Bumped `nitrokey` dependency to `0.3.4`
  - Bumped `rand` dependency to `0.6.4`
  - Removed `rustc_version`, `semver`, and `semver-parser` dependencies
- Bumped `nitrokey-sys` dependency to `3.4.3`
- Bumped `libc` dependency to `0.2.47`


0.2.2
-----
- Added the `-v`/`--verbose` option to control libnitrokey log level
- Added the `-m`/`--model` option to restrict connections to a device
  model
- Added the `-f`/`--format` option for the `otp set` subcommand to
  choose the secret format
  - Deprecated the `--ascii` option
- Honor `NITROCLI_ADMIN_PIN` and `NITROCLI_USER_PIN` as well as
  `NITROCLI_NEW_ADMIN_PIN` and `NITROCLI_NEW_USER_PIN` environment
  variables for non-interactive PIN supply
- Format `nitrokey` reported errors in more user-friendly format
- Bumped `nitrokey` dependency to `0.3.1`


0.2.1
-----
- Added the `pws` command for accessing the password safe
- Added the `lock` command for locking the Nitrokey device
- Adjusted release build compile options to optimize binary for size
- Bumped `nitrokey` dependency to `0.2.3`
  - Bumped `rand` dependency to `0.6.1`
  - Added `rustc_version` version `0.2.3`, `semver` version `0.9.0`, and
    `semver-parser` version `0.7.0` as indirect dependencies
- Bumped `cc` dependency to `1.0.28`


0.2.0
-----
- Use the `nitrokey` crate for the `open`, `close`, and `status`
  commands instead of directly communicating with the Nitrokey device
  - Added `nitrokey` version `0.2.1` as a direct dependency and
    `nitrokey-sys` version `3.4.1` as well as `rand` version `0.4.3` as
    indirect dependencies
  - Removed the `hid`, `hidapi-sys` and `pkg-config` dependencies
- Added the `otp` command for working with one-time passwords
- Added the `config` command for reading and writing the device configuration
- Added the `pin` command for managing PINs
  - Renamed the `clear` command to `pin clear`
- Moved `open` and `close` commands as subcommands into newly introduced
  `storage` command
  - Moved printing of storage related information from `status` command
    into new `storage status` subcommand
- Made `status` command work with Nitrokey Pro devices
- Enabled CI pipeline comprising code style conformance checks, linting,
  and building of the project
- Added badges indicating pipeline status, current `crates.io` published
  version of the crate, and minimum version of `rustc` required
- Fixed wrong messages in the pinentry dialog that were caused by unescaped
  spaces in a string
- Use the `argparse` crate to parse the command-line arguments
  - Added `argparse` dependency in version `0.2.2`


0.1.3
-----
- Show PIN related errors through `pinentry` native reporting mechanism
  instead of emitting them to `stdout`
- Added a `man` page (`nitrocli(1)`) for the program to the repository
- Adjusted program to use Rust Edition 2018
- Enabled more lints
- Applied a couple of `clippy` reported suggestions
- Added categories to `Cargo.toml`
- Changed dependency version requirements to be less strict (only up to
  the minor version and not the patch level)
- Bumped `pkg-config` dependency to `0.3.14`
- Bumped `libc` dependency to `0.2.45`
- Bumped `cc` dependency to `1.0.25`


0.1.2
-----
- Replaced deprecated `gcc` dependency with `cc` and bumped to `1.0.4`
- Bumped `hid` dependency to `0.4.1`
- Bumped `hidapi-sys` dependency to `0.1.4`
- Bumped `libc` dependency to `0.2.36`


0.1.1
-----
- Fixed display of firmware version for `status` command
- Removed workaround for incorrect CRC checksum produced by the Nitrokey
  Storage device
  - The problem has been fixed upstream (`nitrokey-storage-firmware`
    [issue #32](https://github.com/Nitrokey/nitrokey-storage-firmware/issues/32))
  - In order to be usable, a minimum firmware version of 0.47 is required


0.1.0
-----
- Initial release
