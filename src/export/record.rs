//! The files of one staged export, and the manifest that records them.
//!
//! The staging directory holds every file under its final relative name, so
//! publication is one rename of a whole directory. The manifest names each
//! audio file with its identity, and the README labels the two sets as
//! alternatives. Design: `docs/design.md` §5.2.

use std::path::Path;
use std::path::PathBuf;

use crate::export::ExportError;
use crate::manifest::ExportDetails;
use crate::manifest::MANIFEST_FILE;
use crate::manifest::Manifest;
use crate::manifest::StemEntry;

/// The files of one staged export.
///
/// The value holds the path of each audio file, under its final relative name,
/// and the entries that the manifest records for them. A file that the export
/// did not write has no entry.
#[derive(Debug, Default)]
pub struct StagedFiles {
    files: Vec<(StemEntry, PathBuf)>,
    mix: Option<PathBuf>,
    reference_mix: Option<PathBuf>,
}

impl StagedFiles {
    /// Returns the staged files of one export.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one channel stem.
    pub fn add_stem(&mut self, entry: StemEntry, path: PathBuf) {
        self.files.push((entry, path));
    }

    /// Adds the mix of the channel stems.
    pub fn add_mix(&mut self, path: PathBuf) {
        self.mix = Some(path);
    }

    /// Adds the unmuted render as audio.
    pub fn add_reference_mix(&mut self, path: PathBuf) {
        self.reference_mix = Some(path);
    }

    /// Returns the staged entries, in write order.
    #[must_use]
    pub fn entries(&self) -> Vec<StemEntry> {
        self.files.iter().map(|(entry, _)| entry.clone()).collect()
    }

    /// Returns the staged audio paths.
    #[must_use]
    pub fn paths(&self) -> Vec<&Path> {
        self.files.iter().map(|(_, path)| path.as_path()).collect()
    }

    /// Returns the staged mix path.
    #[must_use]
    pub fn mix(&self) -> Option<&Path> {
        self.mix.as_deref()
    }

    /// Returns the staged reference mix path.
    #[must_use]
    pub fn reference_mix(&self) -> Option<&Path> {
        self.reference_mix.as_deref()
    }

    /// Returns the relative name of the mix, for the manifest.
    #[must_use]
    pub fn mix_name(&self) -> Option<String> {
        relative_name(self.mix.as_deref())
    }

    /// Returns the relative name of the reference mix, for the manifest.
    #[must_use]
    pub fn reference_mix_name(&self) -> Option<String> {
        relative_name(self.reference_mix.as_deref())
    }

    /// Returns every audio path that the export wrote.
    #[must_use]
    pub fn audio_paths(&self) -> Vec<&Path> {
        let mut paths = self.paths();
        paths.extend(self.mix.as_deref());
        paths.extend(self.reference_mix.as_deref());
        paths
    }

    /// Writes the manifest of the export into `staging`.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::Manifest`] when the manifest cannot be built or
    /// written, which happens when two entries name one file.
    pub fn write_manifest(
        &self,
        staging: &Path,
        details: ExportDetails,
    ) -> Result<(), ExportError> {
        let manifest = Manifest::new(details, self.entries())?;
        manifest.write(&staging.join(MANIFEST_FILE))?;
        Ok(())
    }
}

/// Returns the last component of one staged path, or `None`.
///
/// A staged file sits directly in the staging directory, so its file name is
/// its path relative to the export root.
fn relative_name(path: Option<&Path>) -> Option<String> {
    let name = path?.file_name()?;
    Some(name.to_string_lossy().into_owned())
}
