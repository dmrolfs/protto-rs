use crate::analysis::{expect_analysis::ExpectMode, type_analysis};
use crate::debug::CallStackDebug;
use crate::field::{
    FieldProcessingContext,
    custom_conversion::CustomConversionStrategy,
    error_mode::ErrorMode,
    info::{self as field_info, ProtoFieldInfo, RustFieldInfo},
};

/// Consolidated field conversion strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldConversionStrategy {
    /// Field is ignored in proto conversion
    Ignore,

    /// Uses custom user-provided functions
    Custom(CustomConversionStrategy),

    /// Custom functions that need error handling
    CustomWithError(CustomConversionStrategy, ErrorMode),

    /// Direct assignment or conversion between compatible types
    Direct(DirectStrategy),

    /// Handles optionality mismatches between rust and proto
    Option(OptionStrategy),

    /// Transparent wrapper field conversion
    Transparent(ErrorMode),

    /// Collection (Vec, etc.) conversions
    Collection(CollectionStrategy),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirectStrategy {
    /// T -> T (same types, no conversion)
    Assignment,

    /// T -> U (different types, use Into trait)
    WithConversion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionStrategy {
    /// T -> Some(T) (required -> optional)
    Wrap,

    /// Some(T) -> T (optional -> required)
    Unwrap(ErrorMode),

    /// Option<T> -> Option<U> (optional -> optional)
    Map,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionStrategy {
    /// Vec<T> -> Vec<U> with conversion
    Collect(ErrorMode),

    /// Option<Vec<T>> -> Option<Vec<U>>
    MapOption,

    /// Vec<ProtoType> -> Vec<ProtoType> (no conversion)
    DirectAssignment,
}

impl FieldConversionStrategy {
    /// Create consolidated strategy from field analysis using simplified decision tree
    pub fn from_field_info(
        ctx: &FieldProcessingContext,
        _field: &syn::Field,
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
    ) -> Self {
        let trace = CallStackDebug::with_context(
            "field::conversion_strategy::FieldConsolidatedStrategy",
            "from_field_info",
            ctx.struct_name,
            ctx.field_name,
            &[
                ("rust_is_option", &rust_field_info.is_option.to_string()),
                ("rust_is_vec", &rust_field_info.is_vec.to_string()),
                (
                    "proto_is_optional",
                    &proto_field_info.is_optional().to_string(),
                ),
                (
                    "proto_is_repeated",
                    &proto_field_info.is_repeated().to_string(),
                ),
            ],
        );

        // Handle special cases first (sequential elimination)
        if rust_field_info.has_proto_ignore {
            trace.decision("proto_ignore", "Field marked with #[protto(ignore)]");
            Self::Ignore
        } else if let Some(custom_strategy) =
            CustomConversionStrategy::from_field_info(ctx.struct_name, rust_field_info)
        {
            trace.decision("custom_functions", "Custom conversion functions detected");
            if Self::custom_needs_error_handling(ctx, rust_field_info, proto_field_info) {
                let error_mode = ErrorMode::from_field_context(ctx, rust_field_info);
                Self::CustomWithError(custom_strategy, error_mode)
            } else {
                Self::Custom(custom_strategy)
            }
        } else if rust_field_info.has_transparent {
            trace.decision("transparent_field", "Transparent wrapper detected");
            let error_mode = ErrorMode::from_field_context(ctx, rust_field_info);
            Self::Transparent(error_mode)
        } else if Self::is_collection_conversion(rust_field_info, proto_field_info) {
            trace.decision("collection_conversion", "Collection type detected");
            Self::Collection(Self::determine_collection_strategy(
                ctx,
                rust_field_info,
                proto_field_info,
                &trace,
            ))
        } else if rust_field_info.has_default || ctx.default_fn.is_some() {
            if rust_field_info.is_option
                && proto_field_info.is_optional()
                && ctx.default_fn.is_none()
                && !rust_field_info.has_default
            {
                trace.decision("map_optional", "Option<T> -> Option<U>");
                Self::Option(OptionStrategy::Map)
            } else {
                trace.decision("default_field", "Field has default value");
                let error_mode = ErrorMode::from_field_context(ctx, rust_field_info);
                Self::Option(OptionStrategy::Unwrap(error_mode))
            }
        } else {
            // Handle optionality patterns (simple 2x2 matrix)
            let rust_optional = rust_field_info.is_option;
            let proto_optional = proto_field_info.is_optional();

            match (rust_optional, proto_optional) {
                (true, false) => {
                    trace.decision("wrap_optional", "Rust Option<T> -> Proto T");
                    Self::Option(OptionStrategy::Wrap)
                }
                (false, true) => {
                    trace.decision("unwrap_optional", "Proto Option<T> -> Rust T");
                    let error_mode = ErrorMode::from_field_context(ctx, rust_field_info);
                    Self::Option(OptionStrategy::Unwrap(error_mode))
                }
                (true, true) if rust_field_info.expect_mode == ExpectMode::None => {
                    trace.decision("map_optional", "Option<T> -> Option<U>");
                    Self::Option(OptionStrategy::Map)
                }
                (true, true) => {
                    trace.decision(
                        "unwrap_optional_for_some_wrap",
                        "Proto optional -> unwrap -> wrap in Some",
                    );
                    let error_mode = ErrorMode::from_field_context(ctx, rust_field_info);
                    Self::Option(OptionStrategy::Unwrap(error_mode))
                }
                (false, false) => {
                    trace.decision("direct_conversion", "Both required -> direct conversion");
                    Self::Direct(Self::determine_direct_strategy(
                        ctx,
                        rust_field_info,
                        proto_field_info,
                        &trace,
                    ))
                }
            }
        }
    }

    /// Determine if custom functions need error handling based on field context
    fn custom_needs_error_handling(
        ctx: &FieldProcessingContext,
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
    ) -> bool {
        // Custom functions need error handling when:

        // 1. Explicit error handling attributes
        if rust_field_info.expect_mode != crate::analysis::expect_analysis::ExpectMode::None {
            return true;
        }

        // 2. Field has error function specified
        if ctx.struct_level_error_fn.is_some() {
            return true;
        }

        // 3. Proto optional -> Rust required pattern (needs unwrapping)
        if proto_field_info.is_optional() && !rust_field_info.is_option {
            return true;
        }

        // 4. Collection that might be empty but rust expects content
        if proto_field_info.is_repeated()
            && rust_field_info.is_vec
            && !rust_field_info.has_default
            && !rust_field_info.is_option
        {
            return true;
        }

        // 5. Complex custom types with bidirectional functions get error handling by default (compatibility)
        if !rust_field_info.is_option
            && !rust_field_info.is_primitive
            && rust_field_info.is_custom
            && rust_field_info.from_proto_fn.is_some()
            && rust_field_info.to_proto_fn.is_some()
        {
            return true;
        }

        false
    }

    fn is_collection_conversion(
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
    ) -> bool {
        rust_field_info.is_vec
            || proto_field_info.is_repeated()
            || Self::is_option_vec_type(&rust_field_info.field_type)
    }

    fn is_option_vec_type(field_type: &syn::Type) -> bool {
        type_analysis::get_inner_type_from_option(field_type)
            .map(|inner| type_analysis::is_vec_type(&inner))
            .unwrap_or(false)
    }

    fn determine_collection_strategy(
        ctx: &FieldProcessingContext,
        rust_field_info: &RustFieldInfo,
        _proto_field_info: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> CollectionStrategy {
        let _trace = CallStackDebug::with_context(
            "field::converstion_strategy::FieldConversionStrategy",
            "determine_collection_strategy",
            ctx.struct_name,
            &rust_field_info.field_name,
            &[
                ("rust.has_default", &rust_field_info.has_default.to_string()),
                (
                    "rust.expect_mode",
                    &format!("{:?}", rust_field_info.expect_mode),
                ),
                ("ctx.default_fn", &format!("{:?}", ctx.default_fn)),
            ],
        );

        if Self::is_option_vec_type(&rust_field_info.field_type) {
            trace.decision("option_vec", "Option<Vec<T>> detected");
            CollectionStrategy::MapOption
        } else if let Some(inner_type) =
            type_analysis::get_inner_type_from_vec(&rust_field_info.field_type)
            && type_analysis::is_proto_type(&inner_type, ctx.proto_module)
        {
            // Check for direct assignment (proto types)
            trace.decision("proto_vec_direct", "Vec<ProtoType> -> direct assignment");
            CollectionStrategy::DirectAssignment
        } else if rust_field_info.has_default || ctx.default_fn.is_some() {
            trace.decision(
                "collection_with_default",
                "Collection with default detected",
            );
            // Only apply error handling for collections when there's a default (matches old system)
            let error_mode = ErrorMode::from_field_context(ctx, rust_field_info);
            match error_mode {
                ErrorMode::Error => {
                    trace.decision(
                        "rust_has_default_or_default_fn_w_error",
                        "Vec<ProtoType> -> Standard collection conversion",
                    );
                    CollectionStrategy::Collect(ErrorMode::Error)
                }
                ErrorMode::Default(_) => {
                    let default_fn = if ctx.default_fn.is_none() && rust_field_info.has_default {
                        None
                    } else {
                        ctx.default_fn.clone()
                    };

                    trace.decision("collection_default", "Collection with default value");
                    CollectionStrategy::Collect(ErrorMode::Default(default_fn))
                }
                _ => {
                    trace.decision(
                        "rust_has_default_or_default_fn_wo_error",
                        "Vec<ProtoType> -> Standard collection w default conversion",
                    );
                    CollectionStrategy::Collect(ErrorMode::Default(ctx.default_fn.clone()))
                }
            }
        } else {
            trace.decision("standard_collection", "Standard collection conversion");
            let error_mode = ErrorMode::None;
            CollectionStrategy::Collect(error_mode)
        }
    }

    fn determine_direct_strategy(
        ctx: &FieldProcessingContext,
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> DirectStrategy {
        // Check if types are identical or can be directly assigned
        if Self::types_are_identical(ctx, rust_field_info, proto_field_info) {
            trace.decision(
                "identical_types",
                "Types are identical -> direct assignment",
            );
            DirectStrategy::Assignment
        } else {
            trace.decision("conversion_needed", "Types differ -> conversion with Into");
            DirectStrategy::WithConversion
        }
    }

    fn types_are_identical(
        ctx: &FieldProcessingContext,
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
    ) -> bool {
        (rust_field_info.is_primitive && proto_field_info.mapping == field_info::ProtoMapping::Scalar) // Primitive scalar types
            || type_analysis::is_proto_type(&rust_field_info.field_type, ctx.proto_module) // Proto types (same module)
    }

    /// Get a human-readable description of this strategy
    #[allow(unused)]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Ignore => "field ignored - not in proto",
            Self::Direct(direct) => match direct {
                DirectStrategy::Assignment => "direct assignment (no conversion)",
                DirectStrategy::WithConversion => "direct conversion with Into",
            },
            Self::Option(option) => match option {
                OptionStrategy::Wrap => "wrap value in Some()",
                OptionStrategy::Unwrap(_) => "unwrap Optional with error handling",
                OptionStrategy::Map => "map through optional conversion",
            },
            Self::Transparent(_) => "transparent wrapper conversion",
            Self::Collection(collection) => match collection {
                CollectionStrategy::Collect(_) => "collect vector with conversion",
                CollectionStrategy::MapOption => "map optional vector",
                CollectionStrategy::DirectAssignment => "direct vector assignment",
            },
            Self::Custom(custom) | Self::CustomWithError(custom, ErrorMode::None) => match custom {
                CustomConversionStrategy::FromFn(_) => "custom proto->rust function",
                CustomConversionStrategy::IntoFn(_) => "custom rust->proto function",
                CustomConversionStrategy::Bidirectional(_, _) => "bidirectional custom functions",
            },
            Self::CustomWithError(custom, ErrorMode::Error) => match custom {
                CustomConversionStrategy::FromFn(_) => "custom proto->rust function + error",
                CustomConversionStrategy::IntoFn(_) => "custom rust->proto function + error",
                CustomConversionStrategy::Bidirectional(_, _) => {
                    "bidirectional custom functions + error"
                }
            },
            Self::CustomWithError(custom, ErrorMode::Panic) => match custom {
                CustomConversionStrategy::FromFn(_) => "custom proto->rust function + panic",
                CustomConversionStrategy::IntoFn(_) => "custom rust->proto function + panic",
                CustomConversionStrategy::Bidirectional(_, _) => {
                    "bidirectional custom functions + panic"
                }
            },
            Self::CustomWithError(custom, ErrorMode::Default(_)) => match custom {
                CustomConversionStrategy::FromFn(_) => "custom proto->rust function + default",
                CustomConversionStrategy::IntoFn(_) => "custom rust->proto function + default",
                CustomConversionStrategy::Bidirectional(_, _) => {
                    "bidirectional custom functions + default"
                }
            },
        }
    }

    /// Get the category of this strategy for grouping
    #[allow(unused)]
    pub fn category(&self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Custom(_) | Self::CustomWithError(_, _) => "custom",
            Self::Direct(_) => "direct",
            Self::Option(_) => "option",
            Self::Transparent(_) => "transparent",
            Self::Collection(_) => "collection",
        }
    }
}

/// Migration error types
#[derive(Debug)]
pub enum FieldGenerationError {
    ConversionValidation(String),
}

impl std::fmt::Display for FieldGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldGenerationError::ConversionValidation(msg) => {
                write!(f, "field conversion validation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for FieldGenerationError {}

/// Main entry point for field conversion with migration support
pub fn generate_field_conversions(
    field: &syn::Field,
    ctx: &FieldProcessingContext,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), FieldGenerationError> {
    // Analyze field using new system
    let rust_field_info = RustFieldInfo::analyze(ctx, field);
    let proto_field_info = ProtoFieldInfo::infer_from(ctx, field, &rust_field_info);
    let strategy =
        FieldConversionStrategy::from_field_info(ctx, field, &rust_field_info, &proto_field_info);

    // Validate strategy is reasonable
    strategy.validate_for_context(ctx, &rust_field_info, &proto_field_info)?;

    // Generate code
    let proto_to_rust =
        strategy.generate_proto_to_rust_conversion(ctx, field, &rust_field_info, &proto_field_info);
    let rust_to_proto =
        strategy.generate_rust_to_proto_conversion(ctx, field, &rust_field_info, &proto_field_info);

    Ok((proto_to_rust, rust_to_proto))
}

// Integration with existing field analysis
impl FieldConversionStrategy {
    /// Validate that this strategy is compatible with the given context
    pub fn validate_for_context(
        &self,
        _ctx: &FieldProcessingContext,
        rust_field_info: &RustFieldInfo,
        proto_field_info: &ProtoFieldInfo,
    ) -> Result<(), FieldGenerationError> {
        // Use the existing validation logic from the new system
        match self {
            FieldConversionStrategy::Ignore => {
                if !rust_field_info.has_proto_ignore {
                    return Err(FieldGenerationError::ConversionValidation(
                        "Ignore strategy requires #[protto(ignore)] attribute".to_string(),
                    ));
                }
            }
            FieldConversionStrategy::Custom(custom_strategy) => {
                custom_strategy
                    .validate()
                    .map_err(FieldGenerationError::ConversionValidation)?;
            }
            FieldConversionStrategy::Transparent(_) => {
                if !rust_field_info.has_transparent {
                    return Err(FieldGenerationError::ConversionValidation(
                        "Transparent strategy requires #[protto(transparent)] attribute"
                            .to_string(),
                    ));
                }
                // Additional transparent-specific validation could go here
            }
            FieldConversionStrategy::Collection(_) => {
                if !rust_field_info.is_vec && !proto_field_info.is_repeated() {
                    return Err(FieldGenerationError::ConversionValidation(
                        "Collection strategy requires Vec or repeated field".to_string(),
                    ));
                }
            }
            _ => {
                // Other strategies have their own validation logic
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_descriptions() {
        let ignore = FieldConversionStrategy::Ignore;
        assert_eq!(ignore.description(), "field ignored - not in proto");
        assert_eq!(ignore.category(), "ignore");

        let direct = FieldConversionStrategy::Direct(DirectStrategy::Assignment);
        assert_eq!(direct.category(), "direct");
        assert!(direct.description().contains("direct assignment"));
    }
}
