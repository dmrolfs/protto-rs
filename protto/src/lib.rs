//! # protto
//!
//! Automatically derive conversions between Rust structs and Protocol Buffer messages.
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! protto = "0.2"
//! ```
//!
//!
//! In your code:
//! ```rust,ignore
//! use protto::Protto;
//!
//! #[derive(Protto)]
//! struct User {
//!     name: String,           // Required in proto
//!     email: Option<String>,  // Optional in proto - automatically detected
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/protto/0.2.0")]

// re-export the derive macro
pub use protto_derive::*;
