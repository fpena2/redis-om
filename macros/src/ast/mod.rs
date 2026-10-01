mod container;
mod field;
mod variant;

pub(crate) use container::*;
pub(crate) use field::*;
pub(crate) use variant::*;

use darling::util::SpannedValue;
use darling::{Error, FromMeta, ast::NestedMeta, error::Accumulator};
use ident_case::RenameRule as IdentCaseRule;
use quote::ToTokens;
use std::collections::BTreeSet;
use std::fmt::Display;

#[derive(Default, FromMeta)]
pub(crate) struct RenameParts<T> {
    serialize: Option<SpannedValue<T>>,
    #[darling(multiple)]
    deserialize: Vec<SpannedValue<T>>,
}

#[derive(Default, FromMeta)]
pub(crate) struct RenameAllParts {
    serialize: Option<SpannedValue<RenameRule>>,
    deserialize: Option<SpannedValue<RenameRule>>,
}

pub(crate) enum Rename<T, P> {
    Value(SpannedValue<T>),
    Parts(P),
}

pub(crate) type RedisRename = Rename<String, RenameParts<String>>;
pub(crate) type RenameAll = Rename<RenameRule, RenameAllParts>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum RenameRule {
    Standard(IdentCaseRule),
    Upper,
    ScreamingKebab,
}

impl FromMeta for RenameRule {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value {
            "UPPERCASE" => Ok(Self::Upper),
            "SCREAMING-KEBAB-CASE" => Ok(Self::ScreamingKebab),
            _ => value
                .parse::<IdentCaseRule>()
                .map(Self::Standard)
                .map_err(|_| {
                    Error::custom(format!("unknown rename rule `rename_all = {value:?}`"))
                }),
        }
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

impl<T: FromMeta, P: FromMeta> FromMeta for Rename<T, P> {
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        Ok(Self::Value(SpannedValue::from_value(value)?))
    }

    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        Ok(Self::Parts(P::from_list(items)?))
    }
}

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

