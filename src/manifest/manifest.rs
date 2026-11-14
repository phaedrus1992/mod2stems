//! The manifest that one export writes beside its audio files.
//!
//! The manifest is the durable record of what the files are. A later reader
//! proves the provenance of an export from it, so it names the input digest,
//! the resolved settings, and every file with its identity. Design:
//! `docs/design.md` §5.2.

use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;

use crate::manifest::ExportDetails;
use crate::manifest::ManifestError;
use crate::manifest::StemEntry;

/// The record that one export writes as `manifest.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    details: ExportDetails,
    stems: Vec<StemEntry>,
}

impl Manifest {
    /// Builds the manifest of one export.
    ///
    /// The stems keep the order that the caller passes, so the export writes
    /// them in channel order.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Collision`] when two entries name one path. Such
    /// a set overwrites one stem and publishes an incomplete export.
    pub fn new(details: ExportDetails, stems: Vec<StemEntry>) -> Result<Self, ManifestError> {
        let mut seen = HashSet::with_capacity(stems.len());
        for stem in &stems {
            if !seen.insert(stem.path()) {
                return Err(ManifestError::Collision {
                    name: stem.path().display().to_string(),
                });
            }
        }
        Ok(Self { details, stems })
    }

    /// Returns the export-level facts.
    #[must_use]
    pub fn details(&self) -> &ExportDetails {
        &self.details
    }

    /// Returns the files of the export, in write order.
    #[must_use]
    pub fn stems(&self) -> &[StemEntry] {
        &self.stems
    }

    /// Returns the manifest as indented JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Json`] when the value cannot be serialized.
    pub fn to_json(&self) -> Result<String, ManifestError> {
        serde_json::to_string_pretty(self).map_err(|error| ManifestError::Json {
            detail: error.to_string(),
        })
    }

    /// Reads a manifest from JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Json`] when the text is not valid JSON, or when
    /// it omits a required field.
    pub fn from_json(text: &str) -> Result<Self, ManifestError> {
        serde_json::from_str(text).map_err(|error| ManifestError::Json {
            detail: error.to_string(),
        })
    }

    /// Reads a manifest from the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Io`] when the file cannot be read, and
    /// [`ManifestError::Json`] when it is not a valid manifest.
    pub fn read(path: &std::path::Path) -> Result<Self, ManifestError> {
        let text =
            std::fs::read_to_string(path).map_err(|error| ManifestError::io(path, &error))?;
        Self::from_json(&text)
    }

    /// Writes the manifest to the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Io`] when the file cannot be written.
    pub fn write(&self, path: &std::path::Path) -> Result<(), ManifestError> {
        let text = self.to_json()?;
        std::fs::write(path, text).map_err(|error| ManifestError::io(path, &error))
    }
}
