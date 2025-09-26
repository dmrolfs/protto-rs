use crate::debug::CallStackDebug;
use crate::field::{
    FieldProcessingContext,
    conversion_strategy::{self, FieldGenerationError},
};
use quote::quote;

/// Generate both proto->rust and rust->proto conversions for a field in a single pass
/// This replaces the double iteration approach
pub fn generate_bidirectional_field_conversion(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), FieldGenerationError> {
    let _trace = CallStackDebug::with_context(
        "field::field_processor",
        "generate_bidirectional_field_conversion",
        ctx.struct_name,
        ctx.field_name,
        &[
            ("rust_field_type", &quote!(ctx.field_type).to_string()),
            ("proto_field_ident", &ctx.proto_field_ident.to_string()),
        ],
    );

    conversion_strategy::generate_field_conversions(field, ctx)
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
            _trace.error(format!(
                "Migration error for field '{}': {err}",
                ctx.field_name
            ));

            err
        })
}
