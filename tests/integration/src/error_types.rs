use crate::basic_types::*;
use crate::proto;
use crate::shared_types::*;
use protto::Protto;

#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "HasOptional")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct HasOptionalWithError {
    #[protto(expect)]
    pub track: Option<Track>,
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "HasOptional", error_type = CustomError)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct HasOptionalWithCustomError {
    #[protto(expect, error_fn = "create_missing_track_error")]
    pub track: Option<Track>,
}

// Test error function with different error fns
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "ComplexExpectMessage", error_type = ValidationError)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ComplexExpectStruct {
    #[protto(expect(panic))]
    pub field_with_panic: String,

    #[protto(expect, error_fn = "ValidationError::missing_field")]
    pub field_with_error: String,

    #[protto(expect, error_fn = "ValidationError::invalid_value")]
    pub field_with_custom_error: String,

    #[protto(default = "default_number")]
    pub number_with_default: u64,

    #[protto(expect(panic))]
    pub enum_with_panic: Status,

    #[protto(expect, error_fn = "ValidationError::conversion_failed")]
    pub enum_with_error: Status,

    #[protto(expect, error_fn = "ValidationError::missing_field")]
    pub tracks_with_expect: Vec<Track>,
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(
    module = "proto",
    proto_name = "SimpleMessage",
    error_type = DetailedValidationError
)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MultipleErrorFnsStruct {
    #[protto(
        expect,
        proto_name = "required_field",
        error_fn = "DetailedValidationError::missing_required"
    )]
    pub field_with_detailed_error: String,

    #[protto(
        expect,
        proto_name = "optional_field",
        error_fn = "DetailedValidationError::invalid_format"
    )]
    pub field_with_basic_error: String,

    #[protto(
        expect,
        proto_name = "required_number",
        error_fn = "DetailedValidationError::out_of_range"
    )]
    pub field_with_generated_error: u64,
}

// ValidationError type for basic error handling
#[derive(Debug, PartialEq, Clone)]
pub enum ValidationError {
    MissingField(String),
    InvalidValue(String),
    ConversionFailed(String),
}

#[allow(unused)]
pub fn create_validation_error(field: &str) -> ValidationError {
    ValidationError::MissingField("field_with_custom_error".to_string())
}

impl ValidationError {
    pub fn missing_field(field_name: &str) -> Self {
        Self::MissingField(field_name.to_string())
    }

    pub fn invalid_value(field_name: &str) -> Self {
        Self::InvalidValue(field_name.to_string())
    }

    pub fn conversion_failed(field_name: &str) -> Self {
        Self::ConversionFailed(field_name.to_string())
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingField(field) => write!(f, "Missing required field: {}", field),
            ValidationError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
            ValidationError::ConversionFailed(msg) => write!(f, "Conversion failed: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}
