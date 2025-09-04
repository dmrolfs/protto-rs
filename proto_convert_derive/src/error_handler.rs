use super::*;

pub use error_types::{default_error_name, get_actual_error_type};
use crate::debug::CallStackDebug;
use crate::field_analysis::ConversionStrategy;

/// Main orchestration function for generating all error-related definitions
pub fn generate_error_definitions_if_needed(
    name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    struct_level_error_type: &Option<syn::Type>,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, bool) {
    let requirements = error_analysis::analyze_error_requirements(fields, struct_level_error_type);

    let conversion_error_def = if requirements.needs_try_from
        && requirements.needs_default_error
        && struct_level_error_type.is_none()
    {
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

    (
        conversion_error_def,
        error_conversions,
        requirements.needs_try_from,
    )
}

/// Generates error handling code for a specific field
pub fn generate_error_handling(
    strategy: &ConversionStrategy,
    field_name: &syn::Ident,
    proto_field_ident: &syn::Ident,
    field_type: &syn::Type,
    proto_meta: &attribute_parser::ProtoFieldMeta,
    error_name: &syn::Ident,
    struct_level_error_type: &Option<syn::Type>,
    struct_level_error_fn: &Option<String>,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::with_context(
        "generate_error_handling_with_strategy",
        &field_name.to_string(),
        &proto_field_ident.to_string(),
        &[
            ("strategy", &format!("{:?}", strategy)),
            ("strategy_info", &strategy.debug_info()),
            ("strategy_category", &strategy.category()),
        ],
    );

    error_codegen::generate_error_handling(
        strategy,
        field_name,
        proto_field_ident,
        field_type,
        proto_meta,
        error_name,
        struct_level_error_type,
        struct_level_error_fn,
    )
}

pub fn generate_error_handling_expr(
    proto_field_ident: &syn::Ident,
    proto_meta: &attribute_parser::ProtoFieldMeta,
    struct_level_error_fn: &Option<String>,
    error_name: &syn::Ident,
    needs_into: bool,
) -> proc_macro2::TokenStream {
    let error_fn_to_use = proto_meta
        .error_fn
        .as_ref()
        .or(struct_level_error_fn.as_ref());

    let error_expr = error_fn_to_use
        .map(|error_fn| {
            let error_fn_path: syn::Path =
                syn::parse_str(error_fn).expect("Failed to parse error function path");

            quote! { #error_fn_path(stringify!(#proto_field_ident)) }
        })
        .unwrap_or_else(|| {
            quote! { #error_name::MissingField(stringify!(#proto_field_ident).to_string()) }
        });

    if needs_into {
        quote! {
            proto_struct.#proto_field_ident
                .ok_or_else(|| #error_expr)?
                .into()
        }
    } else {
        quote! {
            proto_struct.#proto_field_ident
                .ok_or_else(|| #error_expr)?
        }
    }
}
