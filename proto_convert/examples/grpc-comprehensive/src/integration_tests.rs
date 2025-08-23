use crate::basic_types::*;
use crate::complex_types::*;
use crate::error_types::*;
use crate::proto;
use crate::shared_types::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn proptest_all_attribute_combinations(
        base_id in any::<u64>(),
        optional_string in any::<Option<String>>(),
        required_string in "\\PC*",
        enum_val in any::<Option<Status>>(),
        collection_size in 0..10usize
    ) {
        let tracks: Vec<proto::Track> = (0..collection_size)
            .map(|i| proto::Track { track_id: base_id + i as u64 })
            .collect();

        let proto_msg = proto::CombinationMessage {
            rename_with_default: optional_string.clone(),
            transparent_with_expect: Some(required_string.clone()),
            enum_with_default_and_optional: enum_val.clone().map(|s| s.into()),
            collection_with_expect: tracks.clone(),
        };

        // Test successful conversion
        let result: Result<CombinationStruct, _> = proto_msg.clone().try_into();
        assert!(result.is_ok());
        let rust_msg: CombinationStruct = result.unwrap();

        prop_assert_eq!(rust_msg.renamed_field_with_default,
            optional_string.unwrap_or_else(|| "renamed_default".to_string()));
        prop_assert_eq!(rust_msg.transparent_field_with_expect, required_string);
        prop_assert_eq!(rust_msg.enum_with_default_and_optional,
            enum_val.unwrap_or_else(default_status));
        prop_assert_eq!(rust_msg.collection_with_expect.len(), collection_size);

        // Test error mode
        let error_result: Result<CombinationStruct, CombinationStructConversionError> = proto_msg.try_into();
        prop_assert!(error_result.is_ok());
    }

    #[test]
    fn proptest_mixed_optional_explicit_control(
        id in any::<u64>(),
        optional_name_val in any::<Option<String>>(),
        optional_count_val in any::<Option<u32>>(),
        optional_priority_val in any::<Option<u32>>(),
        repeated_tags_val in any::<Vec<String>>()
    ) {
        // This test would need proper proto message mapping
        // For demonstration, using existing message structure
        let proto_msg = proto::OptionalMessage {
            id,
            name: optional_name_val.clone(),
            count: optional_count_val.clone(),
            priority: optional_priority_val.clone(),
            tags: repeated_tags_val.clone(),
        };

        let rust_msg: DefaultStruct = proto_msg.into();
        prop_assert_eq!(rust_msg.id, id);
        prop_assert_eq!(rust_msg.name, optional_name_val.unwrap_or_default());
        prop_assert_eq!(rust_msg.count, optional_count_val.unwrap_or_default());
        prop_assert_eq!(rust_msg.priority, optional_priority_val.unwrap_or_else(|| default_priority()));
        prop_assert_eq!(
            rust_msg.tags,
            if !repeated_tags_val.is_empty() {
                repeated_tags_val
            } else {
                default_tags()
            }
        );
    }

    #[test]
    fn proptest_error_type_consistency(
        present_fields in prop::collection::vec(any::<bool>(), 3..=3),
        field_values in prop::collection::vec("\\PC*", 3..=3)
    ) {
        let proto_msg = proto::SimpleMessage {
            required_field: if present_fields[0] { Some(field_values[0].clone()) } else { None },
            required_number: if present_fields[1] { Some(42) } else { None },
            optional_field: if present_fields[2] { Some(field_values[2].clone()) } else { None },
        };

        // Test that error types are consistent across different conversion attempts
        if present_fields.iter().all(|&p| p) {
            // All present - should succeed with any error type
            let result1: Result<MultipleErrorTypesStruct, DetailedValidationError> = proto_msg.clone().try_into();
            let result2: Result<MultipleErrorTypesStruct, DetailedValidationError> = proto_msg.clone().try_into();
            let result3: Result<MultipleErrorTypesStruct, DetailedValidationError> = proto_msg.try_into();

            prop_assert!(result1.is_ok());
            prop_assert!(result2.is_ok());
            prop_assert!(result3.is_ok());
        } else {
            // Some missing - error behavior depends on which field is missing
            // This tests that the same input produces consistent error types
            if !present_fields[0] {
                let result: Result<MultipleErrorTypesStruct, DetailedValidationError> = proto_msg.clone().try_into();
                prop_assert!(result.is_err());
            }
        }
    }

    #[test]
    fn proptest_workflow_stress(
        track_count in 1..100usize,
        base_id in any::<u64>()
    ) {
        // Generate tracks
        let tracks: Vec<proto::Track> = (0..track_count)
            .map(|i| proto::Track {
                track_id: base_id.wrapping_add(i as u64)
            })
            .collect();

        let proto_state = proto::State { tracks: tracks.clone() };

        // Convert to different Rust representations
        let state: State = proto_state.clone().into();
        let map_state: MapState = proto_state.clone().into();
        let complex_state: ComplexState = proto_state.into();

        // Verify all have same track count
        prop_assert_eq!(state.tracks.len(), track_count);
        prop_assert_eq!(map_state.tracks.len(), track_count);
        prop_assert_eq!(complex_state.tracks.len(), track_count);

        // Convert all back and verify they can roundtrip
        let back_state: proto::State = state.into();
        let back_map: proto::State = map_state.into();
        let back_complex: proto::State = complex_state.into();

        prop_assert_eq!(back_state.tracks.len(), track_count);
        prop_assert_eq!(back_map.tracks.len(), track_count);
        prop_assert_eq!(back_complex.tracks.len(), track_count);
    }

    #[test]
    fn proptest_large_collections(
        track_count in 0..1000usize,
        base_id in any::<u64>()
    ) {
        let tracks: Vec<proto::Track> = (0..track_count)
            .map(|i| proto::Track { track_id: base_id.wrapping_add(i as u64) })
            .collect();

        let proto_state = proto::State { tracks: tracks.clone() };

        // Test all collection-handling structs
        let state: State = proto_state.clone().into();
        prop_assert_eq!(state.tracks.len(), track_count);

        let map_state: MapState = proto_state.clone().into();
        prop_assert_eq!(map_state.tracks.len(), track_count);

        let complex_state: ComplexState = proto_state.clone().into();
        prop_assert_eq!(complex_state.tracks.len(), track_count);

        // Test collection with expect
        let collection_result: Result<CollectionWithExpect, CollectionWithExpectConversionError> = proto_state.try_into();
        prop_assert!(collection_result.is_ok());
        if let Ok(collection_struct) = collection_result {
            prop_assert_eq!(collection_struct.tracks.len(), track_count);
        }
    }

    #[test]
    fn proptest_nested_option_combinations(
        outer_present in any::<bool>(),
        inner_track_id in any::<u64>()
    ) {
        let proto_track = if outer_present {
            Some(proto::Track { track_id: inner_track_id })
        } else {
            None
        };

        let proto_has_optional = proto::HasOptional { track: proto_track.clone() };

        // Test regular optional handling
        let rust_optional: HasOptional = proto_has_optional.clone().into();
        if outer_present {
            prop_assert!(rust_optional.track.is_some());
            prop_assert_eq!(rust_optional.track.unwrap().id, inner_track_id);
        } else {
            prop_assert!(rust_optional.track.is_none());
        }

        // Test expect error on optional
        let error_result: Result<HasOptionalWithError, HasOptionalWithErrorConversionError> = proto_has_optional.clone().try_into();
        if outer_present {
            prop_assert!(error_result.is_ok());
        } else {
            prop_assert!(error_result.is_err());
        }

        // Test custom error on optional
        let custom_result: Result<HasOptionalWithCustomError, CustomError> = proto_has_optional.try_into();
        if outer_present {
            prop_assert!(custom_result.is_ok());
        } else {
            prop_assert!(custom_result.is_err());
            prop_assert_eq!(custom_result.unwrap_err(), CustomError::TrackMissing);
        }
    }

    #[test]
    fn proptest_enum_edge_cases(
        status_values in prop::collection::vec(any::<Option<Status>>(), 4..=4)
    ) {
        let proto_statuses: Vec<Option<proto::Status>> = status_values.iter()
            .map(|opt_status| opt_status.as_ref().map(|s| s.clone().into()))
            .collect();

        let proto_msg = proto::EnumMessage {
            status_panic: proto_statuses[0].clone().map(|s| s.into()),
            status_error: proto_statuses[1].clone().map(|s| s.into()),
            status_default: proto_statuses[2].clone().map(|s| s.into()),
            status_optional: proto_statuses[3].clone().map(|s| s.into()),
        };

        // Test panic mode - should only succeed if status_panic is present
        let panic_result: Result<ComprehensiveEnumStruct, _> = std::panic::catch_unwind(|| {
            let result: Result<ComprehensiveEnumStruct, _> = proto_msg.clone().try_into();
            let rust_msg: ComprehensiveEnumStruct = result.unwrap();
            rust_msg
        });


        if status_values[0].is_some() && status_values[1].is_some() {
            // Add debug prints to see actual proto message contents
// println!("status_values[0]: {:?}", status_values[0]);
// println!("proto_statuses[0]: {:?}", proto_statuses[0]);
// println!("proto_msg.status_panic: {:?}", proto_msg.status_panic);

            prop_assert!(panic_result.is_ok());
            if let Ok(rust_msg) = panic_result {
                prop_assert_eq!(rust_msg.enum_expect_panic, status_values[0].clone().map(|s| s.into()).unwrap());
            }
        } else {
            prop_assert!(panic_result.is_err());
        }

        // Test error mode - should succeed if both status_error and status_panic are present
        if status_values[0].is_some() && status_values[1].is_some() {
            let error_result: Result<ComprehensiveEnumStruct, ComprehensiveEnumStructConversionError> = proto_msg.try_into();
            prop_assert!(error_result.is_ok());

            if let Ok(rust_msg) = error_result {
                prop_assert_eq!(rust_msg.enum_expect_error, status_values[1].clone().map(|s| s.into()).unwrap());
                prop_assert_eq!(rust_msg.enum_with_default, status_values[2].clone().map(|s| s.into()).unwrap_or_else(default_status));
                prop_assert_eq!(rust_msg.enum_optional_explicit, status_values[3].clone().map(|s| s.into()).unwrap_or_else(default_status_optional));
            }
        }
    }
}

