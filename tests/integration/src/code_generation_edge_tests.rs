use crate::complex_types::*;
use crate::proto;
use crate::shared_types::*;
use protto::Protto;

/// Test transparent optional error mode generation
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "TransparentOptionalMessage")]
pub struct TransparentOptionalErrorModeStruct {
    // Test ErrorMode::None for Option<TransparentWrapper>
    #[protto(transparent, proto_optional, proto_name = "panic_wrapper")]
    pub none_mode_transparent: Option<TransparentWrapper>,

    // Test ErrorMode::Panic for Option<TransparentWrapper>
    #[protto(
        transparent,
        expect(panic),
        proto_optional,
        proto_name = "error_wrapper"
    )]
    pub panic_mode_transparent: Option<TransparentWrapper>,

    // Test ErrorMode::Default for Option<TransparentWrapper>
    #[protto(
        transparent,
        default_fn = "default_transparent_wrapper",
        proto_optional,
        proto_name = "default_wrapper"
    )]
    pub default_mode_transparent: Option<TransparentWrapper>,
}

/// Test empty vs missing collection scenarios for VecOptionMessage
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "VecOptionMessage")]
pub struct EmptyVsMissingOptionStruct {
    // Test Option<Vec<T>> with empty vec (should be None)
    #[protto(proto_name = "optional_tracks")]
    pub option_vec_empty: Option<Vec<Track>>,

    // Test Option<Vec<T>> with missing field (should be None)
    #[protto(proto_name = "optional_strings")]
    pub option_vec_missing: Option<Vec<String>>,

    // Test Option<Vec<proto::T>> direct assignment
    #[protto(proto_name = "optional_proto_tracks")]
    pub option_proto_vec: Option<Vec<proto::Track>>,
}

/// Test empty vs missing collection scenarios for VecErrorMessage
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "VecErrorMessage")]
pub struct EmptyVsMissingErrorStruct {
    // Test empty vec with default function
    #[protto(default_fn = "default_track_vec", proto_name = "tracks_with_error")]
    pub empty_with_default: Vec<Track>,

    // Test empty vec without default
    #[protto(proto_name = "tags_with_error")]
    pub empty_without_default: Vec<String>,
}

/// Test custom error function precedence and invocation
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(
    module = "proto",
    proto_name = "SimpleMessage",
    error_type = CustomErrorPrecedenceError,
    error_fn = CustomErrorPrecedenceError::missing_field
)]
pub struct CustomErrorPrecedenceStruct {
    // Test field-level error_fn precedence over struct-level error_type
    #[protto(
        expect,
        error_fn = "CustomErrorPrecedenceError::field_level_error",
        proto_name = "required_field"
    )]
    pub field_error_fn_field: String,

    // Test struct-level error_type when no field-level error_fn
    #[protto(expect, proto_name = "required_number")]
    pub struct_error_type_field: u64,

    // Test generated error type when no custom error specified
    #[protto(expect, proto_name = "optional_field")]
    pub generated_error_field: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum CustomErrorPrecedenceError {
    FieldLevelError(String),
    StructLevelError(String),
    GeneratedError(String),
}

impl CustomErrorPrecedenceError {
    pub fn field_level_error(field_name: &str) -> Self {
        Self::FieldLevelError(field_name.to_string())
    }

    pub fn struct_level_error(field_name: &str) -> Self {
        Self::StructLevelError(field_name.to_string())
    }

    pub fn missing_field(field_name: &str) -> Self {
        Self::GeneratedError(field_name.to_string())
    }
}

impl std::fmt::Display for CustomErrorPrecedenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FieldLevelError(field) => write!(f, "Field-level error: {}", field),
            Self::StructLevelError(field) => write!(f, "Struct-level error: {}", field),
            Self::GeneratedError(field) => write!(f, "Generated error: {}", field),
        }
    }
}

impl std::error::Error for CustomErrorPrecedenceError {}

