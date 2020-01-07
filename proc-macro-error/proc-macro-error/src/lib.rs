//! # proc-macro-error
//!
//! This crate aims to make error reporting in proc-macros simple and easy to use.
//! Migrate from `panic!`-based errors for as little effort as possible!
//!
//! Also, there's ability to [append a dummy token stream](dummy/index.html) to your errors.
//!
//! ## Limitations
//!
//! - Warnings are emitted only on nightly, they're ignored on stable.
//! - "help" suggestions cannot have their own span info on stable, (they inherit parent span).
//! - If a panic occurs somewhere in your macro no errors will be displayed. This is not a
//!   technical limitation but intentional design, `panic` is not for error reporting.
//!
//! ## Guide
//!
//! ### Macros
//!
//! First of all - **all the emitting-related API must be used within a function
//! annotated with [`#[proc_macro_error]`](#proc_macro_error-attribute) attribute**. You'll just get a
//! panic otherwise, no errors will be shown.
//!
//! For most of the time you will be using macros.
//!
//! - [`abort!`]:
//!
//!     Very much panic-like usage - abort execution and show the error. Expands to [`!`] (never type).
//!
//! - [`abort_call_site!`]:
//!
//!     Shortcut for `abort!(Span::call_site(), ...)`. Expands to [`!`] (never type).
//!
//! - [`emit_error!`]:
//!
//!     [`proc_macro::Diagnostic`]-like usage - emit the error but do not abort the macro.
//!     The compilation will fail nonetheless. Expands to [`()`] (unit type).
//!
//! - [`emit_call_site_error!`]:
//!
//!     Shortcut for `emit_error!(Span::call_site(), ...)`. Expands to [`()`] (unit type).
//!
//! - [`emit_warning!`]:
//!
//!     Like `emit_error!` but emit a warning instead of error. The compilation won't fail
//!     because of warnings.
//!     Expands to [`()`] (unit type).
//!
//!     **Beware**: warnings are nightly only, they are completely ignored on stable.
//!
//! - [`emit_call_site_warning!`]:
//!
//!     Shortcut for `emit_warning!(Span::call_site(), ...)`. Expands to `()` (unit type).
//!
//! - [`diagnostic`]:
//!
//!     Build instance of `Diagnostic` in format-like style.
//!
//! ### Syntax
//!
//! All the macros have pretty much the same syntax:
//!
//! 1.  ```ignore
//!     abort!(single_expr)
//!     ```
//!     Shortcut for `Diagnostic::from().abort()`
//!
//! 2.  ```ignore
//!     abort!(span, message)
//!     ```
//!     Shortcut for `Diagnostic::spanned(span, message.to_string()).abort()`
//!
//! 3.  ```ignore
//!     abort!(span, format_literal, format_args...)
//!     ```
//!     Shortcut for `Diagnostic::spanned(span, format!(format_literal, format_args...)).abort()`
//!
//! That's it. `abort!`, `emit_warning`, `emit_error` share this exact syntax.
//! `abort_call_site!`, `emit_call_site_warning`, `emit_call_site_error` lack 1 form
//! and do not take span in 2 and 3 forms.
//!
//! `diagnostic!` require `Level` instance between `span` and second argument (1 form is the same).
//!
//! #### Note attachments
//!
//! 3.  Every macro can have "note" attachments (only 2 and 3 form).
//!   ```ignore
//!   let opt_help = if have_some_info { Some("did you mean `this`?") } else { None };
//!
//!   abort!(
//!       span, message; // <--- attachments start with `;` (semicolon)
//!
//!       help = "format {} {}", "arg1", "arg2"; // <--- every attachment ends with `;`,
//!                                              //      maybe except the last one
//!
//!       note = "to_string"; // <--- one arg uses `.to_string()` instead of `format!()`
//!
//!       yay = "I see what {} did here", "you"; // <--- "help =" and "hint =" are mapped to Diagnostic::help
//!                                              //      anything else is Diagnostic::note
//!
//!       wow = note_span => "custom span"; // <--- attachments can have their own span
//!                                         //      it takes effect only on nightly though
//!
//!       hint =? opt_help; // <-- "optional" attachment, get displayed only if `Some`
//!                         //     must be single `Option` expression
//!
//!       note =? note_span => opt_help // <-- optional attachments can have custom spans too
//!   )
//!   ```
//!
//! ### `#[proc_macro_error]` attribute
//!
//! **This attribute MUST be present on the top level of your macro.**
//!
//! This attribute performs the setup and cleanup necessary to make things work.
//!
//! #### Syntax
//!
//! `#[proc_macro_error]` or `#[proc_macro_error(settings...)]`, where `settings...`
//! is a comma-separated list of:
//!
//! - `proc_macro_hack`:
//!
//!     To correctly cooperate with `#[proc_macro_hack]` `#[proc_macro_error]`
//!     attribute must be placed *before* (above) it, like this:
//!
//!     ```ignore
//!     #[proc_macro_error]
//!     #[proc_macro_hack]
//!     #[proc_macro]
//!     fn my_macro(input: TokenStream) -> TokenStream {
//!         unimplemented!()
//!     }
//!     ```
//!
//!     If, for some reason, you can't place it like that you can use
//!     `#[proc_macro_error(proc_macro_hack)]` instead.
//!
//! - `allow_not_macro`:
//!
//!     By default, the attribute checks that it's applied to a proc-macro.
//!     If none of `#[proc_macro]`, `#[proc_macro_derive]` nor `#[proc_macro_attribute]` are
//!     present it will panic. It's the intention - this crate is supposed to be used only with
//!     proc-macros. This setting is made to bypass the check, useful in certain
//!     circumstances.
//!
//!     Please note: the function this attribute is applied to must return `proc_macro::TokenStream`.
//!
//! - `assert_unwind_safe`:
//!
//!     By default, your code must be [unwind safe]. If your code is not unwind safe but you believe
//!     it's correct you can use this setting to bypass the check. This is typically needed
//!     for code that uses `lazy_static` or `thread_local` with `Cell/RefCell` inside.
//!
//!     This setting is implied if `#[proc_macro_error]` is applied to a function
//!     marked as `#[proc_macro]`, `#[proc_macro_derive]` or `#[proc_macro_attribute]`.
//!
//! ### Diagnostic type
//!
//! [`Diagnostic`] type is intentionally designed to be API compatible with [`proc_macro::Diagnostic`].
//! Not all API is implemented, only the part that can be reasonably implemented on stable.
//!
//!
//! [`abort!`]: macro.abort.html
//! [`emit_warning!`]: macro.emit_warning.html
//! [`emit_error!`]: macro.emit_error.html
//! [`abort_call_site!`]: macro.abort_call_site.html
//! [`emit_call_site_warning!`]: macro.emit_call_site_error.html
//! [`emit_call_site_error!`]: macro.emit_call_site_warning.html
//! [`diagnostic!`]: macro.diagnostic.html
//! [proc_macro_error]: ./../proc_macro_error_attr/attr.proc_macro_error.html
//! [`Diagnostic`]: struct.Diagnostic.html
//! [`proc_macro::Diagnostic`]: https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html
//! [unwind safe]: https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html#what-is-unwind-safety
//! [`!`]: https://doc.rust-lang.org/std/primitive.never.html
//! [`()`]: https://doc.rust-lang.org/std/primitive.unit.html

