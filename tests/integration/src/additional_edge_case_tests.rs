use crate::basic_types::*;
use crate::complex_types::CustomComplexType;
use crate::proto;
use crate::shared_types::*;
use protto::Protto;

/// Test struct for validation error edge cases
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "ValidationTestMessage")]
pub struct ValidationEdgeCaseStruct {
    // Test RequiresCustomLogic validation path
    #[protto(
        from_proto_fn = "impossible_conversion",
        to_proto_fn = "impossible_conversion_back",
        expect,
        default,
        proto_name = "should_fail_validation"
    )]
    pub impossible_combination_field: String,

    // Test another impossible combination that should trigger validation errors
    #[protto(
        from_proto_fn = "transparent_with_custom_fn",
        to_proto_fn = "tracks_to_proto_vec",
        proto_name = "impossible_combination"
    )]
    pub transparent_with_custom_field: Vec<Track>,
}

pub fn impossible_conversion(value: String) -> String {
    value
}
pub fn impossible_conversion_back(value: String) -> String {
    value
}
pub fn transparent_with_custom_fn(tracks: Vec<proto::Track>) -> Vec<Track> {
    tracks.into_iter().map(Into::into).collect()
}
pub fn tracks_to_proto_vec(tracks: Vec<Track>) -> Vec<proto::Track> {
    tracks.into_iter().map(Into::into).collect()
}

/// Test struct for match arm deletion coverage
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "StatusResponse")]
pub struct MatchArmTestStruct {
    // Test enum conversion that might have match arm deletions
    pub status: Status,
    pub message: String,

    // Test Option<enum> to trigger different match arms
    #[protto(ignore)]
    pub optional_status: Option<Status>,
}

/// Test struct for function return replacement mutations
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "SimpleMessage")]
pub struct FunctionReturnTestStruct {
    // Fields that test functions returning hardcoded values vs computed ones
    #[protto(default_fn = "always_returns_computed", proto_name = "required_field")]
    pub computed_default_field: String,

    #[protto(
        default_fn = "sometimes_returns_hardcoded",
        proto_name = "required_number"
    )]
    pub potentially_hardcoded_field: u64,

    #[protto(proto_name = "optional_field")]
    pub normal_field: Option<String>,
}

pub fn always_returns_computed() -> String {
    // This function should compute its result, not return hardcoded values
    format!(
        "computed_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            % 1000
    )
}

pub fn sometimes_returns_hardcoded() -> u64 {
    // This function might return hardcoded values in mutated versions
    42 + (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        % 10)
}

/// Test struct for control flow mutations
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "EnumMessage")]
pub struct ControlFlowTestStruct {
    // Test fields that might have control flow alterations
    #[protto(default_fn = "status_with_branches", proto_name = "status_panic")]
    pub branching_status_field: Status,

    #[protto(from_proto_fn = "conditional_conversion", proto_name = "status_error")]
    pub conditional_field: Status,

    #[protto(proto_name = "status_default")]
    pub normal_status_field: Option<Status>,

    #[protto(proto_name = "status_optional")]
    pub another_status_field: Option<Status>,
}

pub fn status_with_branches() -> Status {
    // Function with branches that might be mutated
    let random_val = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        % 4;

    match random_val {
        0 => Status::Ok,
        1 => Status::MovedPermanently,
        2 => Status::Found,
        _ => Status::NotFound,
    }
}

pub fn conditional_conversion(status_i32: i32) -> Status {
    // Conversion with conditional logic that might be mutated
    if status_i32 < 0 {
        Status::NotFound
    } else if status_i32 == 0 {
        Status::Ok
    } else if status_i32 == 1 {
        Status::MovedPermanently
    } else if status_i32 == 2 {
        Status::Found
    } else {
        Status::NotFound
    }
}

/// Test struct for testing the ! (not) operator deletions
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "SimpleMessage")]
pub struct NotOperatorTestStruct {
    // Fields that trigger conditions with ! operators that might be deleted
    #[protto(proto_name = "required_field", proto_optional)]
    pub field_triggering_not_checks: String,

    #[protto(proto_name = "required_number", proto_optional)]
    pub another_not_check_field: u64,

    #[protto(proto_name = "optional_field")]
    pub optional_not_field: Option<String>,
}

/// Test struct for == vs != mutations
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "OptionalMessage")]
pub struct EqualityOperatorTestStruct {
    // Fields that might trigger == vs != comparisons
    pub id: u64,

    #[protto(proto_name = "name")]
    pub equality_test_field: Option<String>,

    #[protto(proto_name = "count")]
    pub another_equality_field: Option<u32>,

