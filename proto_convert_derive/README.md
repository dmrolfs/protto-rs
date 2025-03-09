# proto_convert_derive

Automatically derive conversions between Protobuf-compiled prost types and your native Rust types.

## Overview

`proto_convert_derive` is a procedural macro for bidirectional conversions between Protobuf-generated types (`prost`) and Rust structs. This reduces boilerplate and handles proto3's lack of `required` fields (which result in `Option` and lots `.expect` or `if let Some` in your code. This macro simply `.expect`s types.

### Key Features

- Automatically implements `From<Proto>` for Rust types and vice versa.
- Supports collections like `Vec<Proto>`
- Direct mapping for primitive types.
- Unwraps optional fields with `.expect`.
- Supports newtype wrappers.
- Customizable Protobuf module (default is `proto` via `#[proto(module = "your_module")]`).


## Usage

Define your protobuf messages:

```protobuf
syntax = "proto3";
package service;

message Header {
    string request_id = 1;
    int64 timestamp = 2;
}

message Request {
    Header header = 1;
    string payload = 2;
}

message Track {
    uint64 track_id = 1;
}

message State {
    repeated Track tracks = 1;
}

message HasOptional {
    optional Track track = 1;
}
```

In some cases, you might want to use the `prost`-generated types directly.
However, for more complex scenarios, you may need to convert them into your own
Rust types. This could be necessary if you need to implement PartialEq manually
or if you want to selectively integrate parts of a prost type into your custom
type:

```rust
use proto_convert_derive::ProtoConvert;
mod proto {
    tonic::include_proto!("service");
}

// Overwrite the prost Request type.
#[derive(ProtoConvert, PartialEq, Debug, Clone)]
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

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
pub struct State {
    pub tracks: Vec<Track>, // we support collections as well!
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
pub struct HasOptional {
    pub track: Option<Track>,
}
```