#![cfg_attr(pme_nightly, feature(proc_macro_diagnostic))]
#![forbid(unsafe_code)]

// reexports for use in macros
#[doc(hidden)]
pub extern crate proc_macro;
#[doc(hidden)]
pub extern crate proc_macro2;

pub use self::dummy::set_dummy;
pub use proc_macro_error_attr::proc_macro_error;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use quote::{quote_spanned, ToTokens};
use std::cell::Cell;
use std::panic::{catch_unwind, resume_unwind, UnwindSafe};

pub mod dummy;

mod macros;

#[cfg(not(any(pme_nightly, nightly_fmt)))]
#[path = "stable.rs"]
mod imp;

#[cfg(any(pme_nightly, nightly_fmt))]
#[path = "nightly.rs"]
mod imp;

/// Represents a diagnostic level
///
/// # Warnings
///
/// Warnings are ignored on stable/beta
#[derive(Debug, PartialEq)]
pub enum Level {
    Error,
    Warning,
    #[doc(hidden)]
    NonExhaustive,
}

/// Represents a single diagnostic message
#[derive(Debug)]
pub struct Diagnostic {
    level: Level,
    span: Span,
    msg: String,
    suggestions: Vec<(SuggestionKind, String, Option<Span>)>,
}

/// This traits expands `Result<T, Into<Diagnostic>>` with some handy shortcuts.
pub trait ResultExt {
    type Ok;

