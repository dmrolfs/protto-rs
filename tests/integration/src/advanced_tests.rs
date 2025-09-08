use crate::complex_types::*;
use crate::proto;
use crate::shared_types::*;
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    #[test]
    fn roundtrip_map_state(proto_state in any::<proto::State>()) {
        let rust_state: MapState = proto_state.clone().into();
        let back_to_proto: proto::State = rust_state.into();
        let mut original_tracks: Vec<_> = proto_state.tracks.clone();
        original_tracks.sort_by_key(|track| track.track_id);
        let mut converted_tracks: Vec<_> = back_to_proto.tracks.clone();
        converted_tracks.sort_by_key(|track| track.track_id);
        assert_eq!(original_tracks, converted_tracks);
    }

    #[test]
    fn roundtrip_complex_state(proto_state in any::<proto::State>()) {
        let rust_state: ComplexState = proto_state.clone().into();
        assert_eq!(rust_state.launches, HashMap::new());
        assert_eq!(rust_state.counter.load(std::sync::atomic::Ordering::Relaxed), 0);

        let back_to_proto: proto::State = rust_state.into();
        assert_eq!(proto_state.tracks, back_to_proto.tracks); // Only tracks should match

        let rust_state_again: ComplexState = back_to_proto.into();
        assert_eq!(rust_state_again.launches, HashMap::new());
        assert_eq!(rust_state_again.counter.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
}

#[test]
fn test_complex_conversions_example() {
    let proto_state = proto::State {
        tracks: vec![proto::Track { track_id: 1 }, proto::Track { track_id: 2 }],
    };

    let map_state: MapState = proto_state.clone().into();
    assert_eq!(map_state.tracks.len(), 2);
    assert!(map_state.tracks.contains_key(&TrackId::new(1)));
    assert!(map_state.tracks.contains_key(&TrackId::new(2)));

    let back_to_proto: proto::State = map_state.into();
    // Order might be different, so sort before comparison
    let mut original_tracks = proto_state.tracks.clone();
    original_tracks.sort_by_key(|t| t.track_id);
    let mut converted_tracks = back_to_proto.tracks;
    converted_tracks.sort_by_key(|t| t.track_id);
    assert_eq!(original_tracks, converted_tracks);
}

#[cfg(test)]
mod custom_type_tests {
    use super::*;
    use crate::basic_types::CustomTypeStruct;

    #[test]
    fn test_custom_type_field_conversion() {
        // DMR: This test will expose the .-into() typo if it exists
        let proto_msg = proto::CustomTypeMessage {
            track: Some(proto::Track { track_id: 42 }),
            track_id: Some(123),
            wrapper: Some("test".to_string()),
        };

        // DMR: This conversion should trigger generate_custom_type_field()
        let rust_struct: CustomTypeStruct = proto_msg.into();

        assert_eq!(rust_struct.track.id.into_inner(), 42);
        assert_eq!(rust_struct.track_id.into_inner(), 123);
        assert_eq!(rust_struct.wrapper.as_str(), "test");
    }

    #[test]
    fn test_custom_type_roundtrip() {
        // DMR: Test the reverse conversion (generate_custom_type_from_my_field)
        let rust_struct = CustomTypeStruct {
            track: Track { id: TrackId::new(99) },
            track_id: TrackId::new(88),
            wrapper: TransparentWrapper::new("roundtrip"),
        };

        let proto_msg: proto::CustomTypeMessage = rust_struct.clone().into();
        let back_to_rust: CustomTypeStruct = proto_msg.into();

        assert_eq!(rust_struct, back_to_rust);
    }

    proptest! {
        #[test]
        fn roundtrip_custom_type_struct(original in any::<CustomTypeStruct>()) {
            // DMR: Property-based test to catch edge cases in custom type conversion
            let proto_msg: proto::CustomTypeMessage = original.clone().into();
            let converted_back: CustomTypeStruct = proto_msg.into();
            prop_assert_eq!(original, converted_back);
        }
    }
}

#[cfg(test)]
mod comprehensive_tests {
    use crate::basic_types::Status;
    use super::*;

    #[test]
    fn test_bidirectional_conversion() {
        // DMR: Test DeriveBidirectional strategy
        let proto_msg = proto::BidirectionalMessage {
            custom_field: Some(proto::ComplexType {
                name: "test".to_string(),
                id: 123,
            }),
        };

        let rust_struct: BidirectionalConversionStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.custom_field.inner, "test");
        assert_eq!(rust_struct.custom_field.value, 123);

        let back_to_proto: proto::BidirectionalMessage = rust_struct.into();
        let proto_custom_field = back_to_proto.custom_field.unwrap();
        assert_eq!(proto_custom_field.name, "test");
        assert_eq!(proto_custom_field.id, 123);
    }

    #[test]
    fn test_transparent_required() {
        // DMR: Test TransparentRequired strategy
        let proto_msg = proto::TransparentMessage {
            wrapper_id: "42".to_string(),
        };

        let rust_struct: TransparentRequiredStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.id.as_str(), "42");

        let back_to_proto: proto::TransparentMessage = rust_struct.into();
        assert_eq!(back_to_proto.wrapper_id, "42");
    }

    #[test]
    fn test_transparent_optional_strategies() {
        // DMR: Test TransparentOptionalWith* strategies
        let proto_msg = proto::TransparentOptionalMessage {
            panic_wrapper: Some("10".to_string()),
            error_wrapper: Some("20".to_string()),
            default_wrapper: None, // This should use default
        };

        let rust_struct: TransparentOptionalStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.panic_wrapper.as_str(), "10");
        assert_eq!(rust_struct.error_wrapper.as_str(), "20");
        assert_eq!(rust_struct.default_wrapper.as_str(), "42"); // default value
    }

    #[test]
    fn test_wrap_in_some() {
        // DMR: Test WrapInSome strategy (rust required -> proto optional)
        let rust_struct = WrapInSomeStruct {
            required_rust_field: "test".to_string(),
            status: Status::Ok,
        };

        let proto_msg: proto::WrapInSomeMessage = rust_struct.into();
        assert_eq!(proto_msg.wrapped_field, Some("test".to_string()));
        assert!(proto_msg.wrapped_status.is_some());
    }

    #[test]
    fn test_map_option() {
        // DMR: Test MapOption strategy (both sides optional, no expect/default)
        let proto_msg = proto::MapOptionMessage {
            simple_option: Some("test".to_string()),
            optional_status: Some(1), // Status::MovedPermanently as i32
        };

        let rust_struct: MapOptionStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.optional_string, Some("test".to_string()));
        assert_eq!(rust_struct.optional_status, Some(Status::MovedPermanently));

        // Test None case
        let proto_none = proto::MapOptionMessage {
            simple_option: None,
            optional_status: None,
        };

        let rust_none: MapOptionStruct = proto_none.try_into().unwrap();
        assert_eq!(rust_none.optional_string, None);
        assert_eq!(rust_none.optional_status, None);
    }

    #[test]
    fn test_map_vec_in_option() {
        // DMR: Test MapVecInOption strategy
        let track = proto::Track { track_id: 1 };
        let proto_msg = proto::VecOptionMessage {
            optional_tracks: vec![track.clone()],
            optional_strings: vec!["test".to_string()],
            optional_proto_tracks: vec![track],
        };

        let rust_struct: VecOptionStruct = proto_msg.try_into().unwrap();
        assert!(rust_struct.optional_tracks.is_some());
        assert_eq!(rust_struct.optional_tracks.unwrap().len(), 1);
        assert!(rust_struct.optional_strings.is_some());
        assert_eq!(rust_struct.optional_strings.unwrap().len(), 1);

        // Test empty vec case (should be None in Rust)
        let proto_empty = proto::VecOptionMessage {
            optional_tracks: vec![],
            optional_strings: vec![],
            optional_proto_tracks: vec![],
        };

        let rust_empty: VecOptionStruct = proto_empty.try_into().unwrap();
        assert!(rust_empty.optional_tracks.is_none() || rust_empty.optional_tracks == Some(vec![]));
    }

    #[test]
    fn test_vec_direct_assignment() {
        // DMR: Test VecDirectAssignment strategy
        let track = proto::Track { track_id: 1 };
        let header = proto::Header {
            request_id: "test".to_string(),
            timestamp: 123
        };

        let proto_msg = proto::DirectVecMessage {
            proto_tracks: vec![track],
            proto_headers: vec![header],
        };

        let rust_struct: VecDirectAssignmentStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.proto_tracks.len(), 1);
        assert_eq!(rust_struct.proto_headers.len(), 1);
        assert_eq!(rust_struct.proto_tracks[0].track_id, 1);
    }

    #[test]
    fn test_vec_with_error_success() {
        // DMR: Test CollectVecWithError strategy - success case
        let track = proto::Track { track_id: 1 };
        let proto_msg = proto::VecErrorMessage {
            tracks_with_error: vec![track],
            tags_with_error: vec!["tag1".to_string()],
        };

        let rust_struct: VecWithErrorStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.tracks_with_error.len(), 1);
        assert_eq!(rust_struct.tags_with_error.len(), 1);
    }

    #[test]
    fn test_vec_with_error_default() {
        // DMR: Test CollectVecWithError strategy - default case
        let proto_msg = proto::VecErrorMessage {
            tracks_with_error: vec![], // Empty, should use default
            tags_with_error: vec![],   // Empty, should use default
        };

        let rust_struct: VecWithErrorStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.tracks_with_error.len(), 1); // default_track_vec returns 1 item
        assert_eq!(rust_struct.tags_with_error.len(), 1);   // default_string_vec returns 1 item
    }

    #[test]
    fn test_direct_with_into() {
        // DMR: Test DirectWithInto strategy
        let proto_msg = proto::DirectConversionMessage {
            status_field: 0, // Status::Ok as i32
            track_field: Some(proto::Track { track_id: 42 }),
            track_id: 123,
        };

        let rust_struct: DirectWithIntoStruct = proto_msg.try_into().unwrap();
        assert_eq!(rust_struct.status_field, Status::Ok);
        assert_eq!(rust_struct.track_field.id.as_ref(), &42);
        assert_eq!(rust_struct.track_id.as_ref(), &123);
    }

    #[test]
    fn test_rust_to_proto_strategies() {
        // DMR: Test rust->proto specific strategies
        let rust_struct = RustToProtoStruct {
            rust_required_field: "test".to_string(),
            rust_optional_field: Some("optional".to_string()),
            transparent_required: TrackId::new(42),
            transparent_optional: TrackId::new(99),
        };

        let proto_msg: proto::RustToProtoMessage = rust_struct.into();
        assert_eq!(proto_msg.required_to_optional, "test".to_string());
        assert_eq!(proto_msg.optional_to_required, Some("optional".to_string())); // UnwrapOptional
        assert_eq!(proto_msg.transparent_to_required, 42); // TransparentToRequired
        assert_eq!(proto_msg.transparent_to_optional, Some(99)); // TransparentToOptional
    }

    proptest! {
        #[test]
        fn prop_bidirectional_roundtrip(
            name in ".*",
            id in any::<u64>()
        ) {
            let original = BidirectionalConversionStruct {
                custom_field: CustomComplexType {
                    inner: name.clone(),
                    value: id,
                },
            };

            let proto: proto::BidirectionalMessage = original.clone().into();
            let roundtrip: BidirectionalConversionStruct = proto.try_into().unwrap();

            prop_assert_eq!(original, roundtrip);
        }

        #[test]
        fn prop_transparent_roundtrip(value in any::<String>()) {
            let original = TransparentRequiredStruct {
                id: TransparentWrapper::new(value),
            };

            let proto: proto::TransparentMessage = original.clone().into();
            let roundtrip: TransparentRequiredStruct = proto.try_into().unwrap();

            prop_assert_eq!(original, roundtrip);
        }

        #[test]
        fn prop_wrap_in_some_roundtrip(
            field in ".*",
            status in prop_oneof![
                Just(Status::MovedPermanently),
                Just(Status::Ok),
                Just(Status::Found),
                Just(Status::NotFound)
            ]
        ) {
            let original = WrapInSomeStruct {
                required_rust_field: field,
                status,
            };

            let proto: proto::WrapInSomeMessage = original.clone().into();
            let roundtrip: WrapInSomeStruct = proto.try_into().unwrap();

            prop_assert_eq!(original, roundtrip);
        }

        #[test]
        fn prop_map_option_roundtrip(
            opt_string in prop::option::of(".*"),
            opt_status in prop::option::of(prop_oneof![
                Just(Status::MovedPermanently),
                Just(Status::Ok),
                Just(Status::Found),
                Just(Status::NotFound)
            ])
        ) {
            let original = MapOptionStruct {
                optional_string: opt_string.clone(),
                optional_status: opt_status,
            };

            let proto: proto::MapOptionMessage = original.clone().into();
            let roundtrip: MapOptionStruct = proto.try_into().unwrap();

            prop_assert_eq!(original, roundtrip);
        }

        #[test]
        fn prop_vec_option_roundtrip(
            tracks_len in 0..5usize,
            strings_len in 0..3usize
        ) {
            let tracks = if tracks_len == 0 {
                None
            } else {
                Some((0..tracks_len).map(|i| Track { id: TrackId::new(i as u64) }).collect())
            };
            let strings = if strings_len == 0 {
                None
            } else {
                Some((0..strings_len).map(|i| format!("str{}", i)).collect())
            };

            let original = VecOptionStruct {
                optional_tracks: tracks,
                optional_strings: strings,
                optional_proto_tracks: None, // Simplified for prop test
            };

            let proto: proto::VecOptionMessage = original.clone().into();
            let roundtrip: VecOptionStruct = proto.try_into().unwrap();

            prop_assert_eq!(original.optional_strings, roundtrip.optional_strings);
            // Track comparison requires careful handling of the Track -> proto::Track -> Track conversion
            prop_assert_eq!(
                original.optional_tracks.as_ref().map(|v| v.len()),
                roundtrip.optional_tracks.as_ref().map(|v| v.len())
            );
        }

        #[test]
        fn prop_direct_with_into_roundtrip(
            status in prop_oneof![
                Just(Status::MovedPermanently),
                Just(Status::Ok),
                Just(Status::Found),
                Just(Status::NotFound)
            ],
            track_id in any::<u64>(),
            wrapper_val in any::<u64>()
        ) {
            let original = DirectWithIntoStruct {
                status_field: status,
                track_field: Track { id: TrackId::new(track_id) },
                track_id: TrackId::new(wrapper_val),
            };

            let proto: proto::DirectConversionMessage = original.clone().into();
            let roundtrip: DirectWithIntoStruct = proto.try_into().unwrap();

            prop_assert_eq!(original, roundtrip);
        }

        #[test]
        fn prop_rust_to_proto_strategies(
            required_field in ".*",
            optional_field in prop::option::of(".*"),
            transparent_req in any::<u64>(),
            transparent_opt in any::<u64>()
        ) {
            let original = RustToProtoStruct {
                rust_required_field: required_field.clone(),
                rust_optional_field: optional_field.clone(),
                transparent_required: TrackId::new(transparent_req),
                transparent_optional: TrackId::new(transparent_opt),
            };

            let proto: proto::RustToProtoMessage = original.clone().into();
            let roundtrip: RustToProtoStruct = proto.try_into().unwrap();

            prop_assert_eq!(original, roundtrip);
        }
    }
}
