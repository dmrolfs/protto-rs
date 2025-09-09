use crate::proto;
use crate::shared_types::*;
use protto::Protto;

// Test struct with default handling for optional fields
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "TrackWithOptionals")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct TrackWithDefault {
    #[protto(transparent, proto_name = "track_id")]
    pub id: TrackId,

    // This field would use Default::default() if the proto field is None
    // proto_optional not needed due to default
    #[protto(default)]
    pub name: String, // Would get String::default() = ""

    #[protto(default)]
    pub duration: u32, // Would get u32::default() = 0
}

// Test struct with custom default function
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "TrackWithOptionals")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct TrackWithCustomDefault {
    #[protto(transparent, proto_name = "track_id")]
    pub id: TrackId,

    // Custom default function
    #[protto(default = "default_track_name")]
    pub name: String,

    #[protto(default = "default_duration")]
    pub duration: u32,
}

// Test edge cases with empty vs None values
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "EdgeCaseMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct EdgeCaseStruct {
    #[protto(default)]
    pub empty_vs_none: String,

    #[protto(default = "default_non_empty_vec")]
    pub empty_vs_missing_vec: Vec<String>,

    #[protto(default)]
    pub zero_vs_none: u64,

    #[protto(default)]
    pub false_vs_none: bool,
}
