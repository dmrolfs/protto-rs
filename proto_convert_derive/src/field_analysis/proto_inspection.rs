//! Build-time metadata integration for proto field analysis.
//!
//! This module provides build-time metadata detection using environment variables
//! set by the build script. This approach provides zero-setup experience for
//! external developers.
//!
//! ## Migration to Prost-Style Approach
//!
//! If you encounter limitations with environment variables (see below), you can
//! migrate to a prost-style file inclusion approach:
//!
//! ### When to Consider Migration:
//! - **Large proto files**: >500 messages or >32KB of metadata (Windows env var limit)
//! - **Complex metadata**: Need structured data beyond simple optional/required flags
//! - **Debugging needs**: Want to inspect generated metadata files directly
//! - **Build reproducibility**: Want metadata as part of source artifacts
//!
//! ### Migration Steps:
//! 1. Change `proto_convert_build`'s `write_metadata_file()` to generate Rust code instead of env vars
//! 2. Update this module to use `include!(concat!(env!("OUT_DIR"), "/file.rs"))`
//! 3. Add consumer boilerplate to include generated metadata
//! 4. Update data structures to use static HashMap instead of env var lookup
//!
//! See prost's implementation for reference patterns.

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
#[cfg(feature = "meta-env")]
struct EnvVarMetadataProvider;

#[cfg(feature = "meta-env")]
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
#[cfg(feature = "meta-file")]
struct FileInclusionMetadataProvider;

#[cfg(feature = "meta-file")]
impl MetadataProvider for FileInclusionMetadataProvider {
    fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata> {
        // The proc macro generates code that will call the user's included module
        // This is a placeholder - the real implementation generates code that calls
        // the user's proto_metadata module at expansion time

        // This function is never actually called - it's just for trait compliance
        // The real work happens in generate_module_access_code() below
        None
    }
}

/// Add new function to generate code that accesses the user's module
#[cfg(feature = "meta-file")]
fn generate_module_access_code(message: &str, field: &str) -> proc_macro2::TokenStream {
    quote::quote! {
                crate::proto_metadata::get_field_optionality(#message, #field)
            }
}

/// Fallback provider when build-time metadat is disabled.
#[cfg(not(any(feature = "meta-env", feature = "meta-file")))]
struct NoOpMetadataProvider;

#[cfg(not(any(feature = "meta-env", feature = "meta-file")))]
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
#[allow(unreachable_code)]
fn try_build_time_metadata(ctx: &field_analysis::FieldProcessingContext) -> Option<bool> {
    if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
        eprintln!("=== BUILD-TIME METADATA DEBUG for {}.{} ===", ctx.struct_name, ctx.field_name);
    }

    #[cfg(feature = "meta-env")]
    {
        if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
            let env_key = format!(
                "PROTO_FIELD_{}_{}",
                ctx.proto_name.to_uppercase(), ctx.field_name.to_string().to_uppercase()
            );
            let env_value = std::env::var(&env_key).ok();
            eprintln!("  provider: EnvVar");
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

        return Some(metadata.optional);
    }

    #[cfg(feature = "meta-file")]
    {
        if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
            eprintln!("  provider: FileInclusion (user module access)");
            eprintln!("  will generate code: crate::proto_metadata::get_field_optionality(\"{}\", \"{}\")",
                      ctx.proto_name, ctx.field_name);
            eprintln!("=== END BUILD-TIME METADATA DEBUG ===");
        }

        // For file inclusion, we can't determine at proc macro time
        // Instead, the proc macro will generate code that calls the user's module
        // Return None here, but the field processor will handle file inclusion differently
        return None;
    }

    #[cfg(not(any(feature = "meta-env", feature = "meta-file")))]
    {
        if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
            eprintln!("  provider: NoOp (no build-time metadata enabled)");
            eprintln!("=== END BUILD-TIME METADATA DEBUG ===");
        }

        let _ = ctx; // avoid unused variable warnings
        return None;
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
    // - Suggest enabling meta-* feature
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