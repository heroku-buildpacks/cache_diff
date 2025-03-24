//! Represents a field on a struct i.e. `version: String` for implementing the CacheDiff trait
//!
//! A `syn::Field`` is can be a named or un-named value (in the case of a tuple struct) or
//! also represent an Enum. In the case of CacheDiff, we only deal in named structs.
//!
//! A field can have zero or more `#[cache_diff()]` attribute annotations for example:
//!
//! ```text
//! #[cache_diff(rename = "Ruby Version")]
//! name: String
//! ```
//!
//! These attributes look similar to "container" attributes but are different:
//!
//! ```text
//! #[cache_diff(custom = custom_diff)]
//! struct Metadata {
//!     // ...
//! }
//! ```
//!
//! Container attributes are handled by [CacheDiffContainer] and associated files.
//!
//! Each comma separated attribute is parsed into a [ParsedAttribute] enum and that information is
//! combined to form a full [ParsedField].
//!
//! A one or more [ParsedField::Active]-s lives inside of a [CacheDiffContainer].

use std::str::FromStr;
use strum::IntoEnumIterator;
use syn::{spanned::Spanned, Field, Ident, PathArguments};

use crate::shared::WithSpan;

#[derive(Debug, PartialEq)]
pub(crate) enum ParsedField {
    IgnoredCustom,
    IgnoredOther,
    Active(ActiveField),
}

#[derive(Debug, PartialEq)]
pub(crate) struct ActiveField {
    /// What the user will see when this field differs and invalidates the cache
    /// i.e. `age: usize` will be `"age"``
    pub(crate) name: String,
    /// The function to use when rendering values on the field
    /// i.e. `age: 42` will be `"42"`
    pub(crate) display_fn: syn::Path,
    /// The proc-macro identifier for a field i.e. `name: String` would be a programatic
    /// reference to `name` that can be used along with `quote!` to produce code
    pub(crate) field_identifier: Ident,
}

impl ParsedField {
    pub(crate) fn from_field(field: &Field) -> syn::Result<Self> {
        let mut rename = None;
        let mut display = None;
        let mut ignored = None;
        let field_identifier = field.ident.clone().ok_or_else(|| {
            syn::Error::new(
                field.span(),
                "CacheDiff can only be used on structs with named fields",
            )
        })?;

        for (_, WithSpan(attribute, _)) in
            crate::shared::attribute_lookup::<ParsedAttribute>(&field.attrs)?.drain()
        {
            match attribute {
                ParsedAttribute::rename(inner) => rename = Some(inner),
                ParsedAttribute::display(inner) => display = Some(inner),
                ParsedAttribute::ignore(inner) => ignored = Some(inner),
            }
        }

        if let Some(ignored) = ignored {
            if display.is_some() || rename.is_some() {
                Err(syn::Error::new(field_identifier.span(), format!("The cache_diff attribute `{}` renders other attributes useless, remove additional attributes", KnownAttribute::ignore)))
            } else {
                Ok(ignored.into())
            }
        } else {
            let name = rename.unwrap_or_else(|| field_identifier.to_string().replace("_", " "));
            let display_fn = display.unwrap_or_else(|| {
                if is_pathbuf(&field.ty) {
                    syn::parse_str("std::path::Path::display")
                        .expect("PathBuf::display parses as a syn::Path")
                } else {
                    syn::parse_str("std::convert::identity")
                        .expect("std::convert::identity parses as a syn::Path")
                }
            });
            Ok(ParsedField::Active(ActiveField {
                name,
                display_fn,
                field_identifier,
            }))
        }
    }
}

/// Holds one macro configuration attribute for a field (i.e. `name: String`)
///
/// Enum variants match configuration attribute keys exactly, this allows us to guarantee our error
/// messages are correct.
///
/// Zero or more of these are used to build a [ParsedField]
#[derive(Debug, strum::EnumDiscriminants)]
#[strum_discriminants(
    name(KnownAttribute),
    derive(strum::EnumIter, strum::Display, strum::EnumString, Hash)
)]
enum ParsedAttribute {
    #[allow(non_camel_case_types)]
    rename(String), // #[cache_diff(rename="...")]
    #[allow(non_camel_case_types)]
    display(syn::Path), // #[cache_diff(display="...")]
    #[allow(non_camel_case_types)]
    ignore(Ignored), // #[cache_diff(ignore)]
}

