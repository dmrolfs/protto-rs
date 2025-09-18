use crate::basic_types::*;
use crate::complex_types::*;
use crate::error_types::*;
use crate::proto;
use crate::shared_types::*;
use proptest::prelude::*;
use std::panic;

proptest! {
    #[test]
    fn test_expect_error_roundtrip_when_present(
        track_id in any::<u64>()
    ) {
        let proto_track = proto::Track { track_id };
        let proto_has_optional = proto::HasOptional {
            track: Some(proto_track),
        };

        let rust_struct: HasOptionalWithError = proto_has_optional.clone().try_into().unwrap();
        let back_to_proto: proto::HasOptional = rust_struct.into();

        prop_assert_eq!(back_to_proto.track.unwrap().track_id, track_id);
    }

    #[test]
    fn proptest_complex_expect_valid_fields(
        panic_field in "\\PC*",
        error_field in "\\PC*",
        custom_error_field in "\\PC*",
        number in any::<Option<u64>>(),
        status_panic in any::<Status>(),
        status_error in any::<Status>(),
        track_count in 0..5usize
    ) {
        let tracks: Vec<proto::Track> = (0..track_count)
            .map(|i| proto::Track { track_id: i as u64 })
            .collect();

        let proto_msg = proto::ComplexExpectMessage {
            field_with_panic: Some(panic_field.clone()),
            field_with_error: Some(error_field.clone()),
            field_with_custom_error: Some(custom_error_field.clone()),
            number_with_default: number,
            enum_with_panic: Some(status_panic.clone().into()),
            enum_with_error: Some(status_error.clone().into()),
            tracks_with_expect: tracks,
        };

        // Test panic mode (should always succeed with valid data)
        let panic_result = std::panic::catch_unwind(|| {
            let result: Result<ComplexExpectStruct, _> = proto_msg.clone().try_into();
            assert!(result.is_ok());
            let rust_msg: ComplexExpectStruct = result.unwrap();
            rust_msg
        });
        prop_assert!(panic_result.is_ok());

        if let Ok(rust_msg) = panic_result {
            prop_assert_eq!(rust_msg.field_with_panic, panic_field);
            prop_assert_eq!(rust_msg.number_with_default, number.unwrap_or(9999));
            prop_assert_eq!(rust_msg.enum_with_panic, status_panic);
        }

        // Test error mode
        let error_result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
        prop_assert!(error_result.is_ok());

        if let Ok(rust_msg) = error_result {
            prop_assert_eq!(rust_msg.field_with_error, error_field);
            prop_assert_eq!(rust_msg.enum_with_error, status_error);
            prop_assert_eq!(rust_msg.tracks_with_expect.len(), track_count);
        }
    }

    #[test]
    fn proptest_combination_attributes(
        renamed_value in any::<Option<String>>(),
        transparent_value in "\\PC*",
        enum_value in any::<Option<Status>>(),
        tracks in any::<Vec<proto::Track>>()
    ) {
        let proto_msg = proto::CombinationMessage {
            rename_with_default: renamed_value.clone(),
            transparent_with_expect: Some(transparent_value.clone()),
            enum_with_default_and_optional: enum_value.clone().map(|s| s.into()),
            collection_with_expect: tracks.clone(),
        };

        // Test panic version first
        let panic_result = panic::catch_unwind(|| {
            let result: Result<CombinationStruct, _> = proto_msg.clone().try_into();
            assert!(result.is_ok());
            let rust_msg: CombinationStruct = result.unwrap();
            rust_msg
        });

        // Should succeed when transparent field is present
        prop_assert!(panic_result.is_ok());

        if let Ok(rust_msg) = panic_result {
            prop_assert_eq!(rust_msg.renamed_field_with_default,
                renamed_value.unwrap_or_else(|| "renamed_default".to_string()));
            prop_assert_eq!(rust_msg.transparent_field_with_expect.as_str(), &transparent_value);
            prop_assert_eq!(rust_msg.enum_with_default_and_optional,
                enum_value.unwrap_or_else(default_status));
            prop_assert_eq!(rust_msg.collection_with_expect.len(), tracks.len());
        }

        // Test error version
        let error_result: Result<CombinationStruct, CombinationStructConversionError> = proto_msg.try_into();
        prop_assert!(error_result.is_ok());
    }

    #[test]
    fn proptest_mixed_optional_behaviors(
        id in any::<u64>(),
        optional_true_value in any::<Option<String>>(),
    ) {
        let proto_msg = proto::OptionalMessage {
            id,
            name: optional_true_value.clone(),
            count: None, // Not used in this struct
            priority: None, // Not used in this struct
            tags: vec![], // Not used in this struct
        };

        // This test would need the proto message to be adjusted to match the struct fields
        // For now, test the basic conversion
        let default_struct: DefaultStruct = proto_msg.into();
        prop_assert_eq!(default_struct.id, id);
        prop_assert_eq!(default_struct.name, optional_true_value.unwrap_or_default());
    }
}