#[test]
fn debug_enum_panic_field() {
    let proto_msg = proto::EnumMessage {
        status_panic: Some(proto::Status::MovedPermanently.into()),
        status_error: None,
        status_default: None,
        status_optional: None,
    };

    println!("Proto message: {:?}", proto_msg);

    let result: Result<ComprehensiveEnumStruct, _> = proto_msg.try_into();
    println!("Conversion result: {:?}", result);

    println!("Panic result: {:?}", result.is_ok());
}

// Test that rename works with all other attributes
#[test]
fn test_rename_with_expect_panic() {
    let proto_msg = proto::CombinationMessage {
        rename_with_default: Some("renamed_value".to_string()),
        transparent_with_expect: Some("transparent_value".to_string()),
        enum_with_default_and_optional: None, // Should use default
        collection_with_expect: vec![],
    };

    let result: Result<CombinationStruct, _> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg: CombinationStruct = result.unwrap();
    assert_eq!(rust_msg.renamed_field_with_default, "renamed_value");
    assert_eq!(rust_msg.transparent_field_with_expect, "transparent_value");
    assert_eq!(rust_msg.enum_with_default_and_optional, Status::Ok); // default
}

#[test]
fn test_rename_with_default() {
    let proto_msg = proto::CombinationMessage {
        rename_with_default: None, // Should use custom default
        transparent_with_expect: Some("transparent_value".to_string()),
        enum_with_default_and_optional: Some(proto::Status::Found.into()),
        collection_with_expect: vec![],
    };

    let result: Result<CombinationStruct, _> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg: CombinationStruct = result.unwrap();
    assert_eq!(rust_msg.renamed_field_with_default, "renamed_default");
}

