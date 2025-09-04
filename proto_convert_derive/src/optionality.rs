use crate::debug::CallStackDebug;
use crate::expect_analysis::ExpectMode;
use crate::field_analysis::{FieldProcessingContext, RustFieldInfo};
use crate::{attribute_parser, expect_analysis, type_analysis};
use quote::{ToTokens, quote};

/// Main entry point for proto field optionality detection.
///
/// This function orchestrates multiple detection strategies in order of reliability:
/// 1. explicit user annotation (`#[proto(optional = true)]`)
/// 2. build-time metadata (this module)
/// 3. type-based inference (Option<T> = optional)
/// 4. usage pattern inference (expect/default = optional)
pub fn determine_proto_field_optionality(
    struct_name: &syn::Ident,
    field: &syn::Field,
    proto_name: &str,
    rust_field: &RustFieldInfo,
    ctx: &FieldProcessingContext
) -> FieldOptionality {
    let _trace = CallStackDebug::with_context(
        "determine_proto_field_optionality",
        struct_name,
        field
            .ident
            .as_ref()
            .map(|f| f.to_string())
            .unwrap_or_default(),
        &[("proto_name", proto_name)],
    );

    // 1. Explicit annotation takes absolute precedence
    if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field)
        && proto_meta.has_explicit_optionality()
    {
        _trace.decision(
            "explicit optional",
            "Explicit annotation takes absolute precedence",
        );
        _trace.metadata_lookup(
            proto_name,
            &field.ident.as_ref().unwrap().to_string(),
            Some(proto_meta.is_proto_optional()),
            "explicit_user_annotation"
        );
        return if proto_meta.is_proto_optional() {
            FieldOptionality::Optional
        } else {
            FieldOptionality::Required
        };
    }

    // 2. Pattern-based inference from type structure
    let field_type = &field.ty;
    if type_analysis::is_option_type(field_type) {
        _trace.decision(
            "is_option_type",
            "✓ PATTERN: Option<T> → optional proto field",
        );
        _trace.metadata_lookup(
            proto_name,
            &field.ident.as_ref().unwrap().to_string(),
            Some(true),
            "option_type_pattern"
        );
        return FieldOptionality::Optional;
    }

    if type_analysis::is_vec_type(field_type) {
        _trace.decision(
            "is_vec_type",
            "✓ PATTERN: Vec<T> → required repeated proto field",
        );
        return FieldOptionality::Required;
    }

    // 3. Pattern-based inference from usage indicators
    if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field) {
        _trace.checkpoint("Pattern-based inference from usage indicators");
        // Has default function → likely optional proto field
        if proto_meta.default_fn.is_some() {
            let expect_mode = ExpectMode::from_field_meta(field, &proto_meta);
            // If has default but also has expect mode, the expectation overrides
            if matches!(expect_mode, ExpectMode::None) {
                _trace.decision(
                    "ExpectMode::None",
                    "✓ PATTERN: default_fn (without expect) → optional proto field",
                );
                return FieldOptionality::Optional;
            }
        }

        // Has expect attribute → indicates optional proto field
        if proto_meta.expect {
            _trace.decision(
                "ExpectMode",
                "✓ PATTERN: expect attribute → optional proto field",
            );
            return FieldOptionality::Optional;
        }
    }

    // Check for expect panic syntax in field attributes
    if expect_analysis::has_expect_panic_syntax(field) {
        _trace.decision(
            "ExpectMode::Panic",
            "✓ PATTERN: expect() syntax → optional proto field",
        );
        return FieldOptionality::Optional;
    }

    // 4. No clear pattern found - emit helpful guidance
    _trace.checkpoint("? AMBIGUOUS: Add #[proto(optional = true/false)] for clarity");
    _trace.checkpoint("Suggestion: Most proto primitives without Option<T> are required");

    // Default to Required for primitives/custom types without clear indicators
    // This is the safest assumption and matches most proto field patterns
    FieldOptionality::Required
}

