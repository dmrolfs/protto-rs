use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::parse::Parser;
use syn::{self, Attribute, DeriveInput, Expr, Field, Lit, Meta, Type};
use syn::{punctuated::Punctuated, token::Comma};
use crate::constants::DEFAULT_CONVERSION_ERROR_SUFFIX;

mod constants {
    pub const PRIMITIVE_TYPES: &[&str] = &["i32", "u32", "i64", "u64", "f32", "f64", "bool", "String"];
    pub const DEFAULT_PROTO_MODULE: &str = "proto";
    pub const DEFAULT_CONVERSION_ERROR_SUFFIX: &str = "ConversionError";
}

mod debug {
    use proc_macro2::{Ident, TokenStream};

    pub fn should_output_debug(name: &Ident, field_name: &Ident) -> bool {
        false
        // || name.to_string() == "ComprehensiveEnumStruct"
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

mod field_context {
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
            let expect_mode = determine_expect_mode(field, &proto_meta);
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
}

mod field_processor {
    use super::*;
    use field_context::FieldProcessingContext;

    pub fn generate_from_proto_field(field: &syn::Field, ctx: &FieldProcessingContext) -> proc_macro2::TokenStream {
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

            if type_analysis::is_enum_type_with_explicit_attr(field_type, field) {
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
        let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);

        if proto_is_optional {
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
                #field_name: proto_struct.#proto_field_ident.into()
            }
        }
    }

    fn generate_primitive_field(ctx: &FieldProcessingContext, field: &syn::Field) -> proc_macro2::TokenStream {
        let field_name = ctx.field_name;
        let proto_field_ident = &ctx.proto_field_ident;
        let field_type = ctx.field_type;
        let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);
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
                let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);
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
        let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);

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
        let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);

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
        let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);

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
        let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);

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
        let proto_is_optional = is_optional_proto_field_for_ctx(ctx, field);

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

    fn is_optional_proto_field_for_ctx(ctx: &FieldProcessingContext, field: &syn::Field) -> bool {
        is_optional_proto_field(ctx.struct_name, field, ctx.proto_name)
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
            let screaming_variant = to_screaming_snake_case(&variant_str);
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
            let screaming_variant = to_screaming_snake_case(&variant_str);
            let prefixed_candidate = format!("{}_{}", enum_prefix, screaming_variant);
            let prefixed_candidate_lit = syn::LitStr::new(&prefixed_candidate, Span::call_site());

            quote! {
                #name::#variant_ident => <#proto_enum_path>::from_str_name(#prefixed_candidate_lit)
                    .unwrap_or_else(|| panic!("No matching proto variant for {rust_enum:?}")),
            }
        }).collect()
    }
}

mod error_handler {
    use super::*;

