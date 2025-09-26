use crate::analysis::attribute_parser;
use crate::constants;
use quote::quote;
use std::collections::HashSet;
use std::fmt::Debug;

pub struct ParsedInput {
    pub name: syn::Ident,
    pub proto_module: String,
    pub proto_name: String,
    pub struct_level_error_type: Option<syn::Type>,
    pub struct_level_error_fn: Option<String>,
    pub proto_ignored_fields: HashSet<String>,
    pub proto_path: syn::Path,
}

impl Debug for ParsedInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_type = self
            .struct_level_error_type
            .as_ref()
            .map(|error_type| quote! { #error_type })
            .map(|error_type| error_type.to_string())
            .unwrap_or_default();

        let proto_path = &self.proto_path;
        let proto_path = quote! { #proto_path };
        let proto_path = proto_path.to_string();

        f.debug_struct("ParsedInput")
            .field("name", &self.name)
            .field("proto_module", &self.proto_module)
            .field("proto_name", &self.proto_name)
            .field("struct_level_error_type", &error_type)
            .field("struct_level_error_fn", &self.struct_level_error_fn)
            .field("proto_ignored_fields", &self.proto_ignored_fields)
            .field("proto_path", &proto_path)
            .finish()
    }
}

impl ParsedInput {
    pub fn new(ast: syn::DeriveInput) -> ParsedInput {
        let proto_module = attribute_parser::get_proto_module(&ast.attrs)
            .unwrap_or_else(|| constants::DEFAULT_PROTO_MODULE.to_string());
        let proto_name = attribute_parser::get_proto_struct_name(&ast.attrs)
            .unwrap_or_else(|| ast.ident.to_string());
        let struct_level_error_type = attribute_parser::get_proto_struct_error_type(&ast.attrs);
        let struct_level_error_fn = attribute_parser::get_struct_level_error_fn(&ast.attrs);
        if let Err(msg) = attribute_parser::validate_error_configuration(
            &struct_level_error_type,
            &struct_level_error_fn,
            &Self::fields_from(&ast),
        ) {
            panic!("Invalid error configuration: {msg}");
        }

        let proto_ignored_fields = attribute_parser::get_struct_level_proto_ignore(&ast.attrs);
        let proto_path = syn::parse_str::<syn::Path>(&format!("{}::{}", proto_module, proto_name))
            .expect("Failed to create proto path");

        ParsedInput {
            name: ast.ident,
            proto_module,
            proto_name,
            struct_level_error_type,
            struct_level_error_fn,
            proto_ignored_fields,
            proto_path,
        }
    }

    fn fields_from(
        ast: &syn::DeriveInput,
    ) -> syn::punctuated::Punctuated<syn::Field, syn::token::Comma> {
        match &ast.data {
            syn::Data::Struct(data_struct) => {
                match &data_struct.fields {
                    syn::Fields::Named(fields_named) => fields_named.named.clone(),
                    syn::Fields::Unnamed(_) => {
                        // Tuple structs - validation may not apply the same way
                        syn::punctuated::Punctuated::new()
                    }
                    syn::Fields::Unit => {
                        // Unit structs have no fields
                        syn::punctuated::Punctuated::new()
                    }
                }
            }
            _ => {
                // Enums don't have fields in the same sense, skip validation
                syn::punctuated::Punctuated::new()
            }
        }
    }
}
