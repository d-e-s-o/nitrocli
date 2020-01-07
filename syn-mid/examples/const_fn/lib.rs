#![warn(rust_2018_idioms)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn_mid::ItemFn;

/// An attribute for easy generation of a const function with conditional compilations.
#[proc_macro_attribute]
pub fn const_fn(args: TokenStream, function: TokenStream) -> TokenStream {
    assert!(!args.is_empty(), "requires an argument");

    let mut function = syn::parse_macro_input!(function as ItemFn);
    let mut const_function = function.clone();

    if function.constness.is_some() {
        function.constness = None;
    } else {
        const_function.constness = Some(Default::default());
    }

    let args = TokenStream2::from(args);
    TokenStream::from(quote! {
        #[cfg(not(#args))]
        #function
        #[cfg(#args)]
        #const_function
    })
}
