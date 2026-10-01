use darling::{FromVariant, ast::Fields, util::Flag};
use syn::ext::IdentExt;

use super::Field;
use crate::ast::{FieldOptions, Name, RedisRename, RenameAll, RenameAllRules};

#[derive(FromVariant)]
#[darling(attributes(redis))]
pub(crate) struct VariantOptions {
    rename: Option<RedisRename>,
    #[darling(multiple)]
    alias: Vec<String>,
    skip: Flag,
    skip_deserializing: Flag,
    skip_serializing: Flag,
    rename_all: Option<RenameAll>,
    fields: darling::ast::Fields<FieldOptions>,
}

/// Represents struct or enum attributes supported by redis-om.
pub(crate) struct VariantAttr {
    pub name: Name,
    pub skip_deserializing: bool,
    pub skip_serializing: bool,
    pub rename_all_rules: RenameAllRules,
}

/// A variant of an enum.
pub(crate) struct Variant<'a> {
    pub ident: &'a syn::Ident,
    pub attrs: VariantAttr,
    pub fields: Fields<Field>,
    pub inner: &'a syn::Variant,
}

impl VariantAttr {
    pub(crate) fn from_options(
        variant: &syn::Variant,
        options: VariantOptions,
    ) -> (Self, darling::ast::Fields<FieldOptions>) {
        let VariantOptions {
            rename,
            alias,
            skip,
            skip_deserializing,
            skip_serializing,
            rename_all,
            fields,
        } = options;
        let rename_all_rules = rename_all.map(RenameAllRules::from).unwrap_or_default();

        (
            Self {
                name: Name::from_attrs(variant.ident.unraw().to_string(), rename, alias),
                skip_serializing: skip.is_present() || skip_serializing.is_present(),
                skip_deserializing: skip.is_present() || skip_deserializing.is_present(),
                rename_all_rules,
            },
            fields,
        )
    }
}

#[cfg(test)]
mod tests {
    use darling::FromVariant;

    use super::{RedisRename, VariantOptions};

    #[test]
    fn parses_variant_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            enum Kind {
                #[redis(rename = "renamed", alias = "legacy", skip)]
                Variant {},
            }
        };
        let syn::Data::Enum(data) = input.data else {
            unreachable!()
        };
        let variant = data.variants.iter().next().unwrap();
        let options = VariantOptions::from_variant(variant).unwrap();

        assert!(options.skip.is_present());
        assert_eq!(options.alias, ["legacy"]);
        assert!(matches!(
            options.rename,
            Some(RedisRename::Value(value)) if value.as_ref() == "renamed"
        ));
        assert_eq!(options.fields.style, darling::ast::Style::Struct);
        assert_eq!(options.fields.fields.len(), 0);
    }

    #[test]
    fn rejects_duplicate_variant_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            enum Kind {
                #[redis(rename = "first", rename = "second")]
                Variant,
            }
        };
        let syn::Data::Enum(data) = input.data else {
            unreachable!()
        };

        assert!(VariantOptions::from_variant(data.variants.iter().next().unwrap()).is_err());
    }
}
