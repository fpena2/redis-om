mod r#struct;

use crate::ast::{AccumulatorExt, Container};
use darling::ast::Data;
use darling::error::Accumulator;
use proc_macro2::TokenStream;

pub fn derive(errors: &mut Accumulator, cont: &Container) -> Result<TokenStream, ()> {
    match &cont.data {
        Data::Enum(_) => {
            errors.push_spanned_error(cont.ident, "RedisModel is only supported for structs");
            Err(())
        }
        Data::Struct(fields) => r#struct::derive(errors, cont, &fields.style, &fields.fields),
    }
}
