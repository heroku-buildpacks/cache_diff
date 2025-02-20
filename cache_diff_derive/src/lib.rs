use cache_diff_container::CacheDiffContainer;
use cache_diff_field::ActiveField;
use parse_container::ParseContainer;
use parse_field::ParseField;
use proc_macro::TokenStream;
use syn::DeriveInput;

mod cache_diff_container;
mod cache_diff_field;
mod parse_container;
mod parse_field;

pub(crate) const NAMESPACE: &str = "cache_diff";
pub(crate) const MACRO_NAME: &str = "CacheDiff";

#[proc_macro_derive(CacheDiff, attributes(cache_diff))]
pub fn cache_diff(item: TokenStream) -> TokenStream {
    create_cache_diff_too(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn create_cache_diff_too(item: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let ast: DeriveInput = syn::parse2(item).unwrap();
    let container = ParseContainer::from_derive_input(&ast)?;
    let struct_identifier = &container.ident;

    let mut comparisons = Vec::new();
    for field in container.fields.iter().filter(|f| !f.ignore) {
        let ParseField {
            ident,
            name,
            ignore: _,
            display,
        } = field;

        comparisons.push(quote::quote! {
            if self.#ident != old.#ident {
                differences.push(
                    format!("{name} ({old} to {new})",
                        name = #name,
                        old = self.fmt_value(&#display(&old.#ident)),
                        new = self.fmt_value(&#display(&self.#ident))
                    )
                );
            }
        });
    }

    Ok(quote::quote! {
        impl cache_diff::CacheDiff for #struct_identifier {
            fn diff(&self, old: &Self) -> ::std::vec::Vec<String> {
                let mut differences = ::std::vec::Vec::new();
                #(#comparisons)*
                differences
            }
        }
    })
}

fn create_cache_diff(item: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let ast: DeriveInput = syn::parse2(item).unwrap();
    let container = CacheDiffContainer::from_ast(&ast)?;
    let struct_identifier = &container.identifier;

    let custom_diff = if let Some(ref custom_fn) = container.custom {
        quote::quote! {
            let custom_diff = #custom_fn(old, self);
            for diff in &custom_diff {
                differences.push(diff.to_string())
            }
        }
    } else {
        quote::quote! {}
    };

    let mut comparisons = Vec::new();
    for f in container.fields.iter() {
        let ActiveField {
            name,
            display_fn,
            field_identifier,
        } = f;
        comparisons.push(quote::quote! {
            if self.#field_identifier != old.#field_identifier {
                differences.push(
                    format!("{name} ({old} to {new})",
                        name = #name,
                        old = self.fmt_value(&#display_fn(&old.#field_identifier)),
                        new = self.fmt_value(&#display_fn(&self.#field_identifier))
                    )
                );
            }
        });
    }

    Ok(quote::quote! {
        impl cache_diff::CacheDiff for #struct_identifier {
            fn diff(&self, old: &Self) -> ::std::vec::Vec<String> {
                let mut differences = ::std::vec::Vec::new();
                #custom_diff
                #(#comparisons)*
                differences
            }
        }
    })
}
