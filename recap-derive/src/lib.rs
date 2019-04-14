extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident, Lit, Meta, NestedMeta};

#[proc_macro_derive(Recap, attributes(recap))]
pub fn derive_recap(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    let regex = extract_regex(&item).expect(
        r#"Unable to resolve recap regex.
            Make sure your structure has declared a attribute in the form:
            #[derive(Deserialize, Recap)]
            #[recap(regex ="your-pattern-here")]
            struct YourStruct { ... }
            "#,
    );
    let item_ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let impl_inner = quote! {
        impl #impl_generics std::str::FromStr for #item_ident #ty_generics #where_clause {
            type Err = recap::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                recap::lazy_static! {
                    static ref RE: recap::Regex = recap::Regex::new(#regex)
                        .expect("Failed to compile regex");
                }

                Ok(recap::from_captures(&RE, s)?)
            }
        }
    };

    let injector = Ident::new(
        &format!("IMPL_FROMSTR_FOR_{}", item.ident.to_string()),
        Span::call_site(),
    );

    let out = quote! {
        const #injector: () = {
            extern crate recap;
            #impl_inner
        };
    };

    out.into()
}

fn extract_regex(item: &DeriveInput) -> Option<String> {
    item.attrs
        .iter()
        .flat_map(syn::Attribute::parse_meta)
        .filter_map(|x| match x {
            Meta::List(y) => Some(y),
            _ => None,
        })
        .filter(|x| x.ident == "recap")
        .flat_map(|x| x.nested.into_iter())
        .filter_map(|x| match x {
            NestedMeta::Meta(y) => Some(y),
            _ => None,
        })
        .filter_map(|x| match x {
            Meta::NameValue(y) => Some(y),
            _ => None,
        })
        .find(|x| x.ident == "regex")
        .and_then(|x| match x.lit {
            Lit::Str(y) => Some(y.value()),
            _ => None,
        })
}
