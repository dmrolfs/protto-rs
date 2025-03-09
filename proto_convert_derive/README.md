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
- Ignore individual fields `#[proto(ignore)]`

## Usage

Head to the [examples](https://github.com/protortyp/proto_convert_derive/blob/develop/grpc-example/src/types.rs).
