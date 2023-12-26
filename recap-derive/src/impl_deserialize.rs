use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Data::Struct, DeriveInput, Meta};

/// Takes over implementing [serde::Deserialize] for the struct we're deriving Recap on.
///
/// The main motivation here is to allow nested uses of Recap-based deserialization, namely teaching
/// the Deserialize implementation (or more specifically the Visitor implementation within that)
/// how to handle a string.
///
/// We can't just add an extra `visit_str` implementation to the existing code that gets generated
/// by `#[derive(Deserialize)]` since proc macros can't be nested (can't see/modify the output of
/// other proc macros). But we also don't want to fully re-implement deriving `Deserialize`. So the
/// approach taken here is to generate a hidden inner struct (called `__DeserializeHelper`) with all
/// the same fields and serde attributes and slap a `#[derive(Deserialize)]` on *that*. Our
/// implementation of `Deserialize` will then:
///
/// - Forward the deserialize call to `__DeserializeHelper` for most types of data, moving the
///     fields into an instance of the actual struct once done.
/// - Have a `visit_str` method that knows how to use the given regex to parse the str into
///     capture groups and *then* forward that to `__DeserializeHelper` as above.
///
/// However this is a breaking change, since previously it was expected that you'd
/// put `#[derive(Deserialize, Recap)]` on your struct. Therefore it needs to be opted into with
/// the `#[recap(handle_deserialize)]` attribute (which can also be combined with the `regex = ...`
/// attribute on a single line) to opt into this.
pub fn derive_impl_deserialize(
    item: &DeriveInput,
    item_ident: &Ident,
    nested_metas: Vec<Meta>,
    regex: String,
) -> TokenStream {
    let include_deserialize_impl = nested_metas
        .iter()
        .any(|meta| meta.path().is_ident("handle_deserialize"));
    if !include_deserialize_impl {
        return quote!();
    }

    // Make a copy of the struct with a different name (`__DeserializeHelper`) and without
    // any recap attributes.
    let deserialize_helper_ident = Ident::new("__DeserializeHelper", Span::call_site());
    let mut deserialize_helper_item = item.clone();
    deserialize_helper_item.ident = deserialize_helper_ident.clone();
    deserialize_helper_item
        .attrs
        .retain(|attr| !attr.path.is_ident("recap"));
    match &mut deserialize_helper_item.data {
        Struct(data_struct) => {
            for field in data_struct.fields.iter_mut() {
                field.attrs.retain(|attr| !attr.path.is_ident("recap"));
            }
        }
        _ => panic!("Expected Recap derive on struct only"),
    }

    // Figure out the field names so we can move them from the helper struct to the real one
    let assign_fields = match &item.data {
        Struct(data_struct) => data_struct
            .fields
            .iter()
            .flat_map(|field| {
                field.ident.as_ref().map(|field_ident| {
                    quote! {
                        #field_ident: value.#field_ident,
                    }
                })
            })
            .collect::<Vec<_>>(),
        _ => panic!("Expected Recap derive on struct only"),
    };
    // Needed for the Visitor implementation
    let visitor_expecting = format!("struct {}", item.ident);
    let visitor_forward_methods = vec![
        visitor_forward_primitive("bool", &deserialize_helper_ident),
        visitor_forward_primitive("i8", &deserialize_helper_ident),
        visitor_forward_primitive("i16", &deserialize_helper_ident),
        visitor_forward_primitive("i32", &deserialize_helper_ident),
        visitor_forward_primitive("i64", &deserialize_helper_ident),
        visitor_forward_primitive("i128", &deserialize_helper_ident),
        visitor_forward_primitive("u8", &deserialize_helper_ident),
        visitor_forward_primitive("u16", &deserialize_helper_ident),
        visitor_forward_primitive("u32", &deserialize_helper_ident),
        visitor_forward_primitive("u64", &deserialize_helper_ident),
        visitor_forward_primitive("u128", &deserialize_helper_ident),
        visitor_forward_primitive("f32", &deserialize_helper_ident),
        visitor_forward_primitive("f64", &deserialize_helper_ident),
        visitor_forward_primitive("char", &deserialize_helper_ident),
        visitor_forward_deserializer("some", &deserialize_helper_ident),
        visitor_forward_deserializer("newtype_struct", &deserialize_helper_ident),
        visitor_forward_accessor("seq", &deserialize_helper_ident),
        visitor_forward_accessor("map", &deserialize_helper_ident),
        visitor_forward_accessor("enum", &deserialize_helper_ident),
    ];

    quote! {
        extern crate serde as _serde;
        #[derive(_serde::Deserialize)]
        #deserialize_helper_item

        impl From<#deserialize_helper_ident> for #item_ident {
            fn from(value: #deserialize_helper_ident) -> Self {
                Self {
                    #(#assign_fields)*
                }
            }
        }

        impl<'de> _serde::Deserialize<'de> for #item_ident {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: _serde::Deserializer<'de>,
            {
                struct __Visitor<'de> {
                    marker: std::marker::PhantomData<#item_ident>,
                    lifetime: std::marker::PhantomData<&'de ()>,
                }
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = #item_ident;

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter,
                    ) -> std::fmt::Result {
                        formatter.write_str(#visitor_expecting)
                    }

                    #(#visitor_forward_methods)*

                    fn visit_str<E>(
                        self,
                        v: &str,
                    ) -> core::result::Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        recap::lazy_static! {
                            static ref RE: recap::Regex = recap::Regex::new(#regex)
                                .expect("Failed to compile regex");
                        }
                        recap::from_captures::<#deserialize_helper_ident>(&RE, v)
                            .map(|helper| helper.into())
                            .map_err(|e| serde::de::Error::custom(e))
                    }
                }

                deserializer.deserialize_any(__Visitor {
                    marker: std::marker::PhantomData::<#item_ident>,
                    lifetime: std::marker::PhantomData,
                })
            }
        }
    }
}

