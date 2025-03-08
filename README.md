# proto_convert_derive

Automatically derive conversions between Protobuf-compiled prost types and your native Rust types.

## Overview

`proto_convert_derive` is a procedural macro for bidirectional conversions between Protobuf-generated types (`prost`) and Rust structs. This reduces boilerplate and handles proto3's lack of `required` fields (which result in `Option` and lots `.expect` or `if let Some` in your code. This macro simply `.expect`s types.

### Key Features
- Implements `From<Proto>` for Rust types and vice versa.
- Maps primitive types directly.
- Unwraps optional fields with `.expect` for message types.
- Defaults to `proto` module for Protobuf types, customizable with `#[proto_module = "your_module"]`.

## Usage

```rust
use proto_convert_derive::ProtoConvert;

mod myproto { tonic::include_proto!("state"); }

#[derive(ProtoConvert)]
#[proto_module = "myproto"]
struct Key { pub id: String }

#[derive(ProtoConvert)]
#[proto_module = "myproto"]
struct State { pub key: Key }

fn main() {
    let proto_key = myproto::Key {
        id: Some(myproto::Id {
            id: "my id".to_string(),
        }),
    };
    let my_key: Key = proto_key.into();

    // Conversion from native Rust type to Protobuf:
    let my_state = State { key: my_key };
    let proto_state: myproto::State = my_state.into();
}
```
