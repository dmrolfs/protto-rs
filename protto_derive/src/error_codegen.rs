use crate::analysis::{attribute_parser, type_analysis};
use crate::conversion::ConversionStrategy;
use quote::quote;

/// Generates error handling code for a specific field
#[allow(clippy::too_many_arguments)]
pub fn generate_error_handling(
    strategy: &ConversionStrategy,
    field_name: &syn::Ident,
    proto_field_ident: &syn::Ident,
    field_type: &syn::Type,
    proto_meta: &attribute_parser::ProtoFieldMeta,
    error_name: &syn::Ident,
    _struct_level_error_type: &Option<syn::Type>,
    struct_level_error_fn: &Option<String>,
) -> proc_macro2::TokenStream {
    let is_rust_optional = type_analysis::is_option_type(field_type);
    let error_fn_to_use = proto_meta
        .error_fn
        .as_ref()
        .or(struct_level_error_fn.as_ref());

    if let Some(error_fn) = error_fn_to_use {
        generate_custom_error_handling(
            strategy,
            field_name,
            proto_field_ident,
            is_rust_optional,
            error_fn,
        )
    } else {
        generate_default_error_handling(
            strategy,
            field_name,
            proto_field_ident,
            is_rust_optional,
            error_name,
        )
    }
}

/// Generates error handling using a custom error function
fn generate_custom_error_handling(
    strategy: &ConversionStrategy,
    field_name: &syn::Ident,
    proto_field_ident: &syn::Ident,
    is_rust_optional: bool,
    error_fn: &str,
) -> proc_macro2::TokenStream {
    let error_fn_path: syn::Path =
        syn::parse_str(error_fn).expect("Failed to parse error function path");

    match strategy {
        ConversionStrategy::CollectVecWithError => {
            // Vec<T> error handling - check if empty, not missing
            quote! {
                #field_name: if proto_struct.#proto_field_ident.is_empty() {
                    return Err(#error_fn_path(stringify!(#field_name)));
                } else {
                    proto_struct.#proto_field_ident.into_iter().map(Into::into).collect()
                }
            }
        }
        _ => {
            if is_rust_optional {
                quote! {
                    #field_name: Some(proto_struct.#proto_field_ident.ok_or_else(|| #error_fn_path(stringify!(#proto_field_ident)))?.into())
                }
            } else {
                quote! {
                    #field_name: proto_struct.#proto_field_ident.ok_or_else(|| #error_fn_path(stringify!(#proto_field_ident)))?.into()
                }
            }
        }
    }
}

/// Generates error handling using the default error type
fn generate_default_error_handling(
    _strategy: &ConversionStrategy,
    field_name: &syn::Ident,
    proto_field_ident: &syn::Ident,
    is_rust_optional: bool,
    error_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    // let strategy_info = strategy.debug_info();
    // let error_expr = quote! {
    //     #error_name::MissingField(format!(
    //         "{} (strategy: {})",
    //         stringify!(#proto_field_ident),
    //         #strategy_info
    //     ))
    // };

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
