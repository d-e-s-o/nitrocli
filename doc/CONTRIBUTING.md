The following rules generally apply for pull requests and code changes:

**Submit Pull Requests to the `devel` branch**

The `devel` branch is where experimental features reside. After some
soak time they may be ported over to `main` and a release will be cut
that includes them.

**Keep documentation up-to-date**

Please make an effort to keep the documentation up-to-date to the extent
possible and necessary for the change at hand. That includes adjusting
the [README](../README.md) and [`man` page](nitrocli.1) as well as
regenerating the PDF rendered version of the latter by running `make
doc`.

**Blend with existing patterns and style**

To keep the code as consistent as possible, please try not to diverge
from the existing style used in a file. Specifically for Rust source
code, use [`rustfmt`](https://github.com/rust-lang/rustfmt) and
[`clippy`](https://github.com/rust-lang/rust-clippy) to achieve a
minimum level of consistency and prevent known bugs, respectively.
