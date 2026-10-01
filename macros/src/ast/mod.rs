mod container;
mod field;
mod name;
mod variant;

pub(crate) use container::*;
pub(crate) use field::*;
pub(crate) use name::*;
pub(crate) use variant::*;

use darling::Error;
use darling::error::Accumulator;
use quote::ToTokens;
use std::fmt::Display;

pub(crate) trait AccumulatorExt {
    fn push_spanned_error<A: ToTokens, T: Display>(&mut self, obj: A, msg: T);
}

impl AccumulatorExt for Accumulator {
    fn push_spanned_error<A: ToTokens, T: Display>(&mut self, obj: A, msg: T) {
        self.push(Error::from(syn::Error::new_spanned(
            obj.into_token_stream(),
            msg,
        )));
    }
}

pub(crate) fn style_name(style: darling::ast::Style, field_count: usize) -> &'static str {
    match style {
        darling::ast::Style::Struct => "Struct",
        darling::ast::Style::Tuple if field_count == 1 => "Newtype",
        darling::ast::Style::Tuple => "Tuple",
        darling::ast::Style::Unit => "Unit",
    }
}
