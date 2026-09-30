use crate::ast::{style_name, AccumulatorExt, Container, FieldAttr};
use crate::type_ext::TypeExt;
use darling::ast::{Data, Style};
use darling::error::Accumulator;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Type;

pub fn derive(errors: &mut Accumulator, cont: &Container) -> Result<TokenStream, ()> {
    let type_name = cont.ident;
    // TODO: Find a way to ignore types already implements default trait.

    let mut stream = TokenStream::new();
    #[cfg(feature = "aio")]
    let attributes: Vec<syn::Attribute> = vec![syn::parse_quote!(#[::redis_om::async_trait])];
    #[cfg(not(feature = "aio"))]
    let attributes: Vec<syn::Attribute> = Vec::new();

    crate::value::derive(errors, cont)?.to_tokens(&mut stream);
    crate::redis_model::derive(errors, cont)?.to_tokens(&mut stream);
    redis_schema::derive(errors, cont)?.to_tokens(&mut stream);

    Ok(quote! {
        #stream
        #(#attributes)*
        impl ::redis_om::HashModel for #type_name { }

    })
}

mod redis_schema {
    use super::*;

    pub fn derive(errors: &mut Accumulator, cont: &Container) -> Result<TokenStream, ()> {
        let type_name = cont.ident;
        let prefix_key = cont.prefix_key.as_str();

        let Data::Struct(fields) = &cont.data else {
            let msg = &"Enum is not currenlty supported for redissearch_model";
            errors.push_spanned_error(cont.ident, msg);
            return Err(());
        };

        let Style::Struct = fields.style else {
            let msg = format!(
                "{} Struct is not supported",
                style_name(fields.style, fields.fields.len())
            );
            errors.push_spanned_error(cont.original, msg);
            return Err(());
        };

        let redis_search_schema = format!(
            "ON HASH PREFIX 1 {prefix_key} SCHEMA {}",
            fields
                .iter()
                .map(|field| {
                    let key = &field.attrs.name.serialize;
                    let attrs = &field.attrs;
                    let ty = &field.ty;
                    let mut schema_parts = Vec::new();

                    if attrs.primary_key {
                        schema_parts.push(format!("{key} TAG SEPARATOR |"));
                    } else if attrs.index {
                        schema_parts.push(schema_for_type(attrs, ty));
                    } else if ty.is_list_collection() {
                        let ty = ty.get_inner_type().expect("inner type of list-like type");
                        schema_parts.push(schema_for_type(attrs, ty));
                    }

                    schema_parts.join(" ")
                })
                .collect::<Vec<_>>()
                .join(" ")
        );

        Ok(quote! {
            impl ::redis_om::RedisSearchModel for #type_name {
                const _REDIS_SEARCH_SCHEMA: &'static str = #redis_search_schema;
            }
        })
    }

    fn schema_for_type(attrs: &FieldAttr, ty: &Type) -> String {
        let mut schema: Vec<String> = vec![];
        let name = &attrs.name.serialize;
        if ty.is_list_collection() {
            let ty = ty.get_inner_type().unwrap();
            schema.push(schema_for_type(attrs, ty));
        } else if ty.is_numeric_type() {
            schema.push(format!("{name} NUMERIC"));
        } else if ty.is_ident("String") {
            if attrs.fts {
                schema.push(format!("{name} TAG SEPARATOR | {name} AS {name}_fts TEXT"));
            } else {
                schema.push(format!("{name} TAG SEPARATOR |"));
            }
        } else {
            schema.push(format!("{name} TAG SEPARATOR |"))
        }

        if attrs.sortable {
            schema.push("SORTABLE".into())
        }

        schema.join(" ")
    }
}
