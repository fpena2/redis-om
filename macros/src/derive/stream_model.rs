use crate::ast::Container;
use darling::error::Accumulator;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};

pub fn derive(errors: &mut Accumulator, cont: &Container) -> Result<TokenStream, ()> {
    let type_name = cont.ident;
    let prefix_key = cont.prefix_key.as_str();

    let mut stream = TokenStream::new();
    let consumer_type = format_ident!("{}Manager", type_name);

    crate::value::derive(errors, cont)?.to_tokens(&mut stream);

    Ok(quote! {
        #stream

        #[derive(Clone)]
        pub struct #consumer_type {
            group_name: String,
            consumer_name: String,
        }

        impl #consumer_type {
            #[doc = "Create new StreamModelManager, with consumer_name being auto generated"]
            pub fn new(group_name: impl AsRef<str>) -> Self {
                Self::new_with_consumer_name(group_name, ::rusty_ulid::generate_ulid_string())
            }

            #[doc = "Create new StreamModelManager, with custom consumer_name"]
            pub fn new_with_consumer_name(group_name: impl AsRef<str>, consumer_name: impl AsRef<str>) -> Self {
                Self {
                    group_name: group_name.as_ref().to_owned(),
                    consumer_name: consumer_name.as_ref().to_owned(),
                }
            }
        }

        impl ::redis_om::StreamModel for #consumer_type {
            type Data = #type_name;

            fn group_name(&self) -> &str {
                self.group_name.as_str()
            }

            fn consumer_name(&self) -> &str {
                self.consumer_name.as_str()
            }

            fn stream_key() -> &'static str {
                #prefix_key
            }
        }

    })
}
