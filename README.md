# protto

[![crates.io](https://img.shields.io/crates/v/protto.svg)](https://crates.io/crates/protto)
[![docs.rs](https://docs.rs/protto/badge.svg)](https://docs.rs/protto)

`protto` is a procedural macro for deriving **bidirectional conversions** between `prost`-generated Protobuf types and
Rust structs. It dramatically reduces boilerplate when working with Protobufs in Rust.

---

## Features

- Automatic bidirectional conversion implementations of `From<Proto>` or `TryFrom<Proto>` and `Into<Proto>`
- Support for Rust primitive types (`u32`, `i64`, `String`, etc.)
- Direct newtype wrapper conversion  (`#[protto(transparent)]`)
- Rename fields with `#[protto(proto_name = "...")]`
- User-defined conversion functions (`from_proto_fn`, `to_proto_fn`)
- Skip fields not present in proto (`#[protto(ignore)]`)
- primitive and simple type mapping
- Smart optionality handling between Rust `Option<T>` and proto optional fields
- Vector and repeated field conversion with empty handling
- Automatic inference of conversion strategies based on types
- Configurable proto module defaults to `prost` `proto` module in your application, customizable per struct
- Support for `expect()`, custom error types, and graceful defaults
- Manual override of optionality with `proto_optional`/`proto_required`

---

## Installation

```toml
[dependencies]
protto = "0.6"
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

## Field Conversion Strategy Categories
The macro automatically selects from 6 streamlined conversion strategies:

### 1. Ignore Strategy
Skip fields that don't exist in proto:
```rust
#[derive(Protto)]
pub struct User {
    pub name: String,
    #[protto(ignore)]
    pub runtime_cache: HashMap<String, String>, // Uses Default::default()
}
```

### 2. Custom Strategy
User-defined conversion functions:
```rust
#[derive(Protto)]
pub struct Event {
    #[protto(from_proto_fn = "parse_timestamp", to_proto_fn = "format_timestamp")]
    pub created_at: DateTime<Utc>,
}

fn parse_timestamp(proto_ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(proto_ts, 0).unwrap()
}

fn format_timestamp(dt: DateTime<Utc>) -> i64 {
    dt.timestamp()
}
```

### 3. Transparent Strategy
Direct newtype wrapper conversion:
```rust
#[derive(Protto)]
pub struct UserId(#[protto(transparent)] u64);

#[derive(Protto)]
pub struct User {
    #[protto(transparent)]
    pub id: UserId, // Directly converts inner u64
}
```

### 4. Direct Strategy
Primitive and simple type mapping:
```rust
#[derive(Protto)]
pub struct Config {
    pub name: String,        // Direct assignment
    pub port: u32,           // Direct assignment
    pub enabled: bool,       // Direct assignment
}
```

### 5. Option Strategy
Smart optionality handling with error management:
```rust
#[derive(Protto)]
pub struct Profile {
    pub name: String,                        // Required -> Required
    pub bio: Option<String>,                 // Optional -> Optional

    #[protto(proto_optional, expect(panic))] // Panic with .expect() (uses From)
    pub email: String,

    #[protto(proto_optional, expect)]        // Generate error type (uses TryFrom)
    pub phone: String,

    #[protto(default)]                       // Use default value
    pub role: String,
}
```

### 6. Collection Strategy
Vector and repeated field conversion:
```rust
#[derive(Protto)]
pub struct Playlist {
    pub tracks: Vec<Track>,              // Vec<T> -> repeated T
    pub tags: Option<Vec<String>>,       // Option<Vec<T>> -> repeated T (None for empty)
}
```

## Macro Attribute Reference
### Struct-level Attributes
- `#[protto(module = "path")]` - Specify proto module path
- `#[protto(proto_name = "ProtoName")]` - Map to different proto type name
- `#[protto(error_type = ErrorType)]` - Set error type for fallible conversions (one per struct)

### Field-level Attributes
- `#[protto(transparent)]` - Direct newtype wrapper conversion
- `#[protto(ignore)]` - Skip field (uses `Default::default()`)
- `#[protto(proto_name = "field_name")]` - Map to different proto field name
- `#[protto(from_proto_fn = "function")]` - Custom proto→rust conversion
- `#[protto(to_proto_fn = "function")]` - Custom rust→proto conversion
- `#[protto(proto_optional)]` - Treat proto field as optional (unwrap to required)
- `#[protto(proto_required)]` - Treat proto field as required (wrap to optional)
- `#[protto(expect(panic))]` - Panic with `.expect()` for missing optional fields (uses `From`)
- `#[protto(expect)]` - Generate error handling for missing fields (uses `TryFrom`)
- `#[protto(error_fn = "function")]` - Custom error function (signature: `fn(field_name: &str) -> ErrorType`)
- `#[protto(default)]` - Use `Default::default()` for missing fields
- `#[protto(default = "function")]` - Custom default function

### Advanced Examples

#### Complex Custom Conversions
```rust
#[derive(Protto)]
pub struct StateMap {
    #[protto(from_proto_fn = "into_map", to_proto_fn = "from_map")]
    pub tracks: HashMap<TrackId, Track>,
}

fn into_map(tracks: Vec<proto::Track>) -> HashMap<TrackId, Track> {
    tracks.into_iter().map(|t| (TrackId(t.track_id), t.into())).collect()
}

fn from_map(tracks: HashMap<TrackId, Track>) -> Vec<proto::Track> {
    tracks.into_values().map(Into::into).collect()
}
```

#### Error-handling Strategies
The macro supports three distinct error handling approaches with different implementation strategies:

##### 1. Panic-based Error-handling (`From` implementation)
```rust
#[derive(Protto)]
pub struct User {
    #[protto(proto_optional, expect(panic))]  // Uses .expect(), implements From
    pub email: String,
}

// Generated: impl From<proto::User> for User
// Panics with: "Proto field email is required for transparent conversion"
```

##### 2. Auto-generated Error Type (`TryFrom` implementation)
```rust
#[derive(Protto)]
pub struct User {
    #[protto(proto_optional, expect)]         // Auto-generates UserConversionError
    pub email: String,
}

// Generated error type:
// #[derive(Debug, PartialEq, Clone)]
// pub enum UserConversionError {
//     MissingField(String),  // Note: Takes String, not &'static str
// }
//
// Generated: impl TryFrom<proto::User> for User {
//     type Error = UserConversionError;
// }
```

##### 3. Custom Error Type (`TryFrom` implementation)
```rust
#[derive(Debug, PartialEq)]
pub enum CustomError {
    MissingEmail,
    InvalidData,
}

#[derive(Protto)]
#[protto(error_type = CustomError)]           // Use custom error type
pub struct User {
    #[protto(proto_optional, expect)]         // Uses CustomError with auto-generated variant
    pub email: String,

    #[protto(proto_optional, expect, error_fn = "email_error")]  // Custom error function
    pub phone: String,
}

// Error function signature: fn() -> ErrorType (no parameters)
fn email_error(_field: &str) -> CustomError {
    CustomError::MissingEmail
}

// Generated: impl TryFrom<proto::User> for User {
//     type Error = CustomError;  // One error type per struct
// }
```

##### Key Rules:
- `expect(panic)` → `From` implementation with panic behavior
- `expect` alone → `TryFrom` with auto-generated `<StructName>ConversionError`
- `expect` + `error_type` → `TryFrom` with custom error type
- Only one error type per struct (maps to `TryFrom::Error`)
- Error functions have signature `fn(field: &str) -> ErrorType`
- Auto-generated error enums use `MissingField(String)` variant (not `&'static str`)

##### Error Function Patterns:
```rust
// Method-style error functions (called as static methods)
impl ValidationError {
    pub fn missing_field(field_name: &str) -> Self {
        Self::MissingField(field_name.to_string())
    }
}

// Used as: error_fn = "ValidationError::missing_field"
#[protto(expect, error_fn = "ValidationError::missing_field")]
pub field: String,

// Function-style error functions (no parameters)
fn create_custom_error(_field: &str) -> CustomError {
    CustomError::MissingEmail
}

// Used as: error_fn = "create_custom_error"
#[protto(expect, error_fn = "create_custom_error")]
pub field: String,
```

##### Error Handling Strategies
```rust
// Define custom error type (only one per struct - corresponds to TryFrom::Error)
#[derive(Debug)]
pub enum UserError {
    MissingEmail,
    InvalidRole,
}

#[derive(Protto)]
#[protto(error_type = UserError)]        // Enables TryFrom implementation
pub struct User {
    #[protto(expect)]                    // Auto-generated error on None
    pub id: UserId,

    #[protto(expect, error_fn = "email_error")]  // Custom error function
    pub email: String,

    #[protto(default = "default_role")]  // Custom default function
    pub role: UserRole,
}

// Error function signature: fn() -> ErrorType
fn email_error(_field: &str) -> UserError {
    UserError::MissingEmail
}

fn default_role() -> UserRole {
    UserRole::Guest
}

// Generated code creates:
// impl TryFrom<proto::User> for User {
//     type Error = UserError;  // Only one error type per struct
//     ...
// }
```

## Debugging and Introspection
The macro includes a sophisticated debugging system to understand and troubleshoot
the code generation process during compilation.

### Enable debugging during compilation:

**⚠️ Important**: Always run `cargo clean` before using debug mode. Debug output occurs during macro expansion at
compile time, so already-compiled code won't show debug information even with the environment variable set.

Enable debugging during compilation using the PROTTO_DEBUG environment variable:

```bash
# Debug all structs
cargo clean && PROTTO_DEBUG=all cargo build

# Debug specific structs
cargo clean && PROTTO_DEBUG=Request,Response cargo test

# Debug with patterns
cargo clean && PROTTO_DEBUG="Track*,*User*" cargo build
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
- The macro is re-exported for convenience: `pub use protto_derive::*;`

## Contributing

Contributions, bug reports, and feature requests are welcome!

Please follow the guidelines in [CONTRIBUTING.md](CONTRIBUTING.md) for the recommended workflow and standards.

### Using Nix Development Environment

This project includes a Nix flake for reproducible development environments with all necessary dependencies pre-installed.
Prerequisites: Install [Nix](https://nixos.org/download.html) with flakes enabled.

#### Quick Setup:
```bash
# Clone the repository
git clone https://github.com/your-username/protto-rs
cd protto-rs

# Enter the development environment
nix develop

# You now have access to:
# - Rust toolchain (stable with rust-src, rustc-dev, llvm-tools-preview)
# - protoc (Protocol Buffer compiler)
# - cargo-tarpaulin (code coverage)
# - All formatting and linting tools
```

#### Available Nix Commands:
```bash
# Build the project
nix build

# Run checks (formatting, clippy, build)
nix flake check

# Enter development shell
nix develop
```

#### Development Workflow:
```bash
# Inside nix develop shell:
cargo build                    # Build project
cargo test                     # Run tests
cargo fmt                      # Format code
cargo clippy                   # Run linting
cargo tarpaulin --out Html     # Generate coverage report
```
The Nix environment ensures consistent toolchain versions and dependencies across different machines and CI/CD systems.

#### Standard Setup (without Nix)
Ensure you have Rust and `protoc` installed:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install protoc (varies by system)
# macOS: brew install protobuf
# Ubuntu: apt-get install protobuf-compiler
# Arch: pacman -S protobuf
```

## Attribution
`protto` builds upon the `proto_convert_derive` crate created by: [Christian Engel](mailto:cascade.nab0p@icloud.com)

This crate extends the original functionality with additional features, improved ergonomics, and comprehensive error
handling.
