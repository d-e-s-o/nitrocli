# syn-mid

[![Build Status][azure-badge]][azure-url]
[![Crates.io][crates-version-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]
[![License][crates-license-badge]][crates-url]
[![Minimum supported Rust version][rustc-badge]][rustc-url]

[azure-badge]: https://dev.azure.com/taiki-e/taiki-e/_apis/build/status/taiki-e.syn-mid?branchName=master
[azure-url]: https://dev.azure.com/taiki-e/taiki-e/_build/latest?definitionId=11&branchName=master
[crates-version-badge]: https://img.shields.io/crates/v/syn-mid.svg
[crates-license-badge]: https://img.shields.io/crates/l/syn-mid.svg
[crates-badge]: https://img.shields.io/crates/v/syn-mid.svg
[crates-url]: https://crates.io/crates/syn-mid/
[docs-badge]: https://docs.rs/syn-mid/badge.svg
[docs-url]: https://docs.rs/syn-mid/
[rustc-badge]: https://img.shields.io/badge/rustc-1.31+-lightgray.svg
[rustc-url]: https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html

Providing the features between "full" and "derive" of syn.

This crate provides the following two unique data structures.

* `syn_mid::ItemFn` -- A function whose body is not parsed.

  ```text
  fn process(n: usize) -> Result<()> { ... }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ ^     ^
  ```

* `syn_mid::Block` -- A block whose body is not parsed.

  ```text
  { ... }
  ^     ^
  ```

Other data structures are the same as data structures of [syn]. These are defined in this crate because they cannot be used in [syn] without "full" feature.

[syn]: https://github.com/dtolnay/syn

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
syn-mid = "0.4"
```

The current syn-mid requires Rust 1.31 or later.

[**Examples**](examples)

[**Documentation**](https://docs.rs/syn-mid/)

## Optional features

* **`clone-impls`** — Clone impls for all syntax tree types.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