/// Result of build-time metadata detection for field optionality
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FieldOptionality {
    /// Indicates whether the proto field is optional.
    Optional,

    /// Indicates the proto field is required.
    Required,
}

impl Default for FieldOptionality {
    fn default() -> Self {
        Self::Required
    }
}

impl std::fmt::Display for FieldOptionality {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Optional => write!(f, "Optional"),
            Self::Required => write!(f, "Required"),
        }
    }
}

impl FieldOptionality {
    pub fn new(is_optional: bool) -> Self {
        if is_optional {
            Self::Optional
        } else {
            Self::Required
        }
    }
}

impl FieldOptionality {
    pub fn from_field_context(ctx: &FieldProcessingContext, field: &syn::Field) -> Self {
        let _trace = CallStackDebug::with_context(
            "from_field_context",
            ctx.struct_name,
            ctx.field_name,
            &[],
        );

        // 1. explicit annotation takes absolute precedence
        if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field)
            && proto_meta.has_explicit_optionality()
        {
            _trace.decision(
                "explicit_optionality_flage",
                "explicit annotation takes absolute precedence",
            );
            if proto_meta.is_proto_optional() {
                Self::Optional
            } else {
                Self::Required
            }
        } else if let Some(inferred) = Self::infer_from_patterns(ctx, field) {
            inferred
        } else {
            Self::emit_ambiguity_error(ctx);
            Self::Required
        }
    }

    fn infer_from_patterns(ctx: &FieldProcessingContext, field: &syn::Field) -> Option<Self> {
        let _trace = CallStackDebug::with_context(
            "infer_from_patterns",
            ctx.struct_name,
            ctx.field_name,
            &[],
        );

        let field_type = &field.ty;

        // Pattern: Option<T> in Rust → optional proto field
        if type_analysis::is_option_type(field_type) {
            _trace.decision("is_option", "Option<T> in Rust → optional proto field");
            return Some(Self::Optional);
        }

        // Pattern: Vec<T> → required repeated proto field
        if type_analysis::is_vec_type(field_type) {
            _trace.decision("is_vec", " Vec<T> → required repeated proto field");
            return Some(Self::Required);
        }

        // Pattern: Check for ACTUAL optional usage indicators before assuming optional
        // Only if there are explicit expect/default attributes should we infer optional
        if Self::has_explicit_optional_usage_indicators(ctx, field) {
            _trace.decision(
                "has_explicit_usage_indicators",
                "Fields with explicit expect() or default() → optional proto field",
            );
            return Some(Self::Optional);
        }

        // // Pattern: Fields with expect() or default() → optional proto field
        // // This pattern indicates the field might be missing, so proto should be optional
        // if Self::has_optional_usage_indicators(ctx, field) {
        //     _trace.decision(
        //         "has_usage_indicators",
        //         "Fields with expect() or default() → optional proto field",
        //     );
        //     return Some(Self::Optional);
        // }

        // Pattern: Primitives -> required
        if type_analysis::is_primitive_type(field_type) {
            _trace.decision("is_primitive", "Primitive -> required proto field");
            return Some(Self::Required);
        }

        // Pattern: Enums -> required (like primitives)
        if type_analysis::is_enum_type(field_type) {
            _trace.decision("is_enum", "Enum -> required proto field");
            return Some(Self::Required);
        }

        // DMR: Pattern: Custom types (structs, newtypes) → required by default
        // This is the key fix - custom types without explicit optional indicators should be required
        if Self::is_custom_type_without_optional_indicators(ctx, field_type) {
            _trace.decision(
                "custom_type_required",
                "Custom type without optional indicators → required proto field"
            );
            return Some(Self::Required);
        }

        // Pattern: Newtype wrappers with optional usage -> optional
        if Self::is_newtype_wrapper(field_type) {
            if Self::has_explicit_optional_usage_indicators(ctx, field) {
                _trace.decision(
                    "newtype_with_usage",
                    "Newtype with explicit expect/default → optional",
                );
                return Some(Self::Optional);
            } else {
                _trace.decision(
                    "newtype_without_usage",
                    "Newtype without indicators → required",
                );
                return Some(Self::Required);
            }
        }

        // Fallout Pattern: Custom types without clear indicators -> ambiguous
        _trace.error("? AMBIGUOUS: Requires explicit #[proto(optional = true/false)]");
        None
    }

    // only return true for EXPLICIT optional usage indicators
    fn has_explicit_optional_usage_indicators(ctx: &FieldProcessingContext, field: &syn::Field) -> bool {
        // Check for explicit expect() attribute or usage - not just context
        let has_explicit_expect = expect_analysis::has_expect_panic_syntax(field) ||
            (ctx.expect_mode != ExpectMode::None && Self::has_expect_attribute_on_field(field));

        // Check for explicit default() attribute on the field itself
        let has_explicit_default = Self::has_default_fn_attribute(field) ||
            Self::has_any_default_attribute(field);

        let result = has_explicit_expect || has_explicit_default;

        if result {
            CallStackDebug::new("has_explicit_optional_usage_indicators", ctx.struct_name, ctx.field_name)
                .checkpoint_data("explicit_usage_found", &[
                    ("has_explicit_expect", &has_explicit_expect.to_string()),
                    ("has_explicit_default", &has_explicit_default.to_string()),
                ]);
        }

        result
    }

    // detect custom types that should be required by default
    fn is_custom_type_without_optional_indicators(ctx: &FieldProcessingContext, field_type: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = field_type {
            let segments = &type_path.path.segments;

            // Single-segment types that aren't primitives or std types
            if segments.len() == 1 {
                let is_primitive = type_analysis::is_primitive_type(field_type);
                let is_std_type = Self::is_std_type(field_type);
                let is_proto_type = type_analysis::is_proto_type(field_type, ctx.proto_module);

                // Custom type = not primitive, not std, not proto
                let is_custom = !is_primitive && !is_std_type && !is_proto_type;

                if is_custom {
                    CallStackDebug::new("is_custom_type_without_optional_indicators", ctx.struct_name, ctx.field_name)
                        .checkpoint_data("custom_type_detected", &[
                            ("type_name", &segments[0].ident.to_string()),
                            ("is_primitive", &is_primitive.to_string()),
                            ("is_std_type", &is_std_type.to_string()),
                            ("is_proto_type", &is_proto_type.to_string()),
                        ]);
                }

                is_custom
            } else {
                false
            }
        } else {
            false
        }
    }

    // check if field has explicit expect attribute
    fn has_expect_attribute_on_field(field: &syn::Field) -> bool {
        if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field) {
            proto_meta.expect
        } else {
            false
        }
    }


    /// Check if field has usage patterns indicating optional proto field
    fn has_optional_usage_indicators(ctx: &FieldProcessingContext, field: &syn::Field) -> bool {
        // Check for expect() attribute or usage
        let has_expect = !matches!(ctx.expect_mode, ExpectMode::None)
            || expect_analysis::has_expect_panic_syntax(field);

        // Check for default() attribute
        let has_default = ctx.has_default
            || Self::has_default_fn_attribute(field)
            || Self::has_any_default_attribute(field);

        has_expect || has_default
    }

    /// Check if field has a default_fn attribute
    fn has_default_fn_attribute(field: &syn::Field) -> bool {
        if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field) {
            proto_meta.default_fn.is_some()
        } else {
            false
        }
    }

    fn has_any_default_attribute(field: &syn::Field) -> bool {
        attribute_parser::ProtoFieldMeta::from_field(field)
            .map(|proto_meta| {
                proto_meta.default_fn.is_some() ||
                    // check for #[proto(default)] without value
                    field.attrs.iter().any(|attr| {
                        attr.path().is_ident("proto") &&
                            attr.to_token_stream().to_string().contains("default")
                    })
            })
            .unwrap_or(false)
    }

    fn is_newtype_wrapper(field_type: &syn::Type) -> bool {
        // Detect single-segment path types that aren't primitives or known std types
        if let syn::Type::Path(type_path) = field_type {
            let segments = &type_path.path.segments;
            segments.len() == 1
                && !type_analysis::is_primitive_type(field_type)
                && !Self::is_std_type(field_type)
        } else {
            false
        }
    }

    fn get_newtype_inner_type(field_type: &syn::Type) -> Option<syn::Type> {
        if let syn::Type::Path(type_path) = field_type {
            // For tuple structs like TrackId(u64), we'd need to inspect the struct definition
            // This is complex with syn - simpler approach is pattern matching on common cases

            // For now, assume single-segment non-primitive types are newtypes around primitives
            // This works for your TrackId(u64) case
            if type_path.path.segments.len() == 1 {
                // Could add more sophisticated detection here
                // For MVP, assume primitive inner type
                return Some(syn::parse_quote!(u64)); // Default assumption
            }
        }
        None
    }

    fn is_std_type(field_type: &syn::Type) -> bool {
        // Add detection for common std types that aren't newtypes
        matches!(
            quote!(#field_type).to_string().as_str(),
            "String" | "Vec" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet"
        )
    }

    /// Emit clear error message for ambiguous cases
    fn emit_ambiguity_error(ctx: &FieldProcessingContext) {
        panic!(
            "Cannot infer optionality for field '{}.{}' of type '{}'. \
        Add explicit annotation: #[proto(proto_optional)] or #[proto(proto_required)]",
            ctx.struct_name,
            ctx.field_name,
            quote!(ctx.field_type).to_string()
        );
        // TODO: When proc_macro::Diagnostic is stable, emit proper compiler note
        // For now, this is a placeholder for future implementation

        // if debug::should_output_debug(ctx.struct_name, ctx.field_name) {
        //     eprintln!(
        //         "WARNING: Cannot infer optionality for {}.{} - add #[proto(optional = true/false)]",
        //         ctx.struct_name, ctx.field_name
        //     );
        // }
    }

    /// Check if this field optionality is optional
    pub fn is_optional(self) -> bool {
        matches!(self, Self::Optional)
    }

    /// Check if this field optionality is required
    pub fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }

    // fn is_optional_proto_field(name: &syn::Ident, field: &syn::Field, proto_name: &str) -> bool {
    //     let field_name = field.ident.as_ref().unwrap();
    //
    //     if let Ok(proto_meta) = attribute_parser::ProtoFieldMeta::from_field(field) {
    //         if debug::should_output_debug(name, &field_name) {
    //             eprintln!("=== PROTO META DEBUG for {}.{} ===", proto_name, field_name);
    //             eprintln!("  proto_meta.optional: {:?}", proto_meta.optional);
    //         }
    //
    //         if let Some(optional) = proto_meta.optional {
    //             if debug::should_output_debug(name, &field_name) {
    //                 eprintln!("  RETURNING explicit optional = {optional}");
    //             }
    //             return optional;
    //         }
    //     }
    //
    //     false
    // }
}

