use crate::utils;
use proc_macro2::Span;
use quote::quote;

pub fn generate_enum_conversions(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    proto_module: &str,
) -> proc_macro2::TokenStream {
    let enum_name_str = name.to_string();
    let enum_prefix = enum_name_str.to_uppercase();
    let proto_enum_path: syn::Path = syn::parse_str(&format!("{}::{}", proto_module, name))
        .expect("Failed to parse proto enum path");

    let from_proto_enum_arms = generate_from_proto_enum_arms(variants, name, &enum_prefix);
    let from_proto_arms = generate_from_proto_arms(variants, name, &enum_prefix, &proto_enum_path);

    quote! {
        impl From<i32> for #name {
            fn from(value: i32) -> Self {
                let proto_val = <#proto_enum_path>::from_i32(value)
                    .unwrap_or_else(|| panic!("Unknown enum value: {}", value));
                let proto_str = proto_val.as_str_name();
                match proto_str {
                    #(#from_proto_enum_arms)*
                    _ => panic!("No matching Rust variant for proto enum string: {}", proto_str),
                }
            }
        }

        impl From<#name> for i32 {
            fn from(rust_enum: #name) -> Self {
                let proto: #proto_enum_path = rust_enum.into();
                proto as i32
            }
        }

        impl From<#name> for #proto_enum_path {
            fn from(rust_enum: #name) -> Self {
                match rust_enum {
                    #(#from_proto_arms)*
                }
            }
        }

        impl From<#proto_enum_path> for #name {
            fn from(proto_enum: #proto_enum_path) -> Self {
                let proto_str = proto_enum.as_str_name();
                match proto_str {
                    #(#from_proto_enum_arms)*
                    _ => panic!("No matching Rust variant for proto enum string: {proto_str}"),
                }
            }
        }
    }
}

fn generate_from_proto_enum_arms(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    name: &syn::Ident,
    enum_prefix: &str,
) -> Vec<proc_macro2::TokenStream> {
    variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let variant_str = variant_ident.to_string();
        let screaming_variant = utils::to_screaming_snake_case(&variant_str);
        let prefixed_candidate = format!("{}_{}", enum_prefix, screaming_variant);

        quote! {
            candidate if candidate == #variant_str || candidate == #prefixed_candidate => #name::#variant_ident,
        }
    }).collect()
}

fn generate_from_proto_arms(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    name: &syn::Ident,
    enum_prefix: &str,
    proto_enum_path: &syn::Path,
) -> Vec<proc_macro2::TokenStream> {
    variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let variant_str = variant_ident.to_string();
            let screaming_variant = utils::to_screaming_snake_case(&variant_str);
            let prefixed_candidate = format!("{}_{}", enum_prefix, screaming_variant);
            let prefixed_candidate_lit = syn::LitStr::new(&prefixed_candidate, Span::call_site());

            quote! {
                #name::#variant_ident => <#proto_enum_path>::from_str_name(#prefixed_candidate_lit)
                    .unwrap_or_else(|| panic!("No matching proto variant for {rust_enum:?}")),
            }
        })
        .collect()
}