fn lowercase_first_ascii(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

impl RenameRule {
    pub(crate) fn apply_to_variant(&self, variant: &str) -> String {
        match self {
            Self::Standard(IdentCaseRule::CamelCase)
                if variant.is_empty() || !variant.is_ascii() =>
            {
                lowercase_first_ascii(variant)
            }
            Self::Standard(rule) => rule.apply_to_variant(variant),
            Self::Upper => variant.to_ascii_uppercase(),
            Self::ScreamingKebab => IdentCaseRule::ScreamingSnakeCase
                .apply_to_variant(variant)
                .replace('_', "-"),
        }
    }

    pub(crate) fn apply_to_field(&self, field: &str) -> String {
        match self {
            Self::Standard(IdentCaseRule::CamelCase) => {
                lowercase_first_ascii(&IdentCaseRule::PascalCase.apply_to_field(field))
            }
            Self::Standard(rule) => rule.apply_to_field(field),
            Self::Upper => field.to_ascii_uppercase(),
            Self::ScreamingKebab => IdentCaseRule::ScreamingSnakeCase
                .apply_to_field(field)
                .replace('_', "-"),
        }
    }
}

impl From<RenameAll> for RenameAllRules {
    fn from(rename: RenameAll) -> Self {
        match rename {
            RenameAll::Value(value) => {
                let rule = *value.as_ref();
                Self {
                    serialize: Some(rule),
                    deserialize: Some(rule),
                }
            }
            RenameAll::Parts(parts) => Self {
                serialize: parts.serialize.map(|value| *value.as_ref()),
                deserialize: parts.deserialize.map(|value| *value.as_ref()),
            },
        }
    }
}

pub(crate) struct Name {
    pub serialize: String,
    pub serialize_renamed: bool,
    pub deserialize: String,
    pub deserialize_renamed: bool,
    pub deserialize_aliases: Vec<String>,
}

impl Name {
    pub(crate) fn from_attrs(
        source_name: String,
        rename: Option<RedisRename>,
        mut aliases: Vec<String>,
    ) -> Name {
        let (serialize, deserialize) = match rename {
            Some(RedisRename::Value(value)) => {
                let value = value.as_ref().clone();
                aliases.push(value.clone());
                (Some(value.clone()), Some(value))
            }
            Some(RedisRename::Parts(parts)) => {
                let serialize = parts.serialize.map(|value| value.as_ref().clone());
                let mut deserialize = None;
                for value in parts.deserialize {
                    let value = value.as_ref().clone();
                    if deserialize.is_none() {
                        deserialize = Some(value.clone());
                    }
                    aliases.push(value);
                }
                (serialize, deserialize)
            }
            None => (None, None),
        };

        let deserialize_aliases = aliases
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let serialize_renamed = serialize.is_some();
        let deserialize_renamed = deserialize.is_some();

        Name {
            serialize: serialize.unwrap_or_else(|| source_name.clone()),
            serialize_renamed,
            deserialize: deserialize.unwrap_or(source_name),
            deserialize_renamed,
            deserialize_aliases,
        }
    }

    pub(crate) fn rename_by_rules(
        &mut self,
        rules: &RenameAllRules,
        apply_rule: fn(&RenameRule, &str) -> String,
    ) {
        if !self.serialize_renamed {
            if let Some(rule) = &rules.serialize {
                self.serialize = apply_rule(rule, &self.serialize);
            }
        }
        if !self.deserialize_renamed {
            if let Some(rule) = &rules.deserialize {
                self.deserialize = apply_rule(rule, &self.deserialize);
            }
        }
    }

    pub(crate) fn deserialize_aliases(&self) -> Vec<String> {
        let mut aliases = self.deserialize_aliases.clone();
        if !aliases.contains(&self.deserialize) {
            aliases.push(self.deserialize.clone());
        }
        aliases
    }
}

#[test]
fn name_from_attrs_applies_bare_rename() {
    let rename = RedisRename::from_meta(&syn::parse_quote!(rename = "renamed")).unwrap();
    let name = Name::from_attrs("source".to_owned(), Some(rename), Vec::new());

    assert_eq!(name.serialize, "renamed");
    assert_eq!(name.deserialize, "renamed");
    assert!(name.serialize_renamed);
    assert!(name.deserialize_renamed);
    assert_eq!(name.deserialize_aliases, vec!["renamed".to_owned()]);
}

#[test]
fn name_from_attrs_applies_directional_rename() {
    let rename = RedisRename::from_meta(&syn::parse_quote!(rename(
        serialize = "written",
        deserialize = "first",
        deserialize = "second"
    )))
    .unwrap();
    let name = Name::from_attrs("source".to_owned(), Some(rename), Vec::new());

    assert_eq!(name.serialize, "written");
    assert_eq!(name.deserialize, "first");
    assert!(name.serialize_renamed);
    assert!(name.deserialize_renamed);
    assert_eq!(
        name.deserialize_aliases,
        vec!["first".to_owned(), "second".to_owned()]
    );
}

#[test]
fn name_from_attrs_sorts_and_deduplicates_aliases() {
    let name = Name::from_attrs(
        "source".to_owned(),
        None,
        vec!["zeta".to_owned(), "alpha".to_owned(), "zeta".to_owned()],
    );

    assert_eq!(
        name.deserialize_aliases,
        vec!["alpha".to_owned(), "zeta".to_owned()]
    );
}

#[test]
fn name_rename_by_rules_preserves_explicit_names() {
    let rename = RedisRename::from_meta(&syn::parse_quote!(rename(
        serialize = "written_name",
        deserialize = "read_name"
    )))
    .unwrap();
    let mut explicit_name = Name::from_attrs(
        "source_name".to_owned(),
        Some(rename),
        vec!["legacy_name".to_owned()],
    );
    let rules = RenameAllRules {
        serialize: Some(RenameRule::Standard(IdentCaseRule::PascalCase)),
        deserialize: Some(RenameRule::Standard(IdentCaseRule::PascalCase)),
    };

    explicit_name.rename_by_rules(&rules, |rule, name| rule.apply_to_field(name));

    assert_eq!(explicit_name.serialize, "written_name");
    assert_eq!(explicit_name.deserialize, "read_name");
    assert_eq!(
        explicit_name.deserialize_aliases,
        vec!["legacy_name".to_owned(), "read_name".to_owned()]
    );

    let mut field_name = Name::from_attrs("source_name".to_owned(), None, Vec::new());
    field_name.rename_by_rules(&rules, |rule, name| rule.apply_to_field(name));
    assert_eq!(field_name.serialize, "SourceName");
    assert_eq!(field_name.deserialize, "SourceName");

    let mut variant_name = Name::from_attrs("source_name".to_owned(), None, Vec::new());
    variant_name.rename_by_rules(&rules, |rule, name| rule.apply_to_variant(name));
    assert_eq!(variant_name.serialize, "source_name");
    assert_eq!(variant_name.deserialize, "source_name");
}

#[derive(Default)]
pub(crate) struct RenameAllRules {
    pub serialize: Option<RenameRule>,
    pub deserialize: Option<RenameRule>,
}

#[test]
fn accepts_extended_rename_all_rules() {
    assert!(RenameAll::from_meta(&syn::parse_quote!(rename_all = "UPPERCASE")).is_ok());
    assert!(RenameAll::from_meta(&syn::parse_quote!(rename_all = "SCREAMING-KEBAB-CASE")).is_ok());
}

#[test]
fn rejects_unknown_rename_all_rule() {
    let Err(error) = RenameAll::from_meta(&syn::parse_quote!(rename_all = "unknown-case-rule"))
    else {
        panic!("unknown rename rule should be rejected");
    };
    assert!(error.to_string().contains("unknown rename rule"));
}

#[test]
fn camel_case_handles_empty_and_unicode_field_names() {
    let rule = RenameRule::Standard(IdentCaseRule::CamelCase);
    assert_eq!(rule.apply_to_field(""), "");
    assert_eq!(rule.apply_to_field("État"), "État");
}

#[test]
fn rename_variants() {
    for &(spelling, original, expected) in &[
        ("lowercase", "Outcome", "outcome"),
        ("UPPERCASE", "Outcome", "OUTCOME"),
        ("PascalCase", "VeryTasty", "VeryTasty"),
        ("camelCase", "VeryTasty", "veryTasty"),
        ("snake_case", "VeryTasty", "very_tasty"),
        ("SCREAMING_SNAKE_CASE", "VeryTasty", "VERY_TASTY"),
        ("kebab-case", "VeryTasty", "very-tasty"),
        ("SCREAMING-KEBAB-CASE", "VeryTasty", "VERY-TASTY"),
    ] {
        let rule = RenameRule::from_string(spelling).unwrap();
        assert_eq!(rule.apply_to_variant(original), expected);
    }
}

#[test]
fn rename_fields() {
    for &(spelling, original, expected) in &[
        ("lowercase", "outcome", "outcome"),
        ("UPPERCASE", "outcome", "OUTCOME"),
        ("PascalCase", "very_tasty", "VeryTasty"),
        ("camelCase", "very_tasty", "veryTasty"),
        ("snake_case", "very_tasty", "very_tasty"),
        ("SCREAMING_SNAKE_CASE", "very_tasty", "VERY_TASTY"),
        ("kebab-case", "very_tasty", "very-tasty"),
        ("SCREAMING-KEBAB-CASE", "very_tasty", "VERY-TASTY"),
    ] {
        let rule = RenameRule::from_string(spelling).unwrap();
        assert_eq!(rule.apply_to_field(original), expected);
    }
}
