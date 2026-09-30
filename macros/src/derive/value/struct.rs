use crate::ast::{style_name, AccumulatorExt, Container, Field};
use darling::ast::{Fields, Style};
use darling::error::Accumulator;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) fn derive(
    errors: &mut Accumulator,
    cont: &Container,
    fields: &Fields<Field>,
) -> Result<TokenStream, ()> {
    let style = &fields.style;
    let fields = &fields.fields;
    let to_redis_args = derive_to_redis_args(errors, cont, style, fields)?;
    let from_redis_args = derive_from_redis(errors, cont, style, fields)?;

    Ok(quote![
       #to_redis_args
       #from_redis_args
    ])
}

fn derive_to_redis_args(
    errors: &mut Accumulator,
    cont: &Container,
    style: &Style,
    fields: &[Field],
) -> Result<TokenStream, ()> {
    let type_name = cont.ident;
    let elms = match style {
        Style::Struct => fields
            .iter()
            .filter(|f| !f.attrs.skip_serializing)
            .map(|f| {
                let ident = f.ident.as_ref().unwrap();
                let key = &f.attrs.name.serialize;

                quote! {
                    match ToRedisArgs::to_redis_args(&self.#ident) {
                        redis_args if redis_args.len() == 1 => {
                            out.write_arg_fmt(#key);
                            out.write_arg(&redis_args[0]);
                        },
                        redis_args => {
                            for (idx, item) in redis_args.iter().enumerate() {
                                out.write_arg_fmt(format!("{}.{}", #key, idx));
                                out.write_arg(&item)
                            }
                        }
                    }
                }
            }),
        Style::Tuple | Style::Unit => {
            let msg = format!(
                "{} Struct is not supported",
                style_name(*style, fields.len())
            );
            errors.push_spanned_error(cont.original, msg);
            return Err(());
        }
    };

    Ok(quote! {
        impl ::redis_om::redis::ToRedisArgs for #type_name {
            fn write_redis_args<W : ?Sized + ::redis_om::redis::RedisWrite>(&self, out: &mut W) {
                use ::redis_om::redis::*;
                #(#elms)*
            }
        }
    })
}

fn derive_from_redis(
    errors: &mut Accumulator,
    cont: &Container,
    style: &Style,
    fields: &[Field],
) -> Result<TokenStream, ()> {
    let ident = cont.ident;
    match style {
        Style::Struct => {
            let err_msg = "the data is not in the bulk data format or the length is not / 2";
            let err = quote!(
                RedisError::from((ErrorKind::TypeError, #err_msg, format!("{:#?}", v)))
            );
            for field in fields.iter().filter(|f| f.attrs.skip_deserializing) {
                errors.push_spanned_error(
                    field.ident.as_ref().unwrap(),
                    "cannot skip deserializing a required field without a default",
                );
            }
            if fields.iter().any(|f| f.attrs.skip_deserializing) {
                return Err(());
            }

            let mut has_key_collision = false;
            for (multi_arg_index, multi_arg_field) in fields.iter().enumerate() {
                let multi_arg_may_emit_many = may_emit_multiple_args(&multi_arg_field.ty);

                if multi_arg_may_emit_many
                    && multi_arg_field
                        .attrs
                        .name
                        .deserialize_aliases()
                        .into_iter()
                        .any(|key| {
                            is_collection_item_key(&multi_arg_field.attrs.name.serialize, &key)
                        })
                {
                    errors.push_spanned_error(
                        multi_arg_field.ident.as_ref().unwrap(),
                        format!(
                            "deserialize key conflicts with indexed Redis item keys emitted by `{}`",
                            multi_arg_field.attrs.name.serialize
                        ),
                    );
                    has_key_collision = true;
                }

                for field in fields.iter().skip(multi_arg_index + 1) {
                    let field_may_emit_many = may_emit_multiple_args(&field.ty);
                    if multi_arg_may_emit_many
                        && field_key_conflicts_with_multi_arg_field(multi_arg_field, field)
                    {
                        errors.push_spanned_error(
                            field.ident.as_ref().unwrap(),
                            format!(
                                "field key conflicts with Redis keys emitted by multi-argument field `{}`",
                                multi_arg_field.attrs.name.serialize
                            ),
                        );
                        has_key_collision = true;
                    }
                    if field_may_emit_many
                        && field_key_conflicts_with_multi_arg_field(field, multi_arg_field)
                    {
                        errors.push_spanned_error(
                            multi_arg_field.ident.as_ref().unwrap(),
                            format!(
                                "field key conflicts with Redis keys emitted by multi-argument field `{}`",
                                field.attrs.name.serialize
                            ),
                        );
                        has_key_collision = true;
                    }
                }
            }
            if has_key_collision {
                return Err(());
            }

            let (idents, defs): (Vec<_>, Vec<_>) = fields
                .iter()
                .filter(|f| !f.attrs.skip_deserializing)
                .map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let ident_str = ident.to_string();
                    let keys_ident = format_ident!("{}_POSSIBLE_KEYS", ident_str.to_uppercase());
                    let possible_keys = f.attrs.name.deserialize_aliases();
                    let possible_keys_len = possible_keys.len();

                    // TODO: Support default in deserialization
                    let def = quote! {
                        const #keys_ident: [&str; #possible_keys_len] = [#(#possible_keys),*];
                        let #ident = from_redis_value(
                          #keys_ident
                            .into_iter()
                            .find(|v| fm.contains_key(*v))
                            .map(|v| fm.get(v).unwrap())
                            .unwrap_or(&Value::Nil),
                        )?;
                    };
                    (ident, def)
                })
                .unzip();
            let possible_field_keys = fields
                .iter()
                .filter(|field| !field.attrs.skip_deserializing)
                .flat_map(|field| field.attrs.name.deserialize_aliases())
                .collect::<Vec<_>>();
            let possible_field_keys_len = possible_field_keys.len();
            let possible_field_keys_ident = format_ident!("__REDIS_OM_POSSIBLE_FIELD_KEYS");

            Ok(quote! {
                impl ::redis_om::redis::FromRedisValue for #ident {
                    fn from_redis_value(v: &::redis_om::redis::Value) -> ::redis_om::redis::RedisResult<Self> {
                        use ::redis_om::redis::*;
                        const #possible_field_keys_ident: [&str; #possible_field_keys_len] = [#(#possible_field_keys),*];

                        let Value::Bulk(bulk) = v else { return Err(#err); };
                        if bulk.len() % 2 != 0 { return Err(#err); };
                        let mut fm = std::collections::HashMap::new();

                        for chunks in bulk.chunks(2) {
                            let key: String = from_redis_value(&chunks[0])?;
                            let value: Value = chunks[1].clone();
                            if #possible_field_keys_ident.contains(&key.as_str()) {
                                fm.insert(key, value);
                                continue;
                            }
                            let Some((key, idx)) = key.split_once(".") else {
                                fm.insert(key, value);
                                continue;
                            };
                            let Some(Value::Bulk(vec)) = fm.get_mut(key) else {
                                fm.insert(key.into(), Value::Bulk(vec![value]));
                                continue;
                            };

                            vec.push(value);
                        }

                        #(#defs)*

                        Ok(Self { #(#idents,)* })
                    }
                }
            })
        }
        Style::Tuple | Style::Unit => {
            let msg = format!(
                "{} Struct is not supported",
                style_name(*style, fields.len())
            );
            errors.push_spanned_error(cont.original, msg);
            Err(())
        }
    }
}

fn may_emit_multiple_args(ty: &syn::Type) -> bool {
    !is_known_single_arg_type(ty)
}

fn is_known_single_arg_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Reference(reference) => is_known_single_arg_type(&reference.elem),
        syn::Type::Paren(paren) => is_known_single_arg_type(&paren.elem),
        syn::Type::Group(group) => is_known_single_arg_type(&group.elem),
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                return false;
            }
            let path = type_path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            let Some(segment) = type_path.path.segments.last() else {
                return false;
            };
            if matches!(
                path.as_slice(),
                [root, module, option]
                    if (root == "std" || root == "core")
                        && module == "option"
                        && option == "Option"
            ) {
                let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                    return false;
                };
                if arguments.args.len() != 1 {
                    return false;
                }
                let Some(syn::GenericArgument::Type(inner)) = arguments.args.first() else {
                    return false;
                };
                return is_known_single_arg_type(inner);
            }

            let Some(ident) = path.last().map(String::as_str) else {
                return false;
            };
            let one_segment = path.len() == 1;
            let standard_string = matches!(
                path.as_slice(),
                [root, module, string]
                    if (root == "std" || root == "alloc")
                        && module == "string"
                        && string == "String"
            );
            let primitive = matches!(
                ident,
                "str"
                    | "bool"
                    | "char"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "f32"
                    | "f64"
            );
            let standard_primitive = one_segment
                || matches!(
                    path.as_slice(),
                    [root, module, _] if (root == "std" || root == "core") && module == "primitive"
                );

            standard_string || (primitive && standard_primitive)
        }
        _ => false,
    }
}

fn is_collection_item_key(collection_key: &str, field_key: &str) -> bool {
    let Some(index) = field_key
        .strip_prefix(collection_key)
        .and_then(|suffix| suffix.strip_prefix('.'))
    else {
        return false;
    };

    index
        .parse::<usize>()
        .map(|parsed| parsed.to_string() == index)
        .unwrap_or(false)
}

fn field_key_conflicts_with_multi_arg_field(multi_arg_field: &Field, field: &Field) -> bool {
    let multi_arg_key = &multi_arg_field.attrs.name.serialize;
    let serialize_collision = !multi_arg_field.attrs.skip_serializing
        && !field.attrs.skip_serializing
        && (field.attrs.name.serialize == *multi_arg_key
            || is_collection_item_key(multi_arg_key, &field.attrs.name.serialize));
    let deserialize_collision = !multi_arg_field.attrs.skip_deserializing
        && !field.attrs.skip_deserializing
        && field
            .attrs
            .name
            .deserialize_aliases()
            .into_iter()
            .any(|key| key == *multi_arg_key || is_collection_item_key(multi_arg_key, &key));

    serialize_collision || deserialize_collision
}
