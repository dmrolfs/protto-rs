use quote::quote;
use crate::conversion::ConversionStrategy;
use crate::debug::CallStackDebug;
use crate::analysis::{ attribute_parser, error_analysis, };
use crate::error_codegen;
use crate::error_types;

pub use error_types::{default_error_name, get_actual_error_type};

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
#[allow(clippy::too_many_arguments)]
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
        "error_handler",
        "generate_error_handling_with_strategy",
        field_name.to_string(),
        proto_field_ident.to_string(),
        &[
            ("strategy", &format!("{:?}", strategy)),
            ("strategy_info", strategy.debug_info()),
            ("strategy_category", strategy.category()),
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
