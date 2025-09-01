//! # proto_convert
//!
//! Automatically derive conversions between Rust structs and Protocol Buffer messages.
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! proto_convert = "0.2"
//! ```
//!
//!
//! In your code:
//! ```rust,ignore
//! use proto_convert::ProtoConvert;
//!
//! #[derive(ProtoConvert)]
//! struct User {
//!     name: String,           // Required in proto
//!     email: Option<String>,  // Optional in proto - automatically detected
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/proto_convert/0.2.0")]

// re-export the derive macro
pub use proto_convert_derive::*;
