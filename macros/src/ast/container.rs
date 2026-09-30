use super::{Field, FieldOptions, Variant, VariantAttr, VariantOptions};
use crate::ast::{AccumulatorExt, RenameAll, RenameAllRules, RenameRule};
use darling::ast::{Data, Fields};
use darling::error::Accumulator;
use darling::FromDeriveInput;

#[derive(FromDeriveInput)]
#[darling(attributes(redis))]
struct ContainerOptions {
    prefix_key: Option<String>,
    key: Option<String>,
    rename_all: Option<RenameAll>,
    data: Data<VariantOptions, FieldOptions>,
}

pub(crate) struct Container<'a> {
    /// The struct or enum name (without generics).
    pub ident: &'a syn::Ident,
    /// The contents of the struct or enum.
    pub data: Data<Variant<'a>, Field>,
    /// Redis database prefix key or redis stream name
    pub prefix_key: String,
    /// Original input.
    pub original: &'a syn::DeriveInput,
}

impl<'a> Container<'a> {
    /// Convert the raw Syn ast into a parsed container object, collecting errors in `errors`.
    pub(crate) fn new(
        errors: &mut Accumulator,
        item: &'a syn::DeriveInput,
    ) -> Option<Container<'a>> {
        let options = match ContainerOptions::from_derive_input(item) {
            Ok(options) => options,
            Err(err) => {
                errors.push(err);
                return None;
            }
        };

        let ContainerOptions {
            prefix_key,
            key,
            rename_all,
            data: parsed_data,
        } = options;

        let prefix_key = match (prefix_key, key) {
            (Some(prefix_key), Some(_key)) => {
                errors.push_spanned_error(item, "duplicate redis attribute `prefix_key`");
                prefix_key
            }
            (Some(prefix_key), None) => prefix_key,
            (None, Some(key)) => key,
            (None, None) => item.ident.to_string().trim_start_matches("r#").to_owned(),
        };

        let rename_all_rules = rename_all.map(RenameAllRules::from).unwrap_or_default();

        let mut data: Data<Variant<'a>, Field> = match (&item.data, parsed_data) {
            (syn::Data::Enum(data), Data::Enum(options)) => Data::Enum(
                data.variants
                    .iter()
                    .zip(options)
                    .map(|(variant, options)| {
                        let (attrs, fields_options) = VariantAttr::from_options(variant, options);
                        let fields = struct_from_options(fields_options);
                        Variant {
                            ident: &variant.ident,
                            attrs,
                            fields,
                            inner: variant,
                        }
                    })
                    .collect(),
            ),
            (syn::Data::Struct(_), Data::Struct(options)) => {
                Data::Struct(struct_from_options(options))
            }
            _ => unreachable!(),
        };

        match &mut data {
            Data::Enum(variants) => {
                for variant in variants {
                    variant
                        .attrs
                        .name
                        .rename_by_rules(&rename_all_rules, RenameRule::apply_to_variant);
                    for field in &mut variant.fields.fields {
                        field.attrs.name.rename_by_rules(
                            &variant.attrs.rename_all_rules,
                            RenameRule::apply_to_field,
                        );
                    }
                }
            }
            Data::Struct(fields) => {
                for field in &mut fields.fields {
                    field
                        .attrs
                        .name
                        .rename_by_rules(&rename_all_rules, RenameRule::apply_to_field);
                }
            }
        }

        let item = Container {
            ident: &item.ident,
            prefix_key,
            data,
            original: item,
        };

        Some(item)
    }
}

fn struct_from_options(options: Fields<FieldOptions>) -> Fields<Field> {
    let mut index = 0;
    let mut fields = options.map(|options| {
        let field = options.into_field(index);
        index += 1;
        field
    });

    let pk_is_set = fields.fields.iter().any(|f| f.attrs.primary_key);
    if !pk_is_set {
        if let Some(id_field) = fields
            .fields
            .iter_mut()
            .find(|f| f.attrs.name.serialize == "id")
        {
            id_field.attrs.primary_key = true;
        };
    }
    fields
}

