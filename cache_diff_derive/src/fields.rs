use crate::attributes::CacheDiffAttributes;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::Data::Struct;
use syn::Fields::Named;
use syn::{DataStruct, DeriveInput, Field, FieldsNamed, Ident, PathArguments};

/// Finalized state needed to construct a comparison
///
/// Represents a single field that may have macro attributes applied
/// such as:
///
/// ```txt
/// #[cache_diff(rename="Ruby version")]
/// version: String,
/// ```
struct CacheDiffField {
    field_identifier: Ident,
    name: String,
    display_fn: syn::Path,
}

impl CacheDiffField {
    fn new(field: &Field, attributes: CacheDiffAttributes) -> syn::Result<Option<Self>> {
        if attributes.ignore.is_some() {
            Ok(None)
        } else {
            let field_identifier = field.ident.clone().ok_or_else(|| {
                syn::Error::new(
                    field.span(),
                    "CacheDiff can only be used on structs with named fields",
                )
            })?;
            let name = attributes
                .rename
                .unwrap_or_else(|| field_identifier.to_string().replace("_", " "));
            let display_fn: syn::Path = attributes.display.unwrap_or_else(|| {
                if is_pathbuf(&field.ty) {
                    syn::parse_str("std::path::Path::display")
                        .expect("PathBuf::display parses as a syn::Path")
                } else {
                    syn::parse_str("std::convert::identity")
                        .expect("std::convert::identity parses as a syn::Path")
                }
            });

            Ok(Some(CacheDiffField {
                field_identifier,
                name,
                display_fn,
            }))
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

/// Represents a single attribute on a container AKA a struct
#[derive(Debug, Default)]
struct ContainerAttribute {
    custom: Option<syn::Path>,
}

/// Represents the Struct
struct Container {
    identifier: Ident,
    custom: Option<syn::Path>,
    fields: Punctuated<Field, Comma>,
}

impl Container {
    fn from_ast(input: &syn::DeriveInput) -> syn::Result<Self> {
        let identifier = input.ident.clone();
        let attrs = input.attrs.clone();

        let attributes = attrs
            .iter()
            .map(|attr| attr.parse_args_with(ContainerAttribute::parse))
            .collect::<syn::Result<Vec<ContainerAttribute>>>()?;

        if attributes.len() > 1 {
            return Err(syn::Error::new(
                input.attrs.last().span(),
                "Too many attributes",
            ));
        }

        let custom = attributes.into_iter().next().unwrap_or_default().custom;
        let fields = match input.data {
            Struct(DataStruct {
                fields: Named(FieldsNamed { ref named, .. }),
                ..
            }) => named,
            _ => unimplemented!("Only implemented for structs"),
        }
        .to_owned();

        Ok(Container {
            identifier,
            custom,
            fields,
        })
    }
}

impl Parse for ContainerAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();
        match name_str.as_ref() {
            "custom" => {
                input.parse::<syn::Token![=]>()?;
                Ok(ContainerAttribute { custom: Some(input.parse()?) })
            }
            _ => Err(syn::Error::new(
                name.span(),
                format!(
                    "Unknown cache_diff attribute on struct: `{name_str}`. Must be one of `custom = <function>`",
                ),
            )),
        }
    }
}

pub fn create_cache_diff(item: TokenStream) -> syn::Result<TokenStream> {
    let ast: DeriveInput = syn::parse2(item).unwrap();
    let container = Container::from_ast(&ast)?;
    let struct_identifier = container.identifier;

    let custom_diff = if let Some(custom_fn) = container.custom {
        quote! {
            let custom_diff = #custom_fn(old, self);
            for diff in &custom_diff {
                differences.push(diff.to_string())
            }
        }
    } else {
        quote! {}
    };

    let mut comparisons = Vec::new();
    for f in container.fields.iter() {
        let attributes = CacheDiffAttributes::from(f)?;
        let field = CacheDiffField::new(f, attributes)?;

        if let Some(CacheDiffField {
            field_identifier: field_ident,
            name,
            display_fn,
        }) = field
        {
            comparisons.push(quote! {
                if self.#field_ident != old.#field_ident {
                    differences.push(
                        format!("{name} ({old} to {now})",
                            name = #name,
                            old = self.fmt_value(&#display_fn(&old.#field_ident)),
                            now = self.fmt_value(&#display_fn(&self.#field_ident))
                        )
                    );
                }
            });
        }
    }

    if comparisons.is_empty() {
        Err(syn::Error::new(
            struct_identifier.span(),
            "No fields to compare for CacheDiff, ensure struct has at least one named field that isn't `cache_diff(ignore)`-d",
        ))
    } else {
        Ok(quote! {
            impl cache_diff::CacheDiff for #struct_identifier {
                fn diff(&self, old: &Self) -> Vec<String> {
                    let mut differences = Vec::new();
                    #custom_diff
                    #(#comparisons)*
                    differences
                }
            }
        })
    }
}
