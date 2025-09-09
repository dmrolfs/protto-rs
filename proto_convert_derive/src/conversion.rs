use quote::quote;
use crate::debug::CallStackDebug;
use crate::expect_analysis::ExpectMode;
use crate::field_analysis::FieldProcessingContext;
use crate::field_info::{ProtoFieldInfo, ProtoMapping, RustFieldInfo};
use crate::optionality::FieldOptionality;
use crate::type_analysis;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConversionStrategy {
    // Ignore fields
    ProtoIgnore,

    // Custom derive functions
    DeriveFromWith(String),
    DeriveIntoWith(String),
    DeriveBidirectional(String, String),    // (from_with_path, into_with_path)

    // Transparent field handling
    TransparentRequired,                    // proto field is required
    TransparentOptionalWithExpect,          // proto optional -> rust required (expect)
    TransparentOptionalWithError,           // proto optional -> rust required (error)
    TransparentOptionalWithDefault,         // proto optional -> rust required (default)

    // Direct conversions (no wrapping/unwrapping)
    DirectAssignment,                       // T -> T (primitives, proto types)
    DirectWithInto,                         // CustomType -> ProtoType

    // Option handling
    WrapInSome,                            // T -> Some(T) (rust required -> proto optional)

    // - Scenario: rust Option<T> field where proto expects T (not Option<T>)
    // - Current gap: determine_option_strategy() always uses WrapInSome for (true, false, _)
    // - Valid case: when proto field is required but rust field is optional
    // - Should construct when: rust=Option<T>, proto=T, no expect/default attributes
    UnwrapOptional,                        // Option<T> -> T (for rust_to_proto)

    UnwrapOptionalWithExpect,              // Option<T> -> T (with expect)
    UnwrapOptionalWithError,               // Option<T> -> T (with error handling)
    UnwrapOptionalWithDefault,             // Option<T> -> T (with default fallback)
    MapOption,                             // Option<T> -> Option<U>
    MapOptionWithDefault,                  // Option<T> -> Some(t)

    // Collection handling
    CollectVec,                            // Vec<T> -> Vec<U>
    CollectVecWithDefault,                 // Vec<T> -> Vec<U> (with default when empty)
    CollectVecWithError,                   // Vec<T> -> Vec<U> (with error handling)
    #[allow(dead_code)]
    MapVecInOption,                        // Option<Vec<T>> -> Option<Vec<U>>
    VecDirectAssignment,                   // Vec<ProtoType> -> Vec<ProtoType> (no conversion needed)

    // Error cases
    // - Scenario: Impossible/complex combinations that need manual handling
    // - Should be constructed when validation detects invalid combinations
    // - Currently never constructed because validation logic isn't integrated
    #[allow(dead_code)]
    RequiresCustomLogic,                    // Complex cases needing manual handling
}

