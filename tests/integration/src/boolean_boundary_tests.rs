use crate::basic_types::*;
use crate::complex_types::*;
use crate::proto;
use crate::shared_types::*;
use protto::Protto;

/// Test combinations that trigger has_explicit_optional_usage_indicators boolean chains
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "CombinationMessage")]
pub struct BooleanLogicTestStruct {
    // Test has_explicit_expect = false, has_explicit_default = true
    #[protto(
        default_fn = "default_renamed_field",
        proto_name = "rename_with_default"
    )]
    pub default_only_field: String,

    // Test has_explicit_expect = true, has_explicit_default = false
    #[protto(transparent, expect(panic), proto_name = "transparent_with_expect")]
    pub expect_only_field: TransparentWrapper,

    // Test has_explicit_expect = false, has_explicit_default = true (enum case)
    #[protto(
        default_fn = "default_status",
        proto_optional,
        proto_name = "enum_with_default_and_optional"
    )]
    pub enum_with_default_field: Status,

    // Test has_explicit_expect = true, has_explicit_default = false (collection case)
    #[protto(expect, proto_name = "collection_with_expect")]
    pub expect_collection_field: Vec<Track>,
}

/// Test custom type detection boundary conditions
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "CustomTypeMessage")]
pub struct CustomTypeDetectionStruct {
    // Test !is_primitive && !is_std_type && !is_proto_type = true
    #[protto(expect(panic), proto_name = "track")]
    pub pure_custom_type: Track,

    // Test !is_primitive && !is_std_type && !is_proto_type = false (primitive)
    #[protto(proto_name = "track_id")]
    pub primitive_field: Option<u64>,

    // Test !is_primitive && !is_std_type && !is_proto_type = false (std type)
    #[protto(expect(panic), proto_name = "wrapper")]
    pub std_type_field: String,
}

/// Test complex boolean chains in ErrorMode detection
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(
    module = "proto",
    proto_name = "ComplexExpectMessage",
    ignore = "enum_with_panic, field_with_error"
)]
pub struct ErrorModeDetectionStruct {
    // Test custom_functions_need_default_panic conditions
    // from_proto_fn.is_some() && to_proto_fn.is_some() && !is_primitive && !is_option
    #[protto(from_proto_fn = string_to_custom, to_proto_fn = custom_to_string, proto_name = "field_with_custom_error")]
    pub bidirectional_custom_complex: CustomComplexType, // DMR-9: Now maps to field_with_custom_error

    // Test negation: expect mode with default function
    #[protto(default_fn = "default_number", proto_name = "number_with_default")]
    pub default_with_function_field: u64, // DMR-9: Now maps to number_with_default

    // Test expect(panic) mode
    #[protto(expect(panic), proto_name = "field_with_panic")]
    pub expect_panic_field: String, // DMR-9: Now maps to field_with_panic

    // Test expect(error) mode with enum
    #[protto(expect, proto_name = "enum_with_error")]
    pub expect_error_enum: Status, // DMR-9: Now maps to enum_with_error

    // Test collection with expect
    #[protto(expect, proto_name = "tracks_with_expect")]
    pub expect_collection_field: Vec<Track>, // DMR-9: Now maps to tracks_with_expect
}

fn string_to_custom(field: String) -> CustomComplexType {
    let value = field.len() as u64;
    CustomComplexType {
        inner: field,
        value,
    }
}

fn custom_to_string(custom: CustomComplexType) -> String {
    custom.inner
}

/// Test proto optionality indicator combinations
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "OptionalMessage")]
#[allow(dead_code)]
pub struct ProtoOptionalityIndicatorsStruct {
    // Test has_default_indicators = true, has_expect_indicators = false, has_explicit_optional = false
    #[protto(default, proto_name = "name")]
    pub default_indicator_only: String,

    // Test has_default_indicators = false, has_expect_indicators = true, has_explicit_optional = false
    #[protto(expect, proto_name = "count")]
    pub expect_indicator_only: u32,

    // Test has_default_indicators = false, has_expect_indicators = false, has_explicit_optional = true
    #[protto(proto_optional, proto_name = "priority")]
    pub explicit_optional_only: u32,

