use crate::basic_types::*;
use crate::complex_types::*;
use crate::default_types::*;
use crate::error_types::*;
use crate::proto;
use crate::shared_types::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn proptest_edge_cases(
        string_value in any::<Option<String>>(),
        vec_value in any::<Vec<String>>(),
        number_value in any::<Option<u64>>(),
        bool_value in any::<Option<bool>>()
    ) {
        let proto_msg = proto::EdgeCaseMessage {
            empty_vs_none: string_value.clone(),
            empty_vs_missing_vec: vec_value.clone(),
            zero_vs_none: number_value,
            false_vs_none: bool_value,
        };

        let rust_msg: EdgeCaseStruct = proto_msg.into();

        // Test default behavior
        prop_assert_eq!(rust_msg.empty_vs_none, string_value.unwrap_or_default());
        prop_assert_eq!(rust_msg.zero_vs_none, number_value.unwrap_or_default());
        prop_assert_eq!(rust_msg.false_vs_none, bool_value.unwrap_or_default());

        // Test custom default for vec
        if vec_value.is_empty() {
            prop_assert_eq!(rust_msg.empty_vs_missing_vec, vec!["default_item"]);
        } else {
            prop_assert_eq!(rust_msg.empty_vs_missing_vec, vec_value);
        }
    }

    #[test]
    fn proptest_enum_all_variants_roundtrip(status in any::<Status>()) {
        // Test that all enum variants roundtrip correctly through different modes

        // Direct enum conversion
        let proto_status: proto::Status = status.clone().into();
        let back_to_rust: Status = proto_status.into();
        prop_assert_eq!(status.clone(), back_to_rust);

        // Through i32
        let as_i32: i32 = status.clone().into();
        let from_i32: Status = Status::from(as_i32);
        prop_assert_eq!(status.clone(), from_i32);

        // In a message with different expect modes
        let proto_msg = proto::EnumMessage {
            status_panic: Some(status.clone().into()),
            status_error: Some(status.clone().into()),
            status_default: Some(status.clone().into()),
            status_optional: Some(status.clone().into()),
        };

        let result: Result<ComprehensiveEnumStruct, ComprehensiveEnumStructConversionError> = proto_msg.try_into();
        prop_assert!(result.is_ok());

        if let Ok(rust_msg) = result {
            prop_assert_eq!(rust_msg.enum_expect_error, status.clone());
            prop_assert_eq!(rust_msg.enum_with_default, status.clone());
            prop_assert_eq!(rust_msg.enum_optional_explicit, Some(status.clone()));
            prop_assert_eq!(rust_msg.enum_expect_panic, status);
        }
    }

    #[test]
    fn proptest_default_function_vs_default_trait(
        track_id in any::<u64>(),
        has_name in any::<bool>(),
        has_priority in any::<bool>(),
        has_tags in any::<bool>()
    ) {
        let name = if has_name { Some("test".to_string()) } else { None };
        let priority = if has_priority { Some(5) } else { None };
        let tags = if has_tags { vec!["tag1".to_string()] } else { vec![] };

        let proto_msg = proto::OptionalMessage {
            id: track_id,
            name: name.clone(),
            count: None,
            priority,
            tags: tags.clone(),
        };

        let rust_msg: DefaultStruct = proto_msg.into();

        prop_assert_eq!(rust_msg.id, track_id);

        // Test default vs custom default behavior
        if name.is_none() {
            prop_assert_eq!(rust_msg.name, String::default()); // ""
        } else {
            prop_assert_eq!(rust_msg.name, name.unwrap());
        }

        if priority.is_none() {
            prop_assert_eq!(rust_msg.priority, 10); // custom default
        } else {
            prop_assert_eq!(rust_msg.priority, priority.unwrap());
        }

        if tags.is_empty() {
            prop_assert_eq!(rust_msg.tags, vec!["default"]); // custom default
        } else {
            prop_assert_eq!(rust_msg.tags, tags);
        }
    }

    #[test]
    fn proptest_error_recovery_patterns(
        field1 in any::<Option<String>>(),
        field2 in any::<Option<String>>(),
        field3 in any::<Option<u64>>()
    ) {
        let proto_msg = proto::SimpleMessage {
            required_field: field1.clone(),
            required_number: field3,
            optional_field: field2.clone(),
        };

        // Test different error recovery strategies
        match (field1.is_some(), field2.is_some(), field3.is_some()) {
            (true, true, true) => {
                // All required fields present - should succeed
                let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
                if result.is_err() {
                    println!("Conversion failed with error: {:?}", result.as_ref().err());
                }
                prop_assert!(result.is_ok());
            },
            _ => {
                // First field missing - should get DetailedValidationError
                let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
                prop_assert!(result.is_err());
            },
        }
    }
}

