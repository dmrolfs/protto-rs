//! # proto_convert_derive
//!
//! Automatically derive conversions between Protobuf-compiled prost types and
//! your native Rust types.
//!
//! ## Overview
//!
//! `proto_convert_derive` is a procedural macro that automates bidirectional
//! conversions between Protobuf-generated types and your local Rust structs. It
//! reduces boilerplate and helps with `required` not being supported in proto3
//! (which results in `Option` types in complex rust types).
//!
//! ### Key Features
//! - Implements `From<Proto>` for your Rust types and vice versa.
//! - Directly maps primitive types.
//! - Automatically unwraps optional fields for message types using `.expect`.
//! - By default searches for prost types in a `proto` module, but this is
//!   customizable via the `#[proto(module="your_own_proto")]` attribute.
//! - Supports field renaming with `#[proto(rename = "protobuf_field_name")]`,
//!   allowing fields in the Rust struct to map to differently named fields in
//!   the Protobuf message.
//! - Use `#[proto(transparent)]` for fields that should be converted directly
//!   using `From` and `Into`, especially useful for newtypes or when the
//!   Protobuf field type differs from the Rust field type.
//!
//! ## Usage
//!
//! Add the attribute to your struct and specify your Protobuf module:
//!
//! ```ignore
//! use proto_convert_derive::ProtoConvert;
//! mod proto {
//!     tonic::include_proto!("service");
//! }
//!
//! // Overwrite the prost Request type.
//! #[derive(ProtoConvert, PartialEq, Debug, Clone)]
//! pub struct Request {
//!     // Here we take the prost Header type instaed
//!     pub header: proto::Header,
//!     pub payload: String,
//! }
//!
//! #[derive(ProtoConvert, PartialEq, Debug, Clone)]
//! #[proto(module = "proto")]
//! pub struct Track {
//!     #[proto(transparent, rename = "track_id")]
//!     id: TrackId,
//! }
//!
//! #[derive(ProtoConvert, PartialEq, Debug, Clone)]
//! pub struct TrackId(u64);
//!
//! #[derive(ProtoConvert, PartialEq, Debug, Clone)]
//! pub struct State {
//!     pub tracks: Vec<Track>, // we support collections as well!
//! }
//!
//! #[derive(ProtoConvert, PartialEq, Debug, Clone)]
//! #[proto(rename = "State")]
//! pub struct ProtoState {
//!     pub tracks: Vec<proto::Track>, // we support collections as well!
//! }
//!
//! #[derive(ProtoConvert, PartialEq, Debug, Clone)]
//! pub struct HasOptional {
//!     pub track: Option<Track>,
//! }
//! ```
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::{self, Attribute, DeriveInput, Expr, Field, Lit, Meta, Type};
use syn::{punctuated::Punctuated, token::Comma};

