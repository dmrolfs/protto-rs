use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::{self, Attribute, DeriveInput, Expr, Field, Lit, Meta, Type};
use syn::{punctuated::Punctuated, token::Comma};

mod constants {
    pub const PRIMITIVE_TYPES: &[&str] =
        &["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
    pub const DEFAULT_PROTO_MODULE: &str = "proto";
    pub const DEFAULT_CONVERSION_ERROR_SUFFIX: &str = "ConversionError";
}

mod attribute_parser;
mod debug;
mod enum_processor;
mod error_analysis;
mod error_codegen;
mod error_handler;
mod error_types;
mod expect_analysis;
mod field_analysis;
mod field_processor;
mod macro_input;
mod proto_inspection;
mod struct_impl;
mod tuple_impl;
mod type_analysis;
// mod conversion;

mod utils {
    use proc_macro2::TokenStream;
    use quote::quote;
    pub fn maybe_option_expr(is_option: bool, inner: TokenStream) -> TokenStream {
        if is_option {
            quote! { Some(#inner) }
        } else {
            inner
        }
    }

    pub fn to_screaming_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i != 0 {
                result.push('_');
            }
            result.push(c.to_ascii_uppercase());
        }
        result
    }
}

#[proc_macro_derive(ProtoConvert, attributes(proto))]
pub fn proto_convert_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let parsed_input = macro_input::parse_derive_input(ast.clone());

    let name = parsed_input.name;

    match &ast.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields_named) => {
                let config = struct_impl::StructImplConfig {
                    name: &name,
                    fields: &fields_named.named,
                    proto_module: &parsed_input.proto_module,
                    proto_name: &parsed_input.proto_name,
                    proto_path: &parsed_input.proto_path,
                    struct_level_error_type: &parsed_input.struct_level_error_type,
                    struct_level_error_fn: &parsed_input.struct_level_error_fn,
                };

                struct_impl::generate_struct_implementations(config).into()
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                tuple_impl::generate_tuple_implementations(&name, fields_unnamed).into()
            }
            syn::Fields::Unit => {
                panic!("ProtoConvert does not support unit structs");
            }
        },
        syn::Data::Enum(data_enum) => {
            let variants = &data_enum.variants;
            enum_processor::generate_enum_conversions(&name, variants, &parsed_input.proto_module)
                .into()
        }
        _ => panic!("ProtoConvert only supports structs and enums, not unions"),
    }
}
