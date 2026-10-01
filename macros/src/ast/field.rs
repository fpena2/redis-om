use darling::{FromField, util::Flag};

use crate::ast::{Name, RedisRename};

#[derive(FromField)]
#[darling(attributes(redis))]
pub(crate) struct FieldOptions {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    rename: Option<RedisRename>,
    #[darling(multiple)]
    alias: Vec<String>,
    primary_key: Flag,
    index: Flag,
    sortable: Flag,
    full_text_search: Flag,
    skip: Flag,
    skip_deserializing: Flag,
    skip_serializing: Flag,
}

/// Represents struct or enum attributes supported by redis-om.
pub(crate) struct FieldAttr {
    pub name: Name,
    /// Whether the key should be considered a primary key
    pub primary_key: bool,
    /// Whether the key should be indexed
    pub index: bool,
    /// Whether the key should be sortable
    pub sortable: bool,
    /// Whether the key should be full text search
    pub fts: bool,
    pub skip_deserializing: bool,
    pub skip_serializing: bool,
}

/// A field of a struct.
pub(crate) struct Field {
    pub ident: Option<syn::Ident>,
    pub attrs: FieldAttr,
    pub ty: syn::Type,
}

impl FieldOptions {
    pub(crate) fn into_field(self, index: usize) -> Field {
        let Self {
            ident,
            ty,
            rename,
            alias,
            primary_key,
            index: is_indexed,
            sortable,
            full_text_search,
            skip,
            skip_deserializing,
            skip_serializing,
        } = self;
        let name = match &ident {
            Some(ident) => ident.to_string().trim_start_matches("r#").to_owned(),
            None => index.to_string(),
        };
        Field {
            ident,
            ty,
            attrs: FieldAttr {
                name: Name::from_attrs(name, rename, alias),
                primary_key: primary_key.is_present(),
                index: is_indexed.is_present(),
                sortable: sortable.is_present(),
                fts: full_text_search.is_present(),
                skip_serializing: skip.is_present() || skip_serializing.is_present(),
                skip_deserializing: skip.is_present() || skip_deserializing.is_present(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use darling::FromField;

    use super::{FieldOptions, RedisRename};

    #[test]
    fn parses_field_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Record {
                #[redis(
                    rename(
                        serialize = "outgoing",
                        deserialize = "incoming",
                        deserialize = "historical"
                    ),
                    alias = "legacy",
                    primary_key,
                    index,
                    sortable,
                    full_text_search,
                    skip_serializing,
                    skip_deserializing
                )]
                field: String
            }
        };
        let syn::Data::Struct(data) = input.data else {
            unreachable!()
        };
        let mut fields = data.fields.iter();
        let field = fields.next().unwrap();
        let options = FieldOptions::from_field(field).unwrap();

        assert_eq!(options.ident.as_ref().unwrap().to_string(), "field");
        assert_eq!(options.ty, syn::parse_quote!(String));
        assert!(options.primary_key.is_present());
        assert!(options.index.is_present());
        assert!(options.sortable.is_present());
        assert!(options.full_text_search.is_present());
        assert!(options.skip_serializing.is_present());
        assert!(options.skip_deserializing.is_present());
        assert_eq!(options.alias, ["legacy"]);
        let Some(RedisRename::Parts(parts)) = options.rename else {
            unreachable!()
        };
        assert_eq!(
            parts
                .serialize
                .as_ref()
                .map(|value| value.as_ref().as_str()),
            Some("outgoing")
        );
        assert_eq!(
            parts
                .deserialize
                .iter()
                .map(|value| value.as_ref().as_str())
                .collect::<Vec<_>>(),
            ["incoming", "historical"]
        );
    }

    #[test]
    fn parses_tuple_field_without_identifier() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Record(String);
        };
        let syn::Data::Struct(data) = input.data else {
            unreachable!()
        };
        let options = FieldOptions::from_field(data.fields.iter().next().unwrap()).unwrap();

        assert!(options.ident.is_none());
        assert_eq!(options.ty, syn::parse_quote!(String));
    }

    #[test]
    fn into_field_normalizes_raw_identifier() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Record {
                r#type: String,
            }
        };
        let syn::Data::Struct(data) = input.data else {
            unreachable!()
        };
        let options = FieldOptions::from_field(data.fields.iter().next().unwrap()).unwrap();
        let field = options.into_field(0);

        assert_eq!(field.attrs.name.serialize, "type");
    }

    #[test]
    fn rejects_values_for_flag_attributes() {
        let input: syn::DeriveInput = syn::parse_quote! {
            struct Record {
                #[redis(index = true)]
                field: String,
            }
        };
        let syn::Data::Struct(data) = input.data else {
            unreachable!()
        };

        assert!(FieldOptions::from_field(data.fields.iter().next().unwrap()).is_err());
    }
}
