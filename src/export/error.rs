//! The error type of one export run.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use crate::encode::EncodeError;
use crate::manifest::ManifestError;
use crate::manifest::StagingReport;
use crate::stems::StemError;

/// One failed export.
///
/// Every variant names the operation, the input, and the fix, because the
/// command prints this text and returns.
#[derive(Debug)]
pub enum ExportError {
    /// The input module cannot be read.
    Input {
        /// The path that the caller named.
        path: PathBuf,
        /// The reason from the filesystem.
        detail: String,
    },
    /// A library call failed.
    ///
    /// The call names the operation that failed and the fix that the caller can
    /// apply. A module that the build rejects is the usual case.
    Library {
        /// The operation that failed.
        operation: &'static str,
        /// The reason from the library.
        detail: String,
        /// The fix that the caller applies.
        fix: String,
    },
    /// The output directory already holds something.
    ///
    /// The initial release refuses an existing destination rather than
    /// replacing its contents (`docs/design.md` §5.2).
    DestinationExists {
        /// The path that the caller named.
        path: PathBuf,
    },
    /// The encoder cannot store the resolved output rate.
    ///
    /// The renderer accepts the rate and the encoder does not, so the export
    /// must fail before any render starts (`docs/design.md` §5.1).
    RateUnsupported {
        /// The rate that the caller named.
        rate: i32,
        /// The format that cannot store it.
        format: &'static str,
    },
    /// The caller asked for pair stems, and this release does not build them.
    PairsUnavailable,
    /// The export is louder than the integer format holds.
    ///
    /// The integer path never clips silently, so the export fails before it
    /// writes a file and the caller chooses one common attenuation
    /// (`docs/design.md` §5.1).
    OverRange {
        /// The largest sample magnitude that the export holds.
        peak: f32,
        /// The format that cannot hold it.
        format: String,
        /// The largest magnitude that the format holds.
        highest: f32,
        /// The common gain that the caller chose.
        gain: f32,
        /// The common gain that would bring the export inside the range.
        required: f32,
    },
    /// The export stopped because the caller cancelled it.
    ///
    /// The staging directory stays, and the export never publishes it
    /// (`docs/design.md` §5.2).
    Cancelled(Box<StagingReport>),
    /// One step of the export failed, and the staging directory stays.
    ///
    /// The report names the files that the failed export kept, so the caller
    /// can see the work and remove it deliberately.
    Failed {
        /// The error of the failed step.
        cause: Box<ExportError>,
        /// The staging directory that the export kept.
        staging: Box<StagingReport>,
    },
    /// A render failed.
    Stem(StemError),
    /// An encode failed.
    Encode(EncodeError),
    /// A layout, manifest, or publication step failed.
    Manifest(ManifestError),
    /// The export cannot report its progress.
    Progress {
        /// The reason from the sink.
        detail: String,
        /// Whether the export published before the failure.
        ///
        /// The rename is the last step, so a sink failure after it leaves a
        /// complete export. The message must say so, because a caller reads
        /// the failure as a stopped export.
        published: bool,
    },
    /// A file of the export cannot be written or read.
    Io {
        /// The path that the call touched.
        path: PathBuf,
        /// The reason from the filesystem.
        detail: String,
    },
}

impl ExportError {
    /// Returns the I/O error for one path.
    pub(crate) fn io(path: &std::path::Path, error: &std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            detail: error.to_string(),
        }
    }

    /// Returns the error of a sink that cannot report.
    ///
    /// `published` says whether the export reached its rename.
    pub(crate) fn progress(error: &std::io::Error, published: bool) -> Self {
        Self::Progress {
            detail: error.to_string(),
            published,
        }
    }

    /// Returns the error of one failed library call.
    pub(crate) fn library(
        operation: &'static str,
        error: &openmpt_ext::OpenmptError,
        fix: &str,
    ) -> Self {
        Self::Library {
            operation,
            detail: error.to_string(),
            fix: fix.to_owned(),
        }
    }

    /// Returns the path of the staging directory that the export kept.
    #[must_use]
    pub fn staging(&self) -> Option<&StagingReport> {
        match self {
            Self::Cancelled(report)
            | Self::Failed {
                staging: report, ..
            } => Some(report),
            _ => None,
        }
    }
}

impl From<StemError> for ExportError {
    fn from(error: StemError) -> Self {
        Self::Stem(error)
    }
}

impl From<EncodeError> for ExportError {
    fn from(error: EncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<ManifestError> for ExportError {
    fn from(error: ManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input { path, detail } => write!(
                formatter,
                "Cannot read the input module {}: {detail}. Check the path and its permissions.",
                path.display()
            ),
            Self::Library {
                operation,
                detail,
                fix,
            } => write!(formatter, "Cannot run {operation}: {detail}. {fix}"),
            Self::DestinationExists { path } => write!(
                formatter,
                "Cannot export to {}: the destination already exists. Choose a new directory, or \
                 remove the old one first, because this release never replaces an export.",
                path.display()
            ),
            Self::RateUnsupported { rate, format } => write!(
                formatter,
                "Cannot encode at {rate} Hz as {format}: the format cannot store that rate. Choose \
                 another rate, such as 48000, or choose float WAV.",
            ),
            Self::PairsUnavailable => write!(
                formatter,
                "Cannot write instrument/channel pair stems: this release does not build them. \
                 Leave --instrument-channel-stems off, and follow #12 and #13 for the pair set."
            ),
            Self::OverRange {
                peak,
                format,
                highest,
                gain,
                required,
            } => write!(
                formatter,
                "Cannot export as {format}: a sample reaches {peak} after the common gain of \
                 {gain}, and {format} holds at most {highest}. Set --gain to {required} or less, \
                 or export as float WAV."
            ),
            Self::Cancelled(report) => write!(formatter, "The export stopped: {report}"),
            Self::Failed { cause, staging } => {
                write!(formatter, "{cause} The export did not publish. {staging}")
            }
            Self::Stem(error) => write!(formatter, "Cannot render the stems: {error}"),
            Self::Encode(error) => write!(formatter, "Cannot encode the stems: {error}"),
            Self::Manifest(error) => write!(formatter, "{error}"),
            Self::Progress {
                detail,
                published: false,
            } => write!(
                formatter,
                "Cannot report the export progress: {detail}. The export stopped before it \
                 published anything."
            ),
            Self::Progress {
                detail,
                published: true,
            } => write!(
                formatter,
                "Cannot report the export progress: {detail}. The export published, so the \
                 directory is complete. Check the destination before you run the export again."
            ),
            Self::Io { path, detail } => write!(
                formatter,
                "Cannot touch {}: {detail}. Check that the parent directory exists and is \
                 writable.",
                path.display()
            ),
        }
    }
}

impl Error for ExportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Stem(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Manifest(error) => Some(error),
            Self::Failed { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
