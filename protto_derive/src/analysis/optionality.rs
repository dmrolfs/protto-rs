use crate::debug::CallStackDebug;
use quote::{ToTokens, quote};
use crate::analysis::{
    attribute_parser,
    expect_analysis::{self, ExpectMode},
    field_analysis::FieldProcessingContext,
    type_analysis,
};

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
    #[allow(unused)]
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
            "analysis::optionality::FieldOptionality",
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
            "analysis::optionality::FieldOptionality",
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

        // Pattern: Custom types (structs, newtypes) → required by default
        // This is the key fix - custom types without explicit optional indicators should be required
        if Self::is_custom_type_without_optional_indicators(ctx, field_type) {
            _trace.decision(
                "custom_type_required",
                "Custom type without optional indicators → required proto field",
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
    fn has_explicit_optional_usage_indicators(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
    ) -> bool {
        // Check for explicit expect() attribute or usage - not just context
        let has_explicit_expect = expect_analysis::has_expect_panic_syntax(field)
            || (ctx.expect_mode != ExpectMode::None && Self::has_expect_attribute_on_field(field));

        // Check for explicit default() attribute on the field itself
        let has_explicit_default =
            Self::has_default_fn_attribute(field) || Self::has_any_default_attribute(field);

        let result = has_explicit_expect || has_explicit_default;

        if result {
            CallStackDebug::new(
                "analysis::optionality::FieldOptionality",
                "has_explicit_optional_usage_indicators",
                ctx.struct_name,
                ctx.field_name,
            ).checkpoint_data(
                "explicit_usage_found",
                &[
                    ("has_explicit_expect", &has_explicit_expect.to_string()),
                    ("has_explicit_default", &has_explicit_default.to_string()),
                ],
            );
        }

        result
    }

    // detect custom types that should be required by default
    fn is_custom_type_without_optional_indicators(
        ctx: &FieldProcessingContext,
        field_type: &syn::Type,
    ) -> bool {
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
                    CallStackDebug::new(
                        "analysis::optionality::FieldOptionality",
                        "is_custom_type_without_optional_indicators",
                        ctx.struct_name,
                        ctx.field_name,
                    )
                    .checkpoint_data(
                        "custom_type_detected",
                        &[
                            ("type_name", &segments[0].ident.to_string()),
                            ("is_primitive", &is_primitive.to_string()),
                            ("is_std_type", &is_std_type.to_string()),
                            ("is_proto_type", &is_proto_type.to_string()),
                        ],
                    );
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
    #[allow(unused)]
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

    #[allow(unused)]
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

    #[allow(unused)]
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
            quote!(ctx.field_type)
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
}
