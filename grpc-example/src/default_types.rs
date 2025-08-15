use crate::proto;
use crate::shared_types::*;
use proto_convert_derive::ProtoConvert;

// Test struct with default handling for optional fields
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "TrackWithOptionals")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct TrackWithDefault {
    #[proto(transparent, rename = "track_id")]
    pub id: TrackId,

    // This field would use Default::default() if the proto field is None
    #[proto(default, optional = true)]
    pub name: String, // Would get String::default() = ""

    #[proto(default, optional = true)]
    pub duration: u32, // Would get u32::default() = 0
}

// Test struct with custom default function
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "TrackWithOptionals")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct TrackWithCustomDefault {
    #[proto(transparent, rename = "track_id")]
    pub id: TrackId,

    // Custom default function
    #[proto(default = "default_track_name", optional = true)]
    pub name: String,

    #[proto(default = "default_duration", optional = true)]
    pub duration: u32,
}

// Test edge cases with empty vs None values
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "EdgeCaseMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct EdgeCaseStruct {
    #[proto(default)]
    pub empty_vs_none: String,

    #[proto(default = "default_non_empty_vec")]
    pub empty_vs_missing_vec: Vec<String>,

    #[proto(default)]
    pub zero_vs_none: u64,

    #[proto(default)]
    pub false_vs_none: bool,
}
