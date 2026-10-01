use super::TokenStream;
use crate::ast::{AccumulatorExt, Container, Variant};
use darling::ast::Style;
use darling::error::Accumulator;
use quote::quote;

pub(super) fn derive(
    errors: &mut Accumulator,
    cont: &Container,
    variants: &[Variant],
) -> Result<TokenStream, ()> {
    let mut has_invalid_variant = false;
    for variant in variants.iter() {
        if !matches!(variant.fields.style, Style::Unit) {
            errors.push_spanned_error(
                variant.inner,
                "Only Enum's Unit variant is currently supported",
            );
            has_invalid_variant = true;
        }
    }
    if has_invalid_variant {
        return Err(());
    }

    let mut has_skip_serializing = false;
    for variant in variants.iter().filter(|v| v.attrs.skip_serializing) {
        errors.push_spanned_error(variant.inner, "cannot skip serializing an enum variant");
        has_skip_serializing = true;
    }
    if has_skip_serializing {
        return Err(());
    }

    let to_redis_args = derive_to_redis_args(cont, variants);
    let from_redis_args = derive_from_redis_args(cont, variants);

    Ok(quote![
       #to_redis_args
       #from_redis_args
    ])
}

fn derive_to_redis_args(cont: &Container, variants: &[Variant]) -> TokenStream {
    let type_name = cont.ident;
    let matches = variants
        .iter()
        .filter(|v| !v.attrs.skip_serializing)
        .map(|v| {
            let name = &v.ident;
            let value = &v.attrs.name.serialize;
            quote!(#type_name::#name => out.write_arg(#value.as_bytes()),)
        });

    quote! {
        impl ::redis_om::redis::ToRedisArgs for #type_name {
            fn write_redis_args<W: ?Sized + ::redis_om::redis::RedisWrite>(&self, out: &mut W) {
                match self { #(#matches)* }
            }
        }
        impl ::redis_om::redis::ToSingleRedisArg for #type_name {}
    }
}

fn derive_from_redis_args(cont: &Container, variants: &[Variant]) -> TokenStream {
    let type_name = cont.ident;

    let (values, matches): (Vec<_>, Vec<_>) = variants
        .iter()
        .filter(|v| !v.attrs.skip_deserializing)
        .map(|v| {
            let name = &v.ident;
            let values = v.attrs.name.deserialize_aliases();
            let matches: Vec<_> = values
                .iter()
                .map(|value| quote! { #value => Ok(#type_name::#name), })
                .collect();
            (values, matches)
        })
        .unzip();

    let values = values.into_iter().flatten().collect::<Vec<_>>().join(", ");
    let matches = matches.into_iter().flatten();

    quote! {
        impl ::redis_om::redis::FromRedisValue for #type_name {
            fn from_redis_value(v: ::redis_om::redis::Value) -> Result<Self, ::redis_om::redis::ParsingError> {
                use ::redis_om::redis::Value;

                let msg = format!("{:?}", v);
                let Value::BulkString(data) = v else {
                    return Err(format!("Expected Redis string, got: {}", msg).into());
                };
                let value = std::str::from_utf8(&data[..])?;

                match value {
                    #(#matches)*
                    v => Err(format!("Invalid enum variant: {}, Expected one of: {}", v, #values).into()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Container;
    use darling::error::Accumulator;

    #[test]
    fn reports_every_non_unit_variant() {
        let input: syn::DeriveInput = syn::parse_quote! {
            enum Record {
                First(String),
                Second(String),
            }
        };
        let mut errors = Accumulator::default();
        let cont = Container::new(&mut errors, &input).expect("container should parse");

        assert!(crate::value::derive(&mut errors, &cont).is_err());
        let rendered = errors.finish().unwrap_err().write_errors().to_string();
        assert_eq!(rendered.matches("Only Enum's Unit variant").count(), 2);
    }
}
