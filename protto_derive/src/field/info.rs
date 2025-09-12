use crate::debug::CallStackDebug;
use quote::quote;
use crate::analysis::{
    attribute_parser,
    expect_analysis::ExpectMode,
    field_analysis::{CollectionType, FieldProcessingContext,},
    optionality::FieldOptionality,
    type_analysis,
};

#[derive(Clone)]
pub struct RustFieldInfo {
    pub field_type: syn::Type,
    // pub type_name: String,
    pub is_option: bool,
    pub is_vec: bool,
    pub is_primitive: bool,
    pub is_custom: bool,
    pub is_enum: bool,
    pub has_transparent: bool,
    pub has_default: bool,
    pub expect_mode: ExpectMode,
    pub has_proto_ignore: bool,
    pub from_proto_fn: Option<String>,
    pub to_proto_fn: Option<String>,
}

impl std::fmt::Debug for RustFieldInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RustFieldInfo")
            .field("field_type", &quote!(self.field_type).to_string())
            .field("is_option", &self.is_option)
            .field("is_vec", &self.is_vec)
            .field("is_primitive", &self.is_primitive)
            .field("is_custom", &self.is_custom)
            .field("is_enum", &self.is_enum)
            .field("has_transparent", &self.has_transparent)
            .field("has_default", &self.has_default)
            .field("expect_mode", &self.expect_mode)
            .field("has_proto_ignore", &self.has_proto_ignore)
            .field("from_proto_fn", &self.from_proto_fn)
            .field("to_proto_fn", &self.to_proto_fn)
            .finish()
    }
}

impl RustFieldInfo {
    pub fn analyze(ctx: &FieldProcessingContext, field: &syn::Field) -> Self {
        let field_type = ctx.field_type.clone();
        let is_option = type_analysis::is_option_type(&field_type);
        let is_vec = type_analysis::is_vec_type(&field_type);
        let is_primitive = type_analysis::is_primitive_type(&field_type);
        let is_custom = type_analysis::is_custom_type(&field_type);
        let is_enum = type_analysis::is_enum_type(&field_type);

        Self {
            field_type,
            is_option,
            is_vec,
            is_primitive,
            is_custom,
            is_enum,
            has_transparent: attribute_parser::has_transparent_attr(field),
            has_default: ctx.has_default,
            expect_mode: ctx.expect_mode,
            has_proto_ignore: attribute_parser::has_proto_ignore(field),
            from_proto_fn: ctx.proto_meta.get_proto_to_rust_fn().map(|s| s.to_string()),
            to_proto_fn: ctx.proto_meta.get_rust_to_proto_fn().map(|s| s.to_string()),
        }
    }

    pub fn type_name(&self) -> String {
        let field_type = &self.field_type;
        quote!(#field_type).to_string()
    }

    pub fn get_inner_type(&self) -> Option<syn::Type> {
        if self.is_option {
            type_analysis::get_inner_type_from_option(&self.field_type)
        } else if self.is_vec {
            type_analysis::get_inner_type_from_vec(&self.field_type)
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ProtoMapping {
    Scalar,        // proto scalar field
    Optional,      // proto optional field
    Repeated,      // proto repeated field
    Message,       // proto message field
    CustomDerived, // handled by custom derive functions
}

impl ProtoMapping {
    #[inline]
    pub fn is_repeated(&self) -> bool {
        matches!(self, Self::Repeated)
    }

    #[inline]
    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional)
    }

    #[allow(unused)]
    #[inline]
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar)
    }

    #[allow(unused)]
    #[inline]
    pub fn is_message(&self) -> bool {
        matches!(self, Self::Message)
    }

