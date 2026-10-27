//! Shared helpers for the integration tests.

use std::path::PathBuf;

/// Path to a file inside the repository's `resources/` directory.
///
/// Resolved from this crate's manifest so the tests work whatever the working directory is.
pub fn resource(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("resources")
        .join(name)
}

/// Same as [`resource`], for APIs that take `&str`.
pub fn resource_str(name: &str) -> String {
    resource(name).to_string_lossy().into_owned()
}
