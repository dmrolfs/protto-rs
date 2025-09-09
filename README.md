# protto

[![crates.io](https://img.shields.io/crates/v/protto.svg)](https://crates.io/crates/protto)
[![docs.rs](https://docs.rs/protto/badge.svg)](https://docs.rs/protto)

`protto` is a procedural macro for deriving **bidirectional conversions** between `prost`-generated Protobuf types and Rust structs. It dramatically reduces boilerplate when working with Protobufs in Rust.

This crate is derived from [`proto_convert_derive`](https://github.com/protortyp/proto_convert_derive) by **Christian Engel <cascade.nab0p@icloud.com>**.

---

## Features

- Automatic `From<Proto>` / `Into<Proto>` conversions
- Support for Rust primitive types (`u32`, `i64`, `String`, etc.)
- Optional fields (`Option<T>`) and collections (`Vec<T>`)
- Transparent newtype wrappers
- Field renaming via `#[protto(proto_name = "...")]`
- Custom conversion functions (`proto_to_rust_fn`, `rust_to_proto_fn`)
- Ignored fields with `#[protto(ignore)]`
- Smart optionality detection
- Configurable Protobuf module path
- Advanced error handling strategies

---

## Installation

```toml
[dependencies]
protto = "0.2"
```

## Quick Start

Protobuf definitions:
```proto
syntax = "proto3";
package service;

message Track {
    uint64 track_id = 1;
}

message State {
    repeated Track tracks = 1;
}
```

Rust usage:
```rust
use protto::Protto;

mod proto {
    tonic::include_proto!("service");
}

#[derive(Protto)]
#[protto(module = "proto")]
pub struct Track {
    #[protto(transparent, proto_name = "track_id")]
    pub id: TrackId,
}

#[derive(Protto)]
pub struct TrackId(u64);

#[derive(Protto)]
pub struct State {
    pub tracks: Vec<Track>,
}
```

## More Information

- Advanced usage, attribute reference, and examples are documented in Rustdoc: [docs.rs/protto](https://docs.rs/protto)
- The macro is re-exported for convenience:
```rust
pub use protto_derive::*;
```

## Contributing

Contributions, bug reports, and feature requests are welcome!  
Please follow the guidelines in [CONTRIBUTING.md](CONTRIBUTING.md) for the recommended workflow and standards.


## Attribution
`protto` builds upon the `proto_convert_derive` crate created by:

[Christian Engel](mailto:cascade.nab0p@icloud.com)

This crate extends the original functionality with additional features, improved ergonomics, and comprehensive error handling.
