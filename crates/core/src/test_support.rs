//! Path resolution for the repository assets used by tests.
//!
//! `cargo test` runs the test binary with the *crate* directory as its working directory, so
//! relative paths like `resources/skin.png` do not resolve. Resolve them against the manifest
//! directory instead, which works for any working directory.

use std::path::PathBuf;

/// Path to a file inside the repository's `resources/` directory.
pub(crate) fn resource(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("resources")
        .join(name)
}

/// Same as [`resource`], for APIs that take `&str`.
pub(crate) fn resource_str(name: &str) -> String {
    resource(name).to_string_lossy().into_owned()
}
