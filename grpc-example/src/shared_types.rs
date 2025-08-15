use crate::proto;
use proptest::prelude::*;
use proto_convert_derive::ProtoConvert;
use std::sync::atomic::AtomicU64;

// Helper to create proto tracks with optional fields
#[allow(unused)]
pub fn arb_proto_track_with_optionals() -> impl Strategy<Value = proto::TrackWithOptionals> {
    (any::<u64>(), any::<Option<String>>(), any::<Option<u32>>()).prop_map(
        |(track_id, name, duration)| proto::TrackWithOptionals {
            track_id,
            name,
            duration,
        },
    )
}

#[derive(ProtoConvert, PartialEq, Debug, Clone, Hash, Eq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct TrackId(u64);

impl TrackId {
    pub fn new(track_id: u64) -> Self {
        TrackId(track_id)
    }

    pub fn into_inner(self) -> u64 {
        self.0
    }
}

impl PartialEq<u64> for TrackId {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl AsRef<u64> for TrackId {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Track {
    #[proto(transparent, rename = "track_id")]
    pub id: TrackId,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct TransparentWrapper(String);

impl TransparentWrapper {
    pub fn new(str: impl Into<String>) -> Self {
        Self(str.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for TransparentWrapper {
    fn from(str: &str) -> Self {
        Self::new(str.to_string())
    }
}

impl PartialEq<String> for TransparentWrapper {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl PartialEq<&str> for TransparentWrapper {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct LaunchId(u64);

#[allow(unused)]
pub fn create_missing_track_error(field: &str) -> CustomError {
    CustomError::TrackMissing
}

#[allow(unused)]
pub fn missing_field_error(field: &str) -> DetailedValidationError {
    DetailedValidationError::MissingRequired("field_with_detailed_error".to_string())
}

// Custom default functions
#[allow(unused)]
pub fn default_track_name() -> String {
    "Unknown Track".to_string()
}

#[allow(unused)]
pub fn default_counter() -> AtomicU64 {
    AtomicU64::new(42)
}

#[allow(unused)]
pub fn default_number() -> u64 {
    9999
}

#[allow(unused)]
fn default_priority() -> u32 {
    10
}

#[allow(unused)]
fn default_tags() -> Vec<String> {
    vec!["default".to_string()]
}
#[allow(unused)]
pub fn default_non_empty_vec() -> Vec<String> {
    vec!["default_item".to_string()]
}

#[allow(unused)]
pub fn default_renamed_field() -> String {
    "renamed_default".to_string()
}

#[allow(unused)]
pub fn default_track_vec() -> Vec<Track> {
    vec![Track { id: TrackId(999) }]
}

#[allow(unused)]
pub fn default_duration() -> u32 {
    180 // 3 minutes in seconds
}

// Custom error type for testing
#[derive(Debug, PartialEq)]
pub enum CustomError {
    TrackMissing,
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CustomError::TrackMissing => write!(f, "Track is required but missing"),
        }
    }
}

impl std::error::Error for CustomError {}

impl From<CustomError> for String {
    fn from(err: CustomError) -> String {
        err.to_string()
    }
}

// Test different error types and functions
#[derive(Debug, PartialEq, Clone)]
pub enum DetailedValidationError {
    MissingRequired(String),
    InvalidFormat(String),
    OutOfRange(String),
}

impl DetailedValidationError {
    pub fn missing_required(field: &str) -> DetailedValidationError {
        DetailedValidationError::MissingRequired(field.to_string())
    }

    pub fn invalid_format(field: &str) -> DetailedValidationError {
        DetailedValidationError::InvalidFormat(field.to_string())
    }

    pub fn out_of_range(field: &str) -> DetailedValidationError {
        DetailedValidationError::OutOfRange(field.to_string())
    }
}

impl std::fmt::Display for DetailedValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetailedValidationError::MissingRequired(field) => {
                write!(f, "Required field missing: {}", field)
            }
            DetailedValidationError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            DetailedValidationError::OutOfRange(msg) => write!(f, "Out of range: {}", msg),
        }
    }
}

impl std::error::Error for DetailedValidationError {}
