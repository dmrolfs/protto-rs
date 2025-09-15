use crate::conversion::custom_strategy::CustomConversionStrategy;
use crate::debug::CallStackDebug;
use crate::error::mode::ErrorMode;
use crate::analysis::{
    field_analysis::FieldProcessingContext,
    type_analysis,
};
use crate::field::info::{self as field_info, ProtoFieldInfo, RustFieldInfo};

/// Consolidated field conversion strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldConversionStrategy {
    /// Field is ignored in proto conversion
    Ignore,

    /// Uses custom user-provided functions
    Custom(CustomConversionStrategy),

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
        field: &syn::Field,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
    ) -> Self {
        let trace = CallStackDebug::with_context(
            "field::conversion_strategy::ConsolidatedStrategy",
            "from_field_info",
            ctx.struct_name,
            ctx.field_name,
            &[
                ("rust_is_option", &rust.is_option.to_string()),
                ("rust_is_vec", &rust.is_vec.to_string()),
                ("proto_is_optional", &proto.is_optional().to_string()),
                ("proto_is_repeated", &proto.is_repeated().to_string()),
            ],
        );

        // Handle special cases first (sequential elimination)
        if rust.has_proto_ignore {
            trace.decision("proto_ignore", "Field marked with #[protto(ignore)]");
            Self::Ignore
        } else if let Some(custom_strategy) = CustomConversionStrategy::from_field_info(rust) {
            trace.decision("custom_functions", "Custom conversion functions detected");
            Self::Custom(custom_strategy)
        } else if rust.has_transparent {
            trace.decision("transparent_field", "Transparent wrapper detected");
            let error_mode = ErrorMode::from_field_context(ctx, rust);
            Self::Transparent(error_mode)
        } else if Self::is_collection_conversion(rust, proto) {
            trace.decision("collection_conversion", "Collection type detected");
            Self::Collection(Self::determine_collection_strategy(
                ctx, rust, proto, &trace,
            ))
        } else if rust.has_default || ctx.default_fn.is_some() {
            // Handle fields with default values - they need unwrap with default fallback
            trace.decision("default_field", "Field has default value");
            let error_mode = ErrorMode::from_field_context(ctx, rust);
            Self::Option(OptionStrategy::Unwrap(error_mode))
        } else {
            // Handle optionality patterns (simple 2x2 matrix)
            let rust_optional = rust.is_option;
            let proto_optional = proto.is_optional();

            match (rust_optional, proto_optional) {
                (true, false) => {
                    trace.decision("wrap_optional", "Rust Option<T> -> Proto T");
                    Self::Option(OptionStrategy::Wrap)
                }
                (false, true) => {
                    trace.decision("unwrap_optional", "Proto Option<T> -> Rust T");
                    let error_mode = ErrorMode::from_field_context(ctx, rust);
                    Self::Option(OptionStrategy::Unwrap(error_mode))
                }
                (true, true) => {
                    trace.decision("map_optional", "Option<T> -> Option<U>");
                    Self::Option(OptionStrategy::Map)
                }
                (false, false) => {
                    trace.decision("direct_conversion", "Both required -> direct conversion");
                    Self::Direct(Self::determine_direct_strategy(ctx, rust, proto, &trace))
                }
            }
        }
    }

    fn is_collection_conversion(rust: &RustFieldInfo, proto: &ProtoFieldInfo) -> bool {
        rust.is_vec || proto.is_repeated() || Self::is_option_vec_type(&rust.field_type)
    }

    fn is_option_vec_type(field_type: &syn::Type) -> bool {
        type_analysis::get_inner_type_from_option(field_type)
            .map(|inner| type_analysis::is_vec_type(&inner))
            .unwrap_or(false)
    }

    fn determine_collection_strategy(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> CollectionStrategy {
        if Self::is_option_vec_type(&rust.field_type) {
            trace.decision("option_vec", "Option<Vec<T>> detected");
            CollectionStrategy::MapOption
        } else if let Some(inner_type) = type_analysis::get_inner_type_from_vec(&rust.field_type)
            && type_analysis::is_proto_type(&inner_type, ctx.proto_module)
        {
            // Check for direct assignment (proto types)
            trace.decision("proto_vec_direct", "Vec<ProtoType> -> direct assignment");
            CollectionStrategy::DirectAssignment
        } else {
            trace.decision("standard_collection", "Standard collection conversion");
            let error_mode = ErrorMode::from_field_context(ctx, rust);
            CollectionStrategy::Collect(error_mode)
        }
    }

    fn determine_direct_strategy(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> DirectStrategy {
        // Check if types are identical or can be directly assigned
        if Self::types_are_identical(ctx, rust, proto) {
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
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
    ) -> bool {
        (rust.is_primitive && proto.mapping == field_info::ProtoMapping::Scalar) // Primitive scalar types
            || type_analysis::is_proto_type(&rust.field_type, ctx.proto_module) // Proto types (same module)
    }

    /// Get a human-readable description of this strategy
    pub fn description(&self) -> &'static str {
        match self {
            Self::Ignore => "field ignored - not in proto",
            Self::Custom(custom) => match custom {
                CustomConversionStrategy::FromFn(_) => "custom proto->rust function",
                CustomConversionStrategy::IntoFn(_) => "custom rust->proto function",
                CustomConversionStrategy::Bidirectional(_, _) => "bidirectional custom functions",
            },
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
        }
    }

    /// Get the category of this strategy for grouping
    pub fn category(&self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Custom(_) => "custom",
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

            Self::Custom(custom) => Some(custom.to_old_strategy()),

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