/*
Helpers for forwarding the deserialize call to the struct that derives Deserialize. Broadly they
convert the given data into a deserializer, call deserialize, and convert the resulting data into
our desired struct.
*/

fn visitor_forward_primitive(
    primitive_name: &str,
    target: &Ident,
) -> TokenStream {
    let method_ident = Ident::new(&format!("visit_{}", primitive_name), Span::call_site());
    let primitive_ident = Ident::new(primitive_name, Span::call_site());

    quote! {
        fn #method_ident<E>(
            self,
            v: #primitive_ident,
        ) -> Result<Self::Value, E>
        where
            E: _serde::de::Error,
        {
            let deserializer =
                <#primitive_ident as _serde::de::IntoDeserializer>::into_deserializer(v);
            <#target as _serde::Deserialize>::deserialize(deserializer)
                .map(|helper| helper.into())
                // Can't figure out how to get the compiler to infer this error type as the return, so just wrap it
                .map_err(|e| _serde::de::Error::custom(e))
        }
    }
}

fn visitor_forward_deserializer(
    name: &str,
    target: &Ident,
) -> TokenStream {
    let method_ident = Ident::new(&format!("visit_{}", name), Span::call_site());

    quote! {
        fn #method_ident<D>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error>
        where
            D: _serde::Deserializer<'de>,
        {
            <#target as _serde::Deserialize>::deserialize(deserializer).map(|helper| helper.into())
        }
    }
}

// Handles the visit_map, visit_seq, and visit_enum, along with the corresponding
// (Map|Seq|Enum)Access and (Map|Seq|Enum)AccessDeserializer types.
fn visitor_forward_accessor(
    type_name: &str,
    target: &Ident,
) -> TokenStream {
    let method_ident = Ident::new(&format!("visit_{}", type_name), Span::call_site());
    let capitalized_type_name = format!("{}{}", type_name[0..1].to_uppercase(), &type_name[1..]);
    let accessor_ident = Ident::new(
        &format!("{}Access", capitalized_type_name),
        Span::call_site(),
    );
    let accessor_deserializer_ident = Ident::new(
        &format!("{}AccessDeserializer", capitalized_type_name),
        Span::call_site(),
    );

    quote! {
        fn #method_ident<A>(
            self,
            v: A,
        ) -> Result<Self::Value, A::Error>
        where
            A: _serde::de::#accessor_ident<'de>,
        {
            <#target as _serde::Deserialize>::deserialize(
                <_serde::de::value::#accessor_deserializer_ident<A>>::new(v),
            )
            .map(|helper| helper.into())
        }
    }
}
