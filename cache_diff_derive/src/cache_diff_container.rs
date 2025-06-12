//! Represents a named struct i.e. `struct Metadat { version: String }` for implementing the CacheDiff trait
//!
//! In syn terminology a "container" is a named struct, un-named (tuple) struct, or an enum. In the
//! case of CacheDiff, it's always a named struct. A container can have zero or more attributes:
//!
//! ```text
//! #[cache_diff(custom = custom_diff)]
//! struct Metadata {
//!     // ...
//! }
//! ```
//!
//! This looks similar to, but is differnt than a field attribute:
//!
//! ```text
//! #[cache_diff(rename = "Ruby Version")]
//! name: String
//! ```
//!
//! Field attributes are handled by [CacheDiffField] and associated functions.
//!
//! One or more comma-separated attributes is parsed into a [ParsedAttribute] for the container.
//! Then one or more named fields are parsed into one or more [ActiveField]-s. Finally this information
//! is brought together to create a fully formed [CacheDiffContainer].

use crate::cache_diff_field::{ActiveField, ParsedField};
use std::str::FromStr;
use syn::parse::Parse;
use syn::Data::Struct;
use syn::Fields::Named;
use syn::{DataStruct, FieldsNamed, Ident};

/// Represents the fully parsed Struct, it's attributes and all of it's parsed fields
#[derive(Debug, PartialEq)]
pub(crate) struct CacheDiffContainer {
    /// The identifier of a struct e.g. `struct Metadata {version: String}` would be `Metadata`
    pub(crate) identifier: Ident,
    /// Info about generics, lifetimes and where clauses i.e. `struct Metadata<T> { name: T }`
    pub(crate) generics: syn::Generics,
    /// An optional path to a custom diff function
    pub(crate) custom: Option<syn::Path>, // #[cache_diff(custom = <function>)]
    /// One or more named fields
    pub(crate) fields: Vec<ActiveField>,
}

impl CacheDiffContainer {
    pub(crate) fn from_ast(input: &syn::DeriveInput) -> syn::Result<Self> {
        let identifier = input.ident.clone();
        let generics = input.generics.clone();
        let mut container_custom = None;

        for attribute in input
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("cache_diff"))
        {
            match attribute.parse_args_with(ParsedAttribute::parse)? {
                ParsedAttribute::custom(path) => container_custom = Some(path),
            }
        }

        let mut fields = Vec::new();
        for ast_field in match input.data {
            Struct(DataStruct {
                fields: Named(FieldsNamed { ref named, .. }),
                ..
            }) => named,
            _ => unimplemented!("CacheDiff derive macro can only be used on named structs"),
        }
        .to_owned()
        .iter()
        {
            match ParsedField::from_field(ast_field)? {
                ParsedField::IgnoredCustom => {
                    if container_custom.is_none() {
                        return Err(syn::Error::new(
                            identifier.span(),
                            format!(
                                "field `{field}` on {container} marked ignored as custom, but no `#[cache_diff(custom = <function>)]` found on `{container}`",
                                field = ast_field.clone().ident.expect("named structs only"),
                                container = &identifier,
                            )
                        ));
                    }
                }
                ParsedField::IgnoredOther => {}
                ParsedField::Active(active_field) => fields.push(active_field),
            }
        }

        if fields.is_empty() {
            Err(syn::Error::new(
            identifier.span(),
            "No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`-d",
        ))
        } else {
            Ok(CacheDiffContainer {
                identifier,
                generics,
                custom: container_custom,
                fields,
            })
        }
    }
}

/// Holds one macro configuration attribute for a field (i.e. `name: String`)
///
/// Enum variants match configuration attribute keys exactly, this allows us to guarantee our error
/// messages are correct.
///
/// Zero or more of these are used to build a [CacheDiffContainer]
#[derive(Debug, strum::EnumDiscriminants)]
#[strum_discriminants(derive(strum::EnumIter, strum::Display, strum::EnumString))]
#[strum_discriminants(name(KnownAttribute))]
enum ParsedAttribute {
    #[allow(non_camel_case_types)]
    custom(syn::Path),
}

/// List all valid attributes for a field, mostly for error messages
fn known_attributes() -> String {
    use strum::IntoEnumIterator;

    KnownAttribute::iter()
        .map(|k| format!("`{k}`"))
        .collect::<Vec<String>>()
        .join(", ")
}

impl syn::parse::Parse for ParsedAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();
        match KnownAttribute::from_str(&name_str).map_err(|_| {
            syn::Error::new(
                name.span(),
                format!(
                    "Unknown cache_diff attribute: `{name_str}`. Must be one of {valid_keys}",
                    valid_keys = known_attributes()
                ),
            )
        })? {
            KnownAttribute::custom => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParsedAttribute::custom(input.parse()?))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use syn::DeriveInput;

    #[test]
    fn test_custom_all_ignored() {
        let input: DeriveInput = syn::parse_quote! {
            struct Metadata {
                #[cache_diff(ignore)]
                version: String
            }
        };

        let result = CacheDiffContainer::from_ast(&input);
        assert!(result.is_err(), "Expected an error, got {result:?}");
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`-d"#
        );
    }

    #[test]
    fn test_no_fields() {
        let input: DeriveInput = syn::parse_quote! {
            struct Metadata {}
        };

        let result = CacheDiffContainer::from_ast(&input);
        assert!(result.is_err(), "Expected an error, got {result:?}");
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`-d"#
        );
    }

    #[test]
    fn test_custom_missing_on_container() {
        let input: DeriveInput = syn::parse_quote! {
            struct Metadata {
                #[cache_diff(ignore = "custom")]
                version: String
            }
        };

        let result = CacheDiffContainer::from_ast(&input);
        assert!(result.is_err(), "Expected an error, got {result:?}");
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"field `version` on Metadata marked ignored as custom, but no `#[cache_diff(custom = <function>)]` found on `Metadata`"#
        );
    }

    #[test]
    fn test_custom_on_container() {
        let input: DeriveInput = syn::parse_quote! {
            #[cache_diff(custom = my_function)]
            struct Metadata {
                version: String
            }
        };

        let container = CacheDiffContainer::from_ast(&input).unwrap();
        assert!(container.custom.is_some());
    }

    #[test]
    fn test_no_custom_on_container() {
        let input: DeriveInput = syn::parse_quote! {
            struct Metadata {
                version: String
            }
        };

        let container = CacheDiffContainer::from_ast(&input).unwrap();
        assert!(container.custom.is_none());
    }
}
