use crate::proto;
use crate::shared_types::*;
use protto::Protto;

#[derive(Protto, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Request {
    // Here we take the prost Header type instead
    #[protto(proto_optional)]
    pub header: proto::Header,
    pub payload: String,
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct State {
    pub tracks: Vec<Track>, // we support collections as well!
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(proto_name = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ProtoState {
    pub tracks: Vec<proto::Track>, // we support collections as well!
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct HasStraight {
    #[protto(expect(panic))]
    pub track: Track,
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct HasOptional {
    pub track: Option<Track>,
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Status {
    MovedPermanently,
    Ok,
    Found,
    NotFound,
}

pub fn default_status() -> Status {
    Status::Ok
}

pub fn default_status_optional() -> Option<Status> {
    Some(default_status())
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum AnotherStatus {
    Ok,
    MovedPermanently,
    Found,
    NotFound,
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct StatusResponse {
    pub status: Status,
    pub message: String,
}

// DefaultStruct for testing default behaviors
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "OptionalMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct DefaultStruct {
    pub id: u64,

    #[protto(default)]
    pub name: String, // Uses String::default() = ""

    #[protto(default)]
    pub count: u32, // Uses u32::default() = 0

    #[protto(default_fn = "default_priority")]
    pub priority: u32, // Uses custom default function

    #[protto(default_fn = default_tags)]
    pub tags: Vec<String>, // Uses custom default function
}

#[allow(unused)]
pub fn default_priority() -> u32 {
    10
}

#[allow(unused)]
pub fn default_tags() -> Vec<String> {
    vec!["default".to_string()]
}

// ExpectPanicStruct for testing expect(panic) behavior
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "SimpleMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ExpectPanicStruct {
    #[protto(expect(panic), proto_name = "required_field")]
    pub required_field: String,

    #[protto(expect(panic), proto_name = "required_number")]
    pub required_number: u64,

    #[protto(proto_name = "optional_field")]
    pub optional_field: Option<String>,
}

// ExpectErrorStruct for testing expect with generated errors
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "SimpleMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ExpectErrorStruct {
    #[protto(expect, proto_name = "required_field")]
    pub required_field: String,

    #[protto(expect, proto_name = "required_number")]
    pub required_number: u64,

    #[protto(proto_name = "optional_field")]
    pub optional_field: Option<String>,
}

// ExpectCustomErrorStruct for testing expect with custom error types
// use crate::error_types::ValidationError;
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(
    module = "proto",
    proto_name = "SimpleMessage",
    error_type = crate::error_types::ValidationError
)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ExpectCustomErrorStruct {
    #[protto(
        expect,
        proto_name = "required_field",
        error_fn = "crate::error_types::ValidationError::missing_field"
    )]
    pub required_field: String,

    #[protto(
        expect,
        proto_name = "required_number",
        error_fn = "crate::error_types::ValidationError::invalid_value"
    )]
    pub required_number: u64,

    #[protto(proto_name = "optional_field")]
    pub optional_field: Option<String>,
}

// This struct will trigger generate_custom_type_field for the track field
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "CustomTypeMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct CustomTypeStruct {
    // This field should trigger generate_custom_type_field()
    // - Not Option<Track> (would go to generate_option_field)
    // - Not Vec<Track> (would go to generate_vec_field)
    // - Not a primitive type
    // - Not a proto:: type
    // - Not marked with any special attributes
    pub track: Track,

    // This should also trigger custom type path
    pub track_id: TrackId,

    // Test with transparent wrapper too
    // #[protto(transparent, expect(panic))]
    pub wrapper: TransparentWrapper,
}