    // Test all true: has_default_indicators = true, has_expect_indicators = true, has_explicit_optional = true
    #[protto(expect, proto_optional, proto_name = "tags")]
    pub all_indicators_present: Vec<String>,

    // Test all false: has_default_indicators = false, has_expect_indicators = false, has_explicit_optional = false
    #[protto(proto_name = "id")]
    pub no_indicators_present: u64,
}

#[test]
fn test_boolean_logic_boundary_conditions() {
    // Test the specific boolean combinations that trigger different code paths
    let proto_msg = proto::CombinationMessage {
        rename_with_default: None, // Should trigger default function
        transparent_with_expect: Some("test_expect".to_string()), // Should succeed
        enum_with_default_and_optional: None, // Should use default enum value
        collection_with_expect: vec![proto::Track { track_id: 42 }], // Should succeed
    };

    let rust_struct: BooleanLogicTestStruct = proto_msg.try_into().unwrap();

    // Verify default_only_field used default function
    assert_eq!(rust_struct.default_only_field, "renamed_default");

    // Verify expect_only_field worked (transparent + panic case succeeded)
    assert_eq!(rust_struct.expect_only_field.as_str(), "test_expect");

    // Verify enum_with_default_field used default
    assert_eq!(rust_struct.enum_with_default_field, Status::Ok);

    // Verify expect_collection_field handled collection
    assert_eq!(rust_struct.expect_collection_field.len(), 1);
    assert_eq!(rust_struct.expect_collection_field[0].id.as_ref(), &42);
}

#[test]
#[should_panic(expected = "Proto field transparent_with_expect is required")]
fn test_expect_only_field_panic() {
    let proto_msg = proto::CombinationMessage {
        rename_with_default: Some("test".to_string()),
        transparent_with_expect: None, // Should panic due to expect(panic)
        enum_with_default_and_optional: Some(proto::Status::Ok.into()),
        collection_with_expect: vec![],
    };

    let _: BooleanLogicTestStruct = proto_msg.try_into().unwrap();
}

#[test]
fn test_custom_type_detection_boundary_cases() {
    let proto_msg = proto::CustomTypeMessage {
        track: Some(proto::Track { track_id: 42 }),
        track_id: Some(99),
        wrapper: Some("test".to_string()),
    };

    let rust_struct: CustomTypeDetectionStruct = proto_msg.try_into().unwrap();

    // Each field tests different branches of the custom type detection logic
    assert_eq!(rust_struct.pure_custom_type.id, 42);
    assert_eq!(rust_struct.std_type_field, "test");
    assert_eq!(rust_struct.primitive_field, Some(99)); // Default
}

#[test]
fn test_error_mode_detection_boolean_chains() {
    // Updated test data to match ComplexExpectMessage structure
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_test".to_string()),
        field_with_error: Some("error_test".to_string()),
        field_with_custom_error: Some("custom_test".to_string()), // This will use custom conversion
        number_with_default: None,                                // Should trigger default function
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()), // This will test expect error mode
        tracks_with_expect: vec![proto::Track { track_id: 123 }], // Collection with expect
    };

    let rust_struct: ErrorModeDetectionStruct = proto_msg.try_into().unwrap();

    // Verify that different boolean combinations in error mode detection work
    assert_eq!(
        rust_struct.bidirectional_custom_complex.inner,
        "custom_test"
    ); // Custom conversion worked
    assert_eq!(
        rust_struct.bidirectional_custom_complex.value,
        "custom_test".len() as u64
    ); // Fixed value from conversion function
    assert_eq!(rust_struct.default_with_function_field, 9999); // Default function was used
    assert_eq!(rust_struct.expect_panic_field, "panic_test"); // Expect panic succeeded
    assert_eq!(rust_struct.expect_error_enum, Status::Found); // Expect error succeeded
    assert_eq!(rust_struct.expect_collection_field.len(), 1); // Collection expect succeeded
    assert_eq!(rust_struct.expect_collection_field[0].id.as_ref(), &123); // Track conversion worked
}