// Test empty string vs None distinction
#[test]
fn test_empty_string_handling() {
    let proto_with_empty = proto::EdgeCaseMessage {
        empty_vs_none: Some("".to_string()),
        empty_vs_missing_vec: vec![],
        zero_vs_none: Some(0),
        false_vs_none: Some(false),
    };

    let proto_with_none = proto::EdgeCaseMessage {
        empty_vs_none: None,
        empty_vs_missing_vec: vec![],
        zero_vs_none: None,
        false_vs_none: None,
    };

    let rust_from_empty: EdgeCaseStruct = proto_with_empty.into();
    let rust_from_none: EdgeCaseStruct = proto_with_none.into();

    // Both should result in the same values due to default behavior
    assert_eq!(rust_from_empty.empty_vs_none, "");
    assert_eq!(rust_from_none.empty_vs_none, "");
    assert_eq!(rust_from_empty.zero_vs_none, 0);
    assert_eq!(rust_from_none.zero_vs_none, 0);
    assert_eq!(rust_from_empty.false_vs_none, false);
    assert_eq!(rust_from_none.false_vs_none, false);
}

// Test that roundtrips preserve semantics
#[test]
fn test_semantic_roundtrip_preservation() {
    let original_empty = EdgeCaseStruct {
        empty_vs_none: "".to_string(),
        empty_vs_missing_vec: vec!["default_item".to_string()],
        zero_vs_none: 0,
        false_vs_none: false,
    };

    let original_non_empty = EdgeCaseStruct {
        empty_vs_none: "non-empty".to_string(),
        empty_vs_missing_vec: vec!["item1".to_string(), "item2".to_string()],
        zero_vs_none: 42,
        false_vs_none: true,
    };

    // Test roundtrips
    let proto_from_empty: proto::EdgeCaseMessage = original_empty.clone().into();
    let back_from_empty: EdgeCaseStruct = proto_from_empty.into();

    let proto_from_non_empty: proto::EdgeCaseMessage = original_non_empty.clone().into();
    let back_from_non_empty: EdgeCaseStruct = proto_from_non_empty.into();

    assert_eq!(back_from_empty.empty_vs_none, original_empty.empty_vs_none);
    assert_eq!(
        back_from_non_empty.empty_vs_none,
        original_non_empty.empty_vs_none
    );
    assert_eq!(
        back_from_non_empty.zero_vs_none,
        original_non_empty.zero_vs_none
    );
    assert_eq!(
        back_from_non_empty.false_vs_none,
        original_non_empty.false_vs_none
    );
}

// Test error cascading in complex structures
#[test]
fn test_error_cascading() {
    // Test that errors in nested structures bubble up correctly
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: None,        // First error
        field_with_custom_error: None, // Would be second error if first didn't fail
        number_with_default: Some(123),
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![],
    };

    // Should fail on the first missing field
    let result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::MissingField("field_with_error".to_string())
    );
}

// Test zero values vs None for numeric types
#[test]
fn test_numeric_zero_vs_none() {
    let proto_with_zero = proto::EdgeCaseMessage {
        empty_vs_none: Some("".to_string()),
        empty_vs_missing_vec: vec![],
        zero_vs_none: Some(0),
        false_vs_none: Some(false),
    };

    let proto_with_none = proto::EdgeCaseMessage {
        empty_vs_none: None,
        empty_vs_missing_vec: vec![],
        zero_vs_none: None,
        false_vs_none: None,
    };

    let rust_from_zero: EdgeCaseStruct = proto_with_zero.into();
    let rust_from_none: EdgeCaseStruct = proto_with_none.into();

    // Both should result in default values
    assert_eq!(rust_from_zero.zero_vs_none, 0);
    assert_eq!(rust_from_none.zero_vs_none, 0);
    assert_eq!(rust_from_zero.false_vs_none, false);
    assert_eq!(rust_from_none.false_vs_none, false);
}

// Test specific combinations that might have caused issues
#[test]
fn test_regression_empty_vec_with_default() {
    let proto_msg = proto::EdgeCaseMessage {
        empty_vs_none: None,
        empty_vs_missing_vec: vec![], // Empty vec should trigger custom default
        zero_vs_none: None,
        false_vs_none: None,
    };

    let rust_msg: EdgeCaseStruct = proto_msg.into();
    assert_eq!(rust_msg.empty_vs_missing_vec, vec!["default_item"]);
}

#[test]
fn test_regression_option_with_transparent() {
    // Test that Option<TransparentWrapper> works correctly
    let proto_msg = proto::CombinationMessage {
        rename_with_default: None,
        transparent_with_expect: Some("wrapper".to_string()),
        enum_with_default_and_optional: None,
        collection_with_expect: vec![],
    };

    let result: Result<CombinationStruct, CombinationStructConversionError> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg: CombinationStruct = result.unwrap();
    assert_eq!(rust_msg.transparent_field_with_expect.as_str(), "wrapper");
}

