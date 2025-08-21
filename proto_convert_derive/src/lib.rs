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

mod macro_input {
    use crate::constants::DEFAULT_PROTO_MODULE;
    use super::*;

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
            .unwrap_or_else(|| DEFAULT_PROTO_MODULE.to_string());
        let proto_name = attribute_parser::get_proto_struct_rename(&ast.attrs)
            .unwrap_or_else(|| name.to_string());
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
}

mod struct_impl_generator {
    use super::*;
    use field_analysis::FieldProcessingContext;

    pub struct StructImplConfig<'a> {
        pub name: &'a syn::Ident,
        pub fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
        pub proto_module: &'a str,
        pub proto_name: &'a str,
        pub proto_path: &'a syn::Path,
        pub struct_level_error_type: &'a Option<syn::Type>,
        pub struct_level_error_fn: &'a Option<String>,
    }

    pub fn generate_struct_implementations(config: StructImplConfig) -> proc_macro2::TokenStream {
        let error_name = error_handler::default_error_name(config.name);

        let (conversion_error_def, error_conversions, needs_try_from) =
            error_handler::generate_error_definitions_if_needed(
                config.name,
                config.fields,
                config.struct_level_error_type
            );

        let actual_error_type = error_handler::get_actual_error_type(
            needs_try_from,
            config.struct_level_error_type,
            &error_name,
        );

        let from_proto_fields = generate_from_proto_fields(config.fields, &config, &error_name);
        let from_my_fields = generate_from_my_fields(config.fields, &config, &error_name);

        let name = config.name;
        let proto_path = config.proto_path;

        if needs_try_from {
            quote! {
                #conversion_error_def
                #error_conversions

                impl TryFrom<#proto_path> for #name {
                    type Error = #actual_error_type;

                    fn try_from(proto_struct: #proto_path) -> Result<Self, Self::Error> {
                        Ok(Self {
                            #(#from_proto_fields),*
                        })
                    }
                }

                impl From<#name> for #proto_path {
                    fn from(my_struct: #name) -> Self {
                        Self {
                            #(#from_my_fields),*
                        }
                    }
                }
            }
        } else {
            quote! {
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
            }
        }
    }

    fn generate_from_proto_fields<'a>(
        fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
        config: &'a StructImplConfig<'a>,
        error_name: &'a syn::Ident,
    ) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
        fields.iter().map(move |field| {
            let ctx = FieldProcessingContext::new(
                config.name,
                field,
                error_name,
                config.struct_level_error_type,
                config.struct_level_error_fn,
                config.proto_module,
                config.proto_name,
            );

            field_processor::generate_from_proto_field(field, &ctx)
        })
    }

    fn generate_from_my_fields<'a>(
        fields: &'a syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
        config: &'a StructImplConfig<'a>,
        error_name: &'a syn::Ident,
    ) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
        fields
            .iter()
            .filter(|field| !attribute_parser::has_proto_ignore(field))
            .map(move |field| {
                let ctx = FieldProcessingContext::new(
                    config.name,
                    field,
                    error_name,
                    config.struct_level_error_type,
                    config.struct_level_error_fn,
                    config.proto_module,
                    config.proto_name,
                );

                field_processor::generate_from_my_field(field, &ctx)
            })
    }
}

mod tuple_impl_generator {
    use super::*;

    pub fn generate_tuple_implementations(
        name: &syn::Ident,
        fields_unnamed: &syn::FieldsUnnamed,
    ) -> proc_macro2::TokenStream {
        if fields_unnamed.unnamed.len() != 1 {
            panic!(
                "ProtoConvert only supports tuple structs with exactly one field, found {}",
                fields_unnamed.unnamed.len()
            );
        }

        let inner_type = &fields_unnamed.unnamed[0].ty;

        quote! {
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
        }
    }
}

mod attribute_parser {
    use super::*;

    #[derive(Debug, Default, Clone)]
    pub struct ProtoFieldMeta {
        pub expect: bool,
        pub error_fn: Option<String>,
        pub error_type: Option<String>,
        pub default_fn: Option<String>,
        pub optional: Option<bool>,
    }

