//! One file of an export, and the identity that the manifest records.
//!
//! The path is relative to the export root, so a manifest stays valid when the
//! export moves. The identity says which slot the file came from, because the
//! sanitized name is text for a person and can repeat.

use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use crate::manifest::ExportSet;
use crate::manifest::Identity;

/// One file of an export, and the facts that the manifest records about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StemEntry {
    path: PathBuf,
    set: ExportSet,
    channel: i32,
    identity: Identity,
    original_name: Option<String>,
    reused: bool,
}

impl StemEntry {
    /// Returns the entry of one channel stem.
    #[must_use]
    pub fn channel(channel: i32, name: Option<String>, path: PathBuf) -> Self {
        Self {
            path,
            set: ExportSet::Channels,
            channel,
            identity: Identity::Channel,
            original_name: name,
            reused: false,
        }
    }

    /// Returns the entry of one pair stem.
    ///
    /// `identity` is [`Identity::Instrument`] for a module with instruments and
    /// [`Identity::Sample`] for a sample-only module.
    #[must_use]
    pub fn pair(channel: i32, identity: Identity, name: Option<String>, path: PathBuf) -> Self {
        Self {
            path,
            set: ExportSet::Pairs,
            channel,
            identity,
            original_name: name,
            reused: false,
        }
    }

    /// Returns the entry, marked as the reuse of its channel stem.
    ///
    /// A channel with one pair reuses the channel stem as its pair stem. The
    /// manifest says so, so nobody reads the file as a separate render
    /// (`docs/design.md` §4.5).
    #[must_use]
    pub fn reused(mut self) -> Self {
        self.reused = true;
        self
    }

    /// Returns the path, relative to the export root.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the set that the file belongs to.
    #[must_use]
    pub fn set(&self) -> ExportSet {
        self.set
    }

    /// Returns the tracker channel.
    #[must_use]
    pub fn channel_index(&self) -> i32 {
        self.channel
    }

    /// Returns the identity of the file.
    #[must_use]
    pub fn identity(&self) -> Identity {
        self.identity
    }

    /// Returns the name that the module gave the channel or the instrument.
    #[must_use]
    pub fn original_name(&self) -> Option<&str> {
        self.original_name.as_deref()
    }

    /// Returns true when the file reuses a channel stem.
    #[must_use]
    pub fn is_reused(&self) -> bool {
        self.reused
    }
}
