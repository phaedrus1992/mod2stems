//! The input content hash that the manifest records.
//!
//! The manifest names the input by content, so a later reader proves which
//! module bytes produced the export. A path alone would name nothing after a
//! file moves or changes.

use std::fmt::Write as _;

use sha2::Digest;
use sha2::Sha256;

/// Returns the SHA-256 digest of `bytes`, with a `sha256:` prefix.
///
/// The prefix states the algorithm, so a reader never has to guess which digest
/// the value holds.
#[must_use]
pub fn hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(7 + digest.len().saturating_mul(2));
    text.push_str("sha256:");
    for byte in digest {
        // Writing to a `String` cannot fail.
        let _ = write!(text, "{byte:02x}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::hash;

    #[test]
    fn the_hash_names_the_algorithm_and_fills_32_bytes() {
        // An empty input has a known digest, so a change in the encoding of the
        // value fails here and not in a published manifest.
        let digest = hash(b"");
        assert_eq!(
            digest,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_hash_changes_with_the_content() {
        assert_ne!(hash(b"a"), hash(b"b"));
    }
}
