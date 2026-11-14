//! The error type of the export layout and the manifest.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// One failed layout, manifest, or publication step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// Two entries name one file.
    ///
    /// A collision overwrites a stem and publishes a set with a missing file
    /// and no message (`docs/design.md` §5.2).
    Collision {
        /// The file name that two entries share.
        name: String,
    },
    /// The destination already holds something.
    ///
    /// The initial release refuses an existing destination rather than
    /// replacing its contents (`docs/design.md` §5.2).
    DestinationExists {
        /// The path that the caller named.
        path: PathBuf,
    },
    /// A file that the manifest names does not exist, or holds no frame.
    MissingFile {
        /// The path of the missing file.
        path: PathBuf,
    },
    /// The staged set cannot be moved onto the destination.
    Publish {
        /// The staging directory.
        staging: PathBuf,
        /// The destination directory.
        destination: PathBuf,
        /// The reason from the filesystem.
        detail: String,
    },
    /// The manifest cannot be written or read.
    Io {
        /// The path that the call touched.
        path: PathBuf,
        /// The reason from the filesystem.
        detail: String,
    },
    /// The manifest text is not valid JSON, or holds no required field.
    Json {
        /// The reason from the parser.
        detail: String,
    },
}

impl ManifestError {
    /// Returns the I/O error for one path.
    pub(crate) fn io(path: &std::path::Path, error: &std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            detail: error.to_string(),
        }
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Collision { name } => write!(
                formatter,
                "Cannot write the export: two entries name the file {name}. Report this, because \
                 the index alone must decide the path."
            ),
            Self::DestinationExists { path } => write!(
                formatter,
                "Cannot export to {}: the destination already exists. Choose a new directory, \
                 or remove the old one first, because this release never replaces an export.",
                path.display()
            ),
            Self::MissingFile { path } => write!(
                formatter,
                "Cannot publish the export: {} is missing or empty, and the manifest names it. \
                 Check the render step, because a published set must hold every file it lists.",
                path.display()
            ),
            Self::Publish {
                staging,
                destination,
                detail,
            } => write!(
                formatter,
                "Cannot publish {} onto {}: {detail}. The staged files stay at the staging path; \
                 move or remove them after you check the disk.",
                staging.display(),
                destination.display()
            ),
            Self::Io { path, detail } => write!(
                formatter,
                "Cannot touch {}: {detail}. Check that the parent directory exists and is \
                 writable.",
                path.display()
            ),
            Self::Json { detail } => write!(
                formatter,
                "Cannot read the manifest: {detail}. Use the manifest from the same tool version \
                 that wrote it."
            ),
        }
    }
}

impl Error for ManifestError {}