// /// Build-time metadata provider trait.
// ///
// /// Defines the metatdata inclusion mechanism
// trait MetadataProvider {
//     /// get field metadata for a specific message and field.
//     fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata>;
// }

// #[allow(dead_code)]
// #[derive(Debug, Clone)]
// pub struct ProtoFieldMetadata {
//     pub optional: bool,
//     pub repeated: bool,
// }
//
// /// Evironment variable-based metadata provider.
// ///
// /// Reads metadata from environment variables set by build script:
// /// - Format: `PROTO_FIELD_{MESSAGE}_{FIELD}={optional|required|repeated}`
// /// - Example: `PROTO_FIELD_USER_NAME=optional`
// // #[cfg(feature = "meta-env")]
// struct EnvVarMetadataProvider;
//
// // #[cfg(feature = "meta-env")]
// impl MetadataProvider for EnvVarMetadataProvider {
//     fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata> {
//         let env_key = format!(
//             "PROTO_FIELD_{}_{}",
//             message.to_uppercase(), field.to_uppercase()
//         );
//
//         match std::env::var(env_key).ok()?.as_str() {
//             "optional" => Some(ProtoFieldMetadata { optional: true, repeated: false }),
//             "repeated" => Some(ProtoFieldMetadata { optional: false, repeated: true }),
//             "required" => Some(ProtoFieldMetadata { optional: false, repeated: false }),
//             _ => None,
//         }
//     }
// }
//
// /// Prost-style file inclusion metadat provider (for future migration).
// ///
// /// When migrated, this would include generated Rust code from OUT_DIR
// /// and provide static HashMap lookup instead of runtime env var access.
// #[cfg(feature = "meta-file")]
// struct FileInclusionMetadataProvider;
//
// #[cfg(feature = "meta-file")]
// impl MetadataProvider for FileInclusionMetadataProvider {
//     fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata> {
//         // The proc macro generates code that will call the user's included module
//         // This is a placeholder - the real implementation generates code that calls
//         // the user's proto_metadata module at expansion time
//
//         // This function is never actually called - it's just for trait compliance
//         // The real work happens in generate_module_access_code() below
//         None
//     }
// }
//
// /// Add new function to generate code that accesses the user's module
// #[cfg(feature = "meta-file")]
// fn generate_module_access_code(message: &str, field: &str) -> proc_macro2::TokenStream {
//     quote::quote! {
//                 crate::proto_metadata::get_field_optionality(#message, #field)
//             }
// }
//
// /// Fallback provider when build-time metadat is disabled.
// #[cfg(not(any(feature = "meta-env", feature = "meta-file")))]
// struct NoOpMetadataProvider;
//
// #[cfg(not(any(feature = "meta-env", feature = "meta-file")))]
// impl MetadataProvider for NoOpMetadataProvider {
//     fn get_field_metadata(message: &str, field: &str) -> Option<ProtoFieldMetadata> {
//         None
//     }
// }