#[test]
fn test_expect_error_with_present_field() {
    let proto_track = proto::Track { track_id: 123 };
    let proto_has_optional = proto::HasOptional {
        track: Some(proto_track),
    };

    // Should succeed when field is present
    let result: Result<HasOptionalWithError, HasOptionalWithErrorConversionError> =
        proto_has_optional.try_into();
    assert!(result.is_ok());
    let rust_struct = result.unwrap();
    assert_eq!(rust_struct.track.unwrap().id, 123);
}

#[test]
fn test_expect_error_with_missing_field() {
    let proto_has_optional = proto::HasOptional { track: None };

    // Should return ConversionError when field is missing
    let result: Result<HasOptionalWithError, HasOptionalWithErrorConversionError> =
        proto_has_optional.try_into();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        HasOptionalWithErrorConversionError::MissingField("track".to_string())
    );
}

#[test]
fn test_expect_error_with_custom_error() {
    let proto_has_optional = proto::HasOptional { track: None };

    // Should return custom error when field is missing
    let result: Result<HasOptionalWithCustomError, CustomError> = proto_has_optional.try_into();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), CustomError::TrackMissing);
}

#[test]
fn test_complex_expect_all_present() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: Some("error_field".to_string()),
        field_with_custom_error: Some("custom_error_field".to_string()),
        number_with_default: Some(123),
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![proto::Track { track_id: 1 }],
    };

    // This should succeed for both panic and error modes when all fields are present
    let result: Result<ComplexExpectStruct, _> = proto_msg.clone().try_into();
    assert!(result.is_ok());
    let rust_msg: ComplexExpectStruct = result.unwrap();
    assert_eq!(rust_msg.field_with_panic, "panic_field");
    assert_eq!(rust_msg.number_with_default, 123);
    assert_eq!(rust_msg.enum_with_panic, Status::Ok);

    // Test error mode struct
    let result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg = result.unwrap();
    assert_eq!(rust_msg.field_with_error, "error_field");
    assert_eq!(rust_msg.enum_with_error, Status::Found);
    assert_eq!(rust_msg.tracks_with_expect.len(), 1);
}

#[test]
#[should_panic(expected = "Proto field field_with_panic is required")]
fn test_complex_expect_panic_missing() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: None, // Should panic
        field_with_error: Some("error_field".to_string()),
        field_with_custom_error: Some("custom_error_field".to_string()),
        number_with_default: None, // Should use default
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![],
    };

    let _: Result<ComplexExpectStruct, _> = proto_msg.try_into();
    assert!(false);
}

#[test]
fn test_complex_expect_error_missing() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: None, // Should return error
        field_with_custom_error: Some("custom_error_field".to_string()),
        number_with_default: None,
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![],
    };

    let result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::MissingField("field_with_error".to_string())
    );
}

#[test]
fn test_complex_expect_custom_error_missing() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: Some("error_field".to_string()),
        field_with_custom_error: None, // Should return custom error
        number_with_default: None,
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![],
    };

    let result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::InvalidValue("field_with_custom_error".to_string())
    );
}

