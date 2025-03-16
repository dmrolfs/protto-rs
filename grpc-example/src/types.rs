use crate::proto;
use proto_convert_derive::ProtoConvert;
use std::{collections::HashMap, sync::atomic::AtomicU64};

// Overwrite the prost Request type.
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Request {
    // Here we take the prost Header type instaed
    pub header: proto::Header,
    pub payload: String,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Track {
    #[proto(transparent, rename = "track_id")]
    id: TrackId,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone, Hash, Eq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct TrackId(u64);

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct State {
    pub tracks: Vec<Track>, // we support collections as well!
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(rename = "State")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ProtoState {
    pub tracks: Vec<proto::Track>, // we support collections as well!
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct HasOptional {
    pub track: Option<Track>,
}

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

#[derive(PartialEq, Debug, Clone)]
pub struct LaunchId(u64);

#[derive(ProtoConvert, Debug)]
#[proto(rename = "State")]
pub struct ComplexState {
    pub tracks: Vec<Track>,
    #[proto(ignore)]
    pub launches: HashMap<TrackId, LaunchId>,
    #[proto(ignore)]
    pub counter: AtomicU64,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Status {
    MovedPermanently,
    Ok,
    Found,
    NotFound,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum AnotherStatus {
    Ok,
    MovedPermanently,
    Found,
    NotFound,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct StatusResponse {
    pub status: Status,
    pub message: String,
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip_track(proto_track in any::<proto::Track>()) {
            let rust_track: Track = proto_track.into();
            let back_to_proto: proto::Track = rust_track.into();
            assert_eq!(proto_track, back_to_proto);
        }

        #[test]
        fn roundtrip_state(proto_state in any::<proto::State>()) {
            let rust_state: State = proto_state.clone().into();
            let back_to_proto: proto::State = rust_state.into();
            assert_eq!(proto_state, back_to_proto);
        }

        #[test]
        fn roundtrip_proto_state(proto_state in any::<proto::State>()) {
            let rust_state: ProtoState = proto_state.clone().into();
            let back_to_proto: proto::State = rust_state.into();
            assert_eq!(proto_state, back_to_proto);
        }

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

        #[test]
          fn roundtrip_request(proto_request in any::<proto::Request>().prop_filter(
              "Header must not be None",
              |req| req.header.is_some()
          )) {
              let rust_request: Request = proto_request.clone().into();
              let back_to_proto: proto::Request = rust_request.into();
              assert_eq!(proto_request, back_to_proto);
          }

        #[test]
        fn roundtrip_has_optional(proto_has_optional in any::<proto::HasOptional>()) {
            let rust_has_optional: HasOptional = proto_has_optional.into();
            let back_to_proto: proto::HasOptional = rust_has_optional.into();
            assert_eq!(proto_has_optional, back_to_proto);
        }

        #[test]
        fn roundtrip_header(proto_header in any::<proto::Header>()) {
            let rust_header: proto::Header = proto_header.clone();
            let back_to_proto: proto::Header = rust_header.clone();
            assert_eq!(proto_header, back_to_proto);
        }

        #[test]
        fn roundtrip_status(status in any::<Status>()) {
            let proto_status: proto::Status = status.clone().into();
            let back_to_rust: Status = proto_status.into();
            assert_eq!(status, back_to_rust);
        }

        #[test]
        fn roundtrip_status_response(status in any::<StatusResponse>()) {
            let proto_status: proto::StatusResponse = status.clone().into();
            let back_to_rust: StatusResponse = proto_status.into();
            assert_eq!(status, back_to_rust);
        }
    }

    #[test]
    fn test_status() {
        let mappings = vec![
            (proto::Status::Ok, Status::Ok),
            (proto::Status::MovedPermanently, Status::MovedPermanently),
            (proto::Status::Found, Status::Found),
            (proto::Status::NotFound, Status::NotFound),
        ];

        // Proto -> rust.
        for (proto_status, rust_status) in &mappings {
            let converted: Status = (*proto_status).into();
            assert_eq!(converted, *rust_status);
        }

        // Rust -> proto.
        for (proto_status, rust_status) in &mappings {
            let converted: proto::Status = (*rust_status).clone().into();
            assert_eq!(converted, *proto_status);
        }
    }

    // Edge case: Ensure None stays None
    #[test]
    fn test_has_optional_none() {
        let proto_msg = proto::HasOptional { track: None };
        let rust_msg: HasOptional = proto_msg.into();
        assert_eq!(rust_msg.track, None);

        let back_to_proto: proto::HasOptional = rust_msg.into();
        assert_eq!(back_to_proto.track, None);
    }

    // Edge case: Ensure empty State roundtrips correctly
    #[test]
    fn test_empty_state() {
        let proto_state = proto::State { tracks: vec![] };
        let rust_state: State = proto_state.clone().into();
        assert_eq!(rust_state.tracks, vec![]);

        let back_to_proto: proto::State = rust_state.into();
        assert_eq!(back_to_proto.tracks, vec![]);
    }
}
