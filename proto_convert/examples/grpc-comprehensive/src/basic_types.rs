use crate::proto;
use crate::shared_types::*;
use proto_convert::ProtoConvert;

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Request {
    // Here we take the prost Header type instead
    pub header: proto::Header,
    pub payload: String,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct State {
    pub tracks: Vec<Track>, // we support collections as well!
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(rename = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ProtoState {
    pub tracks: Vec<proto::Track>, // we support collections as well!
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct HasStraight {
    #[proto(expect(panic))]
    pub track: Track,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct HasOptional {
    pub track: Option<Track>,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
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

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum AnotherStatus {
    Ok,
    MovedPermanently,
    Found,
    NotFound,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct StatusResponse {
    pub status: Status,
    pub message: String,
}

// DefaultStruct for testing default behaviors
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "OptionalMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct DefaultStruct {
    pub id: u64,

    #[proto(default_fn)]
    pub name: String, // Uses String::default() = ""

    #[proto(default)]
    pub count: u32, // Uses u32::default() = 0

    #[proto(default = "default_priority")]
    pub priority: u32, // Uses custom default function

    #[proto(default_fn = default_tags)]
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
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "SimpleMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ExpectPanicStruct {
    #[proto(expect(panic), rename = "required_field")]
    pub required_field: String,

    #[proto(expect(panic), rename = "required_number")]
    pub required_number: u64,

    #[proto(rename = "optional_field")]
    pub optional_field: Option<String>,
}

// ExpectErrorStruct for testing expect with generated errors
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "SimpleMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ExpectErrorStruct {
    #[proto(expect, rename = "required_field", optional = true)]
    pub required_field: String,

    #[proto(expect, rename = "required_number", optional = true)]
    pub required_number: u64,

    #[proto(rename = "optional_field")]
    pub optional_field: Option<String>,
}

// ExpectCustomErrorStruct for testing expect with custom error types
// use crate::error_types::ValidationError;
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(
    module = "proto",
    rename = "SimpleMessage",
    error_type = crate::error_types::ValidationError
)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ExpectCustomErrorStruct {
    #[proto(
        expect,
        rename = "required_field",
        optional = true,
        error_fn = "crate::error_types::ValidationError::missing_field"
    )]
    pub required_field: String,

    #[proto(
        expect,
        rename = "required_number",
        optional = true,
        error_fn = "crate::error_types::ValidationError::invalid_value"
    )]
    pub required_number: u64,

    #[proto(rename = "optional_field")]
    pub optional_field: Option<String>,
}
