use crate::{MACRO_NAME, NAMESPACE};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    str::FromStr,
};

// Code
/// Parses one bare word like "rename" for any iterable enum, and that's it
///
/// Won't parse an equal sign or anything else
pub(crate) fn known_attribute<T>(identity: &syn::Ident) -> syn::Result<T>
where
    T: FromStr + strum::IntoEnumIterator + Display,
{
    let name_str = &identity.to_string();
    T::from_str(name_str).map_err(|_| {
        syn::Error::new(
            identity.span(),
            format!(
                "Unknown {NAMESPACE} attribute: `{identity}`. Must be one of {valid_keys}",
                valid_keys = T::iter()
                    .map(|key| format!("`{key}`"))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        )
    })
}

fn parse_attrs<T>(attrs: &[syn::Attribute]) -> Result<Vec<T>, syn::Error>
where
    T: syn::parse::Parse,
{
    let mut attributes = Vec::new();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident(NAMESPACE)) {
        for attribute in attr
            .parse_args_with(syn::punctuated::Punctuated::<T, syn::Token![,]>::parse_terminated)?
        {
            attributes.push(attribute)
        }
    }

    Ok(attributes)
}

/// Helper type for parsing a type and preserving the original span
///
/// Used with [syn::punctuated::Punctuated] to capture the inner span of an attribute.
#[derive(Debug)]
pub(crate) struct WithSpan<T>(pub(crate) T, pub(crate) proc_macro2::Span);

impl<T> WithSpan<T> {
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<T: syn::parse::Parse> syn::parse::Parse for WithSpan<T> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        Ok(WithSpan(input.parse()?, span))
    }
}

/// Parses all attributes and returns a lookup with the parsed value and span information where it was found
///
/// - Guarantees attributes are not duplicated
pub(crate) fn attribute_lookup<T>(
    attrs: &[syn::Attribute],
) -> Result<HashMap<T::Discriminant, WithSpan<T>>, syn::Error>
where
    T: strum::IntoDiscriminant + syn::parse::Parse + std::fmt::Debug,
    T::Discriminant: Eq + Display + std::hash::Hash + Copy,
{
    let mut seen = HashMap::new();
    let mut errors = VecDeque::new();

    let parsed_attributes = parse_attrs::<WithSpan<T>>(attrs)?;
    for attribute in parsed_attributes {
        let WithSpan(ref parsed, span) = attribute;
        let key = parsed.discriminant();
        if let Some(WithSpan(_, prior)) = seen.insert(key, attribute) {
            errors.push_back(syn::Error::new(
                span,
                format!("{MACRO_NAME} duplicate attribute: `{key}`"),
            ));
            errors.push_back(syn::Error::new(
                prior,
                format!("previously `{key}` defined here"),
            ));
        }
    }

    if let Some(mut error) = errors.pop_front() {
        for e in errors {
            error.combine(e);
        }
        Err(error)
    } else {
        Ok(seen)
    }
}

pub(crate) fn check_empty<T>(lookup: HashMap<T::Discriminant, WithSpan<T>>) -> syn::Result<()>
where
    T: strum::IntoDiscriminant,
    T::Discriminant: Display + std::hash::Hash,
{
    if lookup.is_empty() {
        Ok(())
    } else {
        let mut error = syn::Error::new(
            proc_macro2::Span::call_site(),
            "Internal error: The developer forgot to implement some logic",
        );
        for (key, WithSpan(_, span)) in lookup.into_iter() {
            error.combine(syn::Error::new(
                span,
                format!("Attribute `{key}` parsed but not used"),
            ));
        }
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    // Test use
    use super::*;
    use super::*;
    // Test code
    #[test]
    fn test_parse_attrs_vec_demo() {
        let field: syn::Field = syn::parse_quote! {
            #[cache_diff("Ruby version")]
            name: String
        };

        assert_eq!(
            vec![syn::parse_str::<syn::LitStr>(r#""Ruby version""#).unwrap()],
            parse_attrs::<syn::LitStr>(&field.attrs).unwrap()
        );
    }

    #[test]
    fn test_parse_attrs_with_span_vec_demo() {
        let field: syn::Field = syn::parse_quote! {
            #[cache_diff("Ruby version")]
            name: String
        };

        assert_eq!(
            &syn::parse_str::<syn::LitStr>(r#""Ruby version""#).unwrap(),
            parse_attrs::<WithSpan<syn::LitStr>>(&field.attrs)
                .unwrap()
                .into_iter()
                .map(WithSpan::into_inner)
                .collect::<Vec<_>>()
                .first()
                .unwrap()
        );
    }
}