#[proc_macro_derive(ProtoConvert, attributes(proto))]
pub fn proto_convert_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let proto_module = get_proto_module(&ast.attrs).unwrap_or_else(|| "proto".to_string());
    let proto_name = get_proto_struct_rename(&ast.attrs).unwrap_or_else(|| name.to_string());
    let proto_path =
        syn::parse_str::<syn::Path>(&format!("{}::{}", proto_module, proto_name)).unwrap();

    match &ast.data {
        syn::Data::Struct(data_struct) => {
            match &data_struct.fields {
                syn::Fields::Named(fields_named) => {
                    let fields = &fields_named.named;
                    let primitives = ["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
                    let from_proto_fields = fields.iter().map(|field| {
                        let field_name = field.ident.as_ref().unwrap();
                        let proto_field_ident = if let Some(rename) = get_proto_rename(field) {
                            syn::Ident::new(&rename, Span::call_site())
                        } else {
                            field_name.clone()
                        };
                        let field_type = &field.ty;
                        let is_transparent = has_transparent_attr(field);

                        if is_transparent {
                            quote! {
                                #field_name: <#field_type>::from(proto_struct.#proto_field_ident)
                            }
                        } else if is_option_type(field_type) {
                            let inner_type = get_inner_type_from_option(field_type).unwrap();
                            if is_vec_type(&inner_type) {
                                quote! {
                                    #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                }
                            } else {
                                quote! {
                                    #field_name: proto_struct.#proto_field_ident.map(Into::into)
                                }
                            }
                        } else if is_vec_type(field_type) {
                            if let Some(inner_type) = get_inner_type_from_vec(field_type) {
                                if is_proto_type_with_module(&inner_type, &proto_module) {
                                    quote! {
                                        #field_name: proto_struct.#proto_field_ident
                                    }
                                } else {
                                    quote! {
                                        #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                    }
                                }
                            } else {
                                quote! {
                                    #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                                }
                            }
                        } else if let syn::Type::Path(type_path) = field_type {
                            let is_primitive = type_path.path.segments.len() == 1 &&
                                primitives.iter().any(|&p| type_path.path.segments[0].ident == p);
                            let is_proto_type = type_path.path.segments.first()
                                .is_some_and(|segment| segment.ident == proto_module.as_str());
                            if is_primitive {
                                quote! { #field_name: proto_struct.#proto_field_ident }
                            } else if is_proto_type {
                                quote! {
                                    #field_name: proto_struct.#proto_field_ident.expect(concat!("no ", stringify!(#proto_field_ident), " in proto"))
                                }
                            } else {
                                quote! {
                                    #field_name: proto_struct.#proto_field_ident.expect(concat!("no ", stringify!(#proto_field_ident), " in proto")).into()
                                }
                            }
                        } else {
                            panic!("Only path types are supported for field '{}'", field_name);
                        }
                    });

                    let from_my_fields = fields.iter().map(|field| {
                        let field_name = field.ident.as_ref().unwrap();
                        let proto_field_ident = if let Some(rename) = get_proto_rename(field) {
                            syn::Ident::new(&rename, Span::call_site())
                        } else {
                            field_name.clone()
                        };
                        let field_type = &field.ty;
                        let is_transparent = has_transparent_attr(field);

                        if is_transparent {
                            quote! {
                                #proto_field_ident: my_struct.#field_name.into()
                            }
                        } else if is_option_type(field_type) {
                            let inner_type = get_inner_type_from_option(field_type).unwrap();
                            if is_vec_type(&inner_type) {
                                quote! {
                                    #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
                                }
                            } else {
                                quote! {
                                    #proto_field_ident: my_struct.#field_name.map(Into::into)
                                }
                            }
                        } else if is_vec_type(field_type) {
                            // Check if the inner type is already a proto type.
                            if let Some(inner_type) = get_inner_type_from_vec(field_type) {
                                if is_proto_type_with_module(&inner_type, &proto_module) {
                                    quote! {
                                        #proto_field_ident: my_struct.#field_name
                                    }
                                } else {
                                    quote! {
                                        #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
                                    }
                                }
                            } else {
                                quote! {
                                    #proto_field_ident: my_struct.#field_name.into_iter().map(Into::into).collect()
                                }
                            }
                        } else if let syn::Type::Path(type_path) = field_type {
                            let is_primitive = type_path.path.segments.len() == 1
                                && primitives
                                    .iter()
                                    .any(|&p| type_path.path.segments[0].ident == p);
                            let is_proto_type = type_path
                                .path
                                .segments
                                .first()
                                .is_some_and(|segment| segment.ident == proto_module.as_str());

                            if is_primitive {
                                quote! { #proto_field_ident: my_struct.#field_name }
                            } else if is_proto_type {
                                quote! { #proto_field_ident: Some(my_struct.#field_name) }
                            } else {
                                quote! { #proto_field_ident: Some(my_struct.#field_name.into()) }
                            }
                        } else {
                            panic!("Only path types are supported for field '{}'", field_name);
                        }
                    });

                    let gen = quote! {
                        impl From<#proto_path> for #name {
                            fn from(proto_struct: #proto_path) -> Self {
                                Self {
                                    #(#from_proto_fields),*
                                }
                            }
                        }

                        impl From<#name> for #proto_path {
                            fn from(my_struct: #name) -> Self {
                                Self {
                                    #(#from_my_fields),*
                                }
                            }
                        }
                    };
                    gen.into()
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    if fields_unnamed.unnamed.len() != 1 {
                        panic!("ProtoConvert only supports tuple structs with exactly one field, found {}", fields_unnamed.unnamed.len());
                    }
                    let inner_type = &fields_unnamed.unnamed[0].ty;
                    let gen = quote! {
                        impl From<#inner_type> for #name {
                            fn from(value: #inner_type) -> Self {
                                #name(value)
                            }
                        }

                        impl From<#name> for #inner_type {
                            fn from(my: #name) -> Self {
                                my.0
                            }
                        }
                    };
                    gen.into()
                }
                syn::Fields::Unit => {
                    panic!("ProtoConvert does not support unit structs");
                }
            }
        }
        _ => panic!("ProtoConvert only supports structs, not enums or unions"),
    }
}

// Checks if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
            return true;
        }
    }
    false
}

// Extracts inner type T from Option<T>
fn get_inner_type_from_option(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
            if let syn::PathArguments::AngleBracketed(angle_bracketed) =
                &type_path.path.segments[0].arguments
            {
                if let Some(syn::GenericArgument::Type(inner_type)) = angle_bracketed.args.first() {
                    return Some(inner_type.clone());
                }
            }
        }
    }
    None
}

// Checks if a type is Vec<T>
fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
            return true;
        }
    }
    false
}

// Extracts inner type T from Vec<T>
fn get_inner_type_from_vec(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
            if let syn::PathArguments::AngleBracketed(angle_bracketed) =
                &type_path.path.segments[0].arguments
            {
                if let Some(syn::GenericArgument::Type(inner_type)) = angle_bracketed.args.first() {
                    return Some(inner_type.clone());
                }
            }
        }
    }
    None
}

// Checks if a type is a proto type based on the proto module name.
fn is_proto_type_with_module(ty: &Type, proto_module: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            return segment.ident == proto_module;
        }
    }
    false
}

// Extracts the proto module name from struct-level attributes.
fn get_proto_module(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("module") {
                            if let Expr::Lit(expr_lit) = meta_nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!("module value must be a string literal, e.g., #[proto(module = \"path\")]");
                        }
                    }
                }
            }
        }
    }
    None
}

// Supports struct-level renaming via #[proto(rename = "...")]
fn get_proto_struct_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("rename") {
                            if let Expr::Lit(expr_lit) = meta_nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!("rename value must be a string literal, e.g., #[proto(rename = \"...\")]");
                        }
                    }
                }
            }
        }
    }
    None
}

// Checks for transparent attribute on a field.
fn has_transparent_attr(field: &Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
                let tokens = &meta_list.tokens;
                let token_str = quote!(#tokens).to_string();
                if token_str.contains("transparent") {
                    return true;
                }
            }
        }
    }
    false
}

// Helper to extract the field-level rename attribute.
fn get_proto_rename(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if attr.path().is_ident("proto") {
            if let Meta::List(meta_list) = &attr.meta {
                let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                    .parse2(meta_list.tokens.clone())
                    .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                for meta in nested_metas {
                    if let Meta::NameValue(meta_nv) = meta {
                        if meta_nv.path.is_ident("rename") {
                            if let Expr::Lit(expr_lit) = &meta_nv.value {
                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                            panic!("rename value must be a string literal, e.g., rename = \"xyz\"");
                        }
                    }
                }
            }
        }
    }
    None
}
