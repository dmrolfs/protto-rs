use super::*;
use crate::debug::CallStackDebug;
use field_analysis::FieldProcessingContext;

pub fn generate_from_proto_field(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::with_context(
        "generate_from_proto_field",
        ctx.struct_name,
        ctx.field_name,
        &[
            ("rust_field_type", &quote!(ctx.field_type).to_string()),
            ("proto_field_ident", &ctx.proto_field_ident.to_string()),
            ("proto_name", ctx.proto_name),
            ("proto_module", ctx.proto_module),
        ],
    );

    field_analysis::generate_field_conversions(field, ctx)
        .map(|(proto_to_rust, _)| {
            _trace.generated_code(
                &proto_to_rust,
                ctx.struct_name,
                ctx.field_name,
                "from_proto_field_bidirectional",
                &[("conversion_direction", "proto -> rust")],
            );

            proto_to_rust
        })
        .unwrap_or_else(|error| {
            let error_msg = error.detailed_message();
            _trace.error(&format!("Validation failed: {}", error_msg));

            quote! { compile_error!(#error_msg); }
        })
}

pub fn generate_from_my_field(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::new("generate_from_my_field", ctx.struct_name, ctx.field_name);

    field_analysis::generate_field_conversions(field, ctx)
        .map(|(_, rust_to_proto)| {
            _trace.generated_code(
                &rust_to_proto,
                ctx.struct_name,
                ctx.field_name,
                "from_my_field_bidirectional",
                &[("conversion_direction", "rust -> proto")],
            );

            rust_to_proto
        })
        .unwrap_or_else(|error| {
            let error_msg = error.detailed_message();
            _trace.error(&format!("Validation failed: {}", error));

            quote! { compile_error!(#error_msg); }
        })
}

pub fn generate_default_value(
    field_type: &syn::Type,
    default_fn: Option<&str>,
) -> proc_macro2::TokenStream {
    use crate::constants;

    match default_fn {
        Some(constants::USE_DEFAULT_IMPL) => {
            // Bare 'default' attribute - use Default::default()
            quote! { <#field_type as Default>::default() }
        }
        Some(default_fn_name) => {
            // 'default_fn = "function_name"' - call custom function
            let default_fn_path: syn::Path =
                syn::parse_str(default_fn_name).expect("Failed to parse default_fn path");
            quote! { #default_fn_path() }
        }
        None => {
            // No default specified - use Default::default() as fallback
            quote! { <#field_type as Default>::default() }
        }
    }
}
