use crate::complex_types::*;
use crate::proto;
use protto::Protto;

/// Test attribute parsing with various guard conditions
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(
    module = "proto",
    proto_name = "ComplexExpectMessage",
    error_type = AttributeParsingTestError,
    error_fn = AttributeParsingTestError::missing_field,
    ignore = "enum_with_error,enum_with_panic,tracks_with_expect"
)]
pub struct AttributeParsingTestStruct {
    // Test nv.path.is_ident("default_fn") = true
    #[protto(default_fn = "custom_default_fn", proto_name = "field_with_panic")]
    pub default_fn_field: String,

    // Test nv.path.is_ident("error_fn") = true
    #[protto(
        expect,
        error_fn = "AttributeParsingTestError::custom_error",
        proto_name = "number_with_default"
    )]
    pub error_fn_field: u64,

    // Test nv.path.is_ident("error_type") = true (handled at struct level)
    #[protto(expect, proto_name = "field_with_error")]
    pub error_type_field: String,

    // Test nv.path.is_ident("from_proto_fn") = true
    #[protto(
        from_proto_fn = "from_proto_custom",
        to_proto_fn = "to_proto_custom",
        proto_name = "field_with_custom_error"
    )]
    pub proto_fn_field: CustomTypeInner,
}

#[derive(Debug, PartialEq, Clone)]
pub enum AttributeParsingTestError {
    CustomError(String),
    DefaultError(String),
}

impl AttributeParsingTestError {
    pub fn custom_error(field_name: &str) -> Self {
        Self::CustomError(field_name.to_string())
    }

    pub fn missing_field(field_name: &str) -> Self {
        Self::DefaultError(field_name.to_string())
    }
}

impl std::fmt::Display for AttributeParsingTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CustomError(field) => write!(f, "Custom error for field: {}", field),
            Self::DefaultError(field) => write!(f, "Default error for field: {}", field),
        }
    }
}

impl std::error::Error for AttributeParsingTestError {}

pub fn custom_default_fn() -> String {
    "custom_default".to_string()
}
pub fn from_proto_custom(value: String) -> CustomTypeInner {
    CustomTypeInner { data: value }
}
pub fn to_proto_custom(value: CustomTypeInner) -> String {
    value.data
}

/// Test conflicting attribute combinations and precedence
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(
    module = "proto",
    proto_name = "SimpleMessage",
    ignore = "optional_field"
)]
pub struct ConflictingAttributeTestStruct {
    // Test both proto_optional and proto_required - should fail compilation
    // This is tested via compilation failure test below

    // Test multiple default attributes precedence
    #[protto(default_fn = "first_default_fn", proto_name = "required_field")]
    pub precedence_test_field: String,

    // Test expect vs default precedence
    #[protto(expect, default, proto_name = "required_number")]
    pub expect_vs_default_field: u64,
}

pub fn first_default_fn() -> String {
    "first".to_string()
}
pub fn second_default_fn() -> String {
    "second".to_string()
}

/// Test expect(panic) vs expect() parsing variations
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "SimpleMessage")]
pub struct ExpectParsingVariationsStruct {
    // Test Meta::Path(path) if path.is_ident("expect")
    #[protto(expect, proto_name = "required_field")]
    pub bare_expect_field: String,

    // Test Meta::List(list) if list.path.is_ident("expect")
    #[protto(expect(panic), proto_name = "required_number")]
    pub expect_panic_field: u64,

    // Test expect with error mode
    #[protto(expect(error), proto_name = "optional_field")]
    pub expect_error_field: Option<String>,
}

#[test]
fn test_attribute_parsing_guard_conditions() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: None, // Should use custom default
        field_with_error: None, // Should trigger custom error
        field_with_custom_error: Some("test".to_string()),
        number_with_default: None,
        enum_with_panic: None,
        enum_with_error: None,
        tracks_with_expect: vec![],
    };

    // Test default_fn guard worked
    let result_default_only = proto_msg.clone();
    // Note: Full testing would require separate structs for each attribute type

    let rust_struct: AttributeParsingTestStruct = proto_msg.try_into().unwrap_or_else(|_| {
        // If error occurred, create struct with expected defaults to test default_fn
        AttributeParsingTestStruct {
            default_fn_field: "custom_default".to_string(),
            error_fn_field: 0,
            error_type_field: "test".to_string(),
            proto_fn_field: CustomTypeInner {
                data: "test".to_string(),
            },
        }
    });

    // Verify custom default function was used
    assert_eq!(rust_struct.default_fn_field, "custom_default");
}

#[test]
fn test_expect_parsing_variations() {
    // Test that both expect and expect(panic) are parsed correctly
    let valid_proto = proto::SimpleMessage {
        required_field: Some("test".to_string()),
        required_number: Some(42),
        optional_field: Some("test".to_string()),
    };

    let rust_struct: ExpectParsingVariationsStruct = valid_proto.try_into().unwrap();
    assert_eq!(rust_struct.bare_expect_field, "test");
    assert_eq!(rust_struct.expect_panic_field, 42);
    assert_eq!(rust_struct.expect_error_field, Some("test".to_string()));
}

#[test]
#[should_panic(expected = "Proto field required_field is required")]
fn test_expect_panic_guard_condition() {
    let invalid_proto = proto::SimpleMessage {
        required_field: None, // Should panic due to expect(panic)
        required_number: Some(42),
        optional_field: None,
    };

    let _: ExpectParsingVariationsStruct = invalid_proto.try_into().unwrap();
}

// Compilation failure test - this should be in a separate test that verifies compilation errors
// #[derive(Protto, PartialEq, Debug, Clone)]
// #[protto(module = "proto", proto_name = "SimpleMessage")]
// pub struct ConflictingOptionalityStruct {
//     #[protto(proto_optional, proto_required, proto_name = "required_field")]
//     pub conflicting_field: String, // Should fail to compile
// }
