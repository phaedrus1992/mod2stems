//! Loads vendored fixtures.
//!
//! Each test binary mounts this file at its crate root, because a binary that
//! mounts the whole `support` directory compiles every builder into itself.
//! The header-field helpers live in `header.rs`, beside this file.

use std::path::Path;

use openmpt_ext::Module;

/// Returns the bytes of one vendored test module.
///
/// # Errors
///
/// Returns the file name and the reason when the file does not read.
pub fn fixture_bytes(name: &str) -> Result<Vec<u8>, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/openmpt-ext-sys/vendor/openmpt/test")
        .join(name);
    std::fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

/// Loads a vendored test module.
///
/// # Errors
///
/// Returns the file error or the library error as text.
pub fn load(name: &str) -> Result<Module, String> {
    load_bytes(&fixture_bytes(name)?)
}

/// Loads a module from bytes.
///
/// # Errors
///
/// Returns the library error as text.
pub fn load_bytes(bytes: &[u8]) -> Result<Module, String> {
    Module::from_bytes(bytes, &[]).map_err(|error| error.to_string())
}
