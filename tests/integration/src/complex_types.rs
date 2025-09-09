use crate::basic_types::*;
use crate::proto;
use crate::shared_types::*;
use protto::Protto;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;

#[derive(Protto, PartialEq, Debug, Clone)]
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

#[derive(Protto, Debug)]
#[proto(rename = "State")]
pub struct ComplexState {
    pub tracks: Vec<Track>,
    #[proto(ignore)]
    pub launches: HashMap<TrackId, LaunchId>,
    #[proto(ignore)]
    pub counter: AtomicU64,
}

// Test attribute combinations
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "CombinationMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct CombinationStruct {
    #[proto(
        rename = "rename_with_default",
        default_fn = default_renamed_field,
        proto_optional
    )]
    pub renamed_field_with_default: String,

    #[proto(transparent, expect(panic), rename = "transparent_with_expect")]
    pub transparent_field_with_expect: TransparentWrapper,

    #[proto(default_fn = default_status, proto_optional)]
    pub enum_with_default_and_optional: Status,

    #[proto(expect)]
    pub collection_with_expect: Vec<Track>,
}

// Test mixed optional behaviors with explicit control
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "OptionalMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MixedOptionalStruct {
    pub id: u64,

    #[proto(proto_optional, default, rename = "name")]
    pub optional_true_with_default: String,

    #[proto(proto_optional, expect(panic), rename = "count")]
    pub optional_false_with_panic: u32,

    #[proto(proto_optional, rename = "priority")]
    pub explicit_optional: Option<u32>,

    #[proto(proto_required, rename = "tags")]
    pub explicit_required: Vec<String>,
}

// Test enum with all possible attribute combinations
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "EnumMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ComprehensiveEnumStruct {
    #[proto(expect(panic), rename = "status_panic")]
    pub enum_expect_panic: Status,

    #[proto(expect, rename = "status_error")]
    pub enum_expect_error: Status,

    #[proto(default_fn = "default_status", rename = "status_default")]
    pub enum_with_default: Status,

    #[proto(proto_optional, rename = "status_optional", default_fn = default_status_optional)]
    pub enum_optional_explicit: Option<Status>,
}

// Test collections with different behaviors
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct CollectionWithDefault {
    #[proto(default_fn = "default_track_vec")]
    pub tracks: Vec<Track>,
}

#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct CollectionWithExpect {
    #[proto(expect)]
    pub tracks: Vec<Track>,
}

// Test DeriveBidirectional - both from_with and into_with
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "BidirectionalMessage")]
pub struct BidirectionalConversionStruct {
    #[proto(
        derive_from_with = "custom_from_conversion",
        derive_into_with = "custom_into_conversion"
    )]
    pub custom_field: CustomComplexType,
}

#[derive(PartialEq, Debug, Clone)]
pub struct CustomComplexType {
    pub inner: String,
    pub value: u64,
}

pub fn custom_from_conversion(proto_val: proto::ComplexType) -> CustomComplexType {
    CustomComplexType {
        inner: proto_val.name,
        value: proto_val.id,
    }
}

pub fn custom_into_conversion(rust_val: CustomComplexType) -> proto::ComplexType {
    proto::ComplexType {
        name: rust_val.inner,
        id: rust_val.value,
    }
}

// Test TransparentRequired - transparent field to required proto
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "TransparentMessage")]
pub struct TransparentRequiredStruct {
    #[proto(transparent, rename = "wrapper_id")]
    pub id: TransparentWrapper,  // proto field is required u64
}

// Test TransparentOptionalWith* variants
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "TransparentOptionalMessage")]
pub struct TransparentOptionalStruct {
    #[proto(transparent, expect(panic), rename = "panic_wrapper", proto_optional)]
    pub panic_wrapper: TransparentWrapper,

    #[proto(transparent, expect, rename = "error_wrapper", proto_optional)]
    pub error_wrapper: TransparentWrapper,

    #[proto(transparent, default_fn = default_transparent_wrapper, rename = "default_wrapper", proto_optional)]
    pub default_wrapper: TransparentWrapper,
}

pub fn default_transparent_wrapper() -> TransparentWrapper {
    TransparentWrapper::new("42")
}

// Test WrapInSome - rust required → proto optional
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "WrapInSomeMessage")]
pub struct WrapInSomeStruct {
    #[proto(proto_optional, rename = "wrapped_field")]
    pub required_rust_field: String,  // rust required → proto optional

    #[proto(proto_optional, rename = "wrapped_status")]
    pub status: Status,  // enum rust required → proto optional
}

// Test MapOption - both sides optional, no expect/default
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "MapOptionMessage")]
pub struct MapOptionStruct {
    #[proto(proto_optional, rename = "simple_option")]
    pub optional_string: Option<String>,  // Option<String> → Option<String>

    #[proto(proto_optional, rename = "optional_status")]
    pub optional_status: Option<Status>,  // Option<Status> → Option<i32>
}

