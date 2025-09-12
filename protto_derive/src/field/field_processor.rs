use quote::quote;
use crate::debug::CallStackDebug;
use crate::migration::{self, MigrationError, generate_field_conversions_with_migration};
use crate::analysis::field_analysis::FieldProcessingContext;

/// Initialize migration system at macro startup
pub fn initialize_migration_system() {
    migration::config::from_env();
}

/// Generate both proto->rust and rust->proto conversions for a field in a single pass
/// This replaces the double iteration approach
pub fn generate_bidirectional_field_conversion(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), MigrationError> {
    let _trace = CallStackDebug::with_context(
        "generate_bidirectional_field_conversion",
        ctx.struct_name,
        ctx.field_name,
        &[
            ("rust_field_type", &quote!(ctx.field_type).to_string()),
            ("proto_field_ident", &ctx.proto_field_ident.to_string()),
            ("migration_mode", "enabled"),
        ],
    );

    generate_field_conversions_with_migration(field, ctx)
        .map(|(from_proto, to_proto)| {
            _trace.generated_code(
                &from_proto,
                ctx.struct_name,
                ctx.field_name,
                "bidirectional_proto_to_rust",
                &[("conversion_direction", "proto -> rust")],
            );

            _trace.generated_code(
                &to_proto,
                ctx.struct_name,
                ctx.field_name,
                "bidirectional_rust_to_proto",
                &[("conversion_direction", "rust -> proto")],
            );

            (from_proto, to_proto)
        })
        .map_err(|err| {
            _trace.error(&format!(
                "Migration error for field '{}': {err}",
                ctx.field_name
            ));

            err
        })
}

/// Legacy function for proto->rust conversion (kept for backward compatibility)
pub fn generate_from_proto_field(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::with_context(
        "generate_from_proto_field",
        ctx.struct_name,
        ctx.field_name,
        &[("legacy_mode", "proto_to_rust_only")],
    );

    generate_bidirectional_field_conversion(field, ctx)
        .map(|(from_proto, _)| from_proto)
        .unwrap_or_else(|err| {
            let error_msg = err.to_string();
            _trace.error(&format!("Legacy conversion failed: {err}"));
            quote! { compile_error!(error_msg); }
        })
}

/// Legacy function for rust->proto conversion (kept for backward compatibility)
pub fn generate_from_my_field(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> proc_macro2::TokenStream {
    let _trace = CallStackDebug::new("generate_from_my_field", ctx.struct_name, ctx.field_name);

    generate_bidirectional_field_conversion(field, ctx)
        .map(|(_, to_proto)| to_proto)
        .unwrap_or_else(|err| {
            let error_msg = err.to_string();
            _trace.error(&format!("Legacy conversion failed: {err}"));
            quote! { compile_error!(error_msg); }
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
