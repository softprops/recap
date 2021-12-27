extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use syn::{
    parse_macro_input, Data::Struct, DataStruct, DeriveInput, Fields, Ident, Lit, Meta, NestedMeta,
};

#[proc_macro_derive(Recap, attributes(recap))]
pub fn derive_recap(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as DeriveInput);
    let regex = extract_regex(&item).expect(
        r#"Unable to resolve recap regex.
            Make sure your structure has declared an attribute in the form:
            #[derive(Deserialize, Recap)]
            #[recap(regex ="your-pattern-here")]
            struct YourStruct { ... }
            "#,
    );

    validate(&item, &regex);

    let item_ident = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let has_lifetimes = item.generics.lifetimes().count() > 0;
    let impl_from_str = if !has_lifetimes {
        quote! {
            impl #impl_generics std::str::FromStr for #item_ident #ty_generics #where_clause {
                type Err = recap::Error;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    recap::lazy_static! {
                        static ref RE: recap::Regex = recap::Regex::new(#regex)
                            .expect("Failed to compile regex");
                    }

                    recap::from_captures(&RE, s)
                }
            }
        }
    } else {
        quote! {}
    };

    let lifetimes = item.generics.lifetimes();
    let also_lifetimes = item.generics.lifetimes();
    let impl_inner = quote! {
        impl #impl_generics std::convert::TryFrom<& #(#lifetimes)* str> for #item_ident #ty_generics #where_clause {
            type Error = recap::Error;
            fn try_from(s: & #(#also_lifetimes)* str) -> Result<Self, Self::Error> {
                recap::lazy_static! {
                    static ref RE: recap::Regex = recap::Regex::new(#regex)
                        .expect("Failed to compile regex");
                }

                recap::from_captures(&RE, s)
            }
        }
        #impl_from_str
    };

    let impl_matcher = quote! {
        impl #impl_generics  #item_ident #ty_generics #where_clause {
            /// Recap derived method. Returns true when some input text
            /// matches the regex associated with this type
            pub fn is_match(input: &str) -> bool {
                recap::lazy_static! {
                    static ref RE: recap::Regex = recap::Regex::new(#regex)
                        .expect("Failed to compile regex");
                }
                RE.is_match(input)
            }
        }
    };

    let injector = Ident::new(
        &format!("RECAP_IMPL_FOR_{}", item.ident.to_string()),
        Span::call_site(),
    );

    let out = quote! {
        const #injector: () = {
            extern crate recap;
            #impl_inner
            #impl_matcher
        };
    };

    out.into()
}

fn validate(
    item: &DeriveInput,
    regex: &str,
) {
    let regex = Regex::new(regex).unwrap_or_else(|err| {
        panic!(
            "Invalid regular expression provided for `{}`\n{}",
            &item.ident, err
        )
    });
    let caps = regex.capture_names().flatten().count();
    let fields = match &item.data {
        Struct(DataStruct {
            fields: Fields::Named(fs),
            ..
        }) => fs.named.len(),
        _ => panic!("Recap regex can only be applied to Structs with named fields"),
    };
    if caps != fields {
        panic!(
            "Recap could not derive a `FromStr` impl for `{}`.\n\t\t > Expected regex with {} named capture groups to align with struct fields but found {}",
            item.ident, fields, caps
        );
    }
}

fn extract_regex(item: &DeriveInput) -> Option<String> {
    item.attrs
        .iter()
        .flat_map(syn::Attribute::parse_meta)
        .filter_map(|x| match x {
            Meta::List(y) => Some(y),
            _ => None,
        })
        .filter(|x| x.path.is_ident("recap"))
        .flat_map(|x| x.nested.into_iter())
        .filter_map(|x| match x {
            NestedMeta::Meta(y) => Some(y),
            _ => None,
        })
        .filter_map(|x| match x {
            Meta::NameValue(y) => Some(y),
            _ => None,
        })
        .find(|x| x.path.is_ident("regex"))
        .and_then(|x| match x.lit {
            Lit::Str(y) => Some(y.value()),
            _ => None,
        })
}
