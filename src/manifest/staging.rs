//! The staging directory of one export, and the files that it keeps.
//!
//! An export writes into a staging directory, checks every expected file, then
//! publishes it. A failure reports the staging path and the files that remain,
//! so the caller never publishes a partial set and never loses track of one
//! (`docs/design.md` §5.2).

use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use crate::manifest::ManifestError;
use crate::manifest::STAGING_PREFIX;

/// The staging directory of one export and the files that it kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagingReport {
    path: PathBuf,
    retained: Vec<String>,
}

impl StagingReport {
    /// Returns the report of one staging directory.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            retained: Vec::new(),
        }
    }

    /// Returns the report with the files that the failed export kept.
    #[must_use]
    pub fn with_retained(mut self, retained: Vec<String>) -> Self {
        self.retained = retained;
        self
    }

    /// Returns the staging path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the files that the staging directory still holds.
    #[must_use]
    pub fn retained(&self) -> &[String] {
        &self.retained
    }

    /// Returns the report of a directory that held files when it was read.
    ///
    /// The export calls this before it publishes, so a directory that already
    /// held a file fails instead of being renamed over.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::DestinationExists`] when the path holds an
    /// entry.
    pub fn refuse_existing(path: &Path) -> Result<(), ManifestError> {
        match std::fs::symlink_metadata(path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(ManifestError::io(path, &error)),
            Ok(_) => Err(ManifestError::DestinationExists {
                path: path.to_path_buf(),
            }),
        }
    }
}

impl fmt::Display for StagingReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "The export did not publish. The staging directory {} stays for you to \
             inspect.",
            self.path.display()
        )?;
        if self.retained.is_empty() {
            return Ok(());
        }
        write!(formatter, " It holds {} file(s):", self.retained.len())?;
        for name in &self.retained {
            write!(formatter, " {name}")?;
        }
        Ok(())
    }
}

/// Returns the staging path for one destination.
///
/// The path sits beside the destination and shares its parent, so a rename from
/// the staging directory to the destination stays on one filesystem. The name
/// holds the process id, so two exports to different destinations never share a
/// staging directory.
#[must_use]
pub(crate) fn staging_path(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .map_or_else(|| "export".into(), std::ffi::OsString::from);
    let mut staging = std::ffi::OsString::from(STAGING_PREFIX);
    staging.push(name);
    staging.push(format!("-{}", std::process::id()));
    destination
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(staging)
}

/// Returns the files that one directory holds, sorted, as relative names.
///
/// The walk is one level deep plus the pair directory, which is the only
/// subdirectory that an export writes.
///
/// # Errors
///
/// Returns [`ManifestError::Io`] when the directory cannot be read.
pub(crate) fn retained_files(export: &Path) -> Result<Vec<String>, ManifestError> {
    let mut names = Vec::new();
    collect(export, "", &mut names)?;
    let pairs = export.join(crate::manifest::PAIR_DIRECTORY);
    if pairs.is_dir() {
        collect(&pairs, crate::manifest::PAIR_DIRECTORY, &mut names)?;
    }
    names.sort();
    Ok(names)
}

/// Appends the entries of one directory under `prefix`.
fn collect(directory: &Path, prefix: &str, names: &mut Vec<String>) -> Result<(), ManifestError> {
    for entry in
        std::fs::read_dir(directory).map_err(|error| ManifestError::io(directory, &error))?
    {
        let entry = entry.map_err(|error| ManifestError::io(directory, &error))?;
        let name = entry.file_name();
        let text = name.to_string_lossy();
        if prefix.is_empty() {
            names.push(text.into_owned());
        } else {
            names.push(format!("{prefix}/{text}"));
        }
    }
    Ok(())
}
