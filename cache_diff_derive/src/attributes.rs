//! Holds logic for parsing the `#[cache_diff(...)]` attribute and its keys
//!
use std::str::FromStr;
use strum::IntoEnumIterator;
use syn::{punctuated::Punctuated, Attribute, Field, Ident, Token};

/// Valid keys for the `#[cache_diff(...)]` attribute
///
/// Used in parsing the user input and validating it
///
/// Requires imports to function:
///
/// ```
/// use std::str::FromStr;
/// use strum::IntoEnumIterator;
/// ```
///
/// Majority of functionality comes from strum derives
#[derive(Debug, strum::EnumIter, strum::EnumString, PartialEq, strum::Display)]
#[allow(non_camel_case_types)]
enum Key {
    rename,  // #[cache_diff(rename="...")]
    display, // #[cache_diff(display="...")]
    ignore,  // #[cache_diff(ignore)]
}

/// Holds the one or more attributes from `#[cache_diff(...)]` attribute configurations
///
/// Attributes are parsed into this struct using `CacheAttributes::parse_all` and then that
/// information is used to build the diff comparison.
#[derive(Debug, PartialEq, Eq, Default)]
pub(crate) struct CacheDiffAttributes {
    /// When present indicates the given string should be used as a name instead of the field name
    pub(crate) rename: Option<String>,

    /// When present indicates the given path to a function should be used to customize the display of the field value
    pub(crate) display: Option<syn::Path>,

    /// When `Some` indicates the field should be ignored in the diff comparison
    pub(crate) ignore: Option<()>,
}

impl CacheDiffAttributes {
    pub(crate) fn from(field: &Field) -> syn::Result<Self> {
        if let Some(attributes) = field
            .attrs
            .iter()
            .find(|&attr| attr.path().is_ident("cache_diff"))
        {
            CacheDiffAttributes::parse_all(attributes)
        } else {
            Ok(CacheDiffAttributes::default())
        }
    }

    /// Parse all attributes inside of `#[cache_diff(...)]` and return a single CacheAttributes value
    fn parse_all(input: &Attribute) -> syn::Result<Self> {
        let mut attribute = CacheDiffAttributes::default();

        match &input.meta {
            syn::Meta::List(meta_list) => {
                for attr in meta_list.parse_args_with(
                    Punctuated::<CacheDiffAttributes, Token![,]>::parse_terminated,
                )? {
                    if let Some(value) = attr.rename {
                        attribute.rename = Some(value);
                    }
                    if let Some(display) = attr.display {
                        attribute.display = Some(display);
                    }
                    if let Some(ignore) = attr.ignore {
                        attribute.ignore = Some(ignore);
                    }
                }
                Ok(attribute)
            }
            _ => Err(syn::Error::new(
                input.pound_token.span,
                "Expected a list of attributes",
            )),
        }
    }
}

impl syn::parse::Parse for CacheDiffAttributes {
    // Parse a single attribute inside of a `#[cache_diff(...)]` attribute
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();
        let mut attribute = CacheDiffAttributes::default();
        match Key::from_str(&name_str).map_err(|_| {
            syn::Error::new(
                name.span(),
                format!(
                    "Unknown cache_diff attribute: `{name_str}`. Must be one of {}",
                    Key::iter()
                        .map(|k| format!("`{k}`"))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
        })? {
            Key::rename => {
                input.parse::<syn::Token![=]>()?;
                let value = input.parse::<syn::LitStr>()?;
                attribute.rename = Some(value.value());
            }
            Key::display => {
                input.parse::<syn::Token![=]>()?;
                attribute.display = Some(input.parse()?);
            }
            Key::ignore => {
                attribute.ignore = Some(());
            }
        }
        Ok(attribute)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_all_rename() {
        let input = syn::parse_quote! {
            #[cache_diff(rename="Ruby version")]
        };
        let expected = CacheDiffAttributes {
            rename: Some("Ruby version".to_string()),
            ..Default::default()
        };
        assert_eq!(CacheDiffAttributes::parse_all(&input).unwrap(), expected);
    }

    #[test]
    fn test_parse_all_display() {
        let input = syn::parse_quote! {
            #[cache_diff(display = my_function)]
        };
        let expected = CacheDiffAttributes {
            display: Some(syn::parse_str("my_function").unwrap()),
            ..Default::default()
        };
        assert_eq!(CacheDiffAttributes::parse_all(&input).unwrap(), expected);
    }

    #[test]
    fn test_parse_all_ignore() {
        let input = syn::parse_quote! {
            #[cache_diff(ignore)]
        };
        let expected = CacheDiffAttributes {
            ignore: Some(()),
            ..Default::default()
        };
        assert_eq!(CacheDiffAttributes::parse_all(&input).unwrap(), expected);
    }

    #[test]
    fn test_parse_all_unknown() {
        let input = syn::parse_quote! {
            #[cache_diff(unknown = "IDK")]
        };
        let result = CacheDiffAttributes::parse_all(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"Unknown cache_diff attribute: `unknown`. Must be one of `rename`, `display`, `ignore`"#
        );
    }
}