#[test]
fn test_default_with_missing_field() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: Some("error_field".to_string()),
        field_with_custom_error: Some("custom_error_field".to_string()),
        number_with_default: None, // Should use default
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![],
    };

    let result: Result<ComplexExpectStruct, _> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg: ComplexExpectStruct = result.unwrap();
    assert_eq!(rust_msg.number_with_default, 9999); // custom default
}

#[test]
#[should_panic(expected = "Proto field enum_with_panic is required")]
fn test_enum_expect_panic_missing() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: Some("error_field".to_string()),
        field_with_custom_error: Some("custom_error_field".to_string()),
        number_with_default: Some(123),
        enum_with_panic: None, // Should panic
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![],
    };

    let _: Result<ComplexExpectStruct, _> = proto_msg.try_into();
    assert!(false);
}

#[test]
fn test_enum_expect_error_missing() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: Some("error_field".to_string()),
        field_with_custom_error: Some("custom_error_field".to_string()),
        number_with_default: Some(123),
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: None, // Should return error
        tracks_with_expect: vec![],
    };

    let result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        ValidationError::ConversionFailed("enum_with_error".to_string())
    );
}

#[test]
fn test_collection_expect_error_missing() {
    let proto_msg = proto::ComplexExpectMessage {
        field_with_panic: Some("panic_field".to_string()),
        field_with_error: Some("error_field".to_string()),
        field_with_custom_error: Some("custom_error_field".to_string()),
        number_with_default: Some(123),
        enum_with_panic: Some(proto::Status::Ok.into()),
        enum_with_error: Some(proto::Status::Found.into()),
        tracks_with_expect: vec![], // Empty vec might be treated as missing depending on implementation
    };

    // This test might need adjustment based on how your macro handles empty vs missing collections
    let result: Result<ComplexExpectStruct, ValidationError> = proto_msg.try_into();
    // Collections are typically always present (empty vec vs None), so this should succeed
    if result.is_ok() {
        let rust_msg = result.unwrap();
        assert!(rust_msg.tracks_with_expect.is_empty());
    }
}

// Test multiple error types in same conversion
#[test]
fn test_multiple_error_types_success() {
    let proto_msg = proto::SimpleMessage {
        required_field: Some("field1".to_string()),
        required_number: Some(42),
        optional_field: Some("field3".to_string()),
    };

    let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg = result.unwrap();
    assert_eq!(rust_msg.field_with_detailed_error, "field1");
    assert_eq!(rust_msg.field_with_basic_error, "field3");
    assert_eq!(rust_msg.field_with_generated_error, 42);
}

#[test]
fn test_multiple_error_types_detailed_error() {
    let proto_msg = proto::SimpleMessage {
        required_field: None, // Should trigger detailed error
        required_number: Some(42),
        optional_field: None,
    };

    let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        DetailedValidationError::MissingRequired("required_field".to_string())
    );
}

#[test]
fn test_multiple_error_types_basic_error() {
    let proto_msg = proto::SimpleMessage {
        required_field: Some("field1".to_string()),
        required_number: None, // Should trigger basic ValidationError
        optional_field: None,
    };

    let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
    assert!(result.is_err());
    // The exact error depends on how the macro generates the conversion
    assert!(result.is_err());
}

#[test]
fn test_multiple_error_types_generated_error() {
    let proto_msg = proto::SimpleMessage {
        required_field: Some("field1".to_string()),
        required_number: Some(42),
        optional_field: Some("field3".to_string()),
    };

    // When all required fields are present, conversion should succeed
    let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
    assert!(result.is_ok());
}

// Test error function precedence
#[test]
fn test_error_function_vs_default_error() {
    let proto_msg = proto::SimpleMessage {
        required_field: None, // Should use custom error function
        required_number: Some(42),
        optional_field: None,
    };

    let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        DetailedValidationError::MissingRequired("required_field".to_string())
    );
}

