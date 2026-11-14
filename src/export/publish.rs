//! The staged publication of one export.
//!
//! An export writes into a staging directory beside its destination, checks
//! every file that the manifest names, and then renames the directory onto the
//! destination. The rename is one step, so a published directory is always
//! complete. A failure reports the staging path and the files that remain.
//! Design: `docs/design.md` §5.2.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use crate::export::ExportError;
use crate::manifest::MANIFEST_FILE;
use crate::manifest::Manifest;
use crate::manifest::ManifestError;
use crate::manifest::README_FILE;
use crate::manifest::StagingReport;
use crate::manifest::staging::retained_files;
use crate::manifest::staging::staging_path;

/// Creates the staging directory of one export.
///
/// The staging directory sits beside the destination, so the later rename stays
/// on one filesystem. The call creates one directory and never a parent, and it
/// fails when the path holds anything at all. A check followed by a create
/// would let another process take the path between the two calls.
///
/// # Errors
///
/// Returns [`ExportError::Io`] when the directory cannot be created, and
/// [`ExportError::Manifest`] with the staging reason when a staging path of an
/// earlier run still exists.
pub fn create_staging(destination: &Path) -> Result<PathBuf, ExportError> {
    let staging = staging_path(destination);
    match std::fs::create_dir(&staging) {
        Ok(()) => Ok(staging),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(ExportError::Manifest(ManifestError::StagingExists {
                path: staging,
            }))
        }
        Err(error) => Err(ExportError::io(&staging, &error)),
    }
}

/// Checks the staged set, then publishes the staging directory onto the
/// destination.
///
/// The check reads the manifest back from the file, so the published set is the
/// one that the manifest describes. A file that the manifest names and the
/// staging directory does not hold fails the call. So does a file that the
/// staging directory holds and the manifest does not name, because the rename
/// publishes the whole directory.
///
/// # Errors
///
/// Returns [`ExportError::Manifest`] when the manifest cannot be read, a named
/// file is missing, the staging directory holds an unnamed file, or the rename
/// fails.
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

/// Checks that the staging directory holds every named file and nothing else.
fn verify(staging: &Path) -> Result<(), ExportError> {
    let manifest = Manifest::read(&staging.join(MANIFEST_FILE))?;
    let expected = expected_names(&manifest)?;
    for name in &expected {
        require_file(&staging.join(name))?;
    }
    for found in retained_files(staging)? {
        if !expected.contains(&found) {
            return Err(ExportError::Manifest(ManifestError::UnnamedFile {
                path: staging.join(found),
            }));
        }
    }
    Ok(())
}

/// Returns the relative name of every file that the manifest names.
fn expected_names(manifest: &Manifest) -> Result<Vec<String>, ExportError> {
    let mut names = Vec::with_capacity(manifest.stems().len() + 2);
    for entry in manifest.stems() {
        names.push(relative_text(entry.path())?);
    }
    let mixes = [
        manifest.details().mix.as_deref(),
        manifest.details().reference_mix.as_deref(),
    ];
    for name in mixes.into_iter().flatten() {
        names.push(relative_text(Path::new(name))?);
    }
    names.push(MANIFEST_FILE.to_owned());
    names.push(README_FILE.to_owned());
    Ok(names)
}

/// Returns one manifest path as text relative to the export root.
///
/// A manifest is untrusted text, so the call accepts a path below the export
/// root only. Publication joins every path to the staging directory, and an
/// absolute path or a `..` component would name a file outside the export.
fn relative_text(path: &Path) -> Result<String, ExportError> {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(invalid_path(path));
        };
        parts.push(part.to_string_lossy().into_owned());
    }
    if parts.is_empty() {
        return Err(invalid_path(path));
    }
    Ok(parts.join("/"))
}

/// Returns the error of a manifest path that leaves the export root.
fn invalid_path(path: &Path) -> ExportError {
    ExportError::Manifest(ManifestError::InvalidPath {
        path: path.to_path_buf(),
    })
}

/// Fails unless `path` holds a non-empty file.
///
/// The call tells a missing file from an unreadable one, because the two need
/// different fixes.
fn require_file(path: &Path) -> Result<(), ExportError> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.len() > 0 => Ok(()),
        Ok(_) => Err(missing_file(path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(missing_file(path)),
        Err(error) => Err(ExportError::io(path, &error)),
    }
}

/// Returns the error of one named file that the staging directory lacks.
fn missing_file(path: &Path) -> ExportError {
    ExportError::Manifest(ManifestError::MissingFile {
        path: path.to_path_buf(),
    })
}