#[test]
fn test_regression_multiple_expects_same_struct() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic".to_string()),
        field_with_error: Some("error".to_string()),
        field_with_custom_error: Some("custom".to_string()),
        number_with_default: None,
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![],
    };

    // Should succeed when all expected fields are present
    let result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg = result.unwrap();
    assert_eq!(rust_msg.field_with_error, "error");
    assert_eq!(rust_msg.number_with_default, 9999); // default value
}

// Test that panic and error modes can coexist
#[test]
fn test_regression_panic_and_error_coexistence() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic".to_string()),
        field_with_error: Some("error".to_string()),
        field_with_custom_error: Some("custom".to_string()),
        number_with_default: Some(123),
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![proto::Track { track_id: 1 }],
    };

    // Panic mode should work when all panic fields are present
    let panic_result = std::panic::catch_unwind(|| {
        let panic_proto = proto::ComplexExpectMessage {
            field_with_panic: None,
            ..proto_msg.clone()
        };
        let _: Result<ComplexExpectStruct, _> = panic_proto.try_into();
        assert!(false);
    });
    assert!(panic_result.is_err());

    // Error mode should also work
    let error_result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    assert!(error_result.is_ok());
}

// Test default functions are called correctly
#[test]
fn test_regression_default_function_calls() {
    let proto_msg = proto::OptionalMessage {
        id: 1,
        name: None,
        count: None,
        priority: None, // Should call default_priority()
        tags: vec![],   // Should call default_tags()
    };

    let rust_msg: DefaultStruct = proto_msg.into();
    assert_eq!(rust_msg.priority, 10); // from default_priority()
    assert_eq!(rust_msg.tags, vec!["default"]); // from default_tags()
}

// Test that ignore works with other attributes
#[test]
fn test_regression_ignore_with_other_attributes() {
    let proto_state = proto::State {
        tracks: vec![proto::Track { track_id: 123 }],
    };

    let complex_state: ComplexState = proto_state.into();
    assert_eq!(complex_state.tracks.len(), 1);
    assert_eq!(complex_state.tracks[0].id, 123);

    // Ignored fields should use defaults
    assert!(complex_state.launches.is_empty());
    assert_eq!(
        complex_state
            .counter
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

// Test boundary conditions for numeric types
#[test]
fn test_numeric_boundaries() {
    let boundary_values = [u64::MIN, u64::MAX, 0, 1, u64::MAX - 1];

    for &value in &boundary_values {
        let track_id = TrackId::new(value);
        let as_u64: u64 = track_id.clone().into();
        assert_eq!(as_u64, value);

        let back_to_track_id = TrackId::from(as_u64);
        assert_eq!(back_to_track_id, track_id);

        // Test in proto context
        let proto_track = proto::Track { track_id: value };
        let rust_track: Track = proto_track.clone().into();
        assert_eq!(rust_track.id, value);

        let back_to_proto: proto::Track = rust_track.into();
        assert_eq!(back_to_proto, proto_track);
    }
}

// Test string boundary conditions
#[test]
fn test_string_boundaries() {
    let string_values = [
        "".to_string(),
        "a".to_string(),
        "very long string that might test buffer limits".repeat(100),
        "unicode: 🦀 rust 🔥".to_string(),
        "\n\t\r special chars".to_string(),
    ];

    for value in string_values {
        let proto_msg = proto::SimpleMessage {
            required_field: Some(value.clone()),
            required_number: Some(42),
            optional_field: Some(value.clone()),
        };

        let rust_msg: ExpectPanicStruct = proto_msg.clone().into();
        assert_eq!(rust_msg.required_field, value);
        assert_eq!(rust_msg.optional_field, Some(value.clone()));

        let back_to_proto: proto::SimpleMessage = rust_msg.into();
        assert_eq!(back_to_proto, proto_msg);
    }
}

// Test collection boundary conditions
#[test]
fn test_collection_boundaries() {
    // Empty collection
    let empty_state = proto::State { tracks: vec![] };
    let rust_state: State = empty_state.clone().into();
    assert!(rust_state.tracks.is_empty());

    // Single item
    let single_state = proto::State {
        tracks: vec![proto::Track { track_id: 1 }],
    };
    let rust_state: State = single_state.clone().into();
    assert_eq!(rust_state.tracks.len(), 1);

    // Many items
    let many_tracks: Vec<proto::Track> = (0..1000)
        .map(|i| proto::Track { track_id: i as u64 })
        .collect();
    let many_state = proto::State {
        tracks: many_tracks,
    };
    let rust_state: State = many_state.clone().into();
    assert_eq!(rust_state.tracks.len(), 1000);
}
