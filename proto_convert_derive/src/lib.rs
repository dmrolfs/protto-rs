use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::{self, Attribute, DeriveInput, Expr, Field, Lit, Meta, Type};
use syn::{punctuated::Punctuated, token::Comma};

mod constants {
    pub const PRIMITIVE_TYPES: &[&str] = &["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
    pub const DEFAULT_PROTO_MODULE: &str = "proto";
    pub const DEFAULT_CONVERSION_ERROR_SUFFIX: &str = "ConversionError";
}

mod debug {
    use proc_macro2::{Ident, TokenStream};

    #[allow(unused_variables)]
    pub fn should_output_debug(name: &Ident, field_name: &Ident) -> bool {
        false
        // || name.to_string() == "StatusResponse"
        // || name.to_string() == "Status"
        // || name.to_string() == "MultipleErrorTypesStruct"
        // || name.to_string() == "ExpectCustomErrorStruct" // Enable debug for this struct
        // || name.to_string() == "MixedBehaviorTrack"
        // || name.to_string() == "HasOptionalWithCustomError"
        // || name.to_string() == "CombinationStruct"
        // || name.to_string() == "ComplexExpectStruct"
        // || name.to_string().contains("TrackWith")
    }

    pub fn debug_generated_code(name: &Ident, field_name: &Ident, code: &TokenStream, context: &str) {
        if should_output_debug(name, field_name) {
            eprintln!("=== GENERATED CODE DEBUG: {} for {} ===", context, field_name);
            eprintln!("  {}", code);
            eprintln!("=== END GENERATED CODE ===");
        }
    }

    pub fn debug_field_analysis(
        name: &Ident,
        field_name: &Ident,
        context: &str,
        details: &[(&str, String)]
    ) {
        if should_output_debug(name, field_name) {
            eprintln!("=== {context} for {name}.{field_name}");
            for (key, value) in details {
                eprintln!("  {key}: {value}");
            }
            eprintln!("=== END {context} ===");
        }
    }
}

mod metadata_registry {
    pub fn get_field_optionality(message: &str, field: &str) -> Option<bool> {
        #[cfg(feature = "meta-file")]
        {
            // This is a compile-time code generation hint
            // The actual call will be made to the user's included module
            // We can't actually call it here because we're in the proc macro context
            None
        }
        #[cfg(not(feature = "meta-file"))]
        {
            None
        }
    }

    // Generate code that will call the user's included metadata at expansion time
    #[cfg(feature = "meta-file")]
    pub fn generate_metadata_lookup_code(message: &str, field: &str) -> proc_macro2::TokenStream {
        quote::quote! {
            crate::proto_metadata::get_field_optionality(#message, #field)
        }
    }

    #[cfg(not(feature = "meta-file"))]
    pub fn generate_metadata_lookup_code(_message: &str, _field: &str) -> proc_macro2::TokenStream {
        quote::quote! { None }
    }
}


mod macro_input;
mod struct_impl;
mod tuple_impl;
mod attribute_parser;
mod type_analysis;
mod field_analysis;
mod field_processor;
mod enum_processor;
mod error_analysis;
mod error_types;
mod error_codegen;
mod error_handler;
mod expect_analysis;

mod utils {
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

    match &ast.data {
        syn::Data::Struct(data_struct) => {
            match &data_struct.fields {
                syn::Fields::Named(fields_named) => {
                    let config = struct_impl::StructImplConfig {
                        name: &parsed_input.name,
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
                    tuple_impl::generate_tuple_implementations(&parsed_input.name, fields_unnamed).into()
                }
                syn::Fields::Unit => {
                    panic!("ProtoConvert does not support unit structs");
                }
            }
        },
        syn::Data::Enum(data_enum) => {
            let variants = &data_enum.variants;
            enum_processor::generate_enum_conversions(&parsed_input.name, variants, &parsed_input.proto_module).into()
        },
        _ => panic!("ProtoConvert only supports structs and enums, not unions"),
    }
}
