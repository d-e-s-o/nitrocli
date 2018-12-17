Unreleased
----------
- Use the `nitrokey` crate for the `open`, `close`, and `status`
  commands instead of directly communicating with the Nitrokey device
  - Added `nitrokey` version `0.2.1` as a direct dependency and
    `nitrokey-sys` version `3.4.1` as well as `rand` version `0.4.3` as
    indirect dependencies
  - Removed the `hid`, `hidapi-sys` and `pkg-config` dependencies
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
