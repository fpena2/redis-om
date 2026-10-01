//! Derive proc macros for redis-om crate
#![deny(unstable_features)]

mod ast;
mod derive;
#[path = "ext/type_ext.rs"]
mod type_ext;

use ast::Container;
use darling::error::Accumulator;
use derive::*;
use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(HashModel, attributes(redis))]
pub fn hash_model(attr: TokenStream) -> TokenStream {
    expand(attr, hash_model::derive)
}

#[cfg(feature = "json")]
#[proc_macro_derive(JsonModel, attributes(redis))]
pub fn json_model(attr: TokenStream) -> TokenStream {
    expand(attr, json_model::derive)
}

#[proc_macro_derive(RedisModel, attributes(redis))]
pub fn redis_model(attr: TokenStream) -> TokenStream {
    expand(attr, redis_model::derive)
}

#[proc_macro_derive(StreamModel, attributes(redis))]
pub fn stream_model(attr: TokenStream) -> TokenStream {
    expand(attr, stream_model::derive)
}

#[proc_macro_derive(RedisTransportValue, attributes(redis))]
pub fn redis_transport_value(attr: TokenStream) -> TokenStream {
    expand(attr, value::derive)
}

// --------------------------------------------------------------------------------------
// --------------------------------------------------------------------------------------

fn expand(
    attr: TokenStream,
    derive: for<'a> fn(&mut Accumulator, &Container<'a>) -> Result<proc_macro2::TokenStream, ()>,
) -> TokenStream {
    let input = parse_macro_input!(attr as DeriveInput);
    let mut errors = Accumulator::default();
    let stream = match Container::new(&mut errors, &input) {
        Some(cont) => derive(&mut errors, &cont),
        None => Err(()),
    };
    let output = match errors.finish() {
        Err(error) => error.write_errors(),
        Ok(()) => stream.unwrap_or_default(),
    };
    output.into()
}