/// List all valid attributes for a field, mostly for error messages
fn known_attributes() -> String {
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
            let extra = match name_str.as_ref() {
                "custom" => "\nThe cache_diff attribute `custom` is available on the struct, not the field",
                _ => ""
            };

            syn::Error::new(
                name.span(),
                format!(
                    "Unknown cache_diff attribute: `{name_str}`. Must be one of {valid_keys}{extra}",
                    valid_keys = known_attributes()
                ),
            )
        })? {
            KnownAttribute::rename => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParsedAttribute::rename(input.parse::<syn::LitStr>()?.value()))
            }
            KnownAttribute::display => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParsedAttribute::display(input.parse()?))
            }
            KnownAttribute::ignore => {
                if input.peek(syn::Token![=]) {
                    input.parse::<syn::Token![=]>()?;
                    let value = input.parse::<syn::LitStr>()?.value();
                    if &value == "custom" {
                        Ok(ParsedAttribute::ignore(Ignored::IgnoreCustom))
                    } else {
                        Ok(ParsedAttribute::ignore(Ignored::IgnoreOther))
                    }
                } else {
                    Ok(ParsedAttribute::ignore(Ignored::IgnoreOther))
                }
            }
        }
    }
}

/// Represents whether a field is included in the derive diff comparison or not and why
#[derive(Debug, PartialEq)]
pub(crate) enum Ignored {
    /// Ignored because field is delegated to `custom = <function>` on the container.
    /// This information is needed so we can raise an error when the container does not implement this attribute
    IgnoreCustom,
    /// Ignored for some other reason
    IgnoreOther,
}

impl From<Ignored> for ParsedField {
    fn from(value: Ignored) -> Self {
        match value {
            Ignored::IgnoreCustom => ParsedField::IgnoredCustom,
            Ignored::IgnoreOther => ParsedField::IgnoredOther,
        }
    }
}

fn is_pathbuf(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "PathBuf" && segment.arguments == PathArguments::None;
        }
    }
    false
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::formatdoc;
    use pretty_assertions::assert_eq;
    use syn::Attribute;

    fn attribute_on_field(attribute: Attribute, field: Field) -> Field {
        let mut input = field.clone();
        input.attrs = vec![attribute];
        input
    }

    #[test]
    fn test_parse_all_rename() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(rename="Ruby version")]
            },
            syn::parse_quote! {
                version: String
            },
        );
        let expected = ParsedField::Active(ActiveField {
            name: "Ruby version".to_string(),
            display_fn: syn::parse_str("std::convert::identity").unwrap(),
            field_identifier: input.ident.to_owned().unwrap(),
        });
        assert_eq!(expected, ParsedField::from_field(&input).unwrap());
    }

    #[test]
    fn test_parse_all_display() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(display = my_function)]
            },
            syn::parse_quote! {
                version: String
            },
        );
        let expected = ParsedField::Active(ActiveField {
            name: "version".to_string(),
            display_fn: syn::parse_str("my_function").unwrap(),
            field_identifier: input.ident.to_owned().unwrap(),
        });
        assert_eq!(expected, ParsedField::from_field(&input).unwrap());
    }

    #[test]
    fn test_ignore_with_value() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(ignore = "value")]
            },
            syn::parse_quote! {
                version: String
            },
        );
        assert_eq!(
            ParsedField::IgnoredOther,
            ParsedField::from_field(&input).unwrap()
        );
    }

    #[test]
    fn test_parse_all_ignore_no_value() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(ignore)]
            },
            syn::parse_quote! {
                version: String
            },
        );
        assert_eq!(
            ParsedField::IgnoredOther,
            ParsedField::from_field(&input).unwrap()
        );
    }

    #[test]
    fn test_parse_all_ignore_custom() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(ignore = "custom")]
            },
            syn::parse_quote! {
                version: String
            },
        );
        assert_eq!(
            ParsedField::IgnoredCustom,
            ParsedField::from_field(&input).unwrap()
        );
    }

    #[test]
    fn test_parse_accidental_custom() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(custom = "IDK")]
            },
            syn::parse_quote! {
                version: String
            },
        );

        let result = ParsedField::from_field(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()).trim(),
            formatdoc! {"
                Unknown cache_diff attribute: `custom`. Must be one of `rename`, `display`, `ignore`
                The cache_diff attribute `custom` is available on the struct, not the field
            "}
            .trim()
        );
    }

    #[test]
    fn test_parse_all_unknown() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(unknown = "IDK")]
            },
            syn::parse_quote! {
                version: String
            },
        );
        let result = ParsedField::from_field(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"Unknown cache_diff attribute: `unknown`. Must be one of `rename`, `display`, `ignore`"#
        );
    }

    #[test]
    fn test_ignored_other_attributes() {
        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(ignore = "reasons", display = my_function)]
            },
            syn::parse_quote! {
                version: String
            },
        );
        let result = ParsedField::from_field(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"The cache_diff attribute `ignore` renders other attributes useless, remove additional attributes"#
        );

        let input = attribute_on_field(
            syn::parse_quote! {
                #[cache_diff(display = my_function, ignore = "reasons")]
            },
            syn::parse_quote! {
                version: String
            },
        );
        let result = ParsedField::from_field(&input);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"The cache_diff attribute `ignore` renders other attributes useless, remove additional attributes"#
        );
    }
}
