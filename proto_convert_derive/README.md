# proto_convert_derive

Automatically derive conversions between Protobuf-compiled prost types and your native Rust types.

## Overview

`proto_convert_derive` is a procedural macro for bidirectional conversions between Protobuf-generated types (`prost`) and Rust structs. This reduces boilerplate and handles proto3's lack of `required` fields (which result in `Option` and lots `.expect` or `if let Some` in your code. This macro simply `.expect`s types.

### Key Features

- Automatically implements `From<Proto>` for Rust types and vice versa.
- Direct mapping for primitive types.
- Unwraps optional fields with `.expect`.
- Supports newtype wrappers.
- Customizable Protobuf module (default is `proto` via `#[proto(module = "your_module")]`).


## Usage

Define your protobuf messages.

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
    uint64 id = 1;
}
```

Now you might have a use case, where you can take some of the prost-generated
types directly. For others you want to convert the prost types into your own. It
may be that you need to implement `PartialEq` yourself, or you have a more
complex type where you only want to take over parts of the prost type over.

```rust
use proto_convert_derive::ProtoConvert;
mod proto {
    tonic::include_proto!("service");
}

#[derive(ProtoConvert)]
pub struct Request {
    pub header: proto::Header, // here we take the prost type directly
    pub payload: String,
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
#[proto(module = "proto")]
pub struct Track {
    #[proto(transparent)]
    id: TrackId, // newtype
}

#[derive(ProtoConvert, PartialEq, Debug, Clone)]
pub struct TrackId(u64);
```