impl ConversionStrategy {
    /// Debug enhancement for better tracing
    pub fn debug_info(&self) -> &'static str {
        match self {
            Self::ProtoIgnore => "field ignored - not in proto",
            Self::DeriveFromWith(_) => "custom from function",
            Self::DeriveIntoWith(_) => "custom into function",
            Self::DeriveBidirectional(_, _) => "custom from and into functions",
            Self::TransparentRequired => "transparent field, proto required",
            Self::TransparentOptionalWithExpect => "transparent field, proto optional -> expect",
            Self::TransparentOptionalWithError => "transparent field, proto optional -> error",
            Self::TransparentOptionalWithDefault => "transparent field, proto optional -> default",
            Self::DirectAssignment => "direct assignment (no conversion)",
            Self::DirectWithInto => "direct conversion with Into",
            Self::WrapInSome => "wrap value in Some()",
            Self::UnwrapOptional => "unwrap option (rust_to_proto)",
            Self::UnwrapOptionalWithExpect => "proto optional -> rust required (expect)",
            Self::UnwrapOptionalWithError => "proto optional -> rust required (error)",
            Self::UnwrapOptionalWithDefault => "proto optional -> rust required (default)",
            Self::MapOptionWithDefault => "proto optional -> rust optional with default",
            Self::MapOption => "map option through conversion",
            Self::CollectVec => "collect vector with conversion",
            Self::CollectVecWithDefault => "collect vector with default fallback",
            Self::CollectVecWithError => "collect vector with error handling",
            Self::MapVecInOption => "map optional vector with conversion",
            Self::VecDirectAssignment => "direct vector assignment (proto types)",
            Self::RequiresCustomLogic => "complex case requiring manual handling",
        }
    }

    /// Strategy category for grouping related strategies
    pub fn category(&self) -> &'static str {
        match self {
            Self::ProtoIgnore => "ignore",

            Self::DeriveFromWith(_) | Self::DeriveIntoWith(_) | Self::DeriveBidirectional(_, _) => "custom_derive",

            Self::TransparentRequired | Self::TransparentOptionalWithExpect |
            Self::TransparentOptionalWithError | Self::TransparentOptionalWithDefault => "transparent",

            Self::DirectAssignment | Self::DirectWithInto => "direct",

            Self::WrapInSome | Self::UnwrapOptional | Self::UnwrapOptionalWithExpect |
            Self::UnwrapOptionalWithError | Self::UnwrapOptionalWithDefault |
            Self::MapOption | Self::MapOptionWithDefault => "option",

            Self::CollectVec | Self::CollectVecWithDefault | Self::CollectVecWithError |
            Self::MapVecInOption | Self::VecDirectAssignment => "collection",

            Self::RequiresCustomLogic => "error",
        }
    }

    // Validation to catch impossible strategy combinations early
    pub fn validate_for_context(
        &self,
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
    ) -> Result<(), String> {
        match self {
            // -- Validate unwrap strategies require proto optional --
            Self::UnwrapOptionalWithExpect | Self::UnwrapOptionalWithError | Self::UnwrapOptionalWithDefault => {
                if !proto.is_optional() {
                    return Err(format!(
                        "UnwrapOptional strategy '{}' requires proto field to be optional, but detected as required. \
                         Add #[proto(proto_optional)] if proto field is actually optional.",
                        self.debug_info()
                    ));
                }

                if ctx.proto_meta.is_proto_optional() {
                    Self::validate_proto_optional_usage(ctx, rust, proto)?;
                }
            },

            // -- Validate wrap strategies require rust non-optional to proto optional --
            Self::WrapInSome => {
                if rust.is_option {
                    return Err(
                        "WrapInSome strategy incompatible with rust Option type. Use MapOption instead.".to_string()
                    );
                }
                if !proto.mapping.is_optional() {
                    return Err(
                        "WrapInSome strategy requires proto field to be optional, but detected as required.".to_string()
                    );
                }
            },

            // -- Validate map option requires at least one side to be optional --
            Self::MapOption => {
                if !rust.is_option && !proto.mapping.is_optional() {
                    return Err(format!(
                        "MapOption strategy requires at least one side to be Option type. \
                         Rust: {}, Proto: {} - use DirectWithInto instead.",
                        if rust.is_option { "Option" } else { "Required" },
                        if proto.is_optional() { "Option" } else { "Required" }
                    ));
                }
            },

            // -- Validate transparent strategies require transparent attribute --
            Self::TransparentRequired | Self::TransparentOptionalWithExpect |
            Self::TransparentOptionalWithError | Self::TransparentOptionalWithDefault => {
                if !rust.has_transparent {
                    return Err(format!(
                        "Transparent strategy '{}' requires #[proto(transparent)] attribute on field.",
                        self.debug_info()
                    ));
                }
            },

            // -- Validate vector strategies require vector types --
            Self::CollectVec | Self::CollectVecWithDefault | Self::CollectVecWithError | Self::VecDirectAssignment => {
                if !rust.is_vec && !proto.mapping.is_repeated() {
                    return Err(format!(
                        "Vector strategy '{}' requires rust field to be Vec<T> type.",
                        self.debug_info()
                    ));
                }
            },

            // -- Validate option vec strategy --
            Self::MapVecInOption => {
                if !Self::is_option_vec_type(ctx.field_type) && !proto.mapping.is_repeated() {
                    return Err(format!(
                        "MapVecInOption strategy requires Option<Vec<T>> type, found: {}",
                        quote!(ctx.field_type)
                    ));
                }
            },

            // -- Validate derive strategies have paths --
            Self::DeriveFromWith(path) | Self::DeriveIntoWith(path) => {
                if path.is_empty() {
                    return Err("Derive strategy requires non-empty function path".to_string());
                }
                //todo: Could add path validation here
            },

            Self::DeriveBidirectional(from_path, into_path) => {
                if from_path.is_empty() || into_path.is_empty() {
                    return Err("DeriveBidirectional strategy requires non-empty function paths".to_string());
                }
            },

            // -- Validate ignore strategy --
            Self::ProtoIgnore => {
                if !rust.has_proto_ignore {
                    return Err(
                        "ProtoIgnore strategy requires #[proto(ignore)] attribute on field.".to_string()
                    );
                }
            },

            // -- Other strategies are always valid --
            _ => {}
        }

        Ok(())
    }

    /// Validate proto_optional attribute usage against strategy outcome
    pub fn validate_proto_optional_attribute(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        strategy: &ConversionStrategy,
    ) -> Result<(), String> {
        // Only validate if proto_optional attribute is present
        if !ctx.proto_meta.is_proto_optional() {
            return Ok(());
        }

        if rust.has_transparent {
            return Ok(());
        }

        if strategy == &Self::UnwrapOptionalWithExpect {
            // Only validate the specific problematic pattern:
            // - Primitive field with no attributes
            // - Same field name (suggests direct mapping confusion)
            // - No handling attributes
            if !rust.is_option && rust.is_primitive
            && !rust.has_default && matches!(rust.expect_mode, ExpectMode::None)
            && *ctx.field_name == ctx.proto_name {
                return Err(format!(
                    "INVALID #[proto(proto_optional)] on field '{}'.\n\
                    \n\
                    Field appears to be direct mapping but uses proto_optional.\n\
                    Proto schema likely shows: {} {} = N; (required)\n\
                    For proto_optional, should be: optional {} {} = N;\n\
                    \n\
                    Fix: Remove #[proto(proto_optional)] or add expect()/default if proto is optional",
                        ctx.field_name,
                        rust.type_name(),
                        ctx.field_name,
                        rust.type_name(),
                        ctx.field_name
                    ));
            }
        }

        Ok(())
    }

    pub fn from_field_info(
        ctx: &FieldProcessingContext,
        _field: &syn::Field,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo
    ) -> Self {
        let _trace = CallStackDebug::with_context(
            "ConversionStrategy::from_field_info",
            ctx.struct_name,
            ctx.field_name,
            &[
                ("rust_is_option", &rust.is_option.to_string()),
                ("rust_is_vec", &rust.is_vec.to_string()),
                ("proto_is_option", &proto.is_optional().to_string()),
                ("proto_is_repeated", &proto.is_repeated().to_string()),
                ("proto_optionality", &format!("{:?}", proto.optionality)),
            ],
        );

        // -- Handle special cases first --
        let result = if rust.has_proto_ignore {
            _trace.decision("has_proto_ignore", "ProtoIgnore");
            Self::ProtoIgnore
        } else if let (Some(from_with_path), Some(into_with_path)) = (&rust.derive_from_with, &rust.derive_into_with) {
            _trace.decision("has_both_derive_functions", "DeriveBidirectional");
            Self::DeriveBidirectional(from_with_path.clone(), into_with_path.clone())
        } else if let Some(from_with_path) = &rust.derive_from_with {
            _trace.decision("has_derive_from_with", "DeriveFromWith");
            Self::DeriveFromWith(from_with_path.clone())
        } else if let Some(into_with_path) = &rust.derive_into_with {
            _trace.decision("has_derive_into_with", "DeriveIntoWith");
            Self::DeriveIntoWith(into_with_path.clone())
        } else if rust.has_transparent && rust.is_option {
            _trace.decision("transparent_option_detected", "MapOption for transparent option");
            Self::MapOption
        } else if rust.has_transparent {
            // -- Handle transparent fields --
            Self::determine_transparent_strategy(ctx, rust, proto, &_trace)
        } else {
            match (rust.is_option, proto.mapping, rust.expect_mode) {
                // -- Collection handling first --
                (false, ProtoMapping::Repeated, _) => {
                    _trace.decision("rust_required + proto_repeated", "CollectVec or variant");
                    Self::determine_vec_strategy_by_context(ctx, rust, &_trace)
                },
                (true, ProtoMapping::Repeated, _) => {
                    _trace.decision("rust_optional + proto_repeated", "CollectVec for Option<Vec<T>> → Vec<U>");
                    Self::CollectVec
                },

                // Direct conversions (both required) - CHECK FIRST before option handling
                (false, ProtoMapping::Scalar | ProtoMapping::Message, _) if !proto.is_optional() => {
                    _trace.decision("both_required", "determine direct strategy");
                    Self::determine_direct_strategy(ctx, rust, proto, &_trace)
                },

                // Direct conversions for custom types without transparent attribute
                (false, ProtoMapping::Message, _) if !proto.is_optional() => {
                    _trace.decision("custom_type_direct_message", "Custom type -> direct message conversion");
                    Self::DirectWithInto
                },

                // -- Option unwrapping (proto optional -> rust required) --
                (false, _, ExpectMode::Panic) if proto.is_optional() => {
                    _trace.decision("rust_required + proto_optional + expect_panic", "UnwrapOptionalWithExpect");
                    Self::UnwrapOptionalWithExpect
                },
                (false, _, ExpectMode::Error) if proto.is_optional() => {
                    _trace.decision("rust_required + proto_optional + expect_error", "UnwrapOptionalWithError");
                    Self::UnwrapOptionalWithError
                },
                (false, _, ExpectMode::None) if proto.is_optional() && rust.has_default => {
                    _trace.decision("rust_required + proto_optional + has_default", "UnwrapOptionalWithDefault");
                    Self::UnwrapOptionalWithDefault
                },
                (false, _, ExpectMode::None) if proto.is_optional() => {
                    _trace.decision("rust_required + proto_optional + no_expect + no_default", "UnwrapOptionalWithExpect");
                    Self::UnwrapOptionalWithExpect
                },

                // -- Option wrapping (rust optional -> proto required) --
                (true, ProtoMapping::Scalar | ProtoMapping::Message, ExpectMode::Error) => {
                    if proto.is_optional() {
                        _trace.decision("rust_optional + proto_optional + expect_error", "UnwrapOptionalWithError");
                        Self::UnwrapOptionalWithError  // Honor expect mode before MapOption
                    } else {
                        _trace.decision("rust_optional + proto_required + has_expect_or_default", "WrapInSome");
                        Self::WrapInSome
                    }
                },

                (true, ProtoMapping::Scalar | ProtoMapping::Message, ExpectMode::None) if !rust.has_default => {
                    _trace.decision("rust_optional + proto_required + no_attributes", "UnwrapOptional");
                    if proto.is_optional() {
                        _trace.decision("rust_optional + proto_optional + no_attributes", "MapOption");
                        Self::MapOption
                    } else {
                        _trace.decision("rust_optional + proto_required + no_attributes", "UnwrapOptional");
                        Self::UnwrapOptional
                    }
                },
                (true, ProtoMapping::Scalar | ProtoMapping::Message, _) => {
                    if proto.is_optional() {
                        _trace.decision("rust_optional + proto_optional_but_scalar_mapping", "MapOption");
                        Self::MapOption
                    } else {
                        _trace.decision("rust_optional + proto_required + has_expect_or_default", "WrapInSome");
                        Self::WrapInSome
                    }
                },

                // -- Option mapping (both optional) --
                (true, ProtoMapping::Optional, ExpectMode::Panic) => {
                    _trace.decision("both_optional + expect_panic", "UnwrapOptionalWithExpect");
                    Self::UnwrapOptionalWithExpect
                },
                (true, ProtoMapping::Optional, ExpectMode::Error) => {
                    _trace.decision("both_optional + expect_error", "UnwrapOptionalWithError");
                    Self::UnwrapOptionalWithError
                },
                (true, ProtoMapping::Optional, ExpectMode::None) if rust.has_default => {
                    _trace.decision("both_optional + no_expect + has_default", "MapOptionWithDefault");
                    Self::MapOptionWithDefault
                },
                (true, ProtoMapping::Optional, ExpectMode::None) => {
                    _trace.decision("both_optional + no_expect", "MapOption");
                    Self::MapOption
                },

                // -- Option unwrapping (proto optional -> rust required) --
                (false, ProtoMapping::Optional, ExpectMode::Panic) => {
                    _trace.decision("rust_required + proto_optional + expect_panic", "UnwrapOptionalWithExpect");
                    Self::UnwrapOptionalWithExpect
                },
                (false, ProtoMapping::Optional, ExpectMode::Error) => {
                    _trace.decision("rust_required + proto_optional + expect_error", "UnwrapOptionalWithError");
                    Self::UnwrapOptionalWithError
                },
                (false, ProtoMapping::Optional, ExpectMode::None) if rust.has_default => {
                    _trace.decision("rust_required + proto_optional + has_default", "UnwrapOptionalWithDefault");
                    Self::UnwrapOptionalWithDefault
                },
                (false, ProtoMapping::Optional, ExpectMode::None) => {
                    if ctx.proto_meta.is_proto_optional() {
                        // User explicitly marked proto_optional, meaning they want to handle
                        // the rust required -> proto optional conversion
                        _trace.decision("explicit_proto_optional_for_wrapping", "WrapInSome");
                        Self::WrapInSome
                    } else {
                        // No explicit proto_optional, assume unwrapping direction
                        _trace.decision("rust_required + proto_optional + no_expect + no_default", "UnwrapOptionalWithExpect");
                        Self::UnwrapOptionalWithExpect
                    }
                },

                // -- Custom derived mappings --
                (_, ProtoMapping::CustomDerived, _) => {
                    _trace.decision("proto_custom_derived", "DirectWithInto");
                    Self::DirectWithInto
                },

                _ => {
                    _trace.decision("fallback_pattern", "determine strategy from context");
                    if proto.is_optional() && !rust.is_option {
                        Self::UnwrapOptionalWithExpect
                    } else if !proto.is_optional() && rust.is_option {
                        Self::UnwrapOptional
                    } else {
                        Self::DirectWithInto
                    }
                },
            }
        };

        if let Err(validation_error) = Self::validate_proto_optional_attribute(ctx, rust, &result) {
            panic!("ProtoConvert validation error: {}", validation_error);
        }

        result
    }

    fn determine_vec_strategy_by_context(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        trace: &CallStackDebug,
    ) -> Self {
        // Check if inner type is proto type for direct assignment
        if let Some(inner_type) = type_analysis::get_inner_type_from_vec(ctx.field_type)
            && type_analysis::is_proto_type(&inner_type, ctx.proto_module) {
            trace.decision("vec + proto_inner_type", "VecDirectAssignment");
            return Self::VecDirectAssignment;
        }

        // Determine based on default/error handling
        if rust.has_default {
            match rust.expect_mode {
                ExpectMode::Error => {
                    trace.decision("vec + has_default + expect_error", "CollectVecWithError");
                    Self::CollectVecWithError
                },
                _ => {
                    trace.decision("vec + has_default", "CollectVecWithDefault");
                    Self::CollectVecWithDefault
                }
            }
        } else {
            trace.decision("vec + no_default", "CollectVec");
            Self::CollectVec
        }
    }

    // Helper to detect Option<Vec<T>> pattern
    fn is_option_vec_type(field_type: &syn::Type) -> bool {
        type_analysis::get_inner_type_from_option(field_type)
            .map(|inner_type| type_analysis::is_vec_type(&inner_type))
            .unwrap_or(false)
    }

    fn determine_transparent_strategy(
        _ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> Self {
        match proto.optionality {
            FieldOptionality::Required => {
                trace.decision("transparent + proto_required", "TransparentRequired");
                Self::TransparentRequired
            },
            FieldOptionality::Optional => {
                match rust.expect_mode {
                    ExpectMode::Panic => {
                        trace.decision("transparent + proto_optional + expect_panic", "TransparentOptionalWithExpect");
                        Self::TransparentOptionalWithExpect
                    },
                    ExpectMode::Error => {
                        trace.decision("transparent + proto_optional + expect_error", "TransparentOptionalWithError");
                        Self::TransparentOptionalWithError
                    },
                    ExpectMode::None => {
                        if rust.has_default {
                            trace.decision("transparent + proto_optional + has_default", "TransparentOptionalWithDefault");
                            Self::TransparentOptionalWithDefault
                        } else {
                            trace.decision("transparent + proto_optional + no_expect + no_default", "TransparentRequired");
                            Self::TransparentOptionalWithExpect
                        }
                    }
                }
            }
        }
    }

    // fn determine_vec_strategy(
    //     ctx: &FieldProcessingContext,
    //     rust: &RustFieldInfo,
    //     _proto: &ProtoFieldInfo,
    //     trace: &CallStackDebug,
    // ) -> Self {
    //     // -- Handle Option<Vec<T>> case first --
    //     if Self::is_option_vec_type(ctx.field_type) {
    //         trace.decision("option_vec_type", "MapVecInOption");
    //         return Self::MapVecInOption;
    //     }
    //
    //     // -- Enhanced vector strategy determination with better proto type detection --
    //     if let Some(inner_type) = type_analysis::get_inner_type_from_vec(ctx.field_type) {
    //         let is_proto_inner = type_analysis::is_proto_type(&inner_type, ctx.proto_module);
    //
    //         if is_proto_inner {
    //             trace.decision("vec + proto_inner_type", "VecDirectAssignment");
    //             return Self::VecDirectAssignment;
    //         }
    //     }
    //
    //     if rust.has_default {
    //         match rust.expect_mode {
    //             ExpectMode::Error => {
    //                 trace.decision("vec + has_default + expect_error", "CollectVecWithError");
    //                 Self::CollectVecWithError
    //             },
    //             _ => {
    //                 trace.decision("vec + has_default", "CollectVecWithDefault");
    //                 Self::CollectVecWithDefault
    //             }
    //         }
    //     } else {
    //         trace.decision("vec + no_default", "CollectVec");
    //         Self::CollectVec
    //     }
    // }

    // fn determine_option_strategy(
    //     _ctx: &FieldProcessingContext,
    //     rust: &RustFieldInfo,
    //     proto: &ProtoFieldInfo,
    //     trace: &CallStackDebug,
    // ) -> Self {
    //     match (rust.is_option, proto.is_optional(), rust.expect_mode) {
    //         (false, true, ExpectMode::Panic) => {
    //             trace.decision("rust_required + proto_optional + expect_panic", "UnwrapOptionalWithExpect");
    //             Self::UnwrapOptionalWithExpect
    //         },
    //         (false, true, ExpectMode::Error) => {
    //             trace.decision("rust_required + proto_optional + expect_error", "UnwrapOptionalWithError");
    //             Self::UnwrapOptionalWithError
    //         },
    //         (false, true, ExpectMode::None) if rust.has_default => {
    //             trace.decision("rust_required + proto_optional + has_default", "UnwrapOptionalWithDefault");
    //             Self::UnwrapOptionalWithDefault
    //         },
    //         (false, true, ExpectMode::None) => {
    //             trace.decision("rust_required + proto_optional + no_expect + no_default", "UnwrapOptionalWithExpect");
    //             Self::UnwrapOptionalWithExpect
    //         },
    //
    //         (true, false, ExpectMode::None) if !rust.has_default => {
    //             // Pure rust Optional -> proto required suggests rust->proto unwrapping
    //             trace.decision("rust_optional + proto_required + no_attributes", "UnwrapOptional");
    //             Self::UnwrapOptional
    //         },
    //         (true, false, _) => {
    //             trace.decision("rust_optional + proto_required + has_expect_or_default", "WrapInSome");
    //             Self::WrapInSome
    //         },
    //         (true, true, ExpectMode::Panic) => {
    //             trace.decision("both_optional + expect_panic", "UnwrapOptionalWithExpect");
    //             Self::UnwrapOptionalWithExpect
    //         },
    //         (true, true, ExpectMode::Error) => {
    //             trace.decision("both_optional + expect_error", "UnwrapOptionalWithError");
    //             Self::UnwrapOptionalWithError
    //         },
    //         (true, true, ExpectMode::None) if rust.has_default => {
    //             trace.decision("both_optional + no_expect + has_default", "MapOptionWithDefault");
    //             Self::MapOptionWithDefault
    //         },
    //         (true, true, ExpectMode::None) => {
    //             // -- Both optional with no expect - map through --
    //             trace.decision("both_optional + no_expect", "MapOption");
    //             Self::MapOption
    //         },
    //         (false, false, _) => {
    //             // -- Both required - direct conversion --
    //             trace.decision("both_required", "DirectWithInto");
    //             Self::DirectWithInto
    //         },
    //     }
    // }

    fn determine_direct_strategy(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
        trace: &CallStackDebug,
    ) -> Self {
        match proto.mapping {
            ProtoMapping::Scalar if rust.is_primitive => {
                trace.decision("both_primitive_scalar", "DirectAssignment");
                Self::DirectAssignment
            },
            ProtoMapping::Message | ProtoMapping::Scalar => {
                if Self::is_proto_type_conversion(ctx, rust) {
                    trace.decision("proto_type_conversion", "DirectAssignment");
                    Self::DirectAssignment
                } else {
                    trace.decision("requires_conversion", "DirectWithInto");
                    Self::DirectWithInto
                }
            },
            _ => {
                trace.decision("fallback_direct", "DirectWithInto");
                Self::DirectWithInto
            }
        }
    }


    // proto type detection - more resilient than just checking first segment
    fn is_proto_type_conversion(ctx: &FieldProcessingContext, rust: &RustFieldInfo) -> bool {
        // Check if this is a conversion between proto module types
        type_analysis::is_proto_type(&rust.field_type, ctx.proto_module)
    }

    fn validate_proto_optional_usage(
        ctx: &FieldProcessingContext,
        rust: &RustFieldInfo,
        proto: &ProtoFieldInfo,
    ) -> Result<(), String> {
        // Check: Rust field should not be optional
        if rust.is_option {
            return Err(format!(
                "Invalid #[proto(proto_optional)] on Option<T> field, {}.\n\
                Both rust and proto fields are optional.\n\
                Remove #[proto(proto_optional)] for Option<T> -> Option<U> mapping.",
                ctx.field_name
            ));
        }

        // proto_optional attribute means: proto Optional<T> -> rust T (unwrapping)
        // So proto field MUST be optional and rust field MUST be required
        // Check: Simplified validation with clear error message
        // if proto_optional is used but the strategy selection resulted in the wrong type mismatch,
        // it's likely the proto field isn't actually optional
        if !proto.is_optional() {
            return Err(format!(
                "Invalid #[proto(proto_optional)] - proto field appears to be required.\n\
                \n\
                Expected: proto field should be Optional<{}>\n\
                But inference suggests: proto field is {} (required)\n\
                \n\
                This usually means:\n\
                • Proto schema has '{}' not 'optional {}'\n\
                • Remove #[proto(proto_optional)] for required->required conversion\n\
                • Or change proto schema to: optional {} {} = N;\n\
                • For prost message fields, verify generated code has Option<T>",
                proto.type_name,
                proto.type_name,
                proto.type_name,
                proto.type_name,
                proto.type_name,
                ctx.field_name
            ));
        }

        // Check removed - proto_optional is valid without explicit expect/default
        // The strategy selection will handle choosing appropriate unwrap behavior:
        // - UnwrapOptionalWithExpect (default)
        // - UnwrapOptionalWithError (if expect(error))
        // - UnwrapOptionalWithDefault (if default provided)

        Ok(())
    }
}
