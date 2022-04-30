How to package nitrocli
=======================

This document describes how to update the packaged versions of nitrocli.

Arch Linux
----------

The Arch Linux package is maintained as part of the community repository.

Debian
------

1. Clone or fork the Git repository at
   https://salsa.debian.org/rust-team/debcargo-conf.
2. Execute `./update.sh nitrocli`.
3. Check and, if necessary, update the Debian changelog in the file
   `src/nitrocli/debian/changelog`.
4. Verify that the package builds successfully by running `./build.sh nitrocli`
   in the `build` directory.  (This requires an `sbuild` environment as
   described in the `README.rst` file.)
5. Inspect the generated package by running `dpkg-deb --info` and `dpkg-deb
   --contents` on it.
6. If you have push access to the repository, create the
   `src/nitrocli/debian/RFS` file to indicate that `nitrocli` can be updated.
7. Add and commit your changes.  If you have push access, push them.
   Otherwise create a merge request and indicate that `nitrocli` is ready for
   upload in its description.

For more information, see the [Teams/RustPackaging][] page in the Debian Wiki
and the [README.rst file][] in the debcargo-conf repository.

For detailed information on the status of the Debian package, check the [Debian
Package Tracker][].

Ubuntu
------

The `nitrocli` package for Ubuntu is automatically generated from the Debian
package.  For detailed information on the status of the Ubuntu package, check
[Launchpad][].

[Arch User Repository]: https://wiki.archlinux.org/index.php/Arch_User_Repository
[Teams/RustPackaging]: https://wiki.debian.org/Teams/RustPackaging
[README.rst file]: https://salsa.debian.org/rust-team/debcargo-conf/blob/master/README.rst
[Debian Package Tracker]: https://tracker.debian.org/pkg/rust-nitrocli
[Launchpad]: https://launchpad.net/ubuntu/+source/rust-nitrocli