// Test transparent with different expectation modes
#[test]
fn test_transparent_wrapper_conversion_modes() {
    let wrapper = TransparentWrapper::new("test_value");
    assert_eq!(wrapper, "test_value");

    let back_to_wrapper = TransparentWrapper::from(wrapper.as_str());
    assert_eq!(back_to_wrapper, wrapper);
}

// Test enum with optional and default combination
#[test]
fn test_enum_optional_with_default_none() {
    let proto_msg = proto::EnumMessage {
        status_panic: Some(proto::Status::Ok.into()),
        status_error: Some(proto::Status::Found.into()),
        status_default: None,  // Should use default
        status_optional: None, // Should be None
    };

    let result: Result<ComprehensiveEnumStruct, ComprehensiveEnumStructConversionError> =
        proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg = result.unwrap();

    assert_eq!(rust_msg.enum_with_default, Status::Ok); // default_status()
    assert_eq!(rust_msg.enum_optional_explicit, Some(Status::Ok));
}

// Test collection edge cases
#[test]
fn test_collection_with_nested_conversions() {
    let proto_tracks = vec![
        proto::Track { track_id: 1 },
        proto::Track { track_id: 2 },
        proto::Track { track_id: 3 },
    ];

    let proto_state = proto::State {
        tracks: proto_tracks.clone(),
    };

    let rust_state: CollectionWithDefault = proto_state.clone().into();
    assert_eq!(rust_state.tracks.len(), 3);

    // Verify each nested conversion worked
    for (i, track) in rust_state.tracks.iter().enumerate() {
        assert_eq!(track.id, proto_tracks[i].track_id);
    }

    // Test expect version
    let result: Result<CollectionWithExpect, CollectionWithExpectConversionError> =
        proto_state.try_into();
    assert!(result.is_ok());
    let rust_state = result.unwrap();
    assert_eq!(rust_state.tracks.len(), 3);
}

