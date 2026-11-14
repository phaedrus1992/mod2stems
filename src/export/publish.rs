//! The staged publication of one export.
//!
//! An export writes into a staging directory beside its destination, checks
//! every file that the manifest names, and then renames the directory onto the
//! destination. The rename is one step, so a published directory is always
//! complete. A failure reports the staging path and the files that remain.
//! Design: `docs/design.md` §5.2.

use std::path::Path;
use std::path::PathBuf;

use crate::export::ExportError;
use crate::manifest::Manifest;
use crate::manifest::ManifestError;
use crate::manifest::StagingReport;
use crate::manifest::staging::retained_files;
use crate::manifest::staging::staging_path;

/// Creates the staging directory of one export.
///
/// The staging directory sits beside the destination, so the later rename stays
/// on one filesystem.
///
/// # Errors
///
/// Returns [`ExportError::Io`] when the directory cannot be created, and
/// [`ExportError::DestinationExists`] when a staging path of an earlier run
/// still exists.
pub fn create_staging(destination: &Path) -> Result<PathBuf, ExportError> {
    let staging = staging_path(destination);
    StagingReport::refuse_existing(&staging)?;
    std::fs::create_dir_all(&staging).map_err(|error| ExportError::io(&staging, &error))?;
    Ok(staging)
}

/// Checks every file that the manifest names, then publishes the staging
/// directory onto the destination.
///
/// The check reads the manifest back from the file, so the published set is the
/// one that the manifest describes. A file that the manifest names and the
/// staging directory does not hold fails the call, and then nothing publishes.
///
/// # Errors
///
/// Returns [`ExportError::Manifest`] when the manifest cannot be read or one
/// named file is missing, and [`ExportError::Manifest`] with the publish reason
/// when the rename fails.
pub fn publish(staging: &Path, destination: &Path) -> Result<(), ExportError> {
    verify(staging)?;
    std::fs::rename(staging, destination).map_err(|error| {
        ExportError::Manifest(ManifestError::Publish {
            staging: staging.to_path_buf(),
            destination: destination.to_path_buf(),
            detail: error.to_string(),
        })
    })
}

/// Returns the report of a staging directory that the export kept.
///
/// # Errors
///
/// Returns [`ExportError::Manifest`] when the directory cannot be read.
pub fn report(staging: &Path) -> Result<StagingReport, ExportError> {
    let retained = retained_files(staging)?;
    Ok(StagingReport::new(staging.to_path_buf()).with_retained(retained))
}

/// Checks every audio file that the manifest names.
fn verify(staging: &Path) -> Result<(), ExportError> {
    let manifest = Manifest::read(&staging.join(crate::manifest::MANIFEST_FILE))?;
    for entry in manifest.stems() {
        let path = staging.join(entry.path());
        let present = std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 0);
        if !present {
            return Err(ExportError::Manifest(ManifestError::MissingFile { path }));
        }
    }
    Ok(())
}
