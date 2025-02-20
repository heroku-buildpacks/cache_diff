use crate::{MACRO_NAME, NAMESPACE};
use std::str::FromStr;
use strum::IntoEnumIterator;
use syn::spanned::Spanned;

/// Field (i.e. `name: String`) of a container (struct) and its parsed attributes
/// i.e. `#[cache_diff(rename = "Ruby version")]`
#[derive(Debug)]
pub(crate) struct ParseField {
    /// The proc-macro identifier for a field i.e. `name: String` would be a programatic
    /// reference to `name` that can be used along with `quote!` to produce code.
    pub(crate) ident: syn::Ident,
    /// What the user will see when this field differs and invalidates the cache
    /// i.e. `age: usize` will be `"age"`.
    pub(crate) name: String,
    /// Whether or not the field is included in the derived diff comparison
    pub(crate) ignore: bool,
    /// The function to use when rendering values on the field
    /// i.e. `age: 42` will be `"42"`
    pub(crate) display: syn::Path,
}

fn is_pathbuf(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "PathBuf" && segment.arguments == syn::PathArguments::None;
        }
    }
    false
}

impl ParseField {
    pub(crate) fn from_field(field: &syn::Field) -> Result<Self, syn::Error> {
        let ident = field.ident.clone().ok_or_else(|| {
            syn::Error::new(
                field.span(),
                format!("{MACRO_NAME} can only be used on structs with named fields"),
            )
        })?;

        let attributes = ParseAttribute::from_field(field)?;
        let name = attributes
            .iter()
            .filter_map(|attribute| match attribute {
                ParseAttribute::rename(name) => Some(name.to_owned()),
                _ => None,
            })
            .last()
            .unwrap_or_else(|| ident.to_string().replace("_", " "));
        let ignore = attributes
            .iter()
            .filter_map(|attribute| match attribute {
                ParseAttribute::ignore => Some(true),
                _ => None,
            })
            .last()
            .unwrap_or(false);
        let display = attributes
            .iter()
            .filter_map(|attribute| match attribute {
                ParseAttribute::display(display_fn) => Some(display_fn.to_owned()),
                _ => None,
            })
            .last()
            .unwrap_or_else(|| {
                if is_pathbuf(&field.ty) {
                    syn::parse_str("std::path::Path::display")
                        .expect("PathBuf::display parses as a syn::Path")
                } else {
                    syn::parse_str("std::convert::identity")
                        .expect("std::convert::identity parses as a syn::Path")
                }
            });

        Ok(ParseField {
            ident,
            name,
            ignore,
            display,
        })
    }
}

/// An single field attribute
#[derive(strum::EnumDiscriminants, Debug, PartialEq)]
#[strum_discriminants(derive(strum::EnumIter, strum::Display, strum::EnumString))]
#[strum_discriminants(name(KnownAttribute))]
enum ParseAttribute {
    #[allow(non_camel_case_types)]
    rename(String), // #[cache_diff(rename="...")]
    #[allow(non_camel_case_types)]
    display(syn::Path), // #[cache_diff(display=<function>)]
    #[allow(non_camel_case_types)]
    ignore, // #[cache_diff(ignore)]
}

impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity: syn::Ident = input.parse()?;
        KnownAttribute::from_str(&identity.to_string()).map_err(|_| {
            syn::Error::new(
                identity.span(),
                format!(
                    "Unknown {NAMESPACE} attribute: `{identity}`.Must be one of {valid_keys}",
                    valid_keys = KnownAttribute::iter()
                        .map(|key| format!("`{key}`"))
                        .collect::<Vec<String>>()
                        .join(", ")
                ),
            )
        })
    }
}

impl syn::parse::Parse for ParseAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key: KnownAttribute = input.parse()?;

        match key {
            KnownAttribute::rename => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParseAttribute::rename(
                    input.parse::<syn::LitStr>()?.value(),
                ))
            }
            KnownAttribute::display => {
                input.parse::<syn::Token![=]>()?;
                Ok(ParseAttribute::display(input.parse()?))
            }
            KnownAttribute::ignore => Ok(ParseAttribute::ignore),
        }
    }
}

impl ParseAttribute {
    fn from_field(field: &syn::Field) -> Result<Vec<ParseAttribute>, syn::Error> {
        let mut attributes = Vec::new();
        for attr in field
            .attrs
            .clone()
            .into_iter()
            .filter(|attr| attr.path().is_ident(NAMESPACE))
        {
            for attribute in attr.parse_args_with(
                syn::punctuated::Punctuated::<ParseAttribute, syn::Token![,]>::parse_terminated,
            )? {
                attributes.push(attribute)
            }
        }

        Ok(attributes)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use syn::parse::Parse;

    #[test]
    fn test_parse_rename_attribute() {
        let attribute: syn::Attribute = syn::parse_quote! {
            #[cache_diff(rename="Ruby version")]
        };

        assert_eq!(
            ParseAttribute::rename("Ruby version".to_string()),
            attribute.parse_args_with(ParseAttribute::parse).unwrap()
        );
    }

    #[test]
    fn test_parse_rename_ignore_attribute() {
        let field: syn::Field = syn::parse_quote! {
            #[cache_diff(rename="Ruby version", ignore)]
            name: String
        };

        assert_eq!(
            vec![
                ParseAttribute::rename("Ruby version".to_string()),
                ParseAttribute::ignore,
            ],
            ParseAttribute::from_field(&field).unwrap()
        );
    }

    #[test]
    fn test_parse_field_rename_ignore_attribute() {
        let field: syn::Field = syn::parse_quote! {
            #[cache_diff(rename="Ruby version", ignore)]
            name: String
        };

        let ParseField {
            ident: _,
            name,
            ignore,
            display,
        } = ParseField::from_field(&field).unwrap();

        assert_eq!("Ruby version".to_string(), name);
        assert!(ignore);
        assert_eq!(
            syn::parse_str::<syn::Path>("std::convert::identity").unwrap(),
            display
        );
    }
}