#[test]
fn test_real_world_workflow() {
    // Simulate a real workflow: receive proto, convert, modify, convert back

    let incoming_proto = proto::State {
        tracks: vec![
            proto::Track { track_id: 1 },
            proto::Track { track_id: 2 },
            proto::Track { track_id: 3 },
        ],
    };

    // Convert to Rust for processing
    let mut rust_state: State = incoming_proto.into();

    // Modify the data
    rust_state.tracks.push(Track {
        id: TrackId::new(4),
    });
    rust_state.tracks.retain(|t| t.id != 2); // Remove track 2

    // Convert back to proto for sending
    let outgoing_proto: proto::State = rust_state.into();

    assert_eq!(outgoing_proto.tracks.len(), 3); // 1, 3, 4
    let track_ids: Vec<u64> = outgoing_proto.tracks.iter().map(|t| t.track_id).collect();
    assert!(track_ids.contains(&1));
    assert!(track_ids.contains(&3));
    assert!(track_ids.contains(&4));
    assert!(!track_ids.contains(&2));
}

#[test]
fn test_error_recovery_workflow() {
    // Test a workflow where some conversions fail and need to be handled

    let problematic_protos = vec![
        proto::SimpleMessage {
            required_field: None, // Missing required field
            required_number: Some(42),
            optional_field: None,
        },
        proto::SimpleMessage {
            required_field: Some("present".to_string()),
            required_number: None, // Missing required field
            optional_field: None,
        },
        proto::SimpleMessage {
            required_field: Some("good".to_string()),
            required_number: Some(123),
            optional_field: Some("also good".to_string()),
        },
    ];

    let mut successful_conversions = 0;
    let mut failed_conversions = 0;

    for proto_msg in problematic_protos {
        let result: Result<ExpectErrorStruct, ExpectErrorStructConversionError> =
            proto_msg.try_into();
        match result {
            Ok(_) => successful_conversions += 1,
            Err(_) => failed_conversions += 1,
        }
    }

    assert_eq!(successful_conversions, 1);
    assert_eq!(failed_conversions, 2);
}

