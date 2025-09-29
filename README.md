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
#### Field-level Rust Field Ignoring
Skip rust fields that don't exist in proto:
```rust
#[derive(Protto)]
pub struct User {
    pub name: String,
    #[protto(ignore)]
    pub runtime_cache: HashMap<String, String>, // Uses Default::default()
}
```

#### Struct-level Proto Field Ignoring
Proto fields can be ignored at the struct level using comma-separated names:
```rust
#[derive(Protto)]
#[protto(ignore = "internal_cache, computed_value")]  // Multiple fields
pub struct User {
    pub name: String,
    pub internal_cache: HashMap<String, String>,  // Uses Default::default()
    pub computed_value: usize,                     // Uses Default::default()
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
Direct newtype wrapper conversion - bypasses normal conversion logic:
```rust
#[derive(Protto)]
pub struct UserId(#[protto(transparent)] u64);

#[derive(Protto)]
pub struct User {
    #[protto(transparent)]
    pub id: UserId, // Directly converts inner u64
}
```
#### When to use: 
Newtype wrappers around a single field

#### When NOT to use:
- Structs with multiple fields
- Types requiring custom conversion logic
- Collections (use Collection strategy instead)


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

#### Manual Optionality Override

By default, the macro infers proto field optionality from Rust types. Override when:
- Proto definition doesn't match Rust expectations
- Working with proto2 (required fields) vs proto3 (all fields implicitly optional)
- You want explicit control

**When Proto has `optional` but you want required Rust field**:
```rust
#[derive(Protto)]
struct User {
    // Proto: optional string email = 1;
    // Rust: want String (not Option<String>)
    #[protto(proto_optional, expect)]  // Unwrap with error
    pub email: String,
}
```

**When Proto is required but you want optional Rust field**:
```rust
#[derive(Protto)]
struct User {
    // Proto: string name = 1;  (proto3 - technically optional)
    // Rust: want Option<String>
    #[protto(proto_required)]  // Wrap in Some()
    pub name: Option<String>,
}
```

Most users don't need these - the macro infers correctly from types.

### 6. Collection Strategy
Vector and repeated field conversion:
```rust
#[derive(Protto)]
pub struct Playlist {
    pub tracks: Vec<Track>,              // Vec<T> -> repeated T
    pub tags: Option<Vec<String>>,       // Option<Vec<T>> -> repeated T (None for empty)
}
```

## Attribute Precedence and Conflicts

### Mutually Exclusive Attributes
- `proto_optional` and `proto_required` - cannot use both
- `default` and `default_fn` - use `default = "function"` syntax instead
- `expect(panic)` and `expect` - panic takes precedence
- `transparent` and custom functions - transparent ignores conversion functions

### Precedence Order
When multiple strategies could apply:
1. `ignore` - skips all other processing
2. Custom functions (`from_proto_fn`, `to_proto_fn`)
3. `transparent` - direct wrapper conversion
4. Collections - Vec/repeated handling
5. Default/expect - optionality handling
6. Direct - simple type mapping

## Common Patterns

### Pattern 1: ID wrappers
```rust
#[derive(Protto)]
pub struct UserId(u64);

#[derive(Protto)]
pub struct User {
    #[protto(transparent)]
    pub id: UserId,
}
```

### Pattern 2: Required proto → Optional Rust
```rust
#[derive(Protto)]
struct User {
    #[protto(proto_required)]
    pub name: Option<String>,
}
```

### Pattern 3: Optional proto → Required Rust with default
```rust
#[derive(Protto)]
struct User {
    #[protto(proto_optional, default = "default_role")]
    pub role: UserRole,
}
```

### Pattern 4: Collections with custom types
```rust
#[derive(Protto)]
pub struct Track {
    #[protto(transparent)]
    pub id: TrackId,
}

pub struct Playlist {
    pub tracks: Vec<Track>,  // Automatic collection conversion
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

#### Default Value Strategies
- `#[protto(default)]` - Use `Default::default()` for missing fields
- `#[protto(default = "function_name")]` - Custom default function (preferred syntax)
- `#[protto(default_fn = "function_name")]` - Legacy syntax (deprecated, use `default =` instead)

**Important**: `default_fn` cannot be used with repeated/collection fields. Proto3 repeated fields
cannot be "missing" (only empty `[]`). Use `default` attribute on individual field types if needed.
```rust
#[derive(Protto)]
pub struct Config {
    #[protto(default = "default_timeout")]
    pub timeout: u32,
}

fn default_timeout() -> u32 {
    30
}
```

#### Error-handling Strategies
#### Error Handling: Three Distinct Approaches

Proto fields can be optional (proto3 `optional` keyword). When converting to required Rust fields,
you must handle the missing case:

##### Strategy 1: Panic on Missing (implements `From`)
```rust
#[derive(Protto)]
struct User {
    #[protto(proto_optional, expect(panic))]
    pub email: String,
    // Panics with: "Proto field email is required"
    // Implements: From<proto::User> for User
}
```

##### Strategy 2: Return Error with Auto-generated Type (implements `TryFrom`)
```rust
#[derive(Protto)]
struct User {
    #[protto(proto_optional, expect)]
    pub email: String,
    // Generates: UserConversionError enum with MissingField(String) variant
    // Implements: TryFrom<proto::User> for User { type Error = UserConversionError; }
}
```

##### Strategy 3: Return Error with Custom Type (implements TryFrom)
```rust
#[derive(Debug)]
pub enum UserError {
    MissingEmail,
}

impl UserError {
    fn missing_email(_field: &str) -> UserError {
        UserError::MissingEmail
    }
}

#[derive(Protto)]
#[protto(error_type = UserError)]
pub struct User {
    #[protto(proto_optional, expect, error_fn = "UserError::missing_email")]
    pub email: String,
}
// Implements: TryFrom<proto::User> for User { type Error = UserError; }
```

##### Key Rules:

- Only one error type per struct (maps to `TryFrom::Error`)
- `expect(panic)` → uses `From` trait (no error type)
- `expect` alone → uses `TryFrom` with auto-generated error
- `expect` + `error_type` → uses `TryFrom` with custom error
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

#[derive(Protto)]
struct MyStruct {
    // Used as: error_fn = "ValidationError::missing_field"
    #[protto(expect, error_fn = "ValidationError::missing_field")]
    pub field: String,
}

// Function-style error functions (no parameters)
fn create_custom_error(_field: &str) -> CustomError {
    CustomError::MissingEmail
}

#[derive(Protto)]
struct MyStruct {
    // Used as: error_fn = "create_custom_error"
    #[protto(expect, error_fn = "create_custom_error")]
    pub field: String,
}
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

## When `From` vs `TryFrom` is Generated

The macro automatically chooses between `From` and `TryFrom` trait implementations based on your error handling strategy:

### Generates `From` Implementation
The macro generates infallible `From<ProtoType> for RustType` when:
```rust
// No expect attributes anywhere
#[derive(Protto)]
pub struct User {
    pub name: String,
    pub age: u32,
}
// Implements: From<proto::User> for User

// All fields use expect(panic) 
#[derive(Protto)]
pub struct User {
    #[protto(proto_optional, expect(panic))]
    pub email: String,
}
// Implements: From<proto::User> for User
// Panics at runtime if email is None
```

#### When `From` is used:

- All conversions succeed or panic
- No `Result` type needed
- Use `.into()` without error handling: `let user: User = proto_user.into();`
- Panics have clear messages: `"Proto field email is required"`

### Generates `TryFrom` Implementation
The macro generates fallible `TryFrom<ProtoType> for RustType` when ANY field uses `expect` without `panic`:
```rust
// Auto-generated error type
#[derive(Protto)]
pub struct User {
    #[protto(proto_optional, expect)]  // DMR: No (panic), so TryFrom
    pub email: String,
}
// Implements: TryFrom<proto::User> for User {
//     type Error = UserConversionError;
// }
// Auto-generates:
// #[derive(Debug, PartialEq, Clone)]
// pub enum UserConversionError {
//     MissingField(String),
// }

// Custom error type
#[derive(Debug)]
pub enum UserError {
    MissingEmail,
    InvalidData,
}

#[derive(Protto)]
#[protto(error_type = UserError)]
pub struct User {
    #[protto(proto_optional, expect, error_fn = "UserError::missing_email")]
    pub email: String,
}
// Implements: TryFrom<proto::User> for User {
//     type Error = UserError;
// }
```

#### When `TryFrom` is used:

- Conversions return `Result<RustType, ErrorType>`
` Requires error handling: `let user: User = proto_user.try_into()?;`
- One error type per struct (maps to `TryFrom::Error`)
- Mix of field-level error handling:
  - `expect` with `error_fn` → custom error
  - `expect` without `error_fn` → uses auto-generated error or struct-level `error_fn`
  - `default` / `default_fn` → provides fallback value (no error generated)

### Decision Tree
```
Does ANY field have `expect` (without `panic`)?
├─ NO → From
│  └─ Infallible conversion (.into())
│
└─ YES → TryFrom
├─ Has struct-level `error_type`?
│  ├─ YES → TryFrom with custom error type
│  └─ NO  → TryFrom with auto-generated error type
│
└─ Error type is TryFrom::Error (one per struct)
```

### Mixed Strategies Example
```rust
#[derive(Debug)]
pub enum UserError {
    MissingEmail,
}

#[derive(Protto)]
#[protto(error_type = UserError)]
pub struct User {
    // DMR: Different error handling per field
    #[protto(proto_optional, expect(panic))]  // Panics immediately
    pub id: UserId,
    
    #[protto(proto_optional, expect, error_fn = "UserError::missing_email")]
    pub email: String,  // Returns Err(UserError::MissingEmail)
    
    #[protto(proto_optional, default = "guest_role")]
    pub role: String,  // Uses default, never errors
}
// Implements: TryFrom because `email` uses `expect` without `panic`
// id panics before TryFrom can return error
// email returns error
// role uses default (no error possible)
```

### Key Rules
1. **One implementation per struct**: Either `From` OR `TryFrom`, never both
2. **`expect(panic)` doesn't trigger `TryFrom`**: Only bare `expect` does
3. **`default` doesn't require `TryFrom`**: It provides fallback values
4. **One error type per struct**: `TryFrom::Error = YourErrorType` or auto-generated
5. **Mix panic + error**: Valid combination. `TryFrom` is implemented, but `expect(panic)` fields panic during 
conversion (preventing error return). Execution order is field declaration order - early panic fields prevent later 
error fields from being evaluated.
6. **Auto-generated error naming**: `<StructName>ConversionError`
7. **Auto-generated error variant**: `MissingField(String)` (takes field name)

### Common Patterns
```rust
// Pattern 1: Simple infallible conversion (From)
#[derive(Protto)]
pub struct SimpleConfig {
    pub port: u32,
    pub host: String,
}

// Pattern 2: All-panic strategy (From)
#[derive(Protto)]
pub struct StrictUser {
    #[protto(expect(panic))]
    pub id: UserId,
    #[protto(expect(panic))]
    pub email: String,
}

// Pattern 3: Graceful errors (TryFrom with auto-generated error)
#[derive(Protto)]
pub struct User {
    #[protto(expect)]
    pub id: UserId,
    #[protto(expect)]
    pub email: String,
}

// Pattern 4: Custom error type (TryFrom with custom error)
#[derive(Protto)]
#[protto(error_type = ValidationError)]
pub struct ValidatedUser {
    #[protto(expect, error_fn = "ValidationError::missing_id")]
    pub id: UserId,
}

// Pattern 5: Defaults instead of errors (From)
#[derive(Protto)]
pub struct ConfigWithDefaults {
    #[protto(default = "default_timeout")]
    pub timeout: u32,
    #[protto(default)]
    pub retries: usize,
}
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

#### Development Tools Available
##### Core Rust Toolchain:
```bash
rustc --version       # Rust compiler with rust-src, rustc-dev, llvm-tools-preview
cargo --version       # Package manager
rustfmt --version     # Code formatter
clippy --version      # Linter
rust-analyzer         # LSP support (for IDEs)
miri                  # Undefined behavior detection
```

##### Testing and Coverage:
```bash
# Fast test runner (alternative to cargo test)
cargo nextest run

# Code coverage analysis
cargo tarpaulin --out html --output-dir coverage/
# Or use the convenience script:
nix run .#coverage

# Mutation testing for test quality
cargo mutants --in-place
# Or use the convenience script:
nix run .#mutants
```

##### Security and Code Quality:
```bash
# Security vulnerability audit
cargo audit

# Dependency license and policy checking
cargo deny check

# Find unused dependencies
cargo machete

# Check for outdated dependencies
cargo outdated

# Macro expansion debugging
cargo expand
```

##### Development Workflow:
```bash
# File watching - rebuild/retest on changes
cargo watch -c -x test
cargo watch -c -x 'nextest run'

# Release management
cargo release patch  # Bump patch version
cargo release minor  # Bump minor version

# Documentation with private items
cargo doc --document-private-items --open
```

##### Protocol Buffers:
```bash
protoc --version           # Protocol Buffer compiler
protoc-gen-rust --version  # Rust code generator
```

##### Development Utilities:
```bash
# Command runner (like make but better)
just --list               # Show available commands (if Justfile exists)

# Git hooks management
pre-commit install        # Set up git hooks
pre-commit run --all-files # Run all hooks manually

# Documentation generation
mdbook build              # Build additional docs (if book.toml exists)
```

##### Available Nix Commands
##### Building and Checking:
```bash
# Build the project
nix build

# Build documentation
nix build .#doc

# Run comprehensive checks (formatting, clippy, build, tests, docs, audit)
nix flake check

# Format nix files
nix fmt
```

##### Development Environments:
```bash
# Full development shell (default)
nix develop

# Minimal CI shell (for automation)
nix develop .#ci
```

##### Convenience Scripts:
```bash
# Generate coverage report and open in browser
nix run .#coverage

# Run mutation testing
nix run .#mutants
```

##### Development Wokflow Examples
##### Daily Development:
```bash
# Enter development environment
nix develop

# Start file watching for continuous testing
cargo watch -c -x 'nextest run'

# In another terminal, work on code
# Tests automatically run on file changes
```

##### Pre-commit Checklist:
```bash
# Format code
cargo fmt

# Run linting
cargo clippy -- -D warnings

# Run tests with coverage
cargo tarpaulin --out terminal

# Security audit
cargo audit

# Check documentation
cargo doc --document-private-items
```

##### Release Preparation:
```bash
# Run full validation
nix flake check

# Generate final coverage report
nix run .#coverage

# Assess test quality with mutations
nix run .#mutants

# Check for outdated dependencies
cargo outdated

# Clean up unused dependencies
cargo machete
```

##### Debugging and Analysis:
```bash
# Debug macro expansion issues
PROTTO_DEBUG=all cargo expand
# or
PROTTO_DEBUG=all cargo test

# Check for undefined behavior
cargo +nightly miri test

# Profile compilation time
cargo build --timings

# Analyze binary size
cargo bloat --release
```

##### Cache issues?
```bash
# Clean cargo cache
cargo clean

# Rebuild nix environment
nix develop --rebuild
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
