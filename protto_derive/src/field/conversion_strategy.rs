use crate::analysis::expect_analysis::ExpectMode;
use crate::analysis::{field_analysis::FieldProcessingContext, type_analysis};
use crate::conversion::custom_strategy::CustomConversionStrategy;
use crate::debug::CallStackDebug;
use crate::error::mode::ErrorMode;
use crate::field::info::{self as field_info, ProtoFieldInfo, RustFieldInfo};

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

// Mapping from old strategies to new consolidated strategies
impl FieldConversionStrategy {
    /// Map from old strategy to new consolidated strategy (for migration/testing)
    pub fn from_old_strategy(strategy: &crate::conversion::ConversionStrategy) -> Option<Self> {
        match strategy {
            // Ignore
            crate::conversion::ConversionStrategy::ProtoIgnore => Some(Self::Ignore),

            // Custom functions
            crate::conversion::ConversionStrategy::DeriveProtoToRust(path) => {
                Some(Self::Custom(CustomConversionStrategy::FromFn(path.clone())))
            }
            crate::conversion::ConversionStrategy::DeriveRustToProto(path) => {
                Some(Self::Custom(CustomConversionStrategy::IntoFn(path.clone())))
            }
            crate::conversion::ConversionStrategy::DeriveBidirectional(from, into) => {
                Some(Self::Custom(CustomConversionStrategy::Bidirectional(
                    from.clone(),
                    into.clone(),
                )))
            }

            // Direct conversions
            crate::conversion::ConversionStrategy::DirectAssignment => {
                Some(Self::Direct(DirectStrategy::Assignment))
            }
            crate::conversion::ConversionStrategy::DirectWithInto => {
                Some(Self::Direct(DirectStrategy::WithConversion))
            }

            // Option handling
            crate::conversion::ConversionStrategy::WrapInSome => {
                Some(Self::Option(OptionStrategy::Wrap))
            }
            crate::conversion::ConversionStrategy::UnwrapOptionalWithExpect => {
                Some(Self::Option(OptionStrategy::Unwrap(ErrorMode::Panic)))
            }
            crate::conversion::ConversionStrategy::UnwrapOptionalWithError => {
                Some(Self::Option(OptionStrategy::Unwrap(ErrorMode::Error)))
            }
            crate::conversion::ConversionStrategy::UnwrapOptionalWithDefault => Some(Self::Option(
                OptionStrategy::Unwrap(ErrorMode::Default(None)),
            )),
            crate::conversion::ConversionStrategy::MapOption => {
                Some(Self::Option(OptionStrategy::Map))
            }
            crate::conversion::ConversionStrategy::MapOptionWithDefault => {
                Some(Self::Option(OptionStrategy::Map)) // Note: default handling in ErrorMode
            }
            crate::conversion::ConversionStrategy::UnwrapOptional => {
                Some(Self::Option(OptionStrategy::Unwrap(ErrorMode::None)))
            }

            // Transparent handling
            crate::conversion::ConversionStrategy::TransparentRequired => {
                Some(Self::Transparent(ErrorMode::None))
            }
            crate::conversion::ConversionStrategy::TransparentOptionalWithExpect => {
                Some(Self::Transparent(ErrorMode::Panic))
            }
            crate::conversion::ConversionStrategy::TransparentOptionalWithError => {
                Some(Self::Transparent(ErrorMode::Error))
            }
            crate::conversion::ConversionStrategy::TransparentOptionalWithDefault => {
                Some(Self::Transparent(ErrorMode::Default(None)))
            }

            // Collection handling
            crate::conversion::ConversionStrategy::CollectVec => Some(Self::Collection(
                CollectionStrategy::Collect(ErrorMode::None),
            )),
            crate::conversion::ConversionStrategy::CollectVecWithDefault => Some(Self::Collection(
                CollectionStrategy::Collect(ErrorMode::Default(None)),
            )),
            crate::conversion::ConversionStrategy::CollectVecWithError => Some(Self::Collection(
                CollectionStrategy::Collect(ErrorMode::Error),
            )),
            crate::conversion::ConversionStrategy::MapVecInOption => {
                Some(Self::Collection(CollectionStrategy::MapOption))
            }
            crate::conversion::ConversionStrategy::VecDirectAssignment => {
                Some(Self::Collection(CollectionStrategy::DirectAssignment))
            }

            // Strategies that don't have direct mappings or are obsolete
            _ => None,
        }
    }