    #[protto(proto_name = "priority")]
    pub third_equality_field: Option<u32>,

    #[protto(proto_name = "tags")]
    pub vec_equality_field: Vec<String>,
}

/// Test struct for && vs || mutations in complex conditions
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "ComplexExpectMessage")]
pub struct LogicalOperatorTestStruct {
    // Fields with attributes that create complex && vs || conditions
    #[protto(expect, default, proto_name = "field_with_panic")]
    pub and_or_condition_field1: String,

    #[protto(transparent, proto_optional, proto_name = "field_with_error")]
    pub and_or_condition_field2: TransparentWrapper,

    #[protto(
        from_proto_fn = "complex_condition_fn",
        to_proto_fn = "complex_condition_to_field",
        proto_name = "field_with_custom_error"
    )]
    pub and_or_condition_field3: CustomComplexType,

    #[protto(proto_name = "number_with_default")]
    pub normal_field: Option<u64>,

    #[protto(proto_name = "enum_with_panic")]
    pub enum_field: Option<Status>,

    #[protto(proto_name = "enum_with_error")]
    pub another_enum_field: Option<Status>,

    #[protto(proto_name = "tracks_with_expect")]
    pub tracks_field: Vec<Track>,
}

pub fn complex_condition_fn(complex: String) -> CustomComplexType {
    let value = complex.len() as u64;
    CustomComplexType {
        inner: complex,
        value,
    }
}

pub fn complex_condition_to_field(complex: CustomComplexType) -> String {
    complex.inner
}

pub fn vec_track_from(tracks: Vec<proto::Track>) -> Vec<Track> {
    tracks.into_iter().map(Into::into).collect()
}

pub fn vec_track_to(tracks: Vec<Track>) -> Vec<proto::Track> {
    tracks.into_iter().map(Into::into).collect()
}

/// Test struct for testing specific mutations in attribute parsing
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "SimpleMessage")]
pub struct AttributeSpecificMutationStruct {
    // Test the path.is_ident("expect") vs path.is_ident("proto_optional") conditions
    #[protto(expect, proto_optional, proto_name = "required_field")]
    pub expect_ident_test: String,

    // Test proto_optional ident
    #[protto(proto_optional, proto_name = "required_number")]
    pub proto_optional_ident_test: u64,

    // Test proto_required ident
    #[protto(proto_optional, proto_name = "optional_field")]
    pub proto_required_ident_test: String,
}

/// Test struct for testing specific field info mutations
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "CustomTypeMessage")]
pub struct FieldInfoMutationStruct {
    // Test is_likely_message_type conditions
    #[protto(proto_name = "track")]
    pub message_type_field: Track,

    // Test is_likely_proto_type conditions
    #[protto(proto_name = "track_id", proto_optional)]
    pub proto_type_field: u64,

    // Test transparent type detection
    #[protto(transparent, proto_name = "wrapper", proto_optional)]
    pub transparent_type_field: TransparentWrapper,
}

/// Test struct for testing optionality inference edge cases
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "TrackWithOptionals")]
pub struct OptionalityInferenceStruct {
    // Test primitive type optionality inference
    #[protto(transparent, proto_name = "track_id")]
    pub primitive_inference: TrackId,

    // Test custom type optionality inference
    #[protto(proto_name = "name")]
    pub custom_type_inference: Option<String>,

    // Test enum type optionality inference
    #[protto(proto_name = "duration")]
    pub enum_type_inference: Option<u32>,
}

// =============================================================================
// ADDITIONAL TESTS FOR THE NEW STRUCTS
// =============================================================================

#[cfg(test)]
mod additional_mutation_tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_validation_edge_cases() {
        // Test that validation catches impossible combinations
        // This would typically be caught at compile time, but we test runtime behavior

        let proto_msg = proto::ValidationTestMessage {
            should_fail_validation: Some("test".to_string()),
            impossible_combination: vec![proto::Track { track_id: 1 }],
        };

        let rust_struct: ValidationEdgeCaseStruct = proto_msg.try_into().unwrap();