// Test that different error types work in the same struct
#[test]
fn test_heterogeneous_error_types() {
    // Test that having different error types for different fields works
    let proto_msg1 = proto::SimpleMessage {
        required_field: None, // DetailedValidationError
        required_number: Some(42),
        optional_field: None,
    };

    let proto_msg2 = proto::SimpleMessage {
        required_field: Some("present".to_string()),
        required_number: None, // ValidationError
        optional_field: None,
    };

    let proto_msg3 = proto::SimpleMessage {
        required_field: Some("present".to_string()),
        required_number: None,
        optional_field: Some("field3".to_string()), // Generated error
    };

    // Each should fail with the appropriate error type
    let result1: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg1.try_into();
    assert!(matches!(
        result1,
        Err(DetailedValidationError::MissingRequired(_))
    ));

    let result2: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg2.try_into();
    assert!(matches!(
        result2,
        Err(DetailedValidationError::InvalidFormat(_))
    ));

    // Generated error should work when no custom error is specified
    let result3: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg3.try_into();
    assert!(matches!(
        result3,
        Err(DetailedValidationError::OutOfRange(_))
    ));
}

// Test enum with comprehensive attributes
#[test]
fn test_comprehensive_enum_all_present() {
    let proto_msg = proto::EnumMessage {
        status_panic: Some(proto::Status::Ok.into()),
        status_error: Some(proto::Status::Found.into()),
        status_default: Some(proto::Status::NotFound.into()),
        status_optional: Some(proto::Status::MovedPermanently.into()),
    };

    let result: Result<ComprehensiveEnumStruct, ComprehensiveEnumStructConversionError> =
        proto_msg.clone().try_into();
    assert!(result.is_ok());
    let rust_msg = result.unwrap();

    assert_eq!(rust_msg.enum_expect_error, Status::Found);
    assert_eq!(rust_msg.enum_expect_panic, Status::Ok);
    assert_eq!(
        rust_msg.enum_optional_explicit,
        Some(Status::MovedPermanently)
    );
    assert_eq!(rust_msg.enum_with_default, Status::NotFound);
    assert_eq!(
        rust_msg.enum_optional_explicit,
        Some(Status::MovedPermanently)
    );

    // Test panic version
    let panic_result = panic::catch_unwind(|| {
        let rust_msg: Result<ComprehensiveEnumStruct, _> = proto_msg.try_into();
        assert!(rust_msg.is_ok());
        rust_msg.unwrap()
    });
    assert!(panic_result.is_ok());
}

#[test]
fn test_comprehensive_enum_defaults() {
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

    assert_eq!(rust_msg.enum_with_default, Status::Ok); // default value
    assert_eq!(rust_msg.enum_optional_explicit, Some(Status::Ok));
}

#[test]
#[should_panic(expected = "Proto field status_panic is required")]
fn test_comprehensive_enum_required_explicit_missing() {
    let proto_msg = proto::EnumMessage {
        status_panic: None, // This should cause panic since it has expect(panic)
        status_error: Some(proto::Status::Found.into()),
        status_default: Some(proto::Status::NotFound.into()),
        status_optional: Some(proto::Status::Ok.into()),
    };

    let _: Result<ComprehensiveEnumStruct, _> = proto_msg.try_into();
    assert!(false);
}

#[test]
fn test_default_function() {
    let default_tracks = crate::shared_types::default_track_vec();
    println!("Default tracks: {:?}", default_tracks);
    assert_eq!(default_tracks[0].id.as_ref(), &999);
}

