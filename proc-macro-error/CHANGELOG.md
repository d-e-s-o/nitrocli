# v0.4.4 (2019-11-13)
* Fix `abort_if_dirty` + warnings bug
* Allow trailing commas in macros

# v0.4.2 (2019-11-7)
* FINALLY fixed `__pme__suggestions not found` bug

# v0.4.1 (2019-11-7) YANKED
* Fixed `__pme__suggestions not found` bug
* Documentation improvements, links checked

# v0.4.0 (2019-11-6) YANKED

## New features
* "help" messages that can have their own span on nightly, they
    inherit parent span on stable.
    ```rust
    let cond_help = if condition { Some("some help message") else { None } };
    abort!(
        span, // parent span
        "something's wrong, {} wrongs in total", 10; // main message
        help = "here's a help for you, {}", "take it"; // unconditional help message
        help =? cond_help; // conditional help message, must be Option
        note = note_span => "don't forget the note, {}", "would you?" // notes can have their own span but it's effective only on nightly
    )
    ```
* Warnings via `emit_warning` and `emit_warning_call_site`. Nightly only, they're ignored on stable.
* Now `proc-macro-error` delegates to `proc_macro::Diagnostic` on nightly.

## Breaking changes
* `MacroError` is now replaced by `Diagnostic`. Its API resembles `proc_macro::Diagnostic`.
* `Diagnostic` does not implement `From<&str/String>` so `Result<T, &str/String>::abort_or_exit()`
    won't work anymore (nobody used it anyway).
* `macro_error!` macro is replaced with `diagnostic!`.

## Improvements
* Now `proc-macro-error` renders notes exactly just like rustc does.
* We don't parse a body of a function annotated with `#[proc_macro_error]` anymore,
  only looking at the signature. This should somewhat decrease expansion time for large functions.

# v0.3.3 (2019-10-16)
* Now you can use any word instead of "help", undocumented.

# v0.3.2 (2019-10-16)
* Introduced support for "help" messages, undocumented.

# v0.3.0 (2019-10-8)

## The crate has been completely rewritten from scratch!

## Changes (most are breaking):
* Renamed macros:
  * `span_error` => `abort`
  * `call_site_error` => `abort_call_site`
* `filter_macro_errors` was replaced by `#[proc_macro_error]` attribute.
* `set_dummy` now takes `TokenStream` instead of `Option<TokenStream>`
* Support for multiple errors via `emit_error` and `emit_call_site_error`
* New `macro_error` macro for building errors in format=like style.
* `MacroError` API had been reconsidered. It also now implements `quote::ToTokens`.

# v0.2.6 (2019-09-02)
* Introduce support for dummy implementations via `dummy::set_dummy`
* `multi::*` is now deprecated, will be completely rewritten in v0.3

# v0.2.0 (2019-08-15)

## Breaking changes
* `trigger_error` replaced with `MacroError::trigger` and `filter_macro_error_panics`
  is hidden from docs.
  This is not quite a breaking change since users weren't supposed to use these functions directly anyway.
* All dependencies are updated to `v1.*`.

## New features
* Ability to stack multiple errors via `multi::MultiMacroErrors` and emit them at once.

## Improvements
* Now `MacroError` implements `std::fmt::Display` instead of `std::string::ToString`.
* `MacroError::span` inherent method.
* `From<MacroError> for proc_macro/proc_macro2::TokenStream` implementations.
* `AsRef/AsMut<String> for MacroError` implementations.

# v0.1.x (2019-07-XX)

## New features
* An easy way to report errors inside within a proc-macro via `span_error`,
  `call_site_error` and `filter_macro_errors`.