    #[allow(unused)]
    #[inline]
    pub fn is_custom_derived(&self) -> bool {
        matches!(self, Self::CustomDerived)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProtoFieldInfo {
    pub type_name: String,
    pub mapping: ProtoMapping,
    pub optionality: FieldOptionality,
}

impl ProtoFieldInfo {
    #[inline]
    pub fn is_optional(&self) -> bool {
        !self.mapping.is_repeated()
            && (self.optionality.is_optional() || self.mapping.is_optional())
    }

    #[inline]
    pub fn is_repeated(&self) -> bool {
        self.mapping.is_repeated()
    }
}

impl ProtoFieldInfo {
    pub fn infer_from(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
    ) -> Self {
        let _trace = CallStackDebug::with_context(
            "ProtoFieldInfo::infer_from",
            ctx.struct_name,
            ctx.field_name,
            &[
                ("is_rust_vec", &rust_field.is_vec.to_string()),
                ("is_rust_primitive", &rust_field.is_primitive.to_string()),
                ("is_rust_custom", &rust_field.is_custom.to_string()),
                ("is_rust_enum", &rust_field.is_enum.to_string()),
            ],
        );

        let type_name = Self::infer_proto_type_name(ctx, rust_field);

        if rust_field.from_proto_fn.is_some() || rust_field.to_proto_fn.is_some() {
            // Priority 1 - Handle custom derive scenarios first
            Self::infer_for_custom_derive(ctx, field, rust_field, type_name, &_trace)
        } else if Self::is_any_collection_type(ctx.field_type) {
            // Priority 2 - Handle collection types (including nested Options)
            Self::infer_for_collection_type(ctx, field, rust_field, type_name, &_trace)
        } else {
            // Priority 3 - Handle standard field patterns
            Self::infer_for_standard_field(ctx, field, rust_field, type_name, &_trace)
        }
    }

    fn is_any_collection_type(field_type: &syn::Type) -> bool {
        // Handle Option<Vec<T>>, Option<HashMap<K,V>>, etc.
        if let Some(inner_type) = type_analysis::get_inner_type_from_option(field_type) {
            return Self::is_direct_collection_type(&inner_type);
        }

        Self::is_direct_collection_type(field_type)
    }

    fn is_direct_collection_type(field_type: &syn::Type) -> bool {
        let type_str = quote!(#field_type).to_string();

        type_str.contains("Vec<")
            || type_str.contains("HashMap<")
            || type_str.contains("BTreeMap<")
            || type_str.contains("HashSet<")
            || type_str.contains("BTreeSet<")
            || type_str.contains("VecDeque<")
            || CollectionType::from_field_type(field_type).is_some()
    }

    fn infer_for_custom_derive(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
        type_name: String,
        trace: &CallStackDebug,
    ) -> Self {
        // For custom derives, determine proto mapping from transformation pattern
        let mapping = if Self::is_any_collection_type(ctx.field_type) {
            // Collection with custom derive -> proto repeated field
            trace.decision(
                "custom_derive_collection",
                "Collection + custom derive -> repeated proto field",
            );
            ProtoMapping::Repeated
        } else if rust_field.is_primitive {
            // Primitive with custom derive -> proto scalar/optional
            trace.decision(
                "custom_derive_primitive",
                "Primitive + custom derive -> scalar/optional proto field",
            );
            if Self::has_optional_indicators(ctx, field) {
                ProtoMapping::Optional
            } else {
                ProtoMapping::Scalar
            }
        } else {
            // Complex custom derive transformation
            trace.decision(
                "custom_derive_complex",
                "Complex custom derive -> CustomDerived mapping",
            );
            ProtoMapping::CustomDerived
        };

        let optionality = ctx
            .proto_meta
            .get_proto_optionality()
            .copied()
            .unwrap_or_else(|| {
                if mapping.is_repeated() {
                    FieldOptionality::Required // repeated fields are never optional
                } else {
                    Self::determine_optionality_from_context(ctx, field, trace)
                }
            });

        Self {
            type_name,
            mapping,
            optionality,
        }
    }

    fn infer_for_collection_type(
        ctx: &FieldProcessingContext,
        _field: &syn::Field,
        _rust_field: &RustFieldInfo,
        type_name: String,
        trace: &CallStackDebug,
    ) -> Self {
        trace.decision(
            "collection_type_detected",
            "Collection type -> repeated proto field",
        );

        let optionality = ctx
            .proto_meta
            .get_proto_optionality()
            .copied()
            .unwrap_or(FieldOptionality::Required); // Collections are typically required

        Self {
            type_name,
            mapping: ProtoMapping::Repeated,
            optionality,
        }
    }

    fn infer_for_standard_field(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
        type_name: String,
        trace: &CallStackDebug,
    ) -> Self {
        if let Some(user_specified) = Self::get_explicit_user_optionality(ctx, rust_field, trace) {
            // Check for explicit user annotations
            let mapping = Self::determine_mapping_from_optionality_and_type(
                user_specified,
                rust_field,
                trace,
            );
            Self::create_field_info(type_name, mapping, user_specified, trace)
        } else if let Some(info) = Self::infer_from_context_patterns(ctx, rust_field, trace) {
            info
        } else {
            // Infer from actual proto schema generation patterns
            let (mapping, optionality) =
                Self::infer_from_proto_schema_patterns(ctx, field, rust_field, trace);
            Self::create_field_info(type_name, mapping, optionality, trace)
        }
    }

    fn infer_from_context_patterns(
        ctx: &FieldProcessingContext,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> Option<Self> {
        if Self::has_proto_optionality_indicators(ctx, rust_field, trace) {
            // Pattern - Use existing attribute analysis to detect proto optionality
            trace.decision(
                "context_proto_optionality_detected",
                "Field context indicates proto optional field",
            );

            let mapping = if rust_field.is_enum {
                ProtoMapping::Scalar // Enums become i32 in proto
            } else if Self::is_likely_message_type(ctx, rust_field) {
                ProtoMapping::Message
            } else {
                ProtoMapping::Scalar
            };

            Some(Self {
                type_name: Self::infer_proto_type_name(ctx, rust_field),
                mapping,
                optionality: FieldOptionality::Optional,
            })
        } else if let Some(inferred) = Self::infer_from_struct_context(ctx, rust_field, trace) {
            // Pattern - Analyze field's structural context within the struct
            Some(inferred)
        } else if rust_field.has_transparent {
            // Pattern - Use existing transparent field detection
            trace.decision(
                "context_transparent_detected",
                "Transparent attribute indicates unwrap to inner type",
            );
            Some(Self {
                type_name: Self::infer_proto_type_name(ctx, rust_field),
                mapping: ProtoMapping::Scalar,
                optionality: FieldOptionality::Required,
            })
        } else {
            None
        }
    }

    fn has_proto_optionality_indicators(
        ctx: &FieldProcessingContext,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> bool {
        // Check multiple systematic indicators (not hardcoded type patterns)
        let has_default_indicators = rust_field.has_default || ctx.default_fn.is_some();
        let has_expect_indicators = !matches!(rust_field.expect_mode, ExpectMode::None);
        let has_explicit_optional = ctx.proto_meta.is_proto_optional();

        let result = has_default_indicators || has_expect_indicators || has_explicit_optional;

        if result {
            trace.checkpoint_data(
                "proto_optionality_indicators_found",
                &[
                    ("has_default", &has_default_indicators.to_string()),
                    ("has_expect", &has_expect_indicators.to_string()),
                    ("has_explicit", &has_explicit_optional.to_string()),
                ],
            );
        }

        result
    }

    fn infer_from_struct_context(
        ctx: &FieldProcessingContext,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> Option<Self> {
        if type_analysis::is_proto_type(&rust_field.field_type, ctx.proto_module) {
            // Prost generates message fields as Option<T> even when proto schema shows required
            if rust_field.is_custom && !rust_field.is_enum {
                // This is a proto message type (Header, Track, etc.)
                // Prost generates these as: #[prost(message, optional, tag = "N")] pub field: Option<MessageType>
                // But user struct expects: pub field: proto::MessageType

                trace.decision(
                    "proto_message_type_with_prost_optional_pattern",
                    "Proto message type: prost generates as Option<T>, user expects T",
                );

                // The actual prost field is Option<MessageType>, needs unwrapping
                Some(Self {
                    type_name: Self::infer_proto_type_name(ctx, rust_field),
                    mapping: ProtoMapping::Optional, // prost field is Option<T>
                    optionality: FieldOptionality::Optional, // prost generates as optional
                })
            } else {
                // Non-message proto types (enums, primitives) follow different patterns
                trace.decision(
                    "proto_non_message_type",
                    "Proto non-message type -> required",
                );
                Some(Self {
                    type_name: Self::infer_proto_type_name(ctx, rust_field),
                    mapping: ProtoMapping::Message,
                    optionality: FieldOptionality::Required,
                })
            }
        } else if Self::is_any_collection_type(ctx.field_type) {
            // Use existing collection analysis
            trace.decision(
                "struct_context_collection_detected",
                "Collection type indicates repeated proto field",
            );
            Some(Self {
                type_name: Self::infer_proto_type_name(ctx, rust_field),
                mapping: ProtoMapping::Repeated,
                optionality: FieldOptionality::Required,
            })
        } else {
            None
        }
    }

    fn is_likely_message_type(ctx: &FieldProcessingContext, rust_field: &RustFieldInfo) -> bool {
        // Use existing type analysis rather than hardcoded patterns
        let is_enum = rust_field.is_enum;
        let is_proto_module_type =
            type_analysis::is_proto_type(&rust_field.field_type, ctx.proto_module);

        // Enums typically become scalar fields (i32), proto module types become messages
        !is_enum && (is_proto_module_type || (!rust_field.is_primitive && rust_field.is_custom))
    }

    fn get_explicit_user_optionality(
        ctx: &FieldProcessingContext,
        _rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> Option<FieldOptionality> {
        if ctx.proto_meta.is_proto_optional() {
            trace.decision(
                "explicit_proto_optional_attribute",
                "proto(optional = true) found",
            );
            Some(FieldOptionality::Optional)
        } else if let Some(explicit_optionality) = ctx.proto_meta.get_proto_optionality() {
            match explicit_optionality {
                FieldOptionality::Optional => {
                    trace.decision("explicit_proto_optionality", "User specified: Optional");
                    Some(FieldOptionality::Optional)
                }
                FieldOptionality::Required => {
                    // proto_required is for validation semantics, not field type detection
                    // Let the actual proto schema detection determine field optionality
                    trace.decision(
                        "proto_required_attribute",
                        "proto_required affects validation only, not field type detection",
                    );
                    None // Fall back to schema-based detection
                }
            }
        } else {
            None
        }
    }

    fn infer_from_proto_schema_patterns(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> (ProtoMapping, FieldOptionality) {
        if rust_field.is_enum {
            // Pattern: Rust enums -> prost(enumeration = "EnumName") -> i32 (required scalar)
            Self::handle_enum_pattern(ctx, trace)
        } else if rust_field.has_transparent {
            // Pattern: Transparent wrapper types -> unwrap to inner type
            Self::handle_transparent_pattern(ctx, field, trace)
        } else if rust_field.is_option {
            // Pattern: Option<T> wrapper -> prost(type, optional) -> Option<ProtoType>
            Self::handle_option_wrapper_pattern(ctx, rust_field, trace)
        } else if rust_field.is_primitive {
            // Pattern: Primitive types -> prost(primitive_type) -> PrimitiveType (required)
            Self::handle_primitive_pattern(ctx, trace)
        } else if rust_field.is_custom {
            // Pattern: Custom types - This was the problematic area
            Self::handle_custom_type_pattern(ctx, rust_field, trace)
        } else {
            Self::handle_fallback_pattern(trace)
        }
    }

    fn handle_enum_pattern(
        ctx: &FieldProcessingContext,
        trace: &CallStackDebug,
    ) -> (ProtoMapping, FieldOptionality) {
        // Check if user has optional indicators despite enum type
        if Self::has_optional_usage_indicators(ctx) {
            trace.decision(
                "enum_with_optional_indicators",
                "Enum + expect/default -> optional i32",
            );
            (ProtoMapping::Optional, FieldOptionality::Optional)
        } else {
            trace.decision(
                "enum_standard_pattern",
                "Enum -> prost(enumeration) -> required i32 scalar",
            );
            (ProtoMapping::Scalar, FieldOptionality::Required)
        }
    }

    fn handle_transparent_pattern(
        ctx: &FieldProcessingContext,
        _field: &syn::Field,
        trace: &CallStackDebug,
    ) -> (ProtoMapping, FieldOptionality) {
        if Self::has_optional_usage_indicators(ctx) {
            trace.decision(
                "transparent_with_optional_usage",
                "Transparent + optional indicators -> optional scalar/message",
            );
            (ProtoMapping::Optional, FieldOptionality::Optional)
        } else {
            trace.decision(
                "transparent_unwrap_to_inner",
                "Transparent -> prost(inner_type) -> required (unwraps to inner type)",
            );
            (ProtoMapping::Scalar, FieldOptionality::Required)
        }
    }

    fn handle_option_wrapper_pattern(
        ctx: &FieldProcessingContext,
        _rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> (ProtoMapping, FieldOptionality) {
        // Option<T> always becomes optional in proto
        let inner_mapping =
            if let Some(inner_type) = type_analysis::get_inner_type_from_option(ctx.field_type) {
                if type_analysis::is_primitive_type(&inner_type) {
                    ProtoMapping::Scalar
                } else {
                    ProtoMapping::Message
                }
            } else {
                ProtoMapping::Message
            };

        trace.decision(
            "option_wrapper_pattern",
            "Option<T> -> prost(type, optional) -> Option<ProtoType>",
        );
        (inner_mapping, FieldOptionality::Optional)
    }

    fn handle_primitive_pattern(
        ctx: &FieldProcessingContext,
        trace: &CallStackDebug,
    ) -> (ProtoMapping, FieldOptionality) {
        if Self::has_optional_usage_indicators(ctx) {
            trace.decision(
                "primitive_with_default_indicators",
                "Primitive + default -> prost(primitive_type, optional) -> Option<PrimitiveType>",
            );
            (ProtoMapping::Optional, FieldOptionality::Optional) // Changed from Scalar/Required
        } else {
            trace.decision(
                "primitive_standard_pattern",
                "Primitive -> prost(primitive_type) -> required scalar",
            );
            (ProtoMapping::Scalar, FieldOptionality::Required)
        }
    }

    fn handle_custom_type_pattern(
        ctx: &FieldProcessingContext,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> (ProtoMapping, FieldOptionality) {
        // not all custom types become optional proto messages!

        if rust_field.has_transparent {
            trace.decision(
                "transparent_custom_type",
                "Transparent custom type -> prost(inner_type) -> required field",
            );
            (ProtoMapping::Scalar, FieldOptionality::Required)
        } else if type_analysis::is_proto_type(&rust_field.field_type, ctx.proto_module) {
            // This custom type is actually a proto type - should be required message
            trace.decision(
                "proto_module_custom_type",
                "Custom type from proto module -> required message field",
            );
            (ProtoMapping::Message, FieldOptionality::Required)
        } else if Self::has_optional_usage_indicators(ctx) {
            trace.decision(
                "custom_type_with_optional_indicators",
                "Custom type + expect/default -> prost(message, optional) -> Option<MessageType>",
            );
            (ProtoMapping::Optional, FieldOptionality::Optional)
        } else {
            // For all other custom types, default to optional
            // This matches the observed proto schema where custom types become Option<T>
            trace.decision(
                "custom_type_default_optional",
                "Custom type -> prost(type, optional) -> Option<Type> (default behavior)",
            );
            (ProtoMapping::Optional, FieldOptionality::Optional)
        }
    }

    fn handle_fallback_pattern(trace: &CallStackDebug) -> (ProtoMapping, FieldOptionality) {
        trace.decision(
            "fallback_pattern",
            "Unknown pattern -> required scalar (conservative default)",
        );
        (ProtoMapping::Scalar, FieldOptionality::Required)
    }

    fn has_optional_usage_indicators(ctx: &FieldProcessingContext) -> bool {
        !matches!(ctx.expect_mode, ExpectMode::None)
            || ctx.has_default
            || ctx.default_fn.is_some()
            || ctx.proto_meta.default_fn.is_some()
    }

    fn determine_mapping_from_optionality_and_type(
        optionality: FieldOptionality,
        rust_field: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> ProtoMapping {
        match optionality {
            FieldOptionality::Optional => {
                trace.decision(
                    "user_specified_optional",
                    "User specified optional -> Optional mapping",
                );
                ProtoMapping::Optional
            }
            FieldOptionality::Required => {
                if rust_field.is_enum || rust_field.is_primitive {
                    trace.decision(
                        "user_specified_required_scalar",
                        "User specified required scalar type",
                    );
                    ProtoMapping::Scalar
                } else {
                    trace.decision(
                        "user_specified_required_message",
                        "User specified required message type",
                    );
                    ProtoMapping::Message
                }
            }
        }
    }

    fn create_field_info(
        type_name: String,
        mapping: ProtoMapping,
        optionality: FieldOptionality,
        trace: &CallStackDebug,
    ) -> Self {
        trace.checkpoint_data(
            "standard_field_determined",
            &[
                ("mapping", &format!("{:?}", mapping)),
                ("optionality", &format!("{:?}", optionality)),
                (
                    "is_proto_optional",
                    &(mapping.is_optional() || optionality.is_optional()).to_string(),
                ),
            ],
        );

        Self {
            type_name,
            mapping,
            optionality,
        }
    }

    fn has_optional_indicators(ctx: &FieldProcessingContext, field: &syn::Field) -> bool {
        !matches!(ctx.expect_mode, ExpectMode::None)
            || ctx.has_default
            || Self::has_explicit_optional_attrs(field)
    }

    fn has_explicit_optional_attrs(field: &syn::Field) -> bool {
        attribute_parser::ProtoFieldMeta::from_field(field)
            .map(|proto_meta| proto_meta.expect || proto_meta.default_fn.is_some())
            .unwrap_or(false)
    }

    fn determine_optionality_from_context(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        trace: &CallStackDebug,
    ) -> FieldOptionality {
        // Priority 1 - Check for explicit user annotations
        if let Some(explicit_optionality) = ctx.proto_meta.get_proto_optionality() {
            trace.decision(
                "explicit_proto_optionality",
                &format!("User specified: {:?}", explicit_optionality),
            );
            return *explicit_optionality;
        }

        // Priority 2 - Detect from proto field patterns
        if let Some(inferred_optionality) = Self::infer_from_proto_patterns(ctx, field, trace) {
            return inferred_optionality;
        }

        // Priority 3 - Analyze rust field type to infer proto characteristics
        // Key insight: Option<CustomType> in rust often maps to optional message in proto
        if type_analysis::is_option_type(ctx.field_type) {
            if let Some(inner_type) = type_analysis::get_inner_type_from_option(ctx.field_type) {
                if type_analysis::is_custom_type(&inner_type)
                    || type_analysis::is_enum_type(&inner_type)
                {
                    trace.decision(
                        "option_custom_type",
                        "Option<CustomType> -> likely optional proto message",
                    );
                    return FieldOptionality::Optional;
                }
                if type_analysis::is_primitive_type(&inner_type) {
                    trace.decision(
                        "option_primitive_type",
                        "Option<PrimitiveType> -> likely optional proto scalar",
                    );
                    return FieldOptionality::Optional;
                }
            }
            // Generic Option<T> case
            trace.decision("generic_option_type", "Option<T> -> proto optional");
            return FieldOptionality::Optional;
        }

        // Priority 4: Custom types without Option wrapper
        if type_analysis::is_enum_type(ctx.field_type) {
            // Enums map to i32 in proto, typically required unless explicitly marked optional
            if Self::has_optional_indicators(ctx, field) {
                trace.decision(
                    "enum_with_indicators",
                    "Enum + expect/default -> optional i32",
                );
                return FieldOptionality::Optional;
            } else {
                trace.decision("enum_without_indicators", "Enum -> required proto i32");
                return FieldOptionality::Required;
            }
        }

        // Priority 5: Custom types (non-enum)
        if type_analysis::is_custom_type(ctx.field_type)
            && !type_analysis::is_enum_type(ctx.field_type)
        {
            // Check if it's a transparent field first
            if attribute_parser::has_transparent_attr(field) {
                // Transparent fields map to their inner type - follow existing transparent logic
                if Self::has_optional_indicators(ctx, field) {
                    trace.decision(
                        "transparent_custom_with_indicators",
                        "Transparent custom type + expect/default -> optional",
                    );
                    return FieldOptionality::Optional;
                } else {
                    trace.decision(
                        "transparent_custom_without_indicators",
                        "Transparent custom type -> required",
                    );
                    return FieldOptionality::Required;
                }
            }

            // Non-transparent, non-enum custom types - key insight from your prost output
            // Custom message types in proto are typically generated as Option<MessageType>
            trace.decision(
                "non_enum_custom_type_to_optional_proto",
                "Non-enum custom type -> likely optional proto message field",
            );
            return FieldOptionality::Optional;
        }

        // Priority 6 - Fallback to existing logic
        trace.checkpoint("Falling back to existing optionality detection");
        FieldOptionality::from_field_context(ctx, field)
    }

    /// Infer proto optionality from patterns and context
    fn infer_from_proto_patterns(
        ctx: &FieldProcessingContext,
        field: &syn::Field,
        trace: &CallStackDebug,
    ) -> Option<FieldOptionality> {
        if Self::has_optional_indicators(ctx, field) {
            // Pattern 1: If Rust field has explicit optional usage indicators, proto is likely optional
            trace.decision(
                "optional_indicators_found",
                "proto field is likely optional",
            );
            Some(FieldOptionality::Optional)
        } else if type_analysis::is_primitive_type(ctx.field_type) {
            trace.decision("primitive_no_indicators", "proto field likely required");
            Some(FieldOptionality::Required)
        } else {
            trace.checkpoint("no clear proto pattern detected");
            None
        }
    }

    fn infer_proto_type_name(ctx: &FieldProcessingContext, rust_field: &RustFieldInfo) -> String {
        if rust_field.has_transparent {
            // transparent fields use inner type
            "inner_type".to_string()
        } else if rust_field.is_custom && !Self::is_likely_proto_type(ctx, rust_field) {
            let type_name = rust_field.type_name();
            // custom type may map to proto message types
            if type_name.contains("::") {
                type_name
            } else {
                format!("{}::{}", ctx.proto_module, type_name)
            }
        } else {
            // primitives map directly
            rust_field.type_name()
        }
    }

    // further proto type detection to avoid double-prefixing and improve resilience
    fn is_likely_proto_type(ctx: &FieldProcessingContext, rust_field: &RustFieldInfo) -> bool {
        let type_name = rust_field.type_name();

        if type_name.starts_with(&format!("{}::", ctx.proto_module))
            || type_name.starts_with("proto::")
        {
            // Check for explicit proto module prefixes
            true
        } else if let Ok(parsed_type) = syn::parse_str::<syn::Type>(&type_name)
            && let syn::Type::Path(type_path) = parsed_type
        {
            // More resilient detection - parse as syn::Type and check path segments
            // Check if any segment matches the proto module
            type_path
                .path
                .segments
                .iter()
                .any(|segment| segment.ident == ctx.proto_module || segment.ident == "proto")
        } else {
            false
        }
    }
}
