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
    #[proto(transparent)]
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
        let original_proto = proto::Track { id: 999 };
        let rust_track: Track = original_proto.clone().into();
        let back_to_proto: proto::Track = rust_track.into();
        assert_eq!(original_proto.id, back_to_proto.id);
    }
}
