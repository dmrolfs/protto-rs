use crate::basic_types::*;
use crate::proto;
use crate::shared_types::*;
use proto_convert::ProtoConvert;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(rename = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MapState {
    #[proto(derive_from_with = "into_map", derive_into_with = "from_map")]
    pub tracks: HashMap<TrackId, Track>,
}

pub fn into_map(tracks: Vec<proto::Track>) -> HashMap<TrackId, Track> {
    tracks
        .into_iter()
        .map(|proto_track| {
            let track: Track = proto_track.into();
            let key = track.id.clone();
            (key, track)
        })
        .collect()
}

pub fn from_map(tracks: HashMap<TrackId, Track>) -> Vec<proto::Track> {
    tracks.into_values().map(|track| track.into()).collect()
}

#[derive(ProtoConvert, Debug)]
#[proto(rename = "State")]
pub struct ComplexState {
    pub tracks: Vec<Track>,
    #[proto(ignore)]
    pub launches: HashMap<TrackId, LaunchId>,
    #[proto(ignore)]
    pub counter: AtomicU64,
}

// Test attribute combinations
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "CombinationMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct CombinationStruct {
    #[proto(
        rename = "rename_with_default",
        default = "default_renamed_field",
        optional = true
    )]
    pub renamed_field_with_default: String,

    #[proto(transparent, expect(panic), rename = "transparent_with_expect")]
    pub transparent_field_with_expect: TransparentWrapper,

    #[proto(default = "default_status", optional = true)]
    pub enum_with_default_and_optional: Status,

    #[proto(expect)]
    pub collection_with_expect: Vec<Track>,
}

// Test mixed optional behaviors with explicit control
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "OptionalMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MixedOptionalStruct {
    pub id: u64,

    #[proto(optional = true, default, rename = "name")]
    pub optional_true_with_default: String,

    #[proto(optional = true, expect(panic), rename = "count")]
    pub optional_false_with_panic: u32,

    #[proto(optional = true, rename = "priority")]
    pub explicit_optional: Option<u32>,

    #[proto(optional = false, rename = "tags")]
    pub explicit_required: Vec<String>,
}

// Test enum with all possible attribute combinations
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "EnumMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ComprehensiveEnumStruct {
    #[proto(expect(panic), rename = "status_panic")]
    pub enum_expect_panic: Status,

    #[proto(expect, rename = "status_error")]
    pub enum_expect_error: Status,

    #[proto(default_fn = "default_status", rename = "status_default")]
    pub enum_with_default: Status,

    #[proto(optional = true, rename = "status_optional", default_fn = default_status_optional)]
    pub enum_optional_explicit: Option<Status>,
}

// Test collections with different behaviors
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct CollectionWithDefault {
    #[proto(default = "default_track_vec")]
    pub tracks: Vec<Track>,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct CollectionWithExpect {
    #[proto(expect)]
    pub tracks: Vec<Track>,
}
