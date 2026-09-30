mod r#enum;
mod r#struct;

use crate::ast::Container;
use darling::ast::Data;
use darling::error::Accumulator;
use proc_macro2::TokenStream;

pub fn derive(errors: &mut Accumulator, cont: &Container) -> Result<TokenStream, ()> {
    match &cont.data {
        Data::Enum(variants) => r#enum::derive(errors, cont, variants),
        Data::Struct(fields) => r#struct::derive(errors, cont, fields),
    }
}