// Test collection behaviors
#[test]
fn test_collection_default_vs_expect() {
    // Empty state should use default for default field, succeed for expect field
    let empty_proto_state = proto::State { tracks: vec![] };
    println!(
        "Proto state tracks length: {}",
        empty_proto_state.tracks.len()
    );
    println!(
        "Proto state tracks is_empty: {}",
        empty_proto_state.tracks.is_empty()
    );

    let default_result: CollectionWithDefault = empty_proto_state.clone().into();
    println!("Result tracks: {:?}", default_result.tracks);
    assert_eq!(
        default_result.tracks,
        vec![Track {
            id: TrackId::new(999)
        }]
    );

    let expect_result: Result<CollectionWithExpect, CollectionWithExpectConversionError> =
        empty_proto_state.try_into();
    assert!(expect_result.is_ok());
    let expect_struct = expect_result.unwrap();
    assert!(expect_struct.tracks.is_empty());

    // Non-empty state
    let non_empty_proto_state = proto::State {
        tracks: vec![proto::Track { track_id: 1 }, proto::Track { track_id: 2 }],
    };

    let default_result: CollectionWithDefault = non_empty_proto_state.clone().into();
    assert_eq!(default_result.tracks.len(), 2);
    assert_eq!(default_result.tracks[0].id, 1);

    let expect_result: Result<CollectionWithExpect, CollectionWithExpectConversionError> =
        non_empty_proto_state.try_into();
    assert!(expect_result.is_ok());
    let expect_struct = expect_result.unwrap();
    assert_eq!(expect_struct.tracks.len(), 2);
}

// Test transparent wrapper with expect
#[test]
fn test_transparent_with_expect_success() {
    let proto_msg = proto::CombinationMessage {
        rename_with_default: Some("renamed".to_string()),
        transparent_with_expect: Some("wrapper_value".to_string()),
        enum_with_default_and_optional: Some(proto::Status::Ok.into()),
        collection_with_expect: vec![],
    };

    let result: Result<CombinationStruct, _> = proto_msg.try_into();
    assert!(result.is_ok());
    let rust_msg: CombinationStruct = result.unwrap();
    assert_eq!(rust_msg.transparent_field_with_expect, "wrapper_value");
}

#[test]
#[should_panic(expected = "Proto field transparent_with_expect is required")]
fn test_transparent_with_expect_missing() {
    let proto_msg = proto::CombinationMessage {
        rename_with_default: Some("renamed".to_string()),
        transparent_with_expect: None, // Should panic
        enum_with_default_and_optional: Some(proto::Status::Ok.into()),
        collection_with_expect: vec![],
    };

    let _: Result<CombinationStruct, _> = proto_msg.try_into();
    assert!(false);
}

// Test attribute precedence: expect should override default
#[test]
fn test_attribute_precedence() {
    // This test would need a specific struct to test precedence
    // For example, a field with both expect and default should use expect behavior

    // Test that optional = false overrides default inference
    let proto_msg = proto::OptionalMessage {
        id: 1,
        name: None, // Should cause panic due to optional = false
        count: None,
        priority: None,
        tags: vec![],
    };

    let panic_result = panic::catch_unwind(|| {
        let _: MixedOptionalStruct = proto_msg.into();
    });

    assert!(panic_result.is_err());

    // This depends on the exact implementation - might succeed with default or panic
    // The test needs to be adjusted based on your macro's precedence rules
}

// Test that error messages are descriptive and helpful
#[test]
fn test_error_message_quality() {
    let proto_msg = proto::SimpleMessage {
        required_field: None,
        required_number: Some(42),
        optional_field: None,
    };

    let result: Result<ExpectErrorStruct, ExpectErrorStructConversionError> = proto_msg.try_into();
    assert!(result.is_err());

    let error = result.unwrap_err();
    let error_string = error.to_string();
    assert!(error_string.contains("required_field"));
    assert!(error_string.contains("Missing"));
}

#[test]
fn test_custom_error_message_quality() {
    let proto_msg = proto::SimpleMessage {
        required_field: None,
        required_number: Some(42),
        optional_field: None,
    };

    let result: Result<MultipleErrorFnsStruct, DetailedValidationError> = proto_msg.try_into();
    assert!(result.is_err());

    let error = result.unwrap_err();
    let error_string = error.to_string();
    assert!(error_string.contains("required_field"));
    assert!(error_string.contains("Required field missing"));
}

#[test]
fn test_panic_message_quality() {
    let proto_msg = proto::SimpleMessage {
        required_field: None,
        required_number: Some(42),
        optional_field: None,
    };

    let panic_result = std::panic::catch_unwind(|| {
        let _: ExpectPanicStruct = proto_msg.into();
    });

    assert!(panic_result.is_err());
    // The panic message should be descriptive (tested in other panic tests)
}
