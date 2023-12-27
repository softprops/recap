extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use syn::{
    parse_macro_input, Data::Struct, DataStruct, DeriveInput, Fields, Ident, Lit, Meta,
    MetaNameValue, NestedMeta,
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

    let field_options = extract_field_options_tokens(&item);
    let static_recap_data = quote! {
        recap::lazy_static! {
            static ref RE: recap::Regex = recap::Regex::new(#regex)
                .expect("Failed to compile regex");
            static ref FIELD_OPTIONS: std::collections::HashMap<String, recap::FieldOptions> =
                #field_options
                .into_iter()
                .collect();
        }
    };

    let has_lifetimes = item.generics.lifetimes().count() > 0;
    let impl_from_str = if !has_lifetimes {
        quote! {
            impl #impl_generics std::str::FromStr for #item_ident #ty_generics #where_clause {
                type Err = recap::Error;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    recap::from_captures_with_options(&RE, s, Some(&FIELD_OPTIONS))
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
                recap::from_captures_with_options(&RE, s, Some(&FIELD_OPTIONS))
            }
        }
        #impl_from_str
    };

    let impl_matcher = quote! {
        impl #impl_generics  #item_ident #ty_generics #where_clause {
            /// Recap derived method. Returns true when some input text
            /// matches the regex associated with this type
            pub fn is_match(input: &str) -> bool {
                RE.is_match(input)
            }
        }
    };

    let injector = Ident::new(&format!("RECAP_IMPL_FOR_{}", item.ident), Span::call_site());

    let out = quote! {
        const #injector: () = {
            extern crate recap;
            #static_recap_data
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

fn get_nested_metas(attrs: &[syn::Attribute]) -> impl Iterator<Item = Meta> + '_ {
    attrs
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
}

fn extract_regex(item: &DeriveInput) -> Option<String> {
    get_nested_metas(&item.attrs)
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

/// The resulting tokens will be a (possibly empty) array of pairs in the
/// form of `[("field_name", FieldOptions { ... }), ...]`
fn extract_field_options_tokens(item: &DeriveInput) -> proc_macro2::TokenStream {
    let Struct(DataStruct {
        fields: Fields::Named(fields_named),
        ..
    }) = &item.data
    else {
        panic!("Recap regex can only be applied to Structs with named fields")
    };
    let field_name_options_pairs = fields_named.named.iter().filter_map(|named| {
        let name = named.ident.as_ref().unwrap().to_string();
        let options_tokens = get_nested_metas(&named.attrs)
            // This all probably would need to evolve if/when we ever need to handle more types
            // of attributes but it's probably fine for now?
            .map(|x| match x {
                Meta::NameValue(MetaNameValue {
                    path,
                    lit: Lit::Str(lit),
                    ..
                }) if path.is_ident("delimiter_regex") => {
                    // Validate the regex now
                    Regex::new(&lit.value()).unwrap_or_else(|_| {
                        panic!(
                            "invalid regex given to `delimiter_regex` for field {}",
                            name
                        )
                    });
                    quote! { #path: Some(recap::Regex::new(#lit).unwrap()) }
                }
                _ => panic!(r#"Expected attributes in the form of `delimiter_regex = "..."`"#),
            })
            .collect::<Vec<_>>();
        if options_tokens.is_empty() {
            None
        } else {
            Some(quote! {
                (#name.to_owned(), recap::FieldOptions {
                    #(#options_tokens),*
                })
            })
        }
    });

    quote! {
        [#(#field_name_options_pairs),*]
    }
}
