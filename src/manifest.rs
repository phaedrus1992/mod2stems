//! The export layout, the file names, and the manifest.
//!
//! One export writes one indexed file per channel, plus an optional pair set
//! under `instrument-channels/`. The manifest records what the export held, so
//! a later reader can prove what each file is. Design: `docs/design.md` §5.2.
//!
//! Publication is staged. An export writes into a directory on the same
//! filesystem, checks every expected file, then publishes it. A failure never
//! leaves a partial set under the final name.

pub mod details;
pub mod entry;
pub mod error;
pub mod file_name;
pub mod identity;
// The module holds the `Manifest` type, so its name repeats the directory name.
// The type is the whole job of the module, and a second name would name the same thing.
#[expect(
    clippy::module_inception,
    reason = "the module holds the Manifest type, and the type is its whole job"
)]
pub mod manifest;
pub mod set;
pub mod staging;

/// The directory that holds the pair stems.
///
/// The pair set is another decomposition of the same audio, so it stays in its
/// own directory with its own mix (`docs/design.md` §5.2).
pub const PAIR_DIRECTORY: &str = "instrument-channels";

/// The file that holds the manifest of one export.
pub const MANIFEST_FILE: &str = "manifest.json";

/// The file that labels the two sets as alternatives.
pub const README_FILE: &str = "README.txt";

/// The staging directory prefix, relative to the destination's parent.
pub const STAGING_PREFIX: &str = ".mod2stems-staging-";

pub use details::ExportDetails;
pub use entry::StemEntry;
pub use error::ManifestError;
pub use file_name::file_name;
pub use file_name::pair_file_name;
pub use file_name::sample_file_name;
pub use file_name::sanitize;
pub use identity::Identity;
pub use manifest::Manifest;
pub use set::ExportSet;
pub use staging::StagingReport;
