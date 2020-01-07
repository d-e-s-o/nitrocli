use syn::{Attribute, Lifetime, Token};

use super::PatType;

ast_enum_of_structs! {
    /// An argument in a function signature: the `n: usize` in `fn f(n: usize)`.
    pub enum FnArg {
        /// The `self` argument of an associated method, whether taken by value
        /// or by reference.
        Receiver(Receiver),

        /// A function argument accepted by pattern and type.
        Typed(PatType),
    }
}

ast_struct! {
    /// The `self` argument of an associated method, whether taken by value
    /// or by reference.
    pub struct Receiver {
        pub attrs: Vec<Attribute>,
        pub reference: Option<(Token![&], Option<Lifetime>)>,
        pub mutability: Option<Token![mut]>,
        pub self_token: Token![self],
    }
}

mod parsing {
    use syn::{
        parse::{discouraged::Speculative, Parse, ParseStream, Result},
        Attribute, Token,
    };

    use super::{FnArg, PatType, Receiver};

    impl Parse for FnArg {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            let attrs = input.call(Attribute::parse_outer)?;

            let ahead = input.fork();
            if let Ok(mut receiver) = ahead.parse::<Receiver>() {
                if !ahead.peek(Token![:]) {
                    input.advance_to(&ahead);
                    receiver.attrs = attrs;
                    return Ok(FnArg::Receiver(receiver));
                }
            }

            let mut typed = input.call(fn_arg_typed)?;
            typed.attrs = attrs;
            Ok(FnArg::Typed(typed))
        }
    }

    impl Parse for Receiver {
        fn parse(input: ParseStream<'_>) -> Result<Self> {
            Ok(Self {
                attrs: Vec::new(),
                reference: {
                    if input.peek(Token![&]) {
                        Some((input.parse()?, input.parse()?))
                    } else {
                        None
                    }
                },
                mutability: input.parse()?,
                self_token: input.parse()?,
            })
        }
    }

    fn fn_arg_typed(input: ParseStream<'_>) -> Result<PatType> {
        Ok(PatType {
            attrs: Vec::new(),
            pat: input.parse()?,
            colon_token: input.parse()?,
            ty: Box::new(input.parse()?),
        })
    }
}

mod printing {
    use proc_macro2::TokenStream;
    use quote::{ToTokens, TokenStreamExt};

    use super::Receiver;

    impl ToTokens for Receiver {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            tokens.append_all(&self.attrs);
            if let Some((ampersand, lifetime)) = &self.reference {
                ampersand.to_tokens(tokens);
                lifetime.to_tokens(tokens);
            }
            self.mutability.to_tokens(tokens);
            self.self_token.to_tokens(tokens);
        }
    }
}
