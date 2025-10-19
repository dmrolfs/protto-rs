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
    #[protto(proto_required)]  // Marks field for validation
    pub name: Option<String>,
}
```

**How `proto_required` works:**

- Signals validation intent rather than forcing type conversion
- The macro still infers actual proto field optionality from the schema
- Primarily useful for proto2 compatibility or explicit validation requirements
- For most proto3 usage, type-based inference is sufficient

**When you actually need these overrides:**

- Working with proto2 (which has true required fields)
- Proto schema doesn't match your expectations
- You want explicit validation semantics
- Interfacing with external proto definitions you don't control

### 6. Collection Strategy

Vector and repeated field conversion:

```rust
#[derive(Protto)]
pub struct Playlist {
    pub tracks: Vec<Track>,              // Vec<T> -> repeated T
    pub tags: Option<Vec<String>>,       // Option<Vec<T>> -> repeated T (None for empty)
}
```

### Attribute Syntax Variants

Several attributes accept multiple syntax forms:

**Function references:**

```rust
// String literal (must be quoted)
#[protto(error_fn = "MyError::missing_field")]
#[protto(default = "my_default_fn")]
#[protto(from_proto_fn = "custom_parser")]

// Path expression (no quotes)
#[protto(error_fn = MyError::missing_field)]
#[protto(default = my_default_fn)]
#[protto(from_proto_fn = custom_parser)]
```

Both forms are parsed and work identically. Use quotes for consistency or when the function path contains special characters.

## Attribute Precedence and Conflicts

### Mutually Exclusive Attributes

- `proto_optional` and `proto_required` - cannot use both
- `default` and `default_fn` - use `default = "function"` syntax instead
- `expect(panic)` and `expect` - panic takes precedence
- `transparent` and custom functions - transparent ignores conversion functions

### Precedence Order

When multiple strategies could apply, the macro checks in this order:

1. **`ignore`** - Skips all other processing, uses `Default::default()`
2. **Custom functions** - `from_proto_fn`, `to_proto_fn`, or both
3. **`transparent`** - Direct wrapper conversion (bypasses normal type handling)
4. **Collections** - `Vec`, `HashMap`, or `Option<Vec<T>>` patterns
5. **Explicit defaults** - Fields with `default` or `default_fn` attributes
   - This creates `Option(Unwrap)` strategy with default error mode
   - Takes precedence over general optionality inference
6. **Optionality patterns** - Rust `Option<T>` vs proto optional field detection
7. **Direct mapping** - Simple type assignment (fallback)

**Special case:** If a field has `default` attribute, it gets `Option(Unwrap)` strategy even if optionality matches (both required or both optional). This ensures the default function is used for missing proto values.

**Example:**

```rust
#[derive(Protto)]
struct Config {
    // Has default attribute → Option(Unwrap) with Default error mode
    // Even though both Rust and proto sides are required
    #[protto(default = "default_timeout")]
    pub timeout: u32,
}
```

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

## Type Inference and Detection

The macro automatically infers conversion strategies based on Rust type analysis. Understanding how types are categorized helps you predict behavior:

### Type Categories

**Primitive Types:**

```rust
// Detected via PRIMITIVE_TYPES constant
i32, u32, i64, u64, f32, f64, bool, String
```

- Map directly to proto scalar types
- No conversion functions needed
- Always use `Direct` strategy

**Custom Types:**

```rust
pub struct UserId(u64);  // Custom type
pub struct User { ... }   // Custom type
```

- Detected as: single-segment path, non-primitive, not in `std` or proto module
- Automatically use `Into`/`From` traits
- Can override with `transparent` for newtype wrappers

**Proto Types:**

```rust
proto::Track    // Detected by module prefix
proto::Status   // Detected by module prefix
```

- Detected by matching the configured `proto_module` path
- Use direct assignment (no conversion)
- Can be in collections: `Vec<proto::Track>`

**Enum Types:**

```rust
#[derive(Protto)]
pub enum Status { Ok, NotFound }
```

- Automatically registered in global enum registry during macro expansion
- Converted to/from proto `i32` representation
- Recognition persists across multiple macro invocations in the same compilation

**Collection Types:**

```rust
Vec<T>                    // Standard vector
Option<Vec<T>>            // Optional collection
HashMap<K, V>             // Via custom functions
```

- Detected by type name pattern matching
- Proto `repeated` fields map to `Vec<T>`
- Empty proto repeated `[]` can become `None` for `Option<Vec<T>>`

### Type Detection Order

The macro checks types in this order:

1. Explicit attributes (`transparent`, `ignore`, custom functions)
2. Enum registry lookup (for previously processed enums)
3. Proto module path detection
4. Primitive type matching
5. Collection type patterns (`Vec`, `HashMap`, etc.)
6. Custom type (fallback for everything else)

### Implications for Users

**This means:**

- Enums must be defined before structs that use them in the same file
- Proto module name must match your configuration (default: `"proto"`)
- Newtype wrappers are "custom types" unless marked `transparent`
- Type aliases don't affect detection (underlying type matters)

**Example pitfall:**

```rust
// This will be treated as a custom type requiring Into/From
pub struct MyTrackId(u64);