    /// Behaves like `Result::unwrap`: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via `abort!`.
    fn unwrap_or_abort(self) -> Self::Ok;

    /// Behaves like `Result::expect`: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via `abort!`.
    /// If it aborts then resulting error message will be preceded with `message`.
    fn expect_or_abort(self, msg: &str) -> Self::Ok;
}

/// This traits expands `Option` with some handy shortcuts.
pub trait OptionExt {
    type Some;

    /// Behaves like `Option::expect`: if self is `Some` yield the contained value,
    /// otherwise abort macro execution via `abort_call_site!`.
    /// If it aborts the `message` will be used for [`compile_error!`][compl_err] invocation.
    ///
    /// [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
    fn expect_or_abort(self, msg: &str) -> Self::Some;
}

impl Diagnostic {
    /// Create a new diagnostic message that points to `Span::call_site()`
    pub fn new(level: Level, message: String) -> Self {
        Diagnostic::spanned(Span::call_site(), level, message)
    }

    /// Create a new diagnostic message that points to the `span`
    pub fn spanned(span: Span, level: Level, message: String) -> Self {
        Diagnostic {
            level,
            span,
            msg: message,
            suggestions: vec![],
        }
    }

    /// Attach a "help" note to your main message, note will have it's own span on nightly.
    ///
    /// # Span
    ///
    /// The span is ignored on stable, the note effectively inherits its parent's (main message) span
    pub fn span_help(mut self, span: Span, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Help, msg, Some(span)));
        self
    }

    /// Attach a "help" note to your main message,
    pub fn help(mut self, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Help, msg, None));
        self
    }

    /// Attach a note to your main message, note will have it's own span on nightly.
    ///
    /// # Span
    ///
    /// The span is ignored on stable, the note effectively inherits its parent's (main message) span
    pub fn span_note(mut self, span: Span, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Note, msg, Some(span)));
        self
    }

    /// Attach a note to your main message
    pub fn note(mut self, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Note, msg, None));
        self
    }

    /// The message of main warning/error (no notes attached)
    pub fn message(&self) -> &str {
        &self.msg
    }

    /// Abort the proc-macro's execution and display the diagnostic.
    ///
    /// # Warnings
    ///
    /// Warnings do not get emitted on stable/beta but this function will abort anyway.
    pub fn abort(self) -> ! {
        self.emit();
        abort_now()
    }

    /// Display the diagnostic while not aborting macro execution.
    ///
    /// # Warnings
    ///
    /// Warnings are ignored on stable/beta
    pub fn emit(self) {
        imp::emit_diagnostic(self);
    }
}

/// Abort macro execution and display all the emitted errors, if any.
///
/// Does nothing if no errors were emitted (warnings do not count).
pub fn abort_if_dirty() {
    imp::abort_if_dirty();
}

#[doc(hidden)]
impl Diagnostic {
    pub fn span_suggestion(self, span: Span, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.span_help(span, msg),
            _ => self.span_note(span, msg),
        }
    }

    pub fn suggestion(self, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.help(msg),
            _ => self.note(msg),
        }
    }
}

