extern crate proc_macro;

use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use syn::{
    parse_macro_input, Data::Enum, Data::Struct, DataEnum, DataStruct, DeriveInput, Fields,
    FieldsUnnamed, Ident, Lit, Meta, NestedMeta, Variant,
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
            let data_enum = match item.data {
                Enum(data_enum) => data_enum,
                _ => panic!("expected Enum"),
            };

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

                let try_parse_regexes = data_enum.variants.iter().map(|variant| {
                    let variant_name = &variant.ident;
                    let name = &item.ident;
                    let regex_name_injector = Ident::new(
                        &format!("RE_{}", variant_name),
                        Span::call_site(),
                    );
                    match &variant.fields {
                        Fields::Named(fields) => {
                            let fields: Vec<Ident> = fields.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                            quote! {
                                if let Some(caps) = #regex_name_injector.captures(&s) {
                                    return Ok(#name::#variant_name {
                                        #(#fields: caps.name(stringify!(#fields)).unwrap().as_str().try_into().unwrap(),)*
                                    })
                                }
                            }
                        }
                        Fields::Unnamed(_) => {
                            quote! {
                                if let Some(caps) = #regex_name_injector.captures(&s) {
                                    let inner = caps.get(1).unwrap().as_str();
                                    if let Ok(value) = inner.parse() {
                                        return Ok(#name::#variant_name(value))
                                    }
                                }
                            }
                        }
                        Fields::Unit => {
                            quote! {
                                if #regex_name_injector.is_match(&s) {
                                    return Ok(#name::#variant_name)
                                }
                            }
                        }
                }});

                quote! {
                    impl #impl_generics std::str::FromStr for #item_ident #ty_generics #where_clause {
                        type Err = recap::Error;
                        fn from_str(s: &str) -> Result<Self, Self::Err> {
                            recap::lazy_static! {
                                #(#from_str_regexes)*
                            }

                            #(#try_parse_regexes)*

                            Err(Self::Err::Custom("Uh Oh".to_string()))
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

                let try_parse_regexes = data_enum.variants.iter().map(|variant| {
                    let variant_name = &variant.ident;
                    let name = &item.ident;
                    let regex_name_injector = Ident::new(
                        &format!("RE_{}", variant_name),
                        Span::call_site(),
                    );
                    match &variant.fields {
                        Fields::Named(fields) => {
                            let fields: Vec<Ident> = fields.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                            quote! {
                                if let Some(caps) = #regex_name_injector.captures(&s) {
                                    return Ok(#name::#variant_name {
                                        #(#fields: caps.name(stringify!(#fields)).unwrap().as_str().try_into().unwrap(),)*
                                    })
                                }
                            }
                        }
                        Fields::Unnamed(_) => {
                            quote! {
                                if let Some(caps) = #regex_name_injector.captures(&s) {
                                    let inner = caps.get(1).unwrap().as_str();
                                    if let Ok(value) = inner.parse() {
                                        return Ok(#name::#variant_name(value))
                                    }
                                }
                            }
                        }
                        Fields::Unit => {
                            quote! {
                                if #regex_name_injector.is_match(&s) {
                                    return Ok(#name::#variant_name)
                                }
                            }
                        }
                }});

                quote! {
                    impl #impl_generics std::convert::TryFrom<& #(#lifetimes)* str> for #item_ident #ty_generics #where_clause {
                        type Error = recap::Error;
                        fn try_from(s: & #(#also_lifetimes)* str) -> Result<Self, Self::Error> {
                            recap::lazy_static! {
                                #(#from_str_regexes)*
                            }
                            #(#try_parse_regexes)*

                            Err(Self::Error::Custom("Uh Oh".to_string()))
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
                        let regex =
                            Regex::new(regexes.get(&variant_name).unwrap()).unwrap_or_else(|err| {
                                panic!(
                                    "Invalid regular expression provided for `{}`\n{}",
                                    &item.ident, err
                                )
                            });
                        match &variant.fields {
                            Fields::Named(_) | Fields::Unnamed(_) => {
                                let caps = regex.capture_names().flatten().count();
                                let fields = variant.fields.len();
                                if caps != fields {
                                    panic!(
                                        "Recap could not derive a `FromStr` impl for `{}`.\n\t\t > Expected regex with {} named capture groups to align with struct fields but found {}",
                                        item.ident, fields, caps
                                    );
                                }
                            }
                            Fields::Unit => {}
                        };
                    }
                }
                _ => {
                    panic!("Recap regex can only be applied to Structs and Enums with named fields")
                }
            };
        }
    }
}

fn extract_struct_regex(item: &DeriveInput) -> Option<String> {
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

fn extract_enum_regexes(data_enum: &DataEnum) -> HashMap<String, String> {
    data_enum
        .variants
        .iter()
        .map(|variant| {
            let regex = variant
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
                    Lit::Str(y) => Some(y.value()),
                    _ => None,
                })
                .unwrap();
            (format!("{}", variant.ident), regex)
        })
        .collect()
}

fn extract_regex(item: &DeriveInput) -> Option<Regexes> {
    match &item.data {
        Struct(_) => extract_struct_regex(item).map(Regexes::StructRegex),
        Enum(data_enum) => Some(Regexes::EnumRegexes(extract_enum_regexes(data_enum))),
        _ => None,
    }
}