    impl ProtoFieldMeta {
        pub fn from_field(field: &syn::Field) -> Result<Self, String> {
            let mut meta = ProtoFieldMeta::default();

            for attr in &field.attrs {
                if attr.path().is_ident("proto") {
                    if let Meta::List(meta_list) = &attr.meta {
                        let nested_metas: Result<Punctuated<Meta, Comma>, _> = Punctuated::parse_terminated
                            .parse2(meta_list.tokens.clone());

                        match nested_metas {
                            Ok(metas) => {
                                for nested_meta in metas {
                                    match nested_meta {
                                        Meta::Path(path) if path.is_ident("expect") => {
                                            meta.expect = true;
                                        },
                                        Meta::List(list) if list.path.is_ident("expect") => {
                                            // handle `expect(panic)` syntax
                                            meta.expect = true;
                                        },
                                        Meta::NameValue(nv) if nv.path.is_ident("optional") => {
                                            if let Expr::Lit(expr_lit) = &nv.value {
                                                if let Lit::Bool(lit_bool) = &expr_lit.lit {
                                                    meta.optional = Some(lit_bool.value);
                                                }
                                            }
                                        },
                                        Meta::NameValue(nv) if nv.path.is_ident("error_type") => {
                                            if let Expr::Path(expr_path) = &nv.value {
                                                meta.error_type = Some(quote!(#expr_path).to_string());
                                            }
                                        },
                                        Meta::NameValue(nv) if nv.path.is_ident("error_fn") => {
                                            if let Expr::Lit(expr_lit) = &nv.value {
                                                if let Lit::Str(lit_str) = &expr_lit.lit {
                                                    meta.error_fn = Some(lit_str.value());
                                                }
                                            }
                                        },
                                        Meta::NameValue(nv) if nv.path.is_ident("default_fn") || nv.path.is_ident("default") => {
                                            match &nv.value {
                                                Expr::Lit(expr_lit) => {
                                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                                        meta.default_fn = Some(lit_str.value());
                                                    }
                                                },
                                                Expr::Path(expr_path) => {
                                                    meta.default_fn = Some(quote!(#expr_path).to_string());
                                                },
                                                _ => {
                                                    panic!("default_fn value must be a string literal or path; e.g., default_fn = \"function_path\" or default_fn = function_path");
                                                },
                                            }
                                        },
                                        Meta::Path(path) if path.is_ident("default_fn") || path.is_ident("default") => {
                                            meta.default_fn = Some("Default::default".to_string());
                                        },
                                        _ => {
                                            // ignore other attributes for now
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                return Err(format!("Failed to parse proto attribute: {e}"));
                            },
                        }
                    }
                }
            }

            Ok(meta)
        }
    }

    pub fn get_proto_struct_error_type(attrs: &[Attribute]) -> Option<syn::Type> {
        for attr in attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                        .parse2(meta_list.tokens.clone())
                        .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                    for meta in nested_metas {
                        if let Meta::NameValue(meta_nv) = meta {
                            if meta_nv.path.is_ident("error_type") {
                                if let Expr::Path(expr_path) = &meta_nv.value {
                                    return Some(syn::Type::Path(syn::TypePath {
                                        qself: None,
                                        path: expr_path.path.clone(),
                                    }));
                                }
                                panic!("error_type value must be a type path; e.g., #[proto(error_type = MyError)]");
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_struct_level_error_fn(attrs: &[Attribute]) -> Option<String> {
        for attr in attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                        .parse2(meta_list.tokens.clone())
                        .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {e}"));
                    for meta in nested_metas {
                        if let Meta::NameValue(meta_nv) = meta {
                            if meta_nv.path.is_ident("error_fn") {
                                if let Expr::Lit(expr_lit) = &meta_nv.value {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        return Some(lit_str.value());
                                    }
                                }
                                panic!("error_fn value must be a string literal");
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_proto_module(attrs: &[Attribute]) -> Option<String> {
        for attr in attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
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

    pub fn get_proto_struct_rename(attrs: &[Attribute]) -> Option<String> {
        for attr in attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
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

    pub fn has_transparent_attr(field: &Field) -> bool {
        for attr in &field.attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
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

    pub fn get_proto_rename(field: &Field) -> Option<String> {
        for attr in &field.attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
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

    pub fn get_proto_derive_from_with(field: &Field) -> Option<String> {
        for attr in &field.attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                        .parse2(meta_list.tokens.clone())
                        .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                    for meta in nested_metas {
                        if let Meta::NameValue(meta_nv) = meta {
                            if meta_nv.path.is_ident("derive_from_with") {
                                if let Expr::Lit(expr_lit) = &meta_nv.value {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        return Some(lit_str.value());
                                    }
                                }
                                panic!("derive_from_with value must be a string literal, e.g., derive_from_with = \"path::to::function\"");
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_proto_derive_into_with(field: &Field) -> Option<String> {
        for attr in &field.attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                        .parse2(meta_list.tokens.clone())
                        .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                    for meta in nested_metas {
                        if let Meta::NameValue(meta_nv) = meta {
                            if meta_nv.path.is_ident("derive_into_with") {
                                if let Expr::Lit(expr_lit) = &meta_nv.value {
                                    if let Lit::Str(lit_str) = &expr_lit.lit {
                                        return Some(lit_str.value());
                                    }
                                }
                                panic!("derive_into_with value must be a string literal, e.g., derive_into_with = \"path::to::function\"");
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn has_proto_ignore(field: &Field) -> bool {
        for attr in &field.attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                        .parse2(meta_list.tokens.clone())
                        .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
                    for meta in nested_metas {
                        if let Meta::Path(path) = meta {
                            if path.is_ident("ignore") {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn has_proto_enum_attr(field: &syn::Field) -> bool {
        for attr in &field.attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
                if let Meta::List(meta_list) = &attr.meta {
                    let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
                        .parse2(meta_list.tokens.clone())
                        .unwrap_or_else(|e| panic!("Failed to parse meta list: {e}"));

                    for meta in nested_metas {
                        if let Meta::Path(path) = meta {
                            if path.is_ident("enum") {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }
}

mod type_analysis {
    use super::*;
    use constants::PRIMITIVE_TYPES;

    pub fn is_option_type(ty: &Type) -> bool {
        matches!(ty, Type::Path(type_path) if type_path.path.segments.first().map(|s| s.ident == "Option").unwrap_or(false))
    }

    pub fn get_inner_type_from_option(ty: &Type) -> Option<Type> {
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

    pub fn is_vec_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Vec" {
                return true;
            }
        }
        false
    }

    pub fn get_inner_type_from_vec(ty: &Type) -> Option<Type> {
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

    pub fn is_primitive_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            type_path.path.segments.len() == 1 &&
                PRIMITIVE_TYPES.iter().any(|&p| type_path.path.segments[0].ident == p)
        } else {
            false
        }
    }

    pub fn is_proto_type_with_module(ty: &Type, proto_module: &str) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.first() {
                return segment.ident == proto_module;
            }
        }
        false
    }

    pub fn is_enum_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            // Skip primitive types, collections, and proto types
            if is_primitive_type(ty) || is_vec_type(ty) || is_option_type(ty) {
                return false;
            }

            let is_proto_type = type_path.path.segments.first()
                .map(|segment| segment.ident == "proto")
                .unwrap_or(false);

            if is_proto_type {
                return false;
            }

            // Single-segment non-primitive types are likely enums or simple structs
            type_path.path.segments.len() == 1
        } else {
            false
        }
    }

    pub fn is_enum_type_with_explicit_attr(ty: &Type, field: &Field) -> bool {
        attribute_parser::has_proto_enum_attr(field) || is_enum_type(ty)
    }
}

mod field_analysis {
    use crate::expect_analysis::ExpectMode;
    use super::*;

    #[derive(Clone)]
    pub struct FieldProcessingContext<'a> {
        pub struct_name: &'a syn::Ident,
        pub field_name: &'a syn::Ident,
        pub field_type: &'a syn::Type,
        pub proto_field_ident: syn::Ident,
        pub proto_meta: attribute_parser::ProtoFieldMeta,
        pub expect_mode: ExpectMode,
        pub has_default: bool,
        pub default_fn: Option<String>,
        pub error_name: &'a syn::Ident,
        pub struct_level_error_type: &'a Option<syn::Type>,
        pub struct_level_error_fn: &'a Option<String>,
        pub proto_module: &'a str,
        pub proto_name: &'a str,
    }

    impl<'a> std::fmt::Debug for FieldProcessingContext<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FieldProcessingContext")
            .field("struct_name", &self.struct_name)
            .field("field_name", &self.field_name)
            .field("proto_field_ident", &self.proto_field_ident)
            .field("proto_meta", &self.proto_meta)
            .field("expect_mode", &self.expect_mode)
            .field("has_default", &self.has_default)
            .field("default_fn", &self.default_fn)
            .field("error_name", &self.error_name)
            .field("struct_level_error_fn", &self.struct_level_error_fn)
            .field("proto_module", &self.proto_module)
            .field("proto_name", &self.proto_name)
            .finish()
        }
    }

    impl<'a> FieldProcessingContext<'a> {
        pub fn new(
            struct_name: &'a syn::Ident,
            field: &'a syn::Field,
            error_name: &'a syn::Ident,
            struct_level_error_type: &'a Option<syn::Type>,
            struct_level_error_fn: &'a Option<String>,
            proto_module: &'a str,
            proto_name: &'a str,
        ) -> Self {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
            let expect_mode = ExpectMode::from_field_meta(field, &proto_meta);
            let has_default = proto_meta.default_fn.is_some();
            let default_fn = proto_meta.default_fn.clone();

            let proto_field_ident = if let Some(rename) = attribute_parser::get_proto_rename(field) {
                syn::Ident::new(&rename, proc_macro2::Span::call_site())
            } else {
                field_name.clone()
            };

            Self {
                struct_name,
                field_name,
                field_type,
                proto_field_ident,
                proto_meta,
                expect_mode,
                has_default,
                default_fn,
                error_name,
                struct_level_error_type,
                struct_level_error_fn,
                proto_module,
                proto_name,
            }
        }
    }

    pub use proto_inspection::detect_proto_field_optionality;

    pub fn is_optional_proto_field_for_ctx(ctx: &FieldProcessingContext, field: &syn::Field) -> bool {
        // 1) check if user explicitly specified optionality
        if let Some(explicit) = ctx.proto_meta.optional {
            explicit
        } else if let Some(build_detected) = detect_proto_field_optionality(ctx) {
            // 2) try build-time metadata detection
            build_detected
        } else {
            // 3) fallback to original analysis
            is_optional_proto_field(ctx.struct_name, field, ctx.proto_name)
        }
    }

    fn is_optional_proto_field(name: &syn::Ident, field: &syn::Field, proto_name: &str) -> bool {
        let field_name = field.ident.as_ref().unwrap();

        if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field) {
            if debug::should_output_debug(name, &field_name) {
                eprintln!("=== PROTO META DEBUG for {}.{} ===", proto_name, field_name);
                eprintln!("  proto_meta.optional: {:?}", proto_meta.optional);
            }

            if let Some(optional) = proto_meta.optional {
                if debug::should_output_debug(name, &field_name) {
                    eprintln!("  RETURNING explicit optional = {optional}");
                }
                return optional;
            }
        }

        false
    }

    /// Build-time metadata integration for proto field analysis.
    ///
    /// This module provides build-time metadata detection using environment variables
    /// set by the build script. This approach provides zero-setup experience for
    /// external developers.
    ///
    /// ## Migration to Prost-Style Approach
    ///
    /// If you encounter limitations with environment variables (see below), you can
    /// migrate to a prost-style file inclusion approach:
    ///
    /// ### When to Consider Migration:
    /// - **Large proto files**: >500 messages or >32KB of metadata (Windows env var limit)
    /// - **Complex metadata**: Need structured data beyond simple optional/required flags
    /// - **Debugging needs**: Want to inspect generated metadata files directly
    /// - **Build reproducibility**: Want metadata as part of source artifacts
    ///
    /// ### Migration Steps:
    /// 1. Change `proto_convert_build`'s `write_metadata_file()` to generate Rust code instead of env vars
    /// 2. Update this module to use `include!(concat!(env!("OUT_DIR"), "/file.rs"))`
    /// 3. Add consumer boilerplate to include generated metadata
    /// 4. Update data structures to use static HashMap instead of env var lookup
    ///
    /// See prost's implementation for reference patterns.
    mod proto_inspection {
        use crate::{expect_analysis, field_analysis, type_analysis};

        /// Build-time metadata provider trait.
        ///
        /// Defines the metatdata inclusion mechanism
        trait MetadataProvider {
            /// get field metadata for a specific message and field.
            fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata>;
        }

        #[allow(dead_code)]
        #[derive(Debug, Clone)]
        pub struct ProtoFieldMetadata {
            pub optional: bool,
            pub repeated: bool,
        }

        /// Evironment variable-based metadata provider.
        ///
        /// Reads metadata from environment variables set by build script:
        /// - Format: `PROTO_FIELD_{MESSAGE}_{FIELD}={optional|required|repeated}`
        /// - Example: `PROTO_FIELD_USER_NAME=optional`
        #[cfg(feature = "build-time-metadata")]
        struct EnvVarMetadataProvider;

        #[cfg(feature = "build-time-metadata")]
        impl MetadataProvider for EnvVarMetadataProvider {
            fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata> {
                let env_key = format!(
                    "PROTO_FIELD_{}_{}",
                    message.to_uppercase(), field.to_uppercase()
                );

                match std::env::var(env_key).ok()?.as_str() {
                    "optional" => Some(ProtoFieldMetadata { optional: true, repeated: false }),
                    "repeated" => Some(ProtoFieldMetadata { optional: false, repeated: true }),
                    "required" => Some(ProtoFieldMetadata { optional: false, repeated: false }),
                    _ => None,
                }
            }
        }

        /// Prost-style file inclusion metadat provider (for future migration).
        ///
        /// When migrated, this would include generated Rust code from OUT_DIR
        /// and provide static HashMap lookup instead of runtime env var access.
        #[cfg(feature = "build-time-metadata")]
        #[allow(dead_code)]
        struct FileInclusionMetadataProvider {
            // placeholder for migration - would facilitate:
            // include!(concat!(env!("OUT_DIR"), "/proto_field_metadata.rs"));
            // static METADATA: LazyLock<HashMap<(String, String), ProtoFieldMetadata>> = ...;
        }

        /// Fallback provider when build-time metadat is disabled.
        #[cfg(not(feature = "build-time-metadata"))]
        struct NoOpMetadataProvider;

        #[cfg(not(feature = "build-time-metadata"))]
        impl MetadataProvider for NoOpMetadataProvider {
            fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata> {
                None
            }
        }

        /// Main entry point for proto field optionality detection.
        ///
        /// This function orchestrates multiple detection strategies in order of reliability:
        /// 1. explicit user annotation (`#[proto(optional = true)]`)
        /// 2. build-time metadata (this module)
        /// 3. type-based inference (Option<T> = optional)
        /// 4. usage pattern inference (expect/default = optional)
        pub fn detect_proto_field_optionality(
            ctx: &field_analysis::FieldProcessingContext,
        ) -> Option<bool> {
            // 1. explicit user annotation takes precedence
            if let Some(explicit) = ctx.proto_meta.optional {
                return Some(explicit);
            }

            // 2. build-time metadata (reliable when available)
            if let Some(build_time) = try_build_time_metadata(ctx) {
                return Some(build_time);
            }

            // 3. infer from rust type structure
            if let Some(type_based) = infer_from_rust_type(ctx) {
                return Some(type_based);
            }

            // 4. infer from usage patterns (explicit/default)
            if let Some(usage_based) = infer_from_usage_patterns(ctx) {
                return Some(usage_based);
            }

            // 5. cannot determine - emit helpful warning
            emit_metadata_suggestion(ctx);
            None
        }

        /// Try to get field metadata from build-time generation.
        ///
        /// This function abstracts the metadata provider to make migration easier.
        /// Currently used environment variables, but can be easily switched to
        /// file inclusion approach (but that requires more work by app developer; e.g., `prost` includes.
        fn try_build_time_metadata(ctx: &field_analysis::FieldProcessingContext) -> Option<bool> {
            #[cfg(feature = "build-time-metadata")]
            {
                if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
                    let env_key = format!(
                        "PROTO_FIELD_{}_{}",
                        ctx.proto_name.to_uppercase(), ctx.field_name.to_string().to_uppercase()
                    );
                    let env_value = std::env::var(&env_key).ok();
                    eprintln!("=== BUILD-TIME METADATA DEBUG for {}.{} ===", ctx.struct_name, ctx.field_name);
                    eprintln!("  env_key: {}", env_key);
                    eprintln!("  env_value: {:?}", env_value);
                }

                let metadata = EnvVarMetadataProvider::get_field_metadata(
                    ctx.proto_name,
                    &ctx.field_name.to_string(),
                )?;

                if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
                    eprintln!("  metadata.optional: {}", metadata.optional);
                    eprintln!("=== END BUILD-TIME METADATA DEBUG ===");
                }

                Some(metadata.optional)
            }

            #[cfg(not(feature = "build-time-metadata"))]
            {
                let _ = ctx; // avoid unused variable warnings
                None
            }
        }

        /// Infer optionality from Rust type structure.
        ///
        /// - `Option<T>` typically maps to optional proto fields
        /// - `Vec<T>` typically maps to repeated proto fields (not optional)
        fn infer_from_rust_type(ctx: &field_analysis::FieldProcessingContext) -> Option<bool> {
            let rust_is_optional = type_analysis::is_option_type(ctx.field_type);
            let rust_is_vec = type_analysis::is_vec_type(ctx.field_type);

            if rust_is_vec {
                // Vec<T> typically maps to repeated proto field (not optional)
                Some(false)
            } else if rust_is_optional {
                // Option<T> typically maps to optional proto fields
                Some(true)
            } else {
                // non-optional rust type could map to either required or optional proto field
                None
            }
        }

        /// Infer from usage patterns (expect/default attributes).
        ///
        /// If user provides `expect()` or `default()`, the proto field is likely optional
        /// since these only make sense for fields that might be missing.
        fn infer_from_usage_patterns(ctx: &field_analysis::FieldProcessingContext) -> Option<bool> {
            let has_expect = !matches!(ctx.expect_mode, expect_analysis::ExpectMode::None);
            let has_default = ctx.has_default;

            if has_expect || has_default {
                // if user provides expect() or default(), proto field is likely optional
                Some(true)
            } else {
                None
            }
        }

        /// Emit suggestion for adding build-time metadata when detection fails.
        ///
        /// This helps developers understand when they might benefit from proto
        /// file analysis instead of relying on heuristics.
        fn emit_metadata_suggestion(_ctx: &field_analysis::FieldProcessingContext) {
            // Could emit compiler notes here in the future:
            // - Suggest enabling build-time-metadata feature
            // - Suggest adding explicit #[proto(optional = true/false)]
            // - Point to documentation for proto file analysis setup

            // Note: proc_macro::Diagnostic is not stable yet, so this is placeholder

        //     let struct_name = ctx.struct_name;
        //     let field_name = ctx.field_name;
        //     let proto_name = ctx.proto_name;
        //
        //     // only emit once per compilation per struct
        //     static mut WARNED_STRUCTS: std::collections::HashSet<String> = std::collections::HashSet::new();
        //     let struct_key = format!("{struct_name}::{proto_name}");
        //
        //     unsafe {
        //         if !WARNED_STRUCTS.contains(&struct_key) {
        //             WARNED_STRUCTS.insert(struct_key);
        //
        //              // only in nightly now
        //             proc_macro::Diagnostics::spanned(
        //                 proc_macro2::Span::call_site().unwrap(),
        //                 proc_macro::Level::Note,
        //                 format!(
        //                     "ProtoConvert: Could not determine optionality for field '{}' in '{}'. \
        //                     For better detection, add to build.rs: \
        //                     proto_convert_build::generate_proto_metadata(&[\"path/to/{}.proto\"])",
        //                     field_name, struct_name, proto_name.to_lowercase()
        //                 )
        //             ).emit();
        //         }
        //     }
        }
    }
}

mod field_processor {
    use super::*;
    use field_analysis::FieldProcessingContext;
    use crate::expect_analysis::ExpectMode;

    pub fn generate_from_proto_field(field: &syn::Field, ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        if debug::should_output_debug(ctx.struct_name, ctx.field_name) {
            debug::debug_field_analysis(ctx.struct_name, ctx.field_name, "GENERATE_FROM_PROTO_FIELD DEBUG", &[
                ("field_type", quote!(#(ctx.field_type)).to_string()),
                ("has_proto_ignore", attribute_parser::has_proto_ignore(field).to_string()),
                ("has_transparent_attr", attribute_parser::has_transparent_attr(field).to_string()),
                ("is_option_type", type_analysis::is_option_type(ctx.field_type).to_string()),
                ("is_vec_type", type_analysis::is_vec_type(ctx.field_type).to_string()),
            ]);
        }

        if attribute_parser::has_proto_ignore(field) {
            return generate_ignored_field(ctx);
        }

        let derive_from_with = attribute_parser::get_proto_derive_from_with(field);
        if let Some(from_with_path) = derive_from_with {
            return generate_derive_from_with_field(ctx, &from_with_path)
        }

        if attribute_parser::has_transparent_attr(field) {
            return generate_transparent_field(ctx);
        }

        if type_analysis::is_option_type(ctx.field_type) {
            return generate_option_field(ctx);
        }

        if type_analysis::is_vec_type(ctx.field_type) {
            return generate_vec_field(ctx);
        }

        if let syn::Type::Path(_) = ctx.field_type {
            return generate_path_type_field(ctx, field);
        }

        panic!("Only path types are supported for field '{}'", ctx.field_name);
    }

    pub fn generate_from_my_field(field: &syn::Field, ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        if attribute_parser::has_proto_ignore(field) {
            // Ignored fields are not included in proto struct
            return quote!{};
        }

        let derive_into_with = attribute_parser::get_proto_derive_into_with(field);
        if let Some(into_with_path) = derive_into_with {
            return generate_derive_into_with_field(ctx, &into_with_path);
        }

        if attribute_parser::has_transparent_attr(field) {
            return generate_transparent_from_my_field(ctx, field);
        }

        if type_analysis::is_option_type(ctx.field_type) {
            return generate_option_from_my_field(ctx);
        }

        if type_analysis::is_vec_type(ctx.field_type) {
            return generate_vec_from_my_field(ctx);
        }

        if let syn::Type::Path(_) = ctx.field_type {
            return generate_path_type_from_my_field(ctx, field);
        }

        panic!("Only path types are supported for field '{}'", ctx.field_name);
    }

    fn generate_ignored_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        if let Some(default_fn_name) = &ctx.default_fn {
            let default_fn_path: syn::Path = syn::parse_str(&default_fn_name)
                .expect("Failed to parse default_fn function path");
            quote! { #field_name: #default_fn_path() }
        } else {
            quote! { #field_name: Default::default() }
        }
    }

    fn generate_derive_from_with_field(ctx: &FieldProcessingContext, from_with_path: &str) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let from_with_path: syn::Path = syn::parse_str(&from_with_path).expect("Failed to parse derive_from_with path");
        quote! {
            #field_name: #from_with_path(proto_struct.#proto_field_ident)
        }
    }

    fn generate_transparent_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let field_type = ctx.field_type;

        match ctx.expect_mode {
            ExpectMode::Panic => {
                quote! {
                    #field_name: <#field_type>::from(
                        proto_struct.#proto_field_ident
                            .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                    )
                }
            },
            ExpectMode::Error => {
                error_handler::generate_error_handling(
                    field_name,
                    &proto_field_ident,
                    field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            },
            ExpectMode::None => {
                if ctx.has_default {
                    let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                    quote! {
                        #field_name: <#field_type>::from(
                            proto_struct.#proto_field_ident
                                .unwrap_or_else(|| #default_expr)
                        )
                    }
                } else {
                    quote! {
                        #field_name: <#field_type>::from(proto_struct.#proto_field_ident)
                    }
                }
            },
        }
    }

    fn generate_option_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let field_type = ctx.field_type;
        let inner_type = type_analysis::get_inner_type_from_option(field_type).unwrap();

        match ctx.expect_mode {
            ExpectMode::Panic => {
                quote! {
                    #field_name: Some(proto_struct.#proto_field_ident
                        .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                        .into())
                }
            },
            ExpectMode::Error => {
                error_handler::generate_error_handling(
                    field_name,
                    &proto_field_ident,
                    field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            },
            ExpectMode::None => {
                if ctx.has_default {
                    let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .map(#inner_type::from)
                            .map(Some)
                            .unwrap_or_else(|| #default_expr)
                    }
                } else if type_analysis::is_vec_type(&inner_type) {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                    }
                } else {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident.map(Into::into)
                    }
                }
            },
        }
    }

    fn generate_vec_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let field_type = ctx.field_type;

        if ctx.has_default {
            let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
            match ctx.expect_mode {
                ExpectMode::Panic => {
                    quote! {
                        #field_name: if proto_struct.#proto_field_ident.is_empty() {
                            #default_expr
                        } else {
                            proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                        }
                    }
                },
                ExpectMode::Error => {
                    error_handler::generate_error_handling(
                        field_name,
                        &proto_field_ident,
                        field_type,
                        &ctx.proto_meta,
                        ctx.error_name,
                        ctx.struct_level_error_type,
                        ctx.struct_level_error_fn,
                    )
                },
                ExpectMode::None => {
                    quote! {
                        #field_name: if proto_struct.#proto_field_ident.is_empty() {
                            #default_expr
                        } else {
                            proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                        }
                    }
                },
            }
        } else {
            if let Some(inner_type) = type_analysis::get_inner_type_from_vec(field_type) {
                if type_analysis::is_proto_type_with_module(&inner_type, ctx.proto_module) {
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
        }
    }

    fn generate_path_type_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let field_type = ctx.field_type;

        if let syn::Type::Path(type_path) = field_type {
            let is_primitive = type_analysis::is_primitive_type(field_type);
            let is_proto_type = type_path.path.segments.first()
                .is_some_and(|segment| segment.ident == ctx.proto_module);
            let is_enum = type_analysis::is_enum_type_with_explicit_attr(field_type, field);

            if debug::should_output_debug(ctx.struct_name, ctx.field_name) {
                debug::debug_field_analysis(ctx.struct_name, ctx.field_name, "PATH TYPE FIELD DEBUG", &[
                    ("is_primitive", is_primitive.to_string()),
                    ("is_proto_type", is_proto_type.to_string()),
                    ("is_enum", is_enum.to_string()),
                    ("proto_module", ctx.proto_module.to_string()),
                    ("field_type", quote!(#field_type).to_string()),
                ]);
            }

            if is_enum {
                return generate_enum_field(ctx, field);
            } else if is_primitive {
                return generate_primitive_field(ctx, field);
            } else if is_proto_type {
                return generate_proto_type_field(ctx, field);
            } else {
                return generate_custom_type_field(ctx, field);
            }
        }

        panic!("Only path types are supported for field '{}'", field_name);
    }

    fn generate_enum_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let field_type = ctx.field_type;

        let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

        if debug::should_output_debug(ctx.struct_name, field_name) {
            debug::debug_field_analysis(ctx.struct_name, field_name, "ENUM FIELD DEBUG", &[
                ("proto_is_optional (calculated)", proto_is_optional.to_string()),
                ("expect_mode", format!("{:?}", ctx.expect_mode)),
                ("has_default", ctx.has_default.to_string()),
                ("proto_field_ident", proto_field_ident.to_string()),
                ("field_type", quote!(#field_type).to_string()),
            ]);
        }

        let generated_code = if proto_is_optional {
            match ctx.expect_mode {
                ExpectMode::Panic => {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .map(|v| v.into())
                            .unwrap_or_else(|| panic!("Field {} is required", stringify!(#proto_field_ident)))
                    }
                },
                ExpectMode::Error => {
                    error_handler::generate_error_handling(
                        field_name,
                        &proto_field_ident,
                        field_type,
                        &ctx.proto_meta,
                        ctx.error_name,
                        ctx.struct_level_error_type,
                        ctx.struct_level_error_fn,
                    )
                },
                ExpectMode::None => {
                    if ctx.has_default {
                        let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                        quote! {
                            #field_name: proto_struct.#proto_field_ident
                                .map(#field_type::from)
                                .unwrap_or_else(|| #default_expr)
                        }
                    } else {
                        quote! {
                            #field_name: #field_type::from(
                                proto_struct.#proto_field_ident
                                    .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                            )
                        }
                    }
                },
            }
        } else {
            // direct conversion for non-optional enum fields
            quote! {
                // #field_name: proto_struct.#proto_field_ident.into()
                #field_name: #field_type::from(proto_struct.#proto_field_ident)
            }
        };

        debug::debug_generated_code(ctx.struct_name, field_name, &generated_code, "enum field from_proto");
        generated_code
    }

    fn generate_primitive_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let field_type = ctx.field_type;

        let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

        let rust_is_option = type_analysis::is_option_type(field_type);

        if debug::should_output_debug(ctx.struct_name, field_name) {
            debug::debug_field_analysis(ctx.struct_name, field_name, "PRIMITIVE FIELD DEBUG", &[
                ("proto_is_optional (calculated)", proto_is_optional.to_string()),
                ("rust_is_option", rust_is_option.to_string()),
                ("has_default", ctx.has_default.to_string()),
                ("expect_mode", format!("{:?}", ctx.expect_mode)),
                ("proto_field_ident", proto_field_ident.to_string()),
                ("default_fn", format!("{:?}", ctx.default_fn)),
                ("Expected generated code", format!("{}:proto_struct.{}.unwrap_or(...)", field_name, proto_field_ident)),
            ]);
        }

        let generated_code = match ctx.expect_mode {
            ExpectMode::Panic => {
                if rust_is_option {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .map(|v| Some(v))
                            .unwrap_or_else(|| panic!("Field {} is required", stringify!(#proto_field_ident)))
                    }
                } else {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .unwrap_or_else(|| panic!("Field {} is required", stringify!(#proto_field_ident)))
                    }
                }
            },
            ExpectMode::Error => {
                error_handler::generate_error_handling(
                    field_name,
                    &proto_field_ident,
                    field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            },
            ExpectMode::None => {
                if ctx.has_default {
                    let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                    if rust_is_option {
                        quote! {
                            #field_name: if proto_struct.#proto_field_ident == Default::default() {
                                Some(#default_expr)
                            } else {
                                Some(proto_struct.#proto_field_ident)
                            }
                        }
                    } else {
                        if proto_is_optional {
                            quote! {
                                #field_name: proto_struct.#proto_field_ident
                                    .unwrap_or_else(|| #default_expr)
                            }
                        } else {
                            quote! {
                                //DMR: determine which better: pros/cons
                                #field_name: proto_struct.#proto_field_ident.into()
                                // #field_name: if proto_struct.#proto_field_ident.is_empty() {
                                //     #default_expr
                                // } else {
                                //     proto_struct.#proto_field_ident.into()
                                // }
                            }
                        }
                    }
                } else {
                    // No default handling
                    if rust_is_option {
                        quote! {
                            #field_name: Some(proto_struct.#proto_field_ident)
                        }
                    } else {
                        if proto_is_optional {
                            quote! {
                                #field_name: proto_struct.#proto_field_ident
                                    .unwrap_or_default()
                            }
                        } else {
                            quote! {
                                #field_name: proto_struct.#proto_field_ident
                            }
                        }
                    }
                }
            },
        };

        debug::debug_generated_code(ctx.struct_name, field_name, &generated_code, "primitive field from_proto");
        generated_code
    }

    fn generate_proto_type_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        match ctx.expect_mode {
            ExpectMode::Panic => {
                let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

                if proto_is_optional {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                            .into()
                    }
                } else {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident.into()
                    }
                }
            },
            ExpectMode::Error => {
                error_handler::generate_error_handling(
                    field_name,
                    &proto_field_ident,
                    ctx.field_type,
                    &ctx.proto_meta,
                    ctx.error_name,
                    ctx.struct_level_error_type,
                    ctx.struct_level_error_fn,
                )
            },
            ExpectMode::None => {
                if ctx.has_default {
                    let default_expr = generate_default_value(ctx.field_type, ctx.default_fn.as_deref());
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .unwrap_or_else(|| #default_expr)
                    }
                } else {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                    }
                }
            },
        }
    }

    fn generate_custom_type_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let field_type = ctx.field_type;

        let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

        // custom types - check if proto field is optional
        if proto_is_optional {
            // proto field is optional (Option<T>), Rust field is T
            match ctx.expect_mode {
                ExpectMode::Panic => {
                    quote! {
                        #field_name: proto_struct.#proto_field_ident
                            .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                            .into()
                    }
                },
                ExpectMode::Error => {
                    error_handler::generate_error_handling(
                        field_name,
                        &proto_field_ident,
                        field_type,
                        &ctx.proto_meta,
                        ctx.error_name,
                        ctx.struct_level_error_type,
                        ctx.struct_level_error_fn,
                    )
                },
                ExpectMode::None => {
                    if ctx.has_default {
                        let default_expr = generate_default_value(field_type, ctx.default_fn.as_deref());
                        quote! {
                            #field_name: proto_struct.#proto_field_ident
                                .map(#field_type::from)
                                .unwrap_or_else(|| #default_expr)
                        }
                    } else {
                        quote! {
                            #field_name: proto_struct.#proto_field_ident
                                .map(Into::into)
                                .expect(&format!("Field {} is required", stringify!(#proto_field_ident)))
                        }
                    }
                },
            }
        } else {
            // non-optional proto field - direct conversion
            quote! {
                #field_name: proto_struct.#proto_field_ident.into()
            }
        }
    }

    fn generate_default_value(field_type: &syn::Type, default_fn: Option<&str>) -> proc_macro2::TokenStream {
        if let Some(default_fn_name) = default_fn {
            let default_fn_path: syn::Path = syn::parse_str(&default_fn_name)
                .expect("Failed to parse default_fn path");
            quote! { #default_fn_path() }
        } else {
            quote! { <#field_type as Default>::default() }
        }
    }

    fn generate_derive_into_with_field(ctx: &FieldProcessingContext, into_with_path: &str) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let into_with_path: syn::Path = syn::parse_str(&into_with_path)
            .expect("Failed to parse derive_into_with path");

        quote! {
            #proto_field_ident: #into_with_path(my_struct.#field_name)
        }
    }

    fn generate_transparent_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

        if proto_is_optional {
            quote! {
                #proto_field_ident: Some(my_struct.#field_name.into())
            }
        } else {
            quote! {
                #proto_field_ident: my_struct.#field_name.into()
            }
        }
    }

    fn generate_option_from_my_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let inner_type = type_analysis::get_inner_type_from_option(ctx.field_type).unwrap();

        if type_analysis::is_vec_type(&inner_type) {
            quote! {
                #proto_field_ident: my_struct.#field_name
                    .map(|vec| vec.into_iter().map(Into::into).collect())
            }
        } else {
            quote! {
                #proto_field_ident: my_struct.#field_name.map(Into::into)
            }
        }
    }

    fn generate_vec_from_my_field(ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        if let Some(inner_type) = type_analysis::get_inner_type_from_vec(ctx.field_type) {
            if type_analysis::is_proto_type_with_module(&inner_type, ctx.proto_module) {
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
    }

    fn generate_path_type_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;

        if let syn::Type::Path(type_path) = ctx.field_type {
            let is_primitive = type_analysis::is_primitive_type(ctx.field_type);
            let is_proto_type = type_path.path.segments.first()
                .is_some_and(|segment| segment.ident == ctx.proto_module);

            return if type_analysis::is_enum_type_with_explicit_attr(ctx.field_type, field) {
                generate_enum_from_my_field(ctx, field)
            } else if is_primitive {
                generate_primitive_from_my_field(ctx, field)
            } else if is_proto_type {
                generate_proto_type_from_my_field(ctx, field)
            } else {
                generate_custom_type_from_my_field(ctx, field)
            }
        }

        panic!("Only path types are supported for field '{}'", field_name);
    }

    fn generate_enum_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

        if proto_is_optional {
            quote! {
                #proto_field_ident: Some(my_struct.#field_name.into())
            }
        } else {
            quote! {
                #proto_field_ident: my_struct.#field_name.into()
            }
        }
    }

    fn generate_primitive_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let rust_is_option = type_analysis::is_option_type(ctx.field_type);

        let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

        if debug::should_output_debug(ctx.struct_name, field_name) {
            eprintln!("=== FROM_MY_FIELDS PRIMITIVE ===");
            eprintln!("  proto_is_optional: {}", proto_is_optional);
        }

        match (rust_is_option, proto_is_optional) {
            (true, false) => quote! {
                #proto_field_ident: my_struct.#field_name.unwrap_or_default()
            },
            (false, true) => quote! {
                #proto_field_ident: Some(my_struct.#field_name)
            },
            (true, true) => quote! {
                #proto_field_ident: my_struct.#field_name
            },
            (false, false) => quote! {
                #proto_field_ident: my_struct.#field_name
            },
        }
    }

    fn generate_proto_type_from_my_field(ctx: &FieldProcessingContext, _field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        quote! {
            #proto_field_ident: Some(my_struct.#field_name)
        }
    }

    fn generate_custom_type_from_my_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;

        let proto_is_optional = field_analysis::is_optional_proto_field_for_ctx(ctx, field);

        // Check if proto field is optional before wrapping in Some()
        if proto_is_optional {
            quote! {
                #proto_field_ident: Some(my_struct.#field_name.into())
            }
        } else {
            quote! {
                #proto_field_ident: my_struct.#field_name.into()
            }
        }
    }
}

mod enum_processor {
    use super::*;

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
                    dbg!("DEBUG: Converting i32 {} to {}", value, stringify!(#name));
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
        variants.iter().map(|variant| {
            let variant_ident = &variant.ident;
            let variant_str = variant_ident.to_string();
            let screaming_variant = utils::to_screaming_snake_case(&variant_str);
            let prefixed_candidate = format!("{}_{}", enum_prefix, screaming_variant);
            let prefixed_candidate_lit = syn::LitStr::new(&prefixed_candidate, Span::call_site());

            quote! {
                #name::#variant_ident => <#proto_enum_path>::from_str_name(#prefixed_candidate_lit)
                    .unwrap_or_else(|| panic!("No matching proto variant for {rust_enum:?}")),
            }
        }).collect()
    }
}

mod error_analysis {
    use super::*;
    use crate::expect_analysis::ExpectMode;

    /// Analyzes fields to determine if TryFrom trait is needed
    pub fn requires_try_from(
        fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    ) -> bool {
        fields.iter().any(|field| {
            if attribute_parser::has_proto_ignore(field) {
                false
            } else {
                let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
                let expect_mode = ExpectMode::from_field_meta(field, &proto_meta);
                matches!(expect_mode, ExpectMode::Error)
            }
        })
    }

    /// Analyzes fields to determine if default error type generation is needed
    pub fn requires_default_error_type(
        fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
        struct_level_error_type: &Option<syn::Type>,
    ) -> bool {
        fields.iter().any(|field| {
            if attribute_parser::has_proto_ignore(field) {
                return false;
            }
            let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
            if matches!(ExpectMode::from_field_meta(field, &proto_meta), ExpectMode::Error) {
                let effective_error_type = error_types::get_effective_error_type(&proto_meta, struct_level_error_type);
                effective_error_type.is_none()
            } else {
                false
            }
        })
    }

    /// Comprehensive analysis of error requirements for a struct
    pub struct ErrorRequirements {
        pub needs_try_from: bool,
        pub needs_default_error: bool,
        pub needs_error_conversions: bool,
    }

    pub fn analyze_error_requirements(
        fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
        struct_level_error_type: &Option<syn::Type>,
    ) -> ErrorRequirements {
        let needs_try_from = requires_try_from(fields);
        let needs_default_error = requires_default_error_type(fields, struct_level_error_type);
        let needs_error_conversions = needs_try_from && needs_default_error && struct_level_error_type.is_none();

        ErrorRequirements {
            needs_try_from,
            needs_default_error,
            needs_error_conversions,
        }
    }
}

mod error_types {
    use super::*;

    /// Generates the default error name for a struct
    pub fn default_error_name(struct_name: &syn::Ident) -> syn::Ident {
        syn::Ident::new(
            &format!("{struct_name}{}", crate::constants::DEFAULT_CONVERSION_ERROR_SUFFIX),
            struct_name.span()
        )
    }

    /// Determines the effective error type for a field
    pub fn get_effective_error_type(
        proto_meta: &attribute_parser::ProtoFieldMeta,
        struct_level_error_type: &Option<syn::Type>
    ) -> Option<syn::Type> {
        if let Some(field_error_type) = &proto_meta.error_type {
            return Some(syn::parse_str(field_error_type)
                .expect("Failed to parse field-level error_type"));
        }

        struct_level_error_type.clone()
    }

    /// Determines the actual error type to use in trait implementations
    pub fn get_actual_error_type(
        needs_try_from: bool,
        struct_level_error_type: &Option<syn::Type>,
        error_name: &syn::Ident,
    ) -> syn::Type {
        if needs_try_from {
            struct_level_error_type.clone().unwrap_or_else(|| {
                syn::Type::Path(syn::TypePath {
                    qself: None,
                    path: syn::Path::from(error_name.clone()),
                })
            })
        } else {
            syn::parse_str("String").unwrap()
        }
    }

    /// Generates the conversion error enum definition
    pub fn generate_conversion_error_enum(struct_name: &syn::Ident) -> proc_macro2::TokenStream {
        let error_name = default_error_name(struct_name);

        quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub enum #error_name {
                MissingField(String),
            }

            impl std::fmt::Display for #error_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::MissingField(field) => write!(f, "Missing required field: {field}"),
                    }
                }
            }

            impl std::error::Error for #error_name {}
        }
    }

    /// Generates error conversion implementations
    pub fn generate_error_conversions(error_name: &syn::Ident) -> proc_macro2::TokenStream {
        quote! {
            impl From<String> for #error_name {
                fn from(err: String) -> Self {
                    Self::MissingField(err)
                }
            }
        }
    }
}

mod error_codegen {
    use super::*;

    /// Generates error handling code for a specific field
    pub fn generate_error_handling(
        field_name: &syn::Ident,
        proto_field_ident: &syn::Ident,
        field_type: &syn::Type,
        proto_meta: &attribute_parser::ProtoFieldMeta,
        error_name: &syn::Ident,
        _struct_level_error_type: &Option<syn::Type>,
        struct_level_error_fn: &Option<String>,
    ) -> proc_macro2::TokenStream {
        let is_rust_optional = type_analysis::is_option_type(field_type);
        let error_fn_to_use = proto_meta.error_fn.as_ref().or(struct_level_error_fn.as_ref());

        if let Some(error_fn) = error_fn_to_use {
            generate_custom_error_handling(field_name, proto_field_ident, is_rust_optional, error_fn)
        } else {
            generate_default_error_handling(field_name, proto_field_ident, is_rust_optional, error_name)
        }
    }

    /// Generates error handling using a custom error function
    fn generate_custom_error_handling(
        field_name: &syn::Ident,
        proto_field_ident: &syn::Ident,
        is_rust_optional: bool,
        error_fn: &str,
    ) -> proc_macro2::TokenStream {
        let error_fn_path: syn::Path = syn::parse_str(error_fn)
            .expect("Failed to parse error function path");

        if is_rust_optional {
            quote! {
                #field_name: Some(proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_fn_path(stringify!(#proto_field_ident)))?
                    .into())
            }
        } else {
            quote! {
                #field_name: proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_fn_path(stringify!(#proto_field_ident)))?
                    .into()
            }
        }
    }

    /// Generates error handling using the default error type
    fn generate_default_error_handling(
        field_name: &syn::Ident,
        proto_field_ident: &syn::Ident,
        is_rust_optional: bool,
        error_name: &syn::Ident,
    ) -> proc_macro2::TokenStream {
        let error_expr = quote! {
            #error_name::MissingField(stringify!(#proto_field_ident).to_string())
        };

        if is_rust_optional {
            quote! {
                #field_name: Some(proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_expr)?
                    .into())
            }
        } else {
            quote! {
                #field_name: proto_struct.#proto_field_ident
                    .ok_or_else(|| #error_expr)?
                    .into()
            }
        }
    }
}

mod error_handler {
    use super::*;

    pub use error_types::{default_error_name, get_actual_error_type};

    /// Main orchestration function for generating all error-related definitions
    pub fn generate_error_definitions_if_needed(
        name: &syn::Ident,
        fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
        struct_level_error_type: &Option<syn::Type>,
    ) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, bool) {
        let requirements = error_analysis::analyze_error_requirements(fields, struct_level_error_type);

        let conversion_error_def = if requirements.needs_try_from &&
            requirements.needs_default_error &&
            struct_level_error_type.is_none() {
            error_types::generate_conversion_error_enum(name)
        } else {
            quote! {}
        };

        let error_conversions = if requirements.needs_error_conversions {
            let error_name = error_types::default_error_name(name);
            error_types::generate_error_conversions(&error_name)
        } else {
            quote! {}
        };

        (conversion_error_def, error_conversions, requirements.needs_try_from)
    }

    /// Generates error handling code for a specific field
    pub fn generate_error_handling(
        field_name: &syn::Ident,
        proto_field_ident: &syn::Ident,
        field_type: &syn::Type,
        proto_meta: &attribute_parser::ProtoFieldMeta,
        error_name: &syn::Ident,
        struct_level_error_type: &Option<syn::Type>,
        struct_level_error_fn: &Option<String>,
    ) -> proc_macro2::TokenStream {
        error_codegen::generate_error_handling(
            field_name,
            proto_field_ident,
            field_type,
            proto_meta,
            error_name,
            struct_level_error_type,
            struct_level_error_fn,
        )
    }
}

mod expect_analysis {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum ExpectMode {
        None,
        Error,
        Panic,
    }

    impl ExpectMode {
        pub fn from_field_meta(field: &Field, proto_meta: &attribute_parser::ProtoFieldMeta) -> ExpectMode {
            let field_name = field.ident.as_ref().unwrap();
            let expect_panic = has_expect_panic_syntax(field);

            let struct_name = syn::Ident::new("DEBUG", proc_macro2::Span::call_site());
            if debug::should_output_debug(&struct_name, field_name) {
                eprintln!("=== determine_expect_mode for {field_name} ===");
                eprintln!("  parse_expect_panic: {expect_panic}");
                eprintln!("  proto_meta.expect: {}", proto_meta.expect);
            }

            if expect_panic {
                ExpectMode::Panic
            } else if proto_meta.expect {
                ExpectMode::Error
            } else {
                ExpectMode::None
            }
        }
    }

    pub fn has_expect_panic_syntax(field: &Field) -> bool {
        for attr in &field.attrs {
            if attr.path().is_ident(constants::DEFAULT_PROTO_MODULE) {
                if let Meta::List(meta_list) = &attr.meta {
                    let tokens_str = meta_list.tokens.to_string();
                    if tokens_str.replace(" ", "").contains("expect(panic)") {
                        return true;
                    }
                }
            }
        }
        false
    }

}

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
                    let config = struct_impl_generator::StructImplConfig {
                        name: &parsed_input.name,
                        fields: &fields_named.named,
                        proto_module: &parsed_input.proto_module,
                        proto_name: &parsed_input.proto_name,
                        proto_path: &parsed_input.proto_path,
                        struct_level_error_type: &parsed_input.struct_level_error_type,
                        struct_level_error_fn: &parsed_input.struct_level_error_fn,
                    };

                    struct_impl_generator::generate_struct_implementations(config).into()
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    tuple_impl_generator::generate_tuple_implementations(&parsed_input.name, fields_unnamed).into()
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
