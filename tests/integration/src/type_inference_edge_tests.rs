use crate::proto;
use crate::shared_types::*;
use protto::Protto;

/// Test boundary conditions in type detection
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "EdgeCaseMessage")]
pub struct TypeInferenceEdgeCaseStruct {
    // Test Option<String> vs String detection (proto optional -> rust optional)
    #[protto(proto_name = "empty_vs_none")]
    pub option_string_field: Option<String>,

    // Test Vec<String> detection (proto repeated -> rust vec)
    #[protto(proto_name = "empty_vs_missing_vec")]
    pub vec_field: Vec<String>,

    // Test Option<u64> vs u64 detection (primitive optionality inference)
    #[protto(proto_name = "zero_vs_none")]
    pub option_primitive_field: Option<u64>,

    // Test Option<bool> vs bool detection (boolean optionality inference)
    #[protto(proto_name = "false_vs_none")]
    pub option_bool_field: Option<bool>,
}

/// Test newtype wrapper detection edge cases
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "CustomTypeMessage")]
pub struct NewtypeWrapperTestStruct {
    // Test single-segment path that IS a newtype wrapper (transparent)
    #[protto(transparent, expect(panic), proto_name = "track_id")]
    pub confirmed_newtype: TrackId,

    // Test single-segment path that is NOT a newtype (primitive/std type)
    #[protto(expect(panic), proto_name = "wrapper")]
    pub primitive_single_segment: String,

    // Test custom type (potential newtype) - Track is single-segment custom type
    #[protto(expect(panic), proto_name = "track")]
    pub potential_newtype: Track,
}

/// Test collection type boundary detection
#[derive(Protto, PartialEq, Debug, Clone)]
#[protto(module = "proto", proto_name = "VecOptionMessage")]
pub struct CollectionTypeBoundaryStruct {
    // Test is_direct_collection_type = true (Vec<T> -> repeated proto field)
    #[protto(proto_name = "optional_tracks")]
    pub direct_vec: Vec<Track>,

    // Test Option<Vec<T>> - is_any_collection_type = true, is_direct_collection_type = false
    #[protto(proto_name = "optional_strings")]
    pub option_vec: Option<Vec<String>>,

    // Test Vec<proto::Type> - direct assignment strategy
    #[protto(proto_name = "optional_proto_tracks")]
    pub proto_vec_field: Vec<proto::Track>,
}

#[test]
fn test_type_inference_boundary_cases() {
    let proto_msg = proto::EdgeCaseMessage {
        empty_vs_none: Some("test_string".to_string()), // DMR-9: Option<String> test
        empty_vs_missing_vec: vec!["item1".to_string(), "item2".to_string()], // DMR-9: Vec<String> test
        zero_vs_none: Some(42),    // DMR-9: Option<u64> test
        false_vs_none: Some(true), // DMR-9: Option<bool> test
    };

    let rust_struct: TypeInferenceEdgeCaseStruct = proto_msg.try_into().unwrap();

    // DMR-9: Test that Option<String> inference worked correctly
    assert_eq!(
        rust_struct.option_string_field,
        Some("test_string".to_string())
    );

    // DMR-9: Test that Vec<String> inference worked correctly
    assert_eq!(rust_struct.vec_field.len(), 2);
    assert_eq!(rust_struct.vec_field[0], "item1");
    assert_eq!(rust_struct.vec_field[1], "item2");

    // DMR-9: Test that Option<u64> inference worked correctly
    assert_eq!(rust_struct.option_primitive_field, Some(42));

    // DMR-9: Test that Option<bool> inference worked correctly
    assert_eq!(rust_struct.option_bool_field, Some(true));

    // DMR-9: Test with None/empty values to verify boundary conditions
    let empty_proto_msg = proto::EdgeCaseMessage {
        empty_vs_none: None,          // Should be None
        empty_vs_missing_vec: vec![], // Should be empty vec
        zero_vs_none: Some(0),        // Zero should not be confused with None
        false_vs_none: Some(false),   // False should not be confused with None
    };

    let empty_rust_struct: TypeInferenceEdgeCaseStruct = empty_proto_msg.try_into().unwrap();

    assert_eq!(empty_rust_struct.option_string_field, None);
    assert_eq!(empty_rust_struct.vec_field.len(), 0); // Empty vec stays empty vec
    assert_eq!(empty_rust_struct.option_primitive_field, Some(0)); // Zero is a valid value
    assert_eq!(empty_rust_struct.option_bool_field, Some(false)); // False is a valid value
}

#[test]
fn test_newtype_wrapper_detection_boundaries() {
    let proto_msg = proto::CustomTypeMessage {
        track: Some(proto::Track { track_id: 42 }),
        track_id: Some(123),
        wrapper: Some("wrapper_test".to_string()),
    };

    let rust_struct: NewtypeWrapperTestStruct = proto_msg.try_into().unwrap();

    // DMR-9: Verify transparent newtype detection worked correctly
    assert_eq!(rust_struct.confirmed_newtype.as_ref(), &123);

    // DMR-9: Verify std type (String) handled correctly - not detected as newtype
    assert_eq!(rust_struct.primitive_single_segment, "wrapper_test");

    // DMR-9: Verify custom type (Track) handled correctly - potential newtype
    assert_eq!(rust_struct.potential_newtype.id.as_ref(), &42);
}

#[test]
fn test_collection_type_boundary_detection() {
    let proto_msg = proto::VecOptionMessage {
        optional_tracks: vec![proto::Track { track_id: 1 }],
        optional_strings: vec!["string1".to_string(), "string2".to_string()], // DMR-9: Non-empty vec
        optional_proto_tracks: vec![proto::Track { track_id: 2 }, proto::Track { track_id: 3 }],
    };

    let rust_struct: CollectionTypeBoundaryStruct = proto_msg.try_into().unwrap();

    // DMR-9: Test boundary between direct Vec<T> conversion
    assert_eq!(rust_struct.direct_vec.len(), 1);
    assert_eq!(rust_struct.direct_vec[0].id.as_ref(), &1);

    // DMR-9: Test Option<Vec<T>> with non-empty vec (should be Some)
    assert_eq!(
        rust_struct.option_vec,
        Some(vec!["string1".to_string(), "string2".to_string()])
    );

    // DMR-9: Test Vec<proto::Track> direct assignment
    assert_eq!(rust_struct.proto_vec_field.len(), 2);
    assert_eq!(rust_struct.proto_vec_field[0].track_id, 2);
    assert_eq!(rust_struct.proto_vec_field[1].track_id, 3);

    // DMR-9: Test empty vec case for Option<Vec<T>>
    let empty_proto_msg = proto::VecOptionMessage {
        optional_tracks: vec![],
        optional_strings: vec![], // Empty - should become None for Option<Vec<T>>
        optional_proto_tracks: vec![],
    };

    let empty_rust_struct: CollectionTypeBoundaryStruct = empty_proto_msg.try_into().unwrap();

    assert_eq!(empty_rust_struct.direct_vec.len(), 0); // Empty vec stays empty vec
    assert_eq!(empty_rust_struct.option_vec, None); // Empty vec becomes None for Option<Vec<T>>
    assert_eq!(empty_rust_struct.proto_vec_field.len(), 0); // Empty vec stays empty vec
}
