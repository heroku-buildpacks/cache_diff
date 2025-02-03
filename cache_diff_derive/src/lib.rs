use cache_diff_container::CacheDiffContainer;
use cache_diff_field::ActiveField;
use proc_macro::TokenStream;
use syn::DeriveInput;

mod cache_diff_container;
mod cache_diff_field;

#[proc_macro_derive(CacheDiff, attributes(cache_diff))]
pub fn cache_diff(item: TokenStream) -> TokenStream {
    create_cache_diff(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
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
                    format!("{name} ({old} to {now})",
                        name = #name,
                        old = self.fmt_value(&#display_fn(&old.#field_identifier)),
                        now = self.fmt_value(&#display_fn(&self.#field_identifier))
                    )
                );
            }
        });
    }

    Ok(quote::quote! {
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
