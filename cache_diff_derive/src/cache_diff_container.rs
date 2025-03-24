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
use crate::shared::{self, WithSpan};
use std::collections::VecDeque;
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
        let mut fields = Vec::new();
        let mut errors = VecDeque::new();
        let mut container_custom = None;

        match crate::shared::attribute_lookup::<ParsedAttribute>(&input.attrs) {
            Ok(mut lookup) => {
                for (_, WithSpan(value, _)) in lookup.drain() {
                    match value {
                        ParsedAttribute::custom(path) => container_custom = Some(path),
                    }
                }
            }
            Err(error) => errors.push_back(error),
        }

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
            match ParsedField::from_field(ast_field) {
                Ok(ParsedField::IgnoredCustom) => {
                    if container_custom.is_none() {
                        errors.push_back(
                            syn::Error::new(
                                identifier.span(),
                                format!(
                                    "field `{field}` on {container} marked ignored as custom, but no `#[cache_diff(custom = <function>)]` found on `{container}`",
                                    field = ast_field.clone().ident.expect("named structs only"),
                                    container = &identifier,
                                )
                            )
                        )
                    }
                }
                Ok(ParsedField::IgnoredOther) => {}
                Ok(ParsedField::Active(active_field)) => fields.push(active_field),
                Err(error) => {
                    errors.push_back(error);
                }
            }
        }

        if let Some(mut first) = errors.pop_front() {
            for e in errors {
                first.combine(e);
            }
            Err(first)
        } else if fields.is_empty() {
            Err(syn::Error::new(
            identifier.span(), "No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`-d",
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
#[strum_discriminants(
    name(KnownAttribute),
    derive(strum::EnumIter, strum::Display, strum::EnumString, Hash)
)]
enum ParsedAttribute {
    #[allow(non_camel_case_types)]
    custom(syn::Path),
}

impl syn::parse::Parse for ParsedAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        match shared::known_attribute(&name)? {
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

    fn assert_regex_count_in_text(
        regex: &regex::Regex,
        count: usize,
        text: &str,
    ) -> Result<(), String> {
        let found = regex.find_iter(text).count();
        if count == found {
            Ok(())
        } else {
            Err(format!(
                "Expected {count} matches, found {found} from Regex: `{regex}` in text: {text}"
            ))
        }
    }

    #[test]
    fn test_custom_all_ignored() {
        let input: DeriveInput = syn::parse_quote! {
            struct Metadata {
                #[cache_diff(ignore)]
                version: String
            }
        };

        let result = CacheDiffContainer::from_ast(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
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
        assert!(result.is_err(), "Expected an error, got {:?}", result);
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
        assert!(result.is_err(), "Expected an error, got {:?}", result);
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

    #[test]
    fn test_multiple_fields_error_rollup() {
        let input: DeriveInput = syn::parse_quote! {
            #[cache_diff(custom = my_function)]
            struct Metadata {
                #[cache_diff(unknown)]
                version: String,
                #[cache_diff(unknown)]
                architecture: String,
            }
        };

        let result = CacheDiffContainer::from_ast(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_regex_count_in_text(
            &regex::Regex::new("Unknown cache_diff attribute").unwrap(),
            2,
            &format!("{:?}", result.err().unwrap()),
        )
        .unwrap();
    }

    #[test]
    fn test_multiple_container_attribute_problems() {
        let input: DeriveInput = syn::parse_quote! {
            #[cache_diff(unknown, custom)]
            struct Metadata {
                version: String,
            }
        };

        let result = CacheDiffContainer::from_ast(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_regex_count_in_text(
            &regex::Regex::new("Unknown cache_diff attribute").unwrap(),
            1,
            &format!("{:?}", result.err().unwrap()),
        )
        .unwrap();
    }

    #[test]
    fn test_duplicate_attributes() {
        let input: DeriveInput = syn::parse_quote! {
            #[cache_diff(custom = a, custom = b, custom = c)]
            struct Metadata {
                version: String,
            }
        };

        let result = CacheDiffContainer::from_ast(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        let output = format!("{:?}", result.err().unwrap());
        assert_regex_count_in_text(
            &regex::Regex::new("duplicate attribute").unwrap(),
            2,
            &output,
        )
        .unwrap();
        assert_regex_count_in_text(&regex::Regex::new("defined here").unwrap(), 2, &output)
            .unwrap();
    }
}
