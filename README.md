# protto

[![crates.io](https://img.shields.io/crates/v/protto.svg)](https://crates.io/crates/protto)
[![docs.rs](https://docs.rs/protto/badge.svg)](https://docs.rs/protto)

`protto` is a procedural macro for deriving **bidirectional conversions** between `prost`-generated Protobuf types and 
Rust structs. It dramatically reduces boilerplate when working with Protobufs in Rust.

---

## Features

- Automatic `From<Proto>` / `Into<Proto>` conversions
- Support for Rust primitive types (`u32`, `i64`, `String`, etc.)
- Optional fields (`Option<T>`) and collections (`Vec<T>`)
- Transparent newtype wrappers
- Field renaming via `#[protto(proto_name = "...")]`
- Custom conversion functions (`from_proto_fn`, `to_proto_fn`)
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

## Debugging and Introspection
The `protto_derive` macro includes a sophisticated debugging system to help understand and troubleshoot the code 
generation process during compilation.

### Debug Quick Start

**⚠️ Important**: Always run `cargo clean` before using debug mode. Debug output occurs during macro expansion at 
compile time, so already-compiled code won't show debug information even with the environment variable set.

Enable debugging during compilation using the PROTTO_DEBUG environment variable:

```shell
# Debug all structs
PROTTO_DEBUG=all cargo build

# Debug specific structs
PROTTO_DEBUG=Request,Response cargo build  

# Debug with patterns
PROTTO_DEBUG="Track*,*User*" cargo test
```

### Key Features

- **Environment-controlled**: Enable/disable debugging without code changes
- **Selective targeting**: Debug specific structs using pattern matching
- **Call stack tracking**: Visual function call hierarchy with proper indentation
- **Code visualization**: Pretty-printed generated Rust code with line numbers
- **Type analysis**: Detailed insights into conversion strategy selection
- **Zero runtime overhead**: All debugging is compile-time only

### Debug Output Example
```text
┌─ ENTER: generate_field_conversions [Request.header]
│  📊 strategy: TransparentOptionalWithExpect
│  🔀 IF proto_optional + rust_required THEN expect with panic message
│  🛠️ Generated code:
    1 | header: proto::Header::from(
    2 |     proto_struct.header.expect("Proto field header is required")
    3 | )
└─ EXIT:  generate_field_conversions [Request.header]
```

### Pattern Matching
| Pattern | Description | Example |
|---------|-------------|---------|
| `all` | Debug all structs | `PROTTO_DEBUG=all` |
| `StructName` | Exact struct name | `PROTTO_DEBUG=Request` |
| `Pattern*` | Prefix match | `PROTTO_DEBUG=Track*` |
| `*Pattern` | Suffix match | `PROTTO_DEBUG=*Request` |
| `*Pattern*` | Contains match | `PROTTO_DEBUG=*User*` |
| `A,B,C` | Multiple patterns | `PROTTO_DEBUG=Request,Track*,*Response` |
| `0\|false\|none` | Disable debugging | `PROTTO_DEBUG=false` |

### Common Use Cases
```shell
# Troubleshoot conversion issues
PROTTO_DEBUG=MyStruct cargo build 2>&1 | less

# Understand type resolution for user-related structs  
PROTTO_DEBUG="*User*" cargo test

# Analyze generated code for multiple structs
PROTTO_DEBUG="Request,Response,*Header" cargo build
```

For complete documentation, advanced usage patterns, and programming interface details, see the 
[debug module](./protto_derive/src/debug.rs) documentation.

### Integration with Development Workflow
The debug system integrates seamlessly with standard Rust tooling:

- Works with `cargo build`, `cargo test`, and `cargo check`
- Output is compatible with IDE build panels
- No runtime performance impact
- Structured output works well with `grep`, `less`, `rg`, `bat`, `lnav`, and other CLI tools

---
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

This crate extends the original functionality with additional features, improved ergonomics, and comprehensive error 
handling.
