extern crate proc_macro;

use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use syn::{
    parse_macro_input, Data::Enum, Data::Struct, DataEnum, DataStruct, DeriveInput, Fields, Ident,
    Lit, Meta, NestedMeta,
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
    let out = match regex {
        Regexes::StructRegex(regex) => {
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

            quote! {
                const #injector: () = {
                    extern crate recap;
                    #impl_inner
                    #impl_matcher
                };
            }
        }
        Regexes::EnumRegexes(regexes) => {
            let impl_from_str = if !has_lifetimes {
                let from_str_regexes = regexes.iter().map(|(variant_name, regex)| {
                    let regex_name_injector = Ident::new(
                        &format!("RE_{}", variant_name),
                        Span::call_site(),
                    );
                    quote! {
                            static ref #regex_name_injector: recap::Regex = recap::Regex::new(#regex)
                                .expect("Failed to compile regex");
                    }
                });

                let try_parse_regexes = regexes.iter().map(|(variant_name, regex)| {
                    let regex_name_injector =
                        Ident::new(&format!("RE_{}", variant_name), Span::call_site());
                    quote! {
                        if let Some(caps) = #regex_name_injector.captures(&s) {
                            return Ok(#variant_name {

                            })
                        }
                    }
                });

                quote! {
                    impl #impl_generics std::str::FromStr for #item_ident #ty_generics #where_clause {
                        type Err = recap::Error;
                        fn from_str(s: &str) -> Result<Self, Self::Err> {
                            recap::lazy_static! {
                                #(#from_str_regexes)*
                            }

                            #(#try_parse_regexes)*

                            panic!("AAAAHHHHH");
                        }
                    }
                }
            } else {
                quote! {}
            };

            let lifetimes = item.generics.lifetimes();
            let also_lifetimes = item.generics.lifetimes();
            let impl_inner = {
                let from_str_regexes = regexes.iter().map(|(variant_name, regex)| {
                    let regex_name_injector = Ident::new(
                        &format!("RE_{}", variant_name),
                        Span::call_site(),
                    );
                    quote! {
                            static ref #regex_name_injector: recap::Regex = recap::Regex::new(&#regex)
                                .expect("Failed to compile regex");
                    }
                });

                let try_parse_regexes = regexes.keys().map(|variant_name| {
                    let regex_name_injector =
                        Ident::new(&format!("RE_{}", variant_name), Span::call_site());
                    quote! {
                        match recap::from_captures(&#regex_name_injector, s) {
                            Ok(value) => {return Ok(value)},
                            Err(e) => {
                                panic!("{}", e);
                            }
                        };
                    }
                });

                quote! {
                    impl #impl_generics std::convert::TryFrom<& #(#lifetimes)* str> for #item_ident #ty_generics #where_clause {
                        type Error = recap::Error;
                        fn try_from(s: & #(#also_lifetimes)* str) -> Result<Self, Self::Error> {
                            recap::lazy_static! {
                                #(#from_str_regexes)*
                            }
                            #(#try_parse_regexes)*
                            panic!("AAAAHHHHH");
                        }
                    }
                    #impl_from_str
                }
            };
            let impl_matcher = {
                let from_str_regexes = regexes.iter().map(|(variant_name, regex)| {
                    let regex_name_injector = Ident::new(
                        &format!("RE_{}", variant_name),
                        Span::call_site(),
                    );
                    quote! {
                            static ref #regex_name_injector: recap::Regex = recap::Regex::new(#regex)
                                .expect("Failed to compile regex");
                    }
                });

                let matchers = regexes.iter().map(|(variant_name, regex)| {
                    let regex_name_injector =
                        Ident::new(&format!("RE_{}", variant_name), Span::call_site());
                    quote! {
                        if #regex_name_injector.is_match(input) {
                            return true;
                        };
                    }
                });

                quote! {
                impl #impl_generics  #item_ident #ty_generics #where_clause {
                    /// Recap derived method. Returns true when some input text
                    /// matches the regex associated with this type
                    pub fn is_match(input: &str) -> bool {
                            recap::lazy_static! {
                                #(#from_str_regexes)*
                            }
                            #(#matchers)*
                            false
                        }
                    }
                }
            };

            let injector = Ident::new(
                &format!("RECAP_IMPL_FOR_{}", item.ident.to_string()),
                Span::call_site(),
            );

            quote! {
                const #injector: () = {
                    extern crate recap;
                    #impl_inner
                    #impl_matcher
                };
            }
        }
        _ => panic!("Made it this far"),
    };

    out.into()
}

enum Regexes {
    StructRegex(String),
    EnumRegexes(HashMap<String, String>),
}

fn validate(
    item: &DeriveInput,
    regex_container: &Regexes,
) {
    match regex_container {
        Regexes::StructRegex(regex) => {
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
                _ => {
                    panic!("Recap regex can only be applied to Structs and Enums with named fields")
                }
            };
            if caps != fields {
                panic!(
            "Recap could not derive a `FromStr` impl for `{}`.\n\t\t > Expected regex with {} named capture groups to align with struct fields but found {}",
            item.ident, fields, caps
        );
            }
        }
        Regexes::EnumRegexes(regexes) => {
            match &item.data {
                Enum(DataEnum { variants, .. }) => {
                    for variant in variants {
                        let variant_name = format!("{}", variant.ident);
                        match regexes.get(&variant_name) {
                            Some(regex) => {
                                let regex = Regex::new(regex).unwrap_or_else(|err| {
                                    panic!(
                                        "Invalid regular expression provided for `{}`\n{}",
                                        &item.ident, err
                                    )
                                });
                                let caps = regex.capture_names().flatten().count();
                                let fields = variant.fields.len();
                                if caps != fields {
                                    panic!(
            "Recap could not derive a `FromStr` impl for `{}`.\n\t\t > Expected regex with {} named capture groups to align with struct fields but found {}",
            item.ident, fields, caps
        );
                                }
                            }
                            None => panic!("Recap regex missing on enum variant {}", variant_name),
                        }
                    }
                }
                _ => {
                    panic!("Recap regex can only be applied to Structs and Enums with named fields")
                }
            };
        }
    }
}

fn extract_regex(item: &DeriveInput) -> Option<Regexes> {
    match &item.data {
        Struct(DataStruct {
            fields: Fields::Named(fs),
            ..
        }) => item
            .attrs
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
                Lit::Str(y) => Some(Regexes::StructRegex(y.value())),
                _ => None,
            }),
        Enum(DataEnum {
            enum_token,
            brace_token,
            variants,
        }) => {
            let mut regexes: HashMap<String, String> = HashMap::new();
            for variant in variants {
                let variant_name = format!("{}", variant.ident);
                for attr in variant.attrs.iter() {
                    if attr.path.is_ident("recap") {
                        let meta = attr.parse_meta().unwrap();
                        if let Meta::List(meta_list) = meta {
                            for nested_meta in meta_list.nested {
                                if let syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) = nested_meta
                                {
                                    if nv.path.is_ident("regex") {
                                        if let syn::Lit::Str(lit_str) = nv.lit {
                                            regexes.insert(variant_name.clone(), lit_str.value());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(Regexes::EnumRegexes(regexes))
        }
        _ => None,
    }
}