// Test MapVecInOption - completely missing from current tests
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "VecOptionMessage")]
pub struct VecOptionStruct {
    #[proto(proto_optional, rename = "optional_tracks")]
    pub optional_tracks: Option<Vec<Track>>,  // Option<Vec<Track>> → Option<Vec<proto::Track>>

    #[proto(proto_optional, rename = "optional_strings")]
    pub optional_strings: Option<Vec<String>>,  // Option<Vec<String>> → Option<Vec<String>>

    #[proto(proto_optional, rename = "optional_proto_tracks")]
    pub optional_proto_tracks: Option<Vec<proto::Track>>,  // Option<Vec<proto::Track>> → Option<Vec<proto::Track>>
}

// Test VecDirectAssignment - Vec<proto::Type> scenarios
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "DirectVecMessage")]
pub struct VecDirectAssignmentStruct {
    pub proto_tracks: Vec<proto::Track>,  // Vec<proto::Track> → Vec<proto::Track> (no conversion)
    pub proto_headers: Vec<proto::Header>, // Vec<proto::Header> → Vec<proto::Header> (no conversion)
}

// Test CollectVecWithError - Vec with expect
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "VecErrorMessage", error_type = VecError)]
pub struct VecWithErrorStruct {
    #[proto(expect, default_fn = default_track_vec, error_fn = VecError::empty_tracks)]
    pub tracks_with_error: Vec<Track>,

    #[proto(expect, default_fn = default_string_vec, error_fn = VecError::missing_tags)]
    pub tags_with_error: Vec<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum VecError {
    EmptyTracks(String),
    MissingTags(String),
    ConversionFailed(String),
}

impl VecError {
    pub fn empty_tracks(field_name: &str) -> Self {
        Self::EmptyTracks(field_name.to_string())
    }

    pub fn missing_tags(field_name: &str) -> Self {
        Self::MissingTags(field_name.to_string())
    }
}

impl std::fmt::Display for VecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VecError::EmptyTracks(field) => write!(f, "Empty tracks in field: {}", field),
            VecError::MissingTags(field) => write!(f, "Missing tags in field: {}", field),
            VecError::ConversionFailed(msg) => write!(f, "Vec conversion failed: {}", msg),
        }
    }
}

impl std::error::Error for VecError {}


pub fn default_string_vec() -> Vec<String> {
    vec!["default".to_string()]
}

// Test DirectWithInto - clear custom type → proto type conversion
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "DirectConversionMessage")]
pub struct DirectWithIntoStruct {
    pub status_field: Status,  // Status → i32 via Into
    pub track_field: Track,    // Track → proto::Track via Into
    #[proto(transparent)]
    pub track_id: TrackId,     // TrackId → u64 via Into (if transparent)
}

// Test edge case combinations
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "EdgeCaseCombinationMessage")]
pub struct EdgeCaseCombinationsStruct {
    // Option with custom derive - rare but possible
    #[proto(
        proto_optional,
        derive_from_with = "option_custom_from",
        derive_into_with = "option_custom_into"
    )]
    pub optional_custom: Option<CustomTypeInner>,

    // Vec with custom derive - also rare
    #[proto(derive_from_with = "vec_custom_from", derive_into_with = "vec_custom_into")]
    pub vec_custom: Vec<CustomTypeInner>,

    // Transparent Option (should this even be allowed?)
    #[proto(transparent, proto_optional)]
    pub transparent_option: Option<TransparentWrapper>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct CustomTypeInner {
    pub data: String,
}

pub fn option_custom_into(rust_val: Option<CustomTypeInner>) -> Option<proto::CustomType> {
    rust_val.map(|r| proto::CustomType { data: r.data })
}

pub fn option_custom_from(proto_val: Option<proto::CustomType>) -> Option<CustomTypeInner> {
    proto_val.map(|p| CustomTypeInner { data: p.data })
}

pub fn vec_custom_from(proto_vec: Vec<proto::CustomType>) -> Vec<CustomTypeInner> {
    proto_vec.into_iter().map(|p| CustomTypeInner { data: p.data }).collect()
}

pub fn vec_custom_into(rust_vec: Vec<CustomTypeInner>) -> Vec<proto::CustomType> {
    rust_vec.into_iter().map(|r| proto::CustomType { data: r.data }).collect()
}

// Test rust->proto specific strategies (UnwrapOptional, TransparentToOptional, etc.)
#[derive(Protto, PartialEq, Debug, Clone)]
#[proto(module = "proto", rename = "RustToProtoMessage")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct RustToProtoStruct {
    // This should trigger WrapInSome (rust required -> proto optional)
    #[proto(rename = "required_to_optional")]
    pub rust_required_field: String,

    // This should trigger UnwrapOptional (rust optional -> proto required)
    #[proto(proto_required, rename = "optional_to_required")]
    pub rust_optional_field: Option<String>,

    // This should trigger TransparentToRequired
    #[proto(transparent, rename = "transparent_to_required")]
    pub transparent_required: TrackId,

    // This should trigger TransparentToOptional
    #[proto(transparent, proto_optional, rename = "transparent_to_optional")]
    pub transparent_optional: TrackId,
}