#[cfg(test)]
mod tests {
    use darling::ast::{Data, Style};
    use darling::error::Accumulator;
    use darling::{FromDeriveInput, FromMeta};

    use super::{Container, ContainerOptions, RenameAll};
    use crate::ast::RenameRule;

    fn parse_container<'a>(input: &'a syn::DeriveInput) -> Container<'a> {
        let mut errors = Accumulator::default();
        let container = Container::new(&mut errors, input).expect("container should parse");
        errors.finish().unwrap();
        container
    }

    #[test]
    fn parses_container_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[redis(
                prefix_key = "records",
                key = "stream",
                rename_all(serialize = "camelCase", deserialize = "snake_case")
            )]
            struct Record { id: String }
        };
        let options = ContainerOptions::from_derive_input(&input).unwrap();

        assert_eq!(options.prefix_key.as_deref(), Some("records"));
        assert_eq!(options.key.as_deref(), Some("stream"));
        let Some(RenameAll::Parts(parts)) = options.rename_all else {
            unreachable!()
        };
        assert_eq!(
            parts.serialize.as_ref().map(|value| *value.as_ref()),
            Some(RenameRule::from_string("camelCase").unwrap())
        );
        assert_eq!(
            parts.deserialize.as_ref().map(|value| *value.as_ref()),
            Some(RenameRule::from_string("snake_case").unwrap())
        );
    }

    #[test]
    fn rejects_unknown_container_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[redis(unknown)]
            struct Record { id: String }
        };

        assert!(ContainerOptions::from_derive_input(&input).is_err());
    }

    #[test]
    fn rejects_unknown_rename_all_rules() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[redis(rename_all = "unknown-case-rule")]
            struct Record { id: String }
        };
        let mut errors = Accumulator::default();

        assert!(Container::new(&mut errors, &input).is_none());
        let errors = errors.finish().unwrap_err().write_errors().to_string();
        assert!(errors.contains("unknown-case-rule"));
    }

    #[test]
    fn resolves_prefix_key_legacy_key_and_identifier_fallbacks() {
        let prefix_key: syn::DeriveInput = syn::parse_quote! {
            #[redis(prefix_key = "records")]
            struct Record { id: String }
        };
        assert_eq!(parse_container(&prefix_key).prefix_key, "records");

        let key: syn::DeriveInput = syn::parse_quote! {
            #[redis(key = "stream")]
            struct Record { id: String }
        };
        assert_eq!(parse_container(&key).prefix_key, "stream");

        let fallback: syn::DeriveInput = syn::parse_quote! {
            struct Record { id: String }
        };
        assert_eq!(parse_container(&fallback).prefix_key, "Record");

        let raw_identifier: syn::DeriveInput = syn::parse_quote! {
            struct r#Record { id: String }
        };
        assert_eq!(parse_container(&raw_identifier).prefix_key, "Record");
    }

    #[test]
    fn reports_duplicate_prefix_key_and_key_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[redis(prefix_key = "records", key = "stream")]
            struct Record { id: String }
        };
        let mut errors = Accumulator::default();

        assert!(Container::new(&mut errors, &input).is_some());
        let errors = errors.finish().unwrap_err().write_errors().to_string();
        assert!(errors.contains("duplicate redis attribute `prefix_key`"));
    }

    #[test]
    fn reports_duplicate_deserialize_rename_all_rules() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[redis(rename_all(
                serialize = "camelCase",
                deserialize = "snake_case",
                deserialize = "kebab-case"
            ))]
            struct Record { id: String }
        };
        let mut errors = Accumulator::default();

        assert!(Container::new(&mut errors, &input).is_none());
        let errors = errors.finish().unwrap_err().write_errors().to_string();
        assert!(errors.contains("deserialize"));
    }

    #[test]
    fn preserves_field_style_and_order() {
        let named: syn::DeriveInput = syn::parse_quote! {
            struct Named { first: String, second: u8 }
        };
        let named = parse_container(&named);
        let Data::Struct(fields) = &named.data else {
            unreachable!()
        };
        assert_eq!(fields.style, Style::Struct);
        assert_eq!(
            fields
                .fields
                .iter()
                .map(|field| field.attrs.name.serialize.as_str())
                .collect::<Vec<_>>(),
            ["first", "second"]
        );

        let tuple: syn::DeriveInput = syn::parse_quote! {
            struct Tuple(String, u8);
        };
        let tuple = parse_container(&tuple);
        let Data::Struct(fields) = &tuple.data else {
            unreachable!()
        };
        assert_eq!(fields.style, Style::Tuple);
        assert_eq!(
            fields
                .fields
                .iter()
                .map(|field| field.attrs.name.serialize.as_str())
                .collect::<Vec<_>>(),
            ["0", "1"]
        );

        let unit: syn::DeriveInput = syn::parse_quote! {
            struct Unit;
        };
        let unit = parse_container(&unit);
        let Data::Struct(fields) = &unit.data else {
            unreachable!()
        };
        assert_eq!(fields.style, Style::Unit);
        assert!(fields.fields.is_empty());
    }

    #[test]
    fn infers_primary_key_before_container_rename_rules() {
        let default_id: syn::DeriveInput = syn::parse_quote! {
            #[redis(rename_all = "SCREAMING_SNAKE_CASE")]
            struct Record { id: String }
        };
        let default_id = parse_container(&default_id);
        let Data::Struct(fields) = &default_id.data else {
            unreachable!()
        };
        assert!(fields.fields[0].attrs.primary_key);
        assert_eq!(fields.fields[0].attrs.name.serialize, "ID");

        let renamed_id: syn::DeriveInput = syn::parse_quote! {
            #[redis(rename_all = "SCREAMING_SNAKE_CASE")]
            struct Record {
                #[redis(rename(serialize = "id"))]
                key: String,
            }
        };
        let renamed_id = parse_container(&renamed_id);
        let Data::Struct(fields) = &renamed_id.data else {
            unreachable!()
        };
        assert!(fields.fields[0].attrs.primary_key);
        assert_eq!(fields.fields[0].attrs.name.serialize, "id");
    }

    #[test]
    fn normalizes_raw_field_names() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Record { r#type: String }
        };
        let container = parse_container(&input);
        let Data::Struct(fields) = &container.data else {
            unreachable!()
        };

        assert_eq!(fields.fields[0].attrs.name.serialize, "type");
    }

    #[test]
    fn retains_original_variant_node() {
        let input: syn::DeriveInput = syn::parse_quote! {
            enum Record { Item { value: String } }
        };
        let syn::Data::Enum(original) = &input.data else {
            unreachable!()
        };
        let original_variant = &original.variants[0];
        let container = parse_container(&input);
        let Data::Enum(variants) = &container.data else {
            unreachable!()
        };

        assert!(std::ptr::eq(variants[0].inner, original_variant));
    }

    #[test]
    fn reports_invalid_nested_variant_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            enum Record {
                #[redis(unknown_variant)]
                Item,
            }
        };
        let mut errors = Accumulator::default();

        assert!(Container::new(&mut errors, &input).is_none());
        let errors = errors.finish().unwrap_err().write_errors().to_string();
        assert!(errors.contains("unknown_variant"));
    }

    #[test]
    fn reports_container_attribute_error_before_parsing_data() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[redis(unknown_container)]
            struct Record {
                #[redis(unknown_field)]
                value: String,
            }
        };
        let mut errors = Accumulator::default();

        assert!(Container::new(&mut errors, &input).is_none());
        let errors = errors.finish().unwrap_err().write_errors().to_string();
        assert!(errors.contains("Unknown field: `unknown_container`"));
    }
}
