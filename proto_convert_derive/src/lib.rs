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
//!   customizable via the `#[proto_module = "your_own_proto"]` attribute.
//!
//! ## Usage
//!
//! Add the attribute to your struct and specify your Protobuf module:
//!
//! ```ignore
//! use proto_convert_derive::ProtoConvert;
//!
//! // By default we expect you to use mod proto
//! mod myproto {
//!     tonic::include_proto!("stae");
//! }
//!
//! #[derive(ProtoConvert)]
//! #[proto_module = "myproto"]
//! struct Key {
//!    pub id: string,
//! }
//!
//! #[derive(ProtoConvert)]
//! #[proto_module = "myproto"]
//! struct State {
//!     pub key: Key,
//! }
//!
//! fn main() {
//!     let proto_key = myproto::Key {
//!         id: Some(myproto::Id {
//!             id: "my id".to_string(),
//!         }),
//!     };
//!     let my_key: Key = proto_key.into();
//!
//!     // Conversion from native Rust type to Protobuf:
//!     let my_state = State { key: my_key };
//!     let proto_state: myproto::State = my_state.into();
//! }
//! ```
//!
//! ## Limitations
//!
//! - Not all primitive types are implemented.
//! - Only supports structs with named fields.
//! - Assumes certain patterns for primitive and message type conversion.
use proc_macro::TokenStream;
use quote::quote;
use syn::{self, Attribute, DeriveInput, Field};
use syn::{Expr, Lit, Meta};

#[proc_macro_derive(ProtoConvert, attributes(proto_module, proto))]
pub fn proto_convert_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let proto_module = get_proto_module(&ast.attrs).unwrap_or_else(|| "proto".to_string());
    let proto_path = syn::parse_str::<syn::Path>(&format!("{}::{}", proto_module, name)).unwrap();

    match &ast.data {
        syn::Data::Struct(data_struct) => {
            match &data_struct.fields {
                // Handle structs with named fields
                syn::Fields::Named(fields_named) => {
                    let fields = &fields_named.named;
                    let primitives = ["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
                    let from_proto_fields = fields.iter().map(|field| {
                        let field_name = field.ident.as_ref().unwrap();
                        let field_type = &field.ty;
                        let is_transparent = has_transparent_attr(field);
                        if is_transparent {
                            quote! {
                                #field_name: <#field_type>::from(proto_struct.#field_name)
                            }
                        } else if let syn::Type::Path(type_path) = field_type {
                            let is_primitive = type_path.path.segments.len() == 1 &&
                                primitives.iter().any(|&p| type_path.path.segments[0].ident == p);
                            let is_proto_type = type_path.path.segments.first()
                                .map_or(false, |segment| segment.ident == proto_module.as_str());
                            if is_primitive {
                                quote! { #field_name: proto_struct.#field_name }
                            } else if is_proto_type {
                                quote! {
                                    #field_name: proto_struct.#field_name.expect(concat!("no ", stringify!(#field_name), " in proto"))
                                }
                            } else {
                                quote! {
                                    #field_name: proto_struct.#field_name.expect(concat!("no ", stringify!(#field_name), " in proto")).into()
                                }
                            }
                        } else {
                            panic!("Only path types are supported for field '{}'", field_name);
                        }
                    });

                    let from_my_fields = fields.iter().map(|field| {
                        let field_name = field.ident.as_ref().unwrap();
                        let field_type = &field.ty;
                        let is_transparent = has_transparent_attr(field);
                        if is_transparent {
                            quote! {
                                #field_name: my_struct.#field_name.into()
                            }
                        } else if let syn::Type::Path(type_path) = field_type {
                            let is_primitive = type_path.path.segments.len() == 1
                                && primitives
                                    .iter()
                                    .any(|&p| type_path.path.segments[0].ident == p);
                            let is_proto_type =
                                type_path.path.segments.first().map_or(false, |segment| {
                                    segment.ident == proto_module.as_str()
                                });

                            if is_primitive {
                                quote! { #field_name: my_struct.#field_name }
                            } else if is_proto_type {
                                quote! { #field_name: Some(my_struct.#field_name) }
                            } else {
                                quote! { #field_name: Some(my_struct.#field_name.into()) }
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

fn get_proto_module(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("proto_module") {
            match &attr.meta {
                Meta::NameValue(meta) => {
                    if let Expr::Lit(expr_lit) = &meta.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value());
                        }
                    }
                    panic!("proto_module attribute must be a string literal, e.g., #[proto_module = \"path\"]");
                }
                _ => {
                    panic!("proto_module attribute must be in the form #[proto_module = \"path\"]");
                }
            }
        }
    }
    None
}

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
