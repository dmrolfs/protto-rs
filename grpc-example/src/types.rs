use crate::proto;
use proto_convert_derive::ProtoConvert;

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

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
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
