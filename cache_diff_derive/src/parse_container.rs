use crate::parse_field::ParseField;
use crate::{MACRO_NAME, NAMESPACE};
use std::str::FromStr;
use strum::IntoEnumIterator;

/// Container (i.e. struct Metadata { ... }) and its parsed attributes
/// i.e. `#[cache_diff( ... )]`
#[derive(Debug)]
pub(crate) struct ParseContainer {
    /// The proc-macro identifier for a container i.e. `struct Metadata { }` would be a programatic
    /// reference to `Metadata` that can be used along with `quote!` to produce code.
    pub(crate) ident: syn::Ident,
    /// An optional path to a custom diff function
    /// Set via attribute on the container (#[cache_diff(custom = <function>)])
    pub(crate) custom: Option<syn::Path>,
    /// Fields (i.e. `name: String`) and their associated attributes i.e. `#[cache_diff(...)]`
    pub(crate) fields: Vec<ParseField>,
}

impl ParseContainer {
    pub(crate) fn from_derive_input(input: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let ident = input.ident.clone();
        let attributes = ParseAttribute::from_derive(input)?;
        let custom = attributes
            .into_iter()
            .map(|attribute| match attribute {
                ParseAttribute::custom(path) => path,
            })
            .last();
        let fields = match input.data {
            syn::Data::Struct(syn::DataStruct {
                fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
                ..
            }) => named,
            _ => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("{MACRO_NAME} can only be used on named structs"),
                ))
            }
        }
        .into_iter()
        .map(ParseField::from_field)
        .collect::<Result<Vec<ParseField>, syn::Error>>()?;

        if let Some(field) = fields
            .iter()
            .find(|field| matches!(field.ignore.as_deref(), Some("custom")))
        {
            if custom.is_none() {
                return Err(syn::Error::new(ident.span(),
                            format!(
                                "field `{field}` on {container} marked ignored as custom, but no `#[{NAMESPACE}(custom = <function>)]` found on `{container}`",
                                field = field.ident,
                                container = &ident,
                            )));
            }
        }

        if fields.iter().any(|f| f.ignore.is_none()) {
            Ok(ParseContainer {
                ident,
                fields,
                custom,
            })
        } else {
            Err(syn::Error::new(ident.span(), format!("No fields to compare for {MACRO_NAME}, ensure struct has at least one named field that isn't `{NAMESPACE}(ignore)`")))
        }
    }
}

/// An single field attribute
#[derive(strum::EnumDiscriminants, Debug, PartialEq)]
#[strum_discriminants(derive(strum::EnumIter, strum::Display, strum::EnumString))]
#[strum_discriminants(name(KnownAttribute))]
enum ParseAttribute {
    #[allow(non_camel_case_types)]
    custom(syn::Path), // #[cache_diff(custom=<function>)]
}

impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity: syn::Ident = input.parse()?;
        KnownAttribute::from_str(&identity.to_string()).map_err(|_| {
            syn::Error::new(
                identity.span(),
                format!(
                    "Unknown {NAMESPACE} attribute: `{identity}`. Must be one of {valid_keys}",
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
        input.parse::<syn::Token![=]>()?;
        match key {
            KnownAttribute::custom => Ok(ParseAttribute::custom(input.parse()?)),
        }
    }
}

impl ParseAttribute {
    fn from_derive(input: &syn::DeriveInput) -> Result<Vec<ParseAttribute>, syn::Error> {
        let mut attributes = Vec::new();
        for attr in input
            .attrs
            .iter()
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

    #[test]
    fn test_known_attributes() {
        let attribute: KnownAttribute = syn::parse_str("custom").unwrap();
        assert_eq!(KnownAttribute::custom, attribute);
    }

    #[test]
    fn test_parse_attribute() {
        let attribute: ParseAttribute = syn::parse_str("custom = my_function").unwrap();
        assert!(matches!(attribute, ParseAttribute::custom(_)));

        let result: Result<ParseAttribute, syn::Error> = syn::parse_str("unknown");
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            r"Unknown cache_diff attribute: `unknown`. Must be one of `custom`",
            format!("{}", result.err().unwrap()),
        );
    }

    #[test]
    fn test_custom_parse_attribute() {
        let input: syn::DeriveInput = syn::parse_quote! {
            #[cache_diff(custom = my_function)]
            struct Metadata {
                name: String
            }

        };

        assert!(matches!(
            ParseAttribute::from_derive(&input).unwrap().first(),
            Some(ParseAttribute::custom(_))
        ));
    }

    #[test]
    fn test_parses() {
        let container = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata {
                version: String
            }
        })
        .unwrap();
        assert_eq!(1, container.fields.len());

        let container = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata {
                version: String,
                checksum: String
            }
        })
        .unwrap();
        assert_eq!(2, container.fields.len());
    }

    #[test]
    fn test_no_fields() {
        let result = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata { }
        });
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`"#
        );
    }

    #[test]
    fn test_all_ignored() {
        let result = ParseContainer::from_derive_input(&syn::parse_quote! {
            struct Metadata {
                #[cache_diff(ignore)]
                version: String
            }
        });
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`"#
        );
    }
}
