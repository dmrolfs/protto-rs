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
