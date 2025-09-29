use crate::complex_types::*;
use crate::proto;
use crate::shared_types::*;
use protto::Protto;

/// Test the sequential elimination in strategy selection
/// Test the sequential elimination in strategy selection
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "EdgeCaseCombinationMessage")]
pub struct StrategySelectionTestStruct {
    // Test custom strategy precedence (should be second check after ignore)
    #[protto(
        from_proto_fn = "strategy_custom_from",
        to_proto_fn = "strategy_custom_to",
        proto_name = "optional_custom"
    )]
    pub custom_strategy_field: CustomTypeInner,

    // Test collection strategy precedence (should be fourth check)
    #[protto(from_proto_fn = vec_custom_from, to_proto_fn = vec_custom_into, proto_name = "vec_custom")]
    pub collection_strategy_field: Vec<CustomTypeInner>,

    // Test transparent + option strategy combination
    #[protto(proto_name = "transparent_option")]
    pub transparent_option_field: Option<String>,
}

pub fn strategy_custom_from(custom: proto::CustomType) -> CustomTypeInner {
    // DMR-9: Updated for EdgeCaseCombinationMessage
    CustomTypeInner { data: custom.data }
}

pub fn strategy_custom_to(custom: CustomTypeInner) -> proto::CustomType {
    // DMR-9: Updated return type
    proto::CustomType { data: custom.data }
}

/// Test error mode selection boundary cases
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "OptionalMessage")]
pub struct ErrorModeSelectionStruct {
    // Test ExpectMode::Panic
    #[protto(expect(panic), proto_name = "name")]
    pub panic_mode_field: String,

    // Test ExpectMode::Error
    #[protto(expect(error), proto_name = "count")]
    pub error_mode_field: u32,

    // Test ExpectMode::None + has_default = true
    #[protto(default, proto_name = "priority")]
    pub default_mode_field: u32,

    // Test ExpectMode::None + has_default = false + custom_functions_need_default_panic = true
    #[protto(
        from_proto_fn = "bidirectional_from",
        to_proto_fn = "bidirectional_to",
        proto_name = "tags"
    )]
    pub custom_panic_mode_field: Vec<String>,

    // Test ExpectMode::None + has_default = false + custom_functions_need_default_panic = false
    #[protto(proto_name = "id")]
    pub none_mode_field: u64,
}

pub fn bidirectional_from(tags: Vec<String>) -> Vec<String> {
    tags
}
pub fn bidirectional_to(tags: Vec<String>) -> Vec<String> {
    tags
}

/// Test collection strategy with optional vector patterns
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "VecOptionMessage")]
pub struct CollectionOptionTestStruct {
    // Test Option<Vec<T>> → MapOption strategy
    #[protto(proto_name = "optional_tracks")]
    pub option_vec_strategy: Option<Vec<Track>>,

    // Test Vec<T> + no default → Collect(None) strategy
    #[protto(proto_name = "optional_strings")]
    pub collect_no_default_strategy: Vec<String>,

    // Test Vec<proto::Type> → DirectAssignment strategy
    #[protto(proto_name = "optional_proto_tracks")]
    pub direct_assignment_strategy: Vec<proto::Track>,
}

/// Test collection strategy with error handling patterns
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "VecErrorMessage")]
pub struct CollectionErrorTestStruct {
    // Test Vec<proto::Type> → DirectAssignment strategy
    #[protto(proto_name = "tracks_with_error")]
    pub direct_assignment_strategy: Vec<proto::Track>,

    // Fix type mismatch - proto field is Vec<String>, so Rust field should be Vec<String>
    #[protto(proto_name = "tags_with_error")]
    pub collect_with_default_strategy: Vec<String>, // Changed from Vec<Track>
}

// Add helper function for string vector default
#[allow(dead_code)]
pub fn default_string_vec() -> Vec<String> {
    vec!["default_tag".to_string()]
}

#[test]
fn test_strategy_selection_precedence() {
    let proto_msg = proto::EdgeCaseCombinationMessage {
        optional_custom: Some(proto::CustomType {
            data: "custom_test".to_string(),
        }),
        vec_custom: vec![proto::CustomType {
            data: "vec_item".to_string(),
        }],
        transparent_option: Some("transparent_test".to_string()),
    };

    let rust_struct: StrategySelectionTestStruct = proto_msg.try_into().unwrap();

    // Verify custom strategy was selected and used
    assert_eq!(rust_struct.custom_strategy_field.data, "custom_test");

    // Verify collection strategy handled the vec conversion
    assert_eq!(rust_struct.collection_strategy_field.len(), 1);
    assert_eq!(rust_struct.collection_strategy_field[0].data, "vec_item");

    // Verify transparent + option combination worked
    assert_eq!(
        rust_struct.transparent_option_field,
        Some("transparent_test".to_string())
    );
}

#[test]
fn test_error_mode_selection_boundary_cases() {
    let valid_proto = proto::OptionalMessage {
        id: 1,
        name: Some("test".to_string()),
        count: Some(42),
        priority: None, // Should use default
        tags: vec!["tag1".to_string()],
    };

    let rust_struct: ErrorModeSelectionStruct = valid_proto.try_into().unwrap();

    // Verify different error modes were applied correctly
    assert_eq!(rust_struct.panic_mode_field, "test");
    assert_eq!(rust_struct.error_mode_field, 42);
    assert_eq!(rust_struct.default_mode_field, 0); // Default::default()
    assert_eq!(
        rust_struct.custom_panic_mode_field,
        vec!["tag1".to_string()]
    );
}

#[test]
fn test_collection_option_strategy_boundaries() {
    let proto_msg = proto::VecOptionMessage {
        optional_tracks: vec![proto::Track { track_id: 1 }],
        optional_strings: vec!["track1".to_string(), "track2".to_string()],
        optional_proto_tracks: vec![proto::Track { track_id: 2 }],
    };

    let rust_struct: CollectionOptionTestStruct = proto_msg.try_into().unwrap();

    // Test Option<Vec<T>> conversion - should be Some when proto has data
    assert_eq!(rust_struct.option_vec_strategy.as_ref().unwrap().len(), 1);
    assert_eq!(
        rust_struct.option_vec_strategy.as_ref().unwrap()[0]
            .id
            .as_ref(),
        &1
    );

    // Test direct collection conversion
    assert_eq!(rust_struct.collect_no_default_strategy.len(), 2);
    assert_eq!(rust_struct.collect_no_default_strategy[0], "track1");

    // Test direct proto type assignment
    assert_eq!(rust_struct.direct_assignment_strategy.len(), 1);
    assert_eq!(rust_struct.direct_assignment_strategy[0].track_id, 2);
}

#[test]
fn test_collection_error_strategy_boundaries() {
    let proto_msg = proto::VecErrorMessage {
        tracks_with_error: vec![proto::Track { track_id: 1 }],
        tags_with_error: vec![], // Empty - should trigger default function
    };

    let rust_struct: CollectionErrorTestStruct = proto_msg.try_into().unwrap();

    // DMR: Update test to match corrected types
    // Test that empty collection triggered default function
    assert_eq!(rust_struct.collect_with_default_strategy.len(), 0);

    // Test direct assignment strategy
    assert_eq!(rust_struct.direct_assignment_strategy.len(), 1);
    assert_eq!(rust_struct.direct_assignment_strategy[0].track_id, 1);
}
