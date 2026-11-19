//! Native transport only; parsing, validation and rerun semantics live in Rust
//! SDK.

use napi_derive::napi;

/// Process one versioned SDK request in an independently owned Rust SDK
/// instance.
#[napi]
pub fn request(raw: String) -> String {
    cucumber_sdk::Sdk::new().request(&raw)
}
