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

Gentoo
------

General note: It is recommended (and assumed here) that the most recent existing
ebuild is used as the baseline.

The instructions furthermore assume that you have [`cargo-ebuild`][] installed
and ready for use.

1. Clone the [nitrocli repository][] locally.
2. Check out the tagged release you want to create an ebuild for. E.g., `git
   checkout v0.2.1`.
3. Run `cargo ebuild` inside the `nitrocli/` directory. It will produce an
   ebuild in the same directory.
4. Fork and then clone the official Gentoo Portage tree available at
   https://github.com/gentoo/gentoo.git.
5. Change the working directory to `app-crypt/nitrocli`.
6. Create a copy of the most recently released build. For example,
   `cp nitrocli-0.2.0.ebuild nitrocli-0.2.1.ebuild`
7. Copy over the `CRATES` variable definition from the `cargo-ebuild` created
   ebuild (step 6) into `nitrocli-0.2.1.ebuild` and adjust it as needed (there
   may be duplicate entries).
8. More adjustments to the ebuild may be necessary, depending on the changes
   made to the program.
9. Run `ebuild nitrocli-0.2.1.ebuild manifest` to update the manifest.
10. `FEATURES=test USE=test emerge =nitrocli-0.2.1`
11. Create a commit that includes the ebuild and the *non-ebuild-parts* of
   `Manifest`. Make sure to include a `Signed-off-by` field in the description.
11. Create a pull request for this commit.

[Arch User Repository]: https://wiki.archlinux.org/index.php/Arch_User_Repository
[cargo-ebuild]: https://github.com/cardoe/cargo-ebuild
[nitrocli repository]: https://github.com/d-e-s-o/nitrocli.git
[Teams/RustPackaging]: https://wiki.debian.org/Teams/RustPackaging
[README.rst file]: https://salsa.debian.org/rust-team/debcargo-conf/blob/master/README.rst
[Debian Package Tracker]: https://tracker.debian.org/pkg/rust-nitrocli
[Launchpad]: https://launchpad.net/ubuntu/+source/rust-nitrocli