    /// Convert back to old strategy format (for compatibility during migration)
    pub fn to_old_strategy(&self) -> Option<crate::conversion::ConversionStrategy> {
        match self {
            Self::Ignore => Some(crate::conversion::ConversionStrategy::ProtoIgnore),

            Self::Custom(custom) | Self::CustomWithError(custom, _) => {
                Some(custom.to_old_strategy())
            }

            Self::Direct(DirectStrategy::Assignment) => {
                Some(crate::conversion::ConversionStrategy::DirectAssignment)
            }
            Self::Direct(DirectStrategy::WithConversion) => {
                Some(crate::conversion::ConversionStrategy::DirectWithInto)
            }

            Self::Option(OptionStrategy::Wrap) => {
                Some(crate::conversion::ConversionStrategy::WrapInSome)
            }
            Self::Option(OptionStrategy::Unwrap(ErrorMode::Panic)) => {
                Some(crate::conversion::ConversionStrategy::UnwrapOptionalWithExpect)
            }
            Self::Option(OptionStrategy::Unwrap(ErrorMode::Error)) => {
                Some(crate::conversion::ConversionStrategy::UnwrapOptionalWithError)
            }
            Self::Option(OptionStrategy::Unwrap(ErrorMode::Default(_))) => {
                Some(crate::conversion::ConversionStrategy::UnwrapOptionalWithDefault)
            }
            Self::Option(OptionStrategy::Unwrap(ErrorMode::None)) => {
                Some(crate::conversion::ConversionStrategy::UnwrapOptional)
            }
            Self::Option(OptionStrategy::Map) => {
                Some(crate::conversion::ConversionStrategy::MapOption)
            }

            Self::Transparent(ErrorMode::None) => {
                Some(crate::conversion::ConversionStrategy::TransparentRequired)
            }
            Self::Transparent(ErrorMode::Panic) => {
                Some(crate::conversion::ConversionStrategy::TransparentOptionalWithExpect)
            }
            Self::Transparent(ErrorMode::Error) => {
                Some(crate::conversion::ConversionStrategy::TransparentOptionalWithError)
            }
            Self::Transparent(ErrorMode::Default(_)) => {
                Some(crate::conversion::ConversionStrategy::TransparentOptionalWithDefault)
            }

            Self::Collection(CollectionStrategy::Collect(ErrorMode::None)) => {
                Some(crate::conversion::ConversionStrategy::CollectVec)
            }
            Self::Collection(CollectionStrategy::Collect(ErrorMode::Default(_))) => {
                Some(crate::conversion::ConversionStrategy::CollectVecWithDefault)
            }
            Self::Collection(CollectionStrategy::Collect(ErrorMode::Error)) => {
                Some(crate::conversion::ConversionStrategy::CollectVecWithError)
            }
            Self::Collection(CollectionStrategy::MapOption) => {
                Some(crate::conversion::ConversionStrategy::MapVecInOption)
            }
            Self::Collection(CollectionStrategy::DirectAssignment) => {
                Some(crate::conversion::ConversionStrategy::VecDirectAssignment)
            }

            // Some combinations don't have direct old strategy equivalents
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_mapping_roundtrip() {
        let test_cases = vec![
            crate::conversion::ConversionStrategy::ProtoIgnore,
            crate::conversion::ConversionStrategy::DirectAssignment,
            crate::conversion::ConversionStrategy::DirectWithInto,
            crate::conversion::ConversionStrategy::WrapInSome,
            crate::conversion::ConversionStrategy::UnwrapOptionalWithExpect,
            crate::conversion::ConversionStrategy::CollectVec,
            crate::conversion::ConversionStrategy::TransparentRequired,
        ];

        for old_strategy in test_cases {
            let consolidated = FieldConversionStrategy::from_old_strategy(&old_strategy);
            assert!(
                consolidated.is_some(),
                "Should map old strategy: {:?}",
                old_strategy
            );

            let back_to_old = consolidated.unwrap().to_old_strategy();
            assert!(back_to_old.is_some(), "Should map back to old strategy");
            // Note: exact equality might not hold due to consolidation, but category should match
        }
    }

    #[test]
    fn test_decision_tree_logic() {
        // Test that identical inputs produce deterministic outputs
        //todo: need to create mock contexts and field info for testing
        // This is a placeholder showing the testing approach

        // let ctx = mock_field_context();
        // let rust_info = mock_rust_field_info();
        // let proto_info = mock_proto_field_info();

        // let strategy = ConsolidatedStrategy::from_field_info(&ctx, &field, &rust_info, &proto_info);
        // assert!(matches!(strategy, ConsolidatedStrategy::Direct(_)));
    }

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
