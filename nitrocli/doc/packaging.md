How to package nitrocli
=======================

This document describes how to update the packaged versions of nitrocli.

Arch Linux
----------

1. Clone the Git repository at ssh://aur@aur.archlinux.org/nitrocli.git.
2. Edit the `PKGBUILD` file:
   - Update the `pkgver` variable to the current nitrocli version.
   - If the `pkgrel` variable is not 1, set it to 1.
   - Update the SHA512 hash in the `sha512sums` variable for the new tarball.
3. Update the `.SRCINFO` file by running `makepkg --printsrcinfo > .SRCINFO`.
4. Verify that the package builds sucessfully by running `makepkg`.
5. Verify that the package was built as expected by running `pacman -Qlp $f`
   and `pacman -Qip $f`, where `$f` is `nitrocli-$pkgver.pkg.tar.gz`.
6. Check the package for errors by running `namcap PKGBUILD` and `namcap
   nitrocli-$pkgver.pkg.tar.gz`.
7. Add, commit and push your changes to publish them in the AUR.

If you have to release a new package version without a new nitrocli version,
do not change the `pkgver` variable and instead increment the `pkgrel`
variable.

For more information, see the [Arch User Repository][] page in the Arch Wiki.

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
5. Inspect the generated package by running `dpkg-deb --info` and `dpkg -I` on
   it.
6. If you have push access to the repository, update the `TODO.rst` file to
   indicate that `nitrocli` can be updated.
7. Add and commit your changes.  If you have push access, push them.
   Otherwise create a merge request and indicate that `nitrocli` is ready for
   upload in its description.

For more information, see the [Teams/RustPackaging][] page in the Debian Wiki
and the [README.rst file][] in the debcargo-conf repository.

[Arch User Repository]: https://wiki.archlinux.org/index.php/Arch_User_Repository
[Teams/RustPackaging]: https://wiki.debian.org/Teams/RustPackaging
[README.rst file]: https://salsa.debian.org/rust-team/debcargo-conf/blob/master/README.rst
