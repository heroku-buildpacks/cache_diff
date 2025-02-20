use crate::parse_field::ParseField;
use crate::{MACRO_NAME, NAMESPACE};

/// Container (i.e. struct Metadata { ... }) and its parsed attributes
/// i.e. `#[cache_diff( ... )]`
#[derive(Debug)]
pub(crate) struct ParseContainer {
    /// The proc-macro identifier for a container i.e. `struct Metadata { }` would be a programatic
    /// reference to `Metadata` that can be used along with `quote!` to produce code.
    pub(crate) ident: syn::Ident,
    /// Fields (i.e. `name: String`) and their associated attributes i.e. `#[cache_diff(...)]`
    pub(crate) fields: Vec<ParseField>,
}

impl ParseContainer {
    pub(crate) fn from_derive_input(input: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let ident = input.ident.clone();

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

        if fields.is_empty() || fields.iter().all(|f| f.ignore) {
            Err(syn::Error::new(ident.span(), format!("No fields to compare for {MACRO_NAME}, ensure struct has at least one named field that isn't `{NAMESPACE}(ignore)`")))
        } else {
            Ok(ParseContainer { ident, fields })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
