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