    pub fn generate_error_definitions_if_needed(
        name: &syn::Ident,
        fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
        struct_level_error_type: &Option<syn::Type>,
    ) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, bool) {
        let error_name = default_error_name(name);

        let needs_try_from = fields.iter().any(|field| {
            if attribute_parser::has_proto_ignore(field) {
                false
            } else {
                let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
                let expect_mode = determine_expect_mode(field, &proto_meta);
                matches!(expect_mode, ExpectMode::Error)
            }
        });

        let needs_default_error = fields.iter().any(|field| {
            if attribute_parser::has_proto_ignore(field) { return false; }
            let proto_meta = attribute_parser::ProtoFieldMeta::from_field(field).unwrap_or_default();
            if matches!(determine_expect_mode(field, &proto_meta), ExpectMode::Error) {
                let effective_error_type = get_effective_error_type(&proto_meta, struct_level_error_type);
                effective_error_type.is_none()
            } else {
                false
            }
        });

        let conversion_error_def = if needs_try_from &&
            needs_default_error &&
            struct_level_error_type.is_none() {
            generate_conversion_error(name)
        } else {
            quote! {}
        };

        let error_conversions = if needs_try_from &&
            needs_default_error &&
            struct_level_error_type.is_none() {
            quote! {
                impl From<String> for #error_name {
                    fn from(err: String) -> Self {
                        Self::MissingField(err)
                    }
                }
            }
        } else {
            quote! {}
        };

        (conversion_error_def, error_conversions, needs_try_from)
    }

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
        } else {
            let error_expr = quote! { #error_name::MissingField(stringify!(#proto_field_ident).to_string()) };

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

    fn generate_conversion_error(struct_name: &syn::Ident) -> proc_macro2::TokenStream {
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

    pub fn default_error_name(struct_name: &syn::Ident) -> syn::Ident {
        syn::Ident::new(&format!("{struct_name}{DEFAULT_CONVERSION_ERROR_SUFFIX}"), struct_name.span())
    }

    pub fn get_effective_error_type(proto_meta: &attribute_parser::ProtoFieldMeta, struct_level_error_type: &Option<syn::Type>) -> Option<syn::Type> {
        if let Some(field_error_type) = &proto_meta.error_type {
            return Some(syn::parse_str(field_error_type)
                .expect("Failed to parse field-level error_type"));
        }

        struct_level_error_type.clone()
    }
}


#[derive(Debug, Clone)]
enum ExpectMode {
    None,
    Error,
    Panic,
}

#[proc_macro_derive(ProtoConvert, attributes(proto))]
pub fn proto_convert_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let proto_module = attribute_parser::get_proto_module(&ast.attrs).unwrap_or_else(|| "proto".to_string());
    let proto_name = attribute_parser::get_proto_struct_rename(&ast.attrs).unwrap_or_else(|| name.to_string());

    let struct_level_error_type = attribute_parser::get_proto_struct_error_type(&ast.attrs);
    let struct_level_error_fn = attribute_parser::get_struct_level_error_fn(&ast.attrs);

    let proto_path =
        syn::parse_str::<syn::Path>(&format!("{}::{}", proto_module, proto_name)).unwrap();

    match &ast.data {
        syn::Data::Struct(data_struct) => {
            match &data_struct.fields {
                syn::Fields::Named(fields_named) => {
                    let fields = &fields_named.named;

                    let error_name = error_handler::default_error_name(name);
                    let (conversion_error_def, error_conversions, needs_try_from) =
                        error_handler::generate_error_definitions_if_needed(name, fields, &struct_level_error_type);
                    let actual_error_type = error_handler::get_actual_error_type(
                        needs_try_from,
                        &struct_level_error_type,
                        &error_name,
                    );

                    let from_proto_fields = fields.iter().map(|field| {
                        let ctx = field_context::FieldProcessingContext::new(
                            name,
                            field,
                            &error_name,
                            &struct_level_error_type,
                            &struct_level_error_fn,
                            &proto_module,
                            &proto_name,
                        );

                        field_processor::generate_from_proto_field(field, &ctx)
                    });

                    let from_my_fields = fields
                        .iter()
                        .filter(|field| !attribute_parser::has_proto_ignore(field))
                        .map(|field| {
                            let ctx = field_context::FieldProcessingContext::new(
                                name,
                                field,
                                &error_name,
                                &struct_level_error_type,
                                &struct_level_error_fn,
                                &proto_module,
                                &proto_name,
                            );

                            field_processor::generate_from_my_field(field, &ctx)
                        });

                    let gen = if needs_try_from {
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
        },

        syn::Data::Enum(data_enum) => {
            let variants = &data_enum.variants;
            let gen = enum_processor::generate_enum_conversions(name, variants, &proto_module);
            gen.into()
        },

        _ => panic!("ProtoConvert only supports structs and enums, not unions"),
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

// fn is_defaultable_type(ty: &Type) -> bool {
//     if let Type::Path(type_path) = ty {
//         if type_path.path.segments.len() == 1 {
//             let type_name = type_path.path.segments[0].ident.to_string();
//             matches!(type_name.as_str(),
//                 "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
//                 "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
//                 "f32" | "f64" | "bool" | "String" |
//                 "Vec" | "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet"
//             )
//         } else {
//             false
//         }
//     } else {
//         false
//     }
// }



fn is_optional_proto_field(name: &syn::Ident, field: &syn::Field, proto_name: &str) -> bool {
    let field_name = field.ident.as_ref().unwrap();

    let proto_meta_result = attribute_parser::ProtoFieldMeta::from_field(field);
    if let Ok(proto_meta) = &proto_meta_result {
        if debug::should_output_debug(name, &field_name) {
            eprintln!("=== PROTO META DEBUG for {}.{} ===", proto_name, field_name);
            eprintln!("  proto_meta.optional: {:?}", proto_meta.optional);
            eprintln!("  proto_meta.default_fn: {:?}", proto_meta.default_fn);
            eprintln!("  proto_meta.expect: {:?}", proto_meta.expect);
        }

        // for (i, attr) in field.attrs.iter().enumerate() {
        //     eprintln!("  attr[{i}): {attr:?}");
        // }

        if let Some(optional) = proto_meta.optional {
            if debug::should_output_debug(name, &field_name) {
                eprintln!("  RETURNING explicit optional = {optional}");
            }
            return optional;
        }
    } else if debug::should_output_debug(name, &field_name) {
        eprintln!("  FAILED to parse ProtoFieldMeta from field!");
    }

    let field_type = &field.ty;

    if type_analysis::is_option_type(field_type) {
        return true;
    }

    if type_analysis::is_vec_type(field_type) {
        return false;
    }

    if proto_meta_result.as_ref().map(|m| m.default_fn.is_some()).unwrap_or(false) {
        if let Ok(proto_meta) = &proto_meta_result {
            let expect_mode = determine_expect_mode(field, &proto_meta);
            if !matches!(expect_mode, ExpectMode::None) {
                return false;
            }

            // if it has a default (either bare "default_fn" or "default_fn = <function>"),
            // assume the proto field is optional since that's the typical use case
            return true;
        }

        // fallback: if has_proto_default but can't parse meta, assume optional
        return true;
    }

    let has_expect = has_expect_panic_syntax(field) ||
        proto_meta_result.as_ref().map(|m| m.expect).unwrap_or(false);

    if debug::should_output_debug(name, &field_name) {
        eprintln!("  has_expect: {has_expect}, returning: {has_expect}");
    }

    has_expect
}

// Helper to get proto module from current context (struct-level attributes)
// fn get_proto_module_from_context() -> Option<String> {
//     // This would need to be passed down from the main macro context
//     // For now, return None and fall back to default
//     None
// }



// // helper function to get the proto module for a specific field
// // this checks field-level module override first, then falls back to struct-level
// fn get_proto_module_for_field(field: &syn::Field) -> Option<String> {
//     // first check if the field has its own module specification
//     for attr in &field.attrs {
//         if attr.path().is_ident("proto") {
//             if let Meta::List(meta_list) = &attr.meta {
//                 let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
//                     .parse2(meta_list.tokens.clone())
//                     .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {e}"));
//
//                 for meta in nested_metas {
//                     if let Meta::NameValue(meta_nv) = meta {
//                         if meta_nv.path.is_ident("module") {
//                             if let Expr::Lit(expr_lit) = &meta_nv.value {
//                                 if let Lit::Str(lit_str) = &expr_lit.lit {
//                                     return Some(lit_str.value());
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
//
//     None
// }

fn determine_expect_mode(field: &Field, proto_meta: &attribute_parser::ProtoFieldMeta) -> ExpectMode {
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

fn has_expect_panic_syntax(field: &Field) -> bool {
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

// fn get_proto_error_type(attrs: &[Attribute]) -> Option<syn::Type> {
//     for attr in attrs {
//         if attr.path().is_ident("proto") {
//             if let Meta::List(meta_list) = &attr.meta {
//                 let nested_metas: Punctuated<Meta, Comma> = Punctuated::parse_terminated
//                     .parse2(meta_list.tokens.clone())
//                     .unwrap_or_else(|e| panic!("Failed to parse proto attribute: {}", e));
//                 for meta in nested_metas {
//                     if let Meta::NameValue(meta_nv) = meta {
//                         if meta_nv.path.is_ident("error_type") {
//                             if let Expr::Path(expr_path) = &meta_nv.value {
//                                 return Some(syn::Type::Path(syn::TypePath {
//                                     qself: None,
//                                     path: expr_path.path.clone(),
//                                 }));
//                             }
//                             panic!("error_type value must be a type path, e.g., #[proto(error_type = MyError)]");
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     None
// }

fn to_screaming_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i != 0 {
            result.push('_');
        }
        result.push(c.to_ascii_uppercase());
    }
    result
}