#[test]
fn test_transparent_optional_error_modes() {
    // Test with all fields present
    let present_proto = proto::TransparentOptionalMessage {
        panic_wrapper: Some("present".to_string()),
        error_wrapper: Some("present".to_string()),
        default_wrapper: Some("present".to_string()),
    };

    let rust_struct: TransparentOptionalErrorModeStruct = present_proto.try_into().unwrap();

    // Verify all transparent optional fields handled correctly when present
    assert_eq!(
        rust_struct.none_mode_transparent,
        Some(TransparentWrapper::new("present"))
    );
    assert_eq!(
        rust_struct.panic_mode_transparent,
        Some(TransparentWrapper::new("present"))
    );
    assert_eq!(
        rust_struct.default_mode_transparent,
        Some(TransparentWrapper::new("present"))
    );

    // Test with missing fields to trigger different error modes
    let missing_proto = proto::TransparentOptionalMessage {
        panic_wrapper: None,
        error_wrapper: None,
        default_wrapper: None, // Should use default function
    };

    let rust_struct_missing: TransparentOptionalErrorModeStruct = missing_proto.try_into().unwrap();

    // Verify default mode used default function
    assert_eq!(
        rust_struct_missing.default_mode_transparent,
        Some(TransparentWrapper::new("42"))
    );

    // none_mode should be None when proto field is None
    assert_eq!(rust_struct_missing.none_mode_transparent, None);
}

#[test]
#[should_panic(expected = "Proto field error_wrapper is required")]
fn test_transparent_optional_panic_mode() {
    let panic_proto = proto::TransparentOptionalMessage {
        panic_wrapper: None,
        error_wrapper: None, // Should panic
        default_wrapper: Some("present".to_string()),
    };

    let _: TransparentOptionalErrorModeStruct = panic_proto.try_into().unwrap();
}

#[test]
fn test_empty_vs_missing_option_collection_handling() {
    // Test with some data present
    let present_proto = proto::VecOptionMessage {
        optional_tracks: vec![proto::Track { track_id: 1 }],
        optional_strings: vec!["test".to_string()],
        optional_proto_tracks: vec![proto::Track { track_id: 2 }],
    };

    let rust_struct: EmptyVsMissingOptionStruct = present_proto.try_into().unwrap();

    // Verify Option<Vec<T>> populated when proto has data
    assert_eq!(rust_struct.option_vec_empty.as_ref().unwrap().len(), 1);
    assert_eq!(
        rust_struct.option_vec_empty.as_ref().unwrap()[0]
            .id
            .as_ref(),
        &1
    );

    assert_eq!(rust_struct.option_vec_missing.as_ref().unwrap().len(), 1);
    assert_eq!(rust_struct.option_vec_missing.as_ref().unwrap()[0], "test");

    // Test with empty collections
    let empty_proto = proto::VecOptionMessage {
        optional_tracks: vec![],  // Empty - should become None for Option<Vec<T>>
        optional_strings: vec![], // Empty - should become None for Option<Vec<T>>
        optional_proto_tracks: vec![], // Empty - should become None
    };

    let empty_rust_struct: EmptyVsMissingOptionStruct = empty_proto.try_into().unwrap();

    // Verify empty collections become None for Option<Vec<T>>
    assert_eq!(empty_rust_struct.option_vec_empty, None);
    assert_eq!(empty_rust_struct.option_vec_missing, None);
    assert_eq!(empty_rust_struct.option_proto_vec, None);
}

#[test]
fn test_empty_vs_missing_error_collection_handling() {
    // Test empty collections behavior
    let empty_proto = proto::VecErrorMessage {
        tracks_with_error: vec![], // Empty - should trigger default function
        tags_with_error: vec![],   // Empty - no default, should stay empty
    };

    let rust_struct: EmptyVsMissingErrorStruct = empty_proto.try_into().unwrap();

    // Verify empty vec with default function triggered default
    assert_eq!(rust_struct.empty_with_default.len(), 1);
    assert_eq!(rust_struct.empty_with_default[0].id.as_ref(), &999); // From default_track_vec

    // Verify empty vec without default stayed empty
    assert_eq!(rust_struct.empty_without_default.len(), 0);
}

#[test]
fn test_custom_error_function_precedence() {
    let error_proto = proto::SimpleMessage {
        required_field: None,  // Should trigger field-level error function
        required_number: None, // Should trigger struct-level error type
        optional_field: None,  // Should trigger generated error
    };

    let result: Result<CustomErrorPrecedenceStruct, CustomErrorPrecedenceError> =
        error_proto.try_into();

    assert!(result.is_err());
    let error = result.unwrap_err();

    // Verify field-level error function was used
    match error {
        CustomErrorPrecedenceError::FieldLevelError(field_name) => {
            assert_eq!(field_name, "required_field");
        }
        _ => panic!("Expected field-level error, got: {:?}", error),
    }
}
