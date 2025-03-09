use crate::proto;
use proto_convert_derive::ProtoConvert;

// Overwrite the prost Request type.
#[derive(ProtoConvert)]
pub struct Request {
    // Here we take the prost Header type instaed
    pub header: proto::Header,
    pub payload: String,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto")]
pub struct Track {
    #[proto(transparent, rename = "track_id")]
    id: TrackId,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
pub struct TrackId(u64);

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let original_proto = proto::Track { track_id: 999 };
        let rust_track: Track = original_proto.clone().into();
        let back_to_proto: proto::Track = rust_track.into();
        assert_eq!(original_proto.track_id, back_to_proto.track_id);
    }

    // Mock Protobuf types for testing
    mod proto {
        #[derive(Clone, PartialEq, prost::Message)]
        pub struct TestMessage {
            #[prost(string, tag = "1")]
            pub name: String,
            #[prost(uint64, tag = "2")]
            pub id: u64,
            #[prost(message, tag = "3")]
            pub nested: Option<Nested>,
        }

        #[derive(Clone, PartialEq, prost::Message)]
        pub struct Nested {
            #[prost(int32, tag = "1")]
            pub value: i32,
        }

        #[derive(Clone, PartialEq, prost::Message)]
        pub struct Track {
            #[prost(uint64, tag = "1")]
            pub track_id: u64,
        }
    }

    // Test struct with primitive and nested proto types
    #[derive(ProtoConvert, Clone, PartialEq, Debug)]
    #[proto(module = "proto")]
    struct TestMessage {
        name: String,
        id: u64,
        nested: proto::Nested,
    }

    // Test struct with transparent and renamed field
    #[derive(ProtoConvert, Clone, PartialEq, Debug)]
    #[proto(module = "proto")]
    struct Track {
        #[proto(transparent, rename = "track_id")]
        id: TrackId,
    }

    // Test newtype with transparent conversion
    #[derive(ProtoConvert, Clone, PartialEq, Debug)]
    struct TrackId(u64);

    #[test]
    fn test_from_proto_to_rust() {
        let proto_msg = proto::TestMessage {
            name: "test".to_string(),
            id: 42,
            nested: Some(proto::Nested { value: 123 }),
        };
        let rust_msg = TestMessage::from(proto_msg.clone());
        assert_eq!(
            rust_msg,
            TestMessage {
                name: "test".to_string(),
                id: 42,
                nested: proto::Nested { value: 123 },
            }
        );
    }

    #[test]
    fn test_from_rust_to_proto() {
        let rust_msg = TestMessage {
            name: "test".to_string(),
            id: 42,
            nested: proto::Nested { value: 123 },
        };
        let proto_msg = proto::TestMessage::from(rust_msg.clone());
        assert_eq!(
            proto_msg,
            proto::TestMessage {
                name: "test".to_string(),
                id: 42,
                nested: Some(proto::Nested { value: 123 }),
            }
        );
    }

    #[test]
    fn test_transparent_and_rename() {
        let proto_track = proto::Track { track_id: 456 };
        let rust_track = Track::from(proto_track.clone());
        assert_eq!(rust_track, Track { id: TrackId(456) });

        let rust_track = Track { id: TrackId(789) };
        let proto_track = proto::Track::from(rust_track.clone());
        assert_eq!(proto_track, proto::Track { track_id: 789 });
    }

    #[test]
    #[should_panic(expected = "no nested in proto")]
    fn test_missing_optional_field() {
        let proto_msg = proto::TestMessage {
            name: "test".to_string(),
            id: 42,
            nested: None,
        };
        let _ = TestMessage::from(proto_msg);
    }
}
