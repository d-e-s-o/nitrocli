#[macro_use]
extern crate proc_macro_error;
#[macro_use]
extern crate syn;
extern crate proc_macro;

use proc_macro2::Span;
use proc_macro_error::{set_dummy, Level, OptionExt, ResultExt};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Ident,
};

struct IdentOrUnderscore {
    span: Span,
    part: String,
}

impl IdentOrUnderscore {
    fn new(span: Span, part: String) -> Self {
        IdentOrUnderscore { span, part }
    }
}

impl Parse for IdentOrUnderscore {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let la = input.lookahead1();

        if la.peek(Ident) {
            let t = input.parse::<Ident>().unwrap();
            Ok(IdentOrUnderscore::new(t.span(), t.to_string()))
        } else if la.peek(Token![_]) {
            let t = input.parse::<Token![_]>().unwrap();
            Ok(IdentOrUnderscore::new(t.span(), "_".to_string()))
        } else {
            Err(la.error())
        }
    }
}

struct Args(Vec<IdentOrUnderscore>);

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args = Punctuated::<_, Token![,]>::parse_terminated(input)?;
        Ok(Args(args.into_iter().collect()))
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn make_fn(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut name = String::new();
    let input = parse_macro_input!(input as Args);

    for arg in input.0 {
        match &*arg.part {
            "abort" => abort!(
                arg.span,
                "abort! 3{} args {}", "+", "test";
                hint = "help {} test", "message"
            ),

            "abort_call_site" => abort_call_site!(
                "abort_call_site! 2{} args {}", "+", "test";
                help = "help {} test", "message"
            ),

            "direct_abort" => {
                diagnostic!(arg.span, Level::Error, "direct MacroError::abort() test").abort()
            }

            "result_expect" => {
                let e = syn::Error::new(arg.span, "error");
                Err(e).expect_or_abort("Result::expect_or_abort() test")
            }

            "result_unwrap" => {
                let e = syn::Error::new(arg.span, "Result::unwrap_or_abort() test");
                Err(e).unwrap_or_abort()
            }

            "option_expect" => None.expect_or_abort("Option::expect_or_abort() test"),

            "need_default" => {
                set_dummy(quote! {
                    impl Default for NeedDefault {
                        fn default() -> Self {
                            NeedDefault::A
                        }
                    }
                });

                abort!(arg.span, "set_dummy test")
            }

            part if part.starts_with("multi") => {
                let no_help: Option<String> = Option::None;
                let help = Some("Option help test");
                emit_error!(
                    arg.span,
                    "multiple error part: {}", part;
                    note = "help {} test", "message";
                    hint =? help;
                    wow = "I see what you did here...";
                    help =? no_help
                )
            }

            _ => name.push_str(&arg.part),
        }
    }

    // test that unrelated panics are not affected
    if name.is_empty() {
        panic!("unrelated panic test")
    }

    let name = Ident::new(&name, Span::call_site());
    quote!( fn #name() {} ).into()
}