// /// Try to get field metadata from build-time generation.
// ///
// /// This function abstracts the metadata provider to make migration easier.
// /// Currently used environment variables, but can be easily switched to
// /// file inclusion approach (but that requires more work by app developer; e.g., `prost` includes.
// #[allow(unreachable_code)]
// fn try_build_time_metadata(ctx: &field_analysis::FieldProcessingContext) -> Option<FieldOptionality> {
//     if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
//         eprintln!("=== BUILD-TIME METADATA DEBUG for {}.{} ===", ctx.struct_name, ctx.field_name);
//         eprintln!("DEBUG: Checking for runtime metadata availability");
//     }
//
//     #[cfg(feature = "meta-file")]
//     {
//         if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
//             eprintln!("  provider: FileInclusion (will generate conditional code)");
//             eprintln!("=== END BUILD-TIME METADATA DEBUG ===");
//         }
//         return Some(FieldOptionality::GenerateCode);
//     }
//
//     #[cfg(feature = "meta-env")]
//     {
//         let env_key = format!(
//             "PROTO_FIELD_{}_{}",
//             ctx.proto_name.to_uppercase(),
//             ctx.proto_field_ident.to_string().to_uppercase()
//         );
//
//         if let Ok(env_value) = std::env::var(&env_key) {
//             if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
//                 eprintln!("  provider: EnvVar");
//                 eprintln!("  env_key: {}", env_key);
//                 eprintln!("  env_value: {:?}", env_value);
//             }
//
//             let metadata = EnvVarMetadataProvider::get_field_metadata(
//                 ctx.proto_name,
//                 &ctx.proto_field_ident.to_string(),
//             )?;
//
//             if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
//                 eprintln!("  metadata.optional: {}", metadata.optional);
//                 eprintln!("=== END BUILD-TIME METADATA DEBUG ===");
//             }
//
//             return Some(FieldOptionality::new(metadata.optional));
//         }
//     }
//
//     if crate::debug::should_output_debug(ctx.struct_name, ctx.field_name) {
//         eprintln!("  provider: NoOp (no build-time metadata enabled)");
//         eprintln!("=== END BUILD-TIME METADATA DEBUG ===");
//     }
//
//     panic!("ProtoConvert requires either 'meta-env' or 'meta-file' feature");
//     // None
// }