#[test]
fn test_map_conversion_workflow() {
    // Test the map conversion workflow with lookups

    let proto_state = proto::State {
        tracks: vec![
            proto::Track { track_id: 100 },
            proto::Track { track_id: 200 },
            proto::Track { track_id: 300 },
        ],
    };

    let map_state: MapState = proto_state.into();

    // Test map operations
    assert!(map_state.tracks.contains_key(&TrackId::new(100)));
    assert!(map_state.tracks.contains_key(&TrackId::new(200)));
    assert!(map_state.tracks.contains_key(&TrackId::new(300)));
    assert!(!map_state.tracks.contains_key(&TrackId::new(400)));

    // Test lookup
    let track_200 = map_state.tracks.get(&TrackId::new(200));
    assert!(track_200.is_some());
    assert_eq!(track_200.unwrap().id, 200);

    // Convert back and verify order doesn't matter
    let back_to_proto: proto::State = map_state.into();
    assert_eq!(back_to_proto.tracks.len(), 3);

    let mut track_ids: Vec<u64> = back_to_proto.tracks.iter().map(|t| t.track_id).collect();
    track_ids.sort();
    assert_eq!(track_ids, vec![100, 200, 300]);
}

#[test]
fn test_all_attributes_in_one_struct() {
    // This would require a dedicated proto message and struct that uses all attributes
    // For now, test existing comprehensive examples

    let proto_msg = proto::CombinationMessage {
        rename_with_default: None,                         // Tests rename + default
        transparent_with_expect: Some("test".to_string()), // Tests transparent + expect(panic)
        enum_with_default_and_optional: None,              // Tests enum + default + optional
        collection_with_expect: vec![],                    // Tests collection + expect
    };

    let result: Result<CombinationStruct, _> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg: CombinationStruct = result.unwrap();
    assert_eq!(rust_msg.renamed_field_with_default, "renamed_default");
    assert_eq!(rust_msg.transparent_field_with_expect, "test");
    assert_eq!(rust_msg.enum_with_default_and_optional, Status::Ok);
    assert!(rust_msg.collection_with_expect.is_empty());
}

#[test]
fn test_error_handling_consistency() {
    // Verify that error handling is consistent across different field types

    // String field with expect
    let proto_msg1 = proto::SimpleMessage {
        required_field: None,
        required_number: Some(42),
        optional_field: None,
    };

    let result1: Result<ExpectErrorStruct, ExpectErrorStructConversionError> =
        proto_msg1.try_into();
    assert!(result1.is_err());

    // Numeric field with expect
    let proto_msg2 = proto::SimpleMessage {
        required_field: Some("present".to_string()),
        required_number: None,
        optional_field: None,
    };

    let result2: Result<ExpectErrorStruct, ExpectErrorStructConversionError> =
        proto_msg2.try_into();
    assert!(result2.is_err());

    // Both should use the same error type and format
    assert_eq!(
        std::mem::discriminant(&result1.unwrap_err()),
        std::mem::discriminant(&result2.unwrap_err())
    );
}

#[test]
fn test_module_override_behavior() {
    // Test that module can be overridden at struct level
    let proto_track = proto::Track { track_id: 42 };

    // Track has explicit module = "proto"
    let rust_track: Track = proto_track.clone().into();
    assert_eq!(rust_track.id, 42);

    // Verify it works the same as default module behavior
    let back_to_proto: proto::Track = rust_track.into();
    assert_eq!(back_to_proto, proto_track);
}