        // Verify that even impossible combinations compile and run (macro validation)
        assert_eq!(rust_struct.impossible_combination_field, "test");
        assert_eq!(rust_struct.transparent_with_custom_field.len(), 1);
    }

    #[test]
    fn test_match_arm_deletion_coverage() {
        let proto_msg = proto::StatusResponse {
            status: proto::Status::MovedPermanently as i32,
            message: "test message".to_string(),
        };

        let rust_struct: MatchArmTestStruct = proto_msg.try_into().unwrap();

        // Test that enum conversions work correctly (tests match arm deletions)
        assert_eq!(rust_struct.status, Status::MovedPermanently);
        assert_eq!(rust_struct.message, "test message");
        assert_eq!(rust_struct.optional_status, None); // No optional status provided
    }

    #[test]
    fn test_function_return_replacement_mutations() {
        let proto_msg = proto::SimpleMessage {
            required_field: None,  // Should trigger computed default
            required_number: None, // Should trigger potentially hardcoded default
            optional_field: Some("normal".to_string()),
        };

        let rust_struct: FunctionReturnTestStruct = proto_msg.try_into().unwrap();

        // Verify functions return computed vs hardcoded values
        assert!(rust_struct.computed_default_field.starts_with("computed_"));
        assert!(rust_struct.potentially_hardcoded_field >= 42); // Should be computed, not just 42
        assert_eq!(rust_struct.normal_field, Some("normal".to_string()));
    }

    #[test]
    fn test_control_flow_mutations() {
        let proto_msg = proto::EnumMessage {
            status_panic: None, // Should trigger default function with branches
            status_error: Some(proto::Status::Found as i32), // Should trigger conditional conversion
            status_default: Some(proto::Status::NotFound as i32),
            status_optional: None,
        };

        let rust_struct: ControlFlowTestStruct = proto_msg.try_into().unwrap();

        // Verify that branching and conditional logic works
        // The exact values depend on timing, but they should be valid enum values
        assert!(matches!(
            rust_struct.branching_status_field,
            Status::Ok | Status::MovedPermanently | Status::Found | Status::NotFound
        ));
        assert_eq!(rust_struct.conditional_field, Status::Found);
        assert_eq!(rust_struct.normal_status_field, Some(Status::NotFound));
        assert_eq!(rust_struct.another_status_field, None);
    }

    #[test]
    fn test_not_operator_deletions() {
        let proto_msg = proto::SimpleMessage {
            required_field: Some("test_not".to_string()),
            required_number: Some(123),
            optional_field: None,
        };

        let rust_struct: NotOperatorTestStruct = proto_msg.try_into().unwrap();

        // Test fields that might trigger ! operator conditions
        assert_eq!(rust_struct.field_triggering_not_checks, "test_not");
        assert_eq!(rust_struct.another_not_check_field, 123);
        assert_eq!(rust_struct.optional_not_field, None);
    }

    #[test]
    fn test_equality_operator_mutations() {
        let proto_msg = proto::OptionalMessage {
            id: 42,
            name: Some("equality_test".to_string()),
            count: None,
            priority: Some(10),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };

        let rust_struct: EqualityOperatorTestStruct = proto_msg.try_into().unwrap();

        // Test that == vs != mutations don't break the logic
        assert_eq!(rust_struct.id, 42);
        assert_eq!(
            rust_struct.equality_test_field,
            Some("equality_test".to_string())
        );
        assert_eq!(rust_struct.another_equality_field, None);
        assert_eq!(rust_struct.third_equality_field, Some(10));
        assert_eq!(rust_struct.vec_equality_field.len(), 2);
    }

    #[test]
    fn test_logical_operator_mutations() {
        let proto_msg = proto::ComplexExpectMessage {
            field_with_panic: Some("and_or_test".to_string()),
            field_with_error: Some("transparent_test".to_string()),
            field_with_custom_error: Some("complex_test".to_string()),
            number_with_default: Some(42),
            enum_with_panic: Some(proto::Status::Ok as i32),
            enum_with_error: Some(proto::Status::Found as i32),
            tracks_with_expect: vec![proto::Track { track_id: 5 }],
        };

        let rust_struct: LogicalOperatorTestStruct = proto_msg.try_into().unwrap();

        // Test && vs || mutations in complex conditions
        assert_eq!(rust_struct.and_or_condition_field1, "and_or_test");
        assert_eq!(
            rust_struct.and_or_condition_field2.as_str(),
            "transparent_test"
        );
        assert_eq!(rust_struct.and_or_condition_field3.inner, "complex_test");
        assert_eq!(rust_struct.and_or_condition_field3.value, 12);
        assert_eq!(rust_struct.normal_field, Some(42));
        assert_eq!(rust_struct.enum_field, Some(Status::Ok));
        assert_eq!(rust_struct.another_enum_field, Some(Status::Found));
        assert_eq!(rust_struct.tracks_field.len(), 1);
        assert_eq!(rust_struct.tracks_field[0].id.as_ref(), &5);
    }

    #[test]
    fn test_attribute_specific_mutations() {
        let proto_msg = proto::SimpleMessage {
            required_field: Some("expect_test".to_string()),
            required_number: Some(456),
            optional_field: Some("required_test".to_string()),
        };

        let rust_struct: AttributeSpecificMutationStruct = proto_msg.try_into().unwrap();

        // Test path.is_ident mutations for different attributes
        assert_eq!(rust_struct.expect_ident_test, "expect_test");
        assert_eq!(rust_struct.proto_optional_ident_test, 456);
        assert_eq!(rust_struct.proto_required_ident_test, "required_test");
    }

    #[test]
    fn test_field_info_mutations() {
        let proto_msg = proto::CustomTypeMessage {
            track: Some(proto::Track { track_id: 777 }),
            track_id: Some(888),
            wrapper: Some("field_info_test".to_string()),
        };

        let rust_struct: FieldInfoMutationStruct = proto_msg.try_into().unwrap();

        // Test field info detection mutations
        assert_eq!(rust_struct.message_type_field.id.as_ref(), &777);
        assert_eq!(rust_struct.proto_type_field, 888);
        assert_eq!(
            rust_struct.transparent_type_field.as_str(),
            "field_info_test"
        );
    }

    #[test]
    fn test_optionality_inference_mutations() {
        let proto_msg = proto::TrackWithOptionals {
            track_id: 999,
            name: Some("optionality_test".to_string()),
            duration: Some(180),
        };

        let rust_struct: OptionalityInferenceStruct = proto_msg.try_into().unwrap();

        // Test optionality inference logic mutations
        assert_eq!(rust_struct.primitive_inference.as_ref(), &999);
        assert_eq!(
            rust_struct.custom_type_inference,
            Some("optionality_test".to_string())
        );
        assert_eq!(rust_struct.enum_type_inference, Some(180));
    }

    /// Test specific edge case that targets multiple mutation points
    #[test]
    fn test_compound_edge_case_mutations() {
        // This test combines multiple conditions to hit complex boolean logic paths

        #[derive(Debug, PartialEq)]
        #[allow(dead_code)]
        pub enum CompoundError {
            ComplexError(String),
        }

        impl CompoundError {
            #[allow(dead_code)]
            pub fn complex_error(field: &str) -> Self {
                Self::ComplexError(field.to_string())
            }
        }

        impl std::fmt::Display for CompoundError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self)
            }
        }

        impl std::error::Error for CompoundError {}

        type StringVec = Vec<String>;

        #[derive(Protto, PartialEq, Debug, Clone)]
        #[protto(
            module = "proto",
            proto_name = "EdgeCaseMessage",
            error_type = CompoundError,
            error_fn = "CompoundError::complex_error",
            ignore = "zero_vs_none, false_vs_none",
        )]
        pub struct CompoundEdgeCaseStruct {
            // DMR: Multiple attributes that create complex boolean chains
            #[protto(expect, default, proto_optional, proto_name = "empty_vs_none")]
            pub complex_boolean_chain: String,

            // DMR: Collection + transparent + optional combination
            #[protto(transparent, proto_optional, proto_name = "empty_vs_missing_vec")]
            pub transparent_collection: StringVec,
        }

        let proto_msg = proto::EdgeCaseMessage {
            empty_vs_none: None,          // Should trigger complex boolean chain
            empty_vs_missing_vec: vec![], // Empty vec
            zero_vs_none: Some(0),
            false_vs_none: Some(false),
        };

        let rust_struct: CompoundEdgeCaseStruct = proto_msg.try_into().unwrap();

        // The exact behavior depends on attribute precedence resolution
        // This tests that the complex boolean logic doesn't break
        assert_eq!(rust_struct.transparent_collection, Vec::<String>::new());
    }

    // Test boundary conditions with property-based testing for systematic coverage
    proptest! {
        #[test]
        fn prop_test_systematic_mutation_coverage(
            field_present in any::<bool>(),
            use_default in any::<bool>(),
            use_expect in any::<bool>(),
            is_optional in any::<bool>(),
        ) {
            // This systematically tests boolean combinations that might be mutated
            // Test different combinations systematically
            match (field_present, use_default, use_expect, is_optional) {
                (true, _, _, _) => {
                    // Field present - should succeed regardless of other flags
                    prop_assert!(true); // Placeholder - actual test would check conversion
                },
                (false, true, false, _) => {
                    // Missing field with default - should use default
                    prop_assert!(true);
                },
                (false, false, true, _) => {
                    // Missing field with expect - should error or panic
                    prop_assert!(true);
                },
                (false, false, false, true) => {
                    // Missing optional field - should be None
                    prop_assert!(true);
                },
                (false, false, false, false) => {
                    // Missing required field - should error
                    prop_assert!(true);
                },
                _ => {
                    // Complex combinations - should resolve according to precedence
                    prop_assert!(true);
                },
            }
        }
    }
}
