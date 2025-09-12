use crate::analysis::attribute_parser;
use crate::constants;

pub struct ParsedInput {
    pub name: syn::Ident,
    pub proto_module: String,
    pub proto_name: String,
    pub struct_level_error_type: Option<syn::Type>,
    pub struct_level_error_fn: Option<String>,
    pub proto_path: syn::Path,
}

pub fn parse_derive_input(ast: syn::DeriveInput) -> ParsedInput {
    let name = ast.ident;
    let proto_module = attribute_parser::get_proto_module(&ast.attrs)
        .unwrap_or_else(|| constants::DEFAULT_PROTO_MODULE.to_string());
    let proto_name =
        attribute_parser::get_proto_struct_name(&ast.attrs).unwrap_or_else(|| name.to_string());
    let struct_level_error_type = attribute_parser::get_proto_struct_error_type(&ast.attrs);
    let struct_level_error_fn = attribute_parser::get_struct_level_error_fn(&ast.attrs);
    let proto_path = syn::parse_str::<syn::Path>(&format!("{}::{}", proto_module, proto_name))
        .expect("Failed to create proto path");

    ParsedInput {
        name,
        proto_module,
        proto_name,
        struct_level_error_type,
        struct_level_error_fn,
        proto_path,
    }
}