// Add transparent to unwrap directly to u64
pub struct MyTrackId(#[protto(transparent)] u64);
```

## Macro Attribute Reference

### Struct-level Attributes

- `#[protto(module = "path")]` - Specify proto module path
- `#[protto(proto_name = "ProtoName")]` - Map to different proto type name
- `#[protto(error_type = ErrorType)]` - Set error type for fallible conversions (one per struct)

### Field-level Attributes

- `#[protto(transparent)]` - Direct newtype wrapper conversion
- `#[protto(ignore)]` - Skip field in proto conversion (uses `Default::default()` for proto→rust, omitted in rust→proto)
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

### Struct-level Ignore Details

When using struct-level ignore:

```rust
#[protto(ignore = "field1, field2, field3")]
```

**Parsing behavior:**

- Field names are comma-separated
- Whitespace is automatically trimmed: `"field1, field2"` and `"field1,field2"` are equivalent
- Empty strings are filtered out
- Field names are **case-sensitive** - must match Rust field names exactly
- Generates `Default::default()` for proto→rust conversion
- Omits fields from proto struct in rust→proto conversion

**Example:**

```rust
#[derive(Protto)]
#[protto(ignore = "  cache,  temp  ")]  // Leading/trailing spaces OK
pub struct User {
    pub name: String,
    pub cache: HashMap<String, String>,  // Must be "cache", not "Cache"
    pub temp: Vec<u8>,
}
```

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

**Bare default:**

- `#[protto(default)]` - Use `Default::default()` for missing fields

**Custom default function (two equivalent syntaxes):**

- `#[protto(default = "function_name")]` - String literal syntax
- `#[protto(default_fn = "function_name")]` - Named parameter syntax

Both syntaxes are fully supported and equivalent. Choose whichever is clearer for your use case.

**Important:** Cannot use both `default` and `default_fn` on the same field.

**Important**: `default_fn` cannot be used with repeated/collection fields. Proto3 repeated fields cannot be "missing" (only empty `[]`). Use `default` attribute on individual field types if needed.

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

### Error-handling Strategies

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

**Signature requirements:**

- Must accept `field_name: &str` parameter (even if unused)
- Must return your error type
- Can be a method (`ErrorType::function`) or free function
- Will be called with the proto field name as a string

**Both syntaxes work:**

```rust
// With quotes (string literal)
#[protto(error_fn = "ValidationError::missing_field")]

// Without quotes (path expression)
#[protto(error_fn = ValidationError::missing_field)]
```

**The generated code calls it like this:**

```rust
// For field named "email"
ValidationError::missing_field("email")
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

// Error function receives the field name
fn email_error(field: &str) -> UserError {
    eprintln!("Missing required field: {}", field);
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

### Understanding the Selection Criteria

The macro generates **exactly one** trait implementation per struct:

- `From<ProtoType> for RustType` - Infallible conversion
- `TryFrom<ProtoType> for RustType` - Fallible conversion with `type Error = ...`

**The decision is made at compile time based on field attributes:**

```rust
// ANY field has `expect` without `panic`?
//   ↓ YES → TryFrom
//   ↓ NO  → From

#[derive(Protto)]
pub struct User {
    #[protto(expect(panic))]  // ← panic doesn't trigger TryFrom
    pub id: UserId,

    #[protto(expect)]          // ← THIS triggers TryFrom for whole struct
    pub email: String,

    #[protto(default)]         // ← default doesn't trigger TryFrom
    pub role: String,
}
// Result: TryFrom because of `email` field
```

**Important:** The decision is **all-or-nothing** per struct. One `expect` field without `panic` forces the entire struct to use `TryFrom`, even if other fields use `panic` or `default`.

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
5. **Mix panic + error**: Valid combination. `TryFrom` is implemented, but `expect(panic)` fields panic during conversion (preventing error return). Execution order is field declaration order - early panic fields prevent later
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

## Transparent Wrapper Best Practices

The `transparent` attribute is powerful but has specific use cases and limitations.

### When to Use Transparent

**Perfect for newtype wrappers:**

```rust
#[derive(Protto)]
pub struct UserId(u64);  // ✓ Single field tuple struct

#[derive(Protto)]
pub struct EmailAddress {
    #[protto(transparent)]
    inner: String,  // ✓ Single meaningful field
}
```

### When NOT to Use Transparent

**Multi-field structs:**

```rust
// ✗ Don't use transparent here
pub struct Timestamp {
    #[protto(transparent)]
    seconds: i64,
    nanos: u32,  // ← transparent will ignore this
}
```

**Collections:**

```rust
// ✗ Don't use transparent with Vec
pub struct Tags(#[protto(transparent)] Vec<String>);

// ✓ Instead, let the macro handle it automatically
pub struct Tags(Vec<String>);
```

**Types needing custom logic:**

```rust
// ✗ Transparent bypasses your validation
#[protto(transparent, from_proto_fn = "validate_email")]
pub email: Email;  // from_proto_fn will be ignored!

// ✓ Use custom functions without transparent
#[protto(from_proto_fn = "validate_email")]
pub email: Email;
```

### What Transparent Actually Does

```rust
// With transparent:
#[derive(Protto)]
pub struct UserId(#[protto(transparent)] u64);

// Generated code is effectively:
impl From<proto::UserId> for UserId {
    fn from(proto: proto::UserId) -> Self {
        UserId(proto.0)  // Direct field access, no Into/From call
    }
}
```

**Transparent means:**

1. **Bypasses** normal conversion logic
2. **Unwraps** directly to inner type
3. **Ignores** custom conversion functions on the same field
4. Only works for **single-field** wrappers

### Parser Limitation

⚠️ The macro **does not validate** that your struct has only one field at parse time. If you use `transparent` on a multi-field struct, you'll get:

- Confusing compilation errors
- Or worse, silent incorrect behavior

**The macro trusts you to use it correctly.**

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


For complete documentation, advanced usage patterns, and programming interface details, see the [debug module](./protto_derive/src/debug.rs) documentation.


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

#### Available Nix Commands

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

#### Development Wokflow Examples

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

This crate extends the original functionality with additional features, improved ergonomics, and comprehensive error handling.