// /// Infer optionality from Rust type structure.
// ///
// /// - `Option<T>` typically maps to optional proto fields
// /// - `Vec<T>` typically maps to repeated proto fields (not optional)
// fn infer_from_rust_type(ctx: &field_analysis::FieldProcessingContext) -> Option<bool> {
//     let rust_is_optional = type_analysis::is_option_type(ctx.field_type);
//     let rust_is_vec = type_analysis::is_vec_type(ctx.field_type);
//
//     if rust_is_vec {
//         // Vec<T> typically maps to repeated proto field (not optional)
//         Some(false)
//     } else if rust_is_optional {
//         // Option<T> typically maps to optional proto fields
//         Some(true)
//     } else {
//         // non-optional rust type could map to either required or optional proto field
//         None
//     }
// }

// /// Infer from usage patterns (expect/default attributes).
// ///
// /// If user provides `expect()` or `default()`, the proto field is likely optional
// /// since these only make sense for fields that might be missing.
// fn infer_from_usage_patterns(ctx: &field_analysis::FieldProcessingContext) -> Option<bool> {
//     let has_expect = !matches!(ctx.expect_mode, expect_analysis::ExpectMode::None);
//     let has_default = ctx.has_default;
//
//     if has_expect || has_default {
//         // if user provides expect() or default(), proto field is likely optional
//         Some(true)
//     } else {
//         None
//     }
// }

