use crate::{MACRO_NAME, NAMESPACE};
use std::{collections::HashMap, fmt::Display, str::FromStr};

/// Parses all attributes and returns a lookup with the parsed value and span information where it was found
///
/// - Guarantees attributes are not duplicated
pub(crate) fn attribute_lookup<T>(
    attrs: &[syn::Attribute],
) -> Result<HashMap<T::Discriminant, WithSpan<T>>, syn::Error>
where
    T: strum::IntoDiscriminant + syn::parse::Parse,
    T::Discriminant: Eq + Display + std::hash::Hash + Copy,
{
    let mut seen = HashMap::new();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident(NAMESPACE)) {
        for attribute_span in attr.parse_args_with(
            syn::punctuated::Punctuated::<WithSpan<T>, syn::Token![,]>::parse_terminated,
        )? {
            let WithSpan(parsed, span) = attribute_span;
            let key = parsed.discriminant();
            if let Some(WithSpan(_, prior)) = seen.insert(key, WithSpan(parsed, span)) {
                let mut error =
                    syn::Error::new(span, format!("{MACRO_NAME} duplicate attribute: `{key}`"));
                error.combine(syn::Error::new(
                    prior,
                    format!("previously `{key}` defined here"),
                ));
                return Err(error);
            }
        }
    }

    Ok(seen)
}

/// Parses one bare word like "rename" for any iterable enum and that's it
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
