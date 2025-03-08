use crate::proto;
use proto_convert_derive::ProtoConvert;

// Overwrite the prost Request type.
#[derive(ProtoConvert)]
pub struct Request {
    // Here we take the prost Header type instaed
    pub header: proto::Header,
    #[proto(transparent)]
    pub payload: String,
}

pub struct Payload {
    data: String,
}

#[derive(ProtoConvert)]
pub struct State {
    pub key: proto::Key,
}