impl ToTokens for Diagnostic {
    fn to_tokens(&self, ts: &mut TokenStream) {
        use std::borrow::Cow;

        fn ensure_lf(buf: &mut String, s: &str) {
            if s.ends_with('\n') {
                buf.push_str(s);
            } else {
                buf.push_str(s);
                buf.push('\n');
            }
        }

        let Diagnostic {
            ref msg,
            ref suggestions,
            ref level,
            ..
        } = *self;

        if *level == Level::Warning {
            return;
        }

        let message = if suggestions.is_empty() {
            Cow::Borrowed(msg)
        } else {
            let mut message = String::new();
            ensure_lf(&mut message, msg);
            message.push('\n');

            for (kind, note, _span) in suggestions {
                message.push_str("  = ");
                message.push_str(kind.name());
                message.push_str(": ");
                ensure_lf(&mut message, note);
            }
            message.push('\n');

            Cow::Owned(message)
        };

        let span = &self.span;
        let msg = syn::LitStr::new(&*message, *span);
        ts.extend(quote_spanned!(*span=> compile_error!(#msg); ));
    }
}

impl<T, E: Into<Diagnostic>> ResultExt for Result<T, E> {
    type Ok = T;

    fn unwrap_or_abort(self) -> T {
        match self {
            Ok(res) => res,
            Err(e) => e.into().abort(),
        }
    }

    fn expect_or_abort(self, message: &str) -> T {
        match self {
            Ok(res) => res,
            Err(e) => {
                let mut e = e.into();
                e.msg = format!("{}: {}", message, e.msg);
                e.abort()
            }
        }
    }
}

impl<T> OptionExt for Option<T> {
    type Some = T;

    fn expect_or_abort(self, message: &str) -> T {
        match self {
            Some(res) => res,
            None => abort_call_site!(message),
        }
    }
}

#[derive(Debug)]
enum SuggestionKind {
    Help,
    Note,
}

impl SuggestionKind {
    fn name(&self) -> &'static str {
        match self {
            SuggestionKind::Note => "note",
            SuggestionKind::Help => "help",
        }
    }
}

impl From<syn::Error> for Diagnostic {
    fn from(e: syn::Error) -> Self {
        Diagnostic::spanned(e.span(), Level::Error, e.to_string())
    }
}

/// This is the entry point for a proc-macro.
///
/// **NOT PUBLIC API, SUBJECT TO CHANGE WITHOUT ANY NOTICE**
#[doc(hidden)]
pub fn entry_point<F>(f: F, proc_macro_hack: bool) -> proc_macro::TokenStream
where
    F: FnOnce() -> proc_macro::TokenStream + UnwindSafe,
{
    ENTERED_ENTRY_POINT.with(|flag| flag.set(true));
    let caught = catch_unwind(f);
    let dummy = dummy::cleanup();
    let err_storage = imp::cleanup();
    ENTERED_ENTRY_POINT.with(|flag| flag.set(false));

    let mut appendix = TokenStream::new();
    if proc_macro_hack {
        appendix.extend(quote! {
            #[allow(unused)]
            macro_rules! proc_macro_call {
                () => ( unimplemented!() )
            }
        });
    }

    match caught {
        Ok(ts) => {
            if err_storage.is_empty() {
                ts
            } else {
                quote!( #(#err_storage)* #dummy #appendix ).into()
            }
        }

        Err(boxed) => match boxed.downcast::<AbortNow>() {
            Ok(_) => quote!( #(#err_storage)* #dummy #appendix ).into(),
            Err(boxed) => resume_unwind(boxed),
        },
    }
}

fn abort_now() -> ! {
    check_correctness();
    panic!(AbortNow)
}

thread_local! {
    static ENTERED_ENTRY_POINT: Cell<bool> = Cell::new(false);
}

struct AbortNow;

fn check_correctness() {
    if !ENTERED_ENTRY_POINT.with(|flag| flag.get()) {
        panic!(
            "proc-macro-error API cannot be used outside of `entry_point` invocation, \
             perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
        );
    }
}