// /// Emit suggestion for adding build-time metadata when detection fails.
// ///
// /// This helps developers understand when they might benefit from proto
// /// file analysis instead of relying on heuristics.
// fn emit_metadata_suggestion(_ctx: &field_analysis::FieldProcessingContext) {
//     // Could emit compiler notes here in the future:
//     // - Suggest enabling meta-* feature
//     // - Suggest adding explicit #[proto(optional = true/false)]
//     // - Point to documentation for proto file analysis setup
//
//     // Note: proc_macro::Diagnostic is not stable yet, so this is placeholder
//
//     //     let struct_name = ctx.struct_name;
//     //     let field_name = ctx.field_name;
//     //     let proto_name = ctx.proto_name;
//     //
//     //     // only emit once per compilation per struct
//     //     static mut WARNED_STRUCTS: std::collections::HashSet<String> = std::collections::HashSet::new();
//     //     let struct_key = format!("{struct_name}::{proto_name}");
//     //
//     //     unsafe {
//     //         if !WARNED_STRUCTS.contains(&struct_key) {
//     //             WARNED_STRUCTS.insert(struct_key);
//     //
//     //              // only in nightly now
//     //             proc_macro::Diagnostics::spanned(
//     //                 proc_macro2::Span::call_site().unwrap(),
//     //                 proc_macro::Level::Note,
//     //                 format!(
//     //                     "ProtoConvert: Could not determine optionality for field '{}' in '{}'. \
//     //                     For better detection, add to build.rs: \
//     //                     proto_convert_build::generate_proto_metadata(&[\"path/to/{}.proto\"])",
//     //                     field_name, struct_name, proto_name.to_lowercase()
//     //                 )
//     //             ).emit();
//     //         }
//     //     }
// }
