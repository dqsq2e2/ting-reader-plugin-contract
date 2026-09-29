//! Transport-independent plugin contracts. Runtime adapters are responsible
//! for moving the same DTOs across WASM, JavaScript and Native boundaries.
//!
//! Capability declarations and format DTOs are independent of the backend.
//! Runtime adapters enforce these contracts at registration and invocation.

pub mod capability;
pub mod format;
pub mod format_calls;
pub mod format_registry;
pub mod manifest;
pub mod native_abi;
pub mod protocol;
pub mod resources;
pub mod scraper;

/// Earliest core release that accepts the new plugin contract.
pub const MIN_PLUGIN_CORE_VERSION: &str = "2.0.0";
