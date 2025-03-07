use crate::{
    shared::{attribute_lookup, known_attribute, WithSpan},
    MACRO_NAME, NAMESPACE,
};
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
    pub(crate) ignore: Option<String>,
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

        let mut lookup = attribute_lookup(&field.attrs)?;
        let name = lookup
            .remove(&KnownAttribute::rename)
            .map(WithSpan::into_inner)
            .map(|parsed| match parsed {
                ParseAttribute::rename(inner) => inner,
                _ => unreachable!(),
            })
            .unwrap_or_else(|| ident.to_string().replace("_", " "));
        let display = lookup
            .remove(&KnownAttribute::display)
            .map(WithSpan::into_inner)
            .map(|parsed| match parsed {
                ParseAttribute::display(inner) => inner,
                _ => unreachable!(),
            })
            .unwrap_or_else(|| {
                if is_pathbuf(&field.ty) {
                    syn::parse_str("std::path::Path::display")
                        .expect("PathBuf::display parses as a syn::Path")
                } else {
                    syn::parse_str("std::convert::identity")
                        .expect("std::convert::identity parses as a syn::Path")
                }
            });
        let ignore = lookup
            .remove(&KnownAttribute::ignore)
            .map(WithSpan::into_inner)
            .map(|parsed| match parsed {
                ParseAttribute::ignore(inner) => inner,
                _ => unreachable!(),
            });

        Ok(ParseField {
            ident,
            name,
            ignore,
            display,
        })
    }
}

/// A single attribute
#[derive(strum::EnumDiscriminants, Debug, PartialEq)]
#[strum_discriminants(
    name(KnownAttribute),
    derive(strum::EnumIter, strum::Display, strum::EnumString, Hash)
)]
enum ParseAttribute {
    #[allow(non_camel_case_types)]
    rename(String), // #[cache_diff(rename="...")]
    #[allow(non_camel_case_types)]
    display(syn::Path), // #[cache_diff(display=<function>)]
    #[allow(non_camel_case_types)]
    ignore(String), // #[cache_diff(ignore)]
}

impl syn::parse::Parse for KnownAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let identity = input.parse::<syn::Ident>()?;
        known_attribute(&identity).map_err(|mut err| {
            if identity == "custom" {
                err.combine(syn::Error::new(
                    identity.span(),
                    format!(
                    "\nThe {NAMESPACE} attribute `custom` is available on the struct, not the field"
                ),
                ))
            };
            err
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
            KnownAttribute::ignore => {
                if input.peek(syn::Token![=]) {
                    input.parse::<syn::Token![=]>()?;
                    Ok(ParseAttribute::ignore(
                        input.parse::<syn::LitStr>()?.value(),
                    ))
                } else {
                    Ok(ParseAttribute::ignore("default".to_string()))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use syn::parse::Parse;

    #[test]
    fn lol() {
        let attribute: syn::Attribute = syn::parse_quote! {
            #[cache_diff(rename="Ruby version", rename = "oops")]
        };

        let result = attribute_lookup::<ParseAttribute>(&[attribute]);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            "CacheDiff duplicate attribute: `rename`".to_string(),
            format!("{}", result.err().unwrap())
        );
    }

    #[test]
    fn test_known_attributes() {
        let parsed: KnownAttribute = syn::parse_str("rename").unwrap();
        assert_eq!(KnownAttribute::rename, parsed);

        let parsed: KnownAttribute = syn::parse_str("ignore").unwrap();
        assert_eq!(KnownAttribute::ignore, parsed);

        let parsed: KnownAttribute = syn::parse_str("display").unwrap();
        assert_eq!(KnownAttribute::display, parsed);

        let result: Result<KnownAttribute, syn::Error> = syn::parse_str("unknown");
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"Unknown cache_diff attribute: `unknown`. Must be one of `rename`, `display`, `ignore`"#
        );
    }

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

        let mut lookup = attribute_lookup::<ParseAttribute>(&field.attrs).unwrap();
        assert_eq!(
            lookup.remove(&KnownAttribute::rename).unwrap().into_inner(),
            ParseAttribute::rename("Ruby version".to_string())
        );

        assert_eq!(
            lookup.remove(&KnownAttribute::ignore).unwrap().into_inner(),
            ParseAttribute::ignore("default".to_string())
        );
    }

    #[test]
    fn test_requires_named_struct() {
        let field: syn::Field = syn::parse_quote! {()};

        let result = ParseField::from_field(&field);
        assert!(result.is_err(), "Expected an error, got {:?}", result);
        assert_eq!(
            format!("{}", result.err().unwrap()),
            r#"CacheDiff can only be used on structs with named fields"#
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
        assert!(ignore.is_some());
        assert_eq!(
            syn::parse_str::<syn::Path>("std::convert::identity").unwrap(),
            display
        );
    }
}
