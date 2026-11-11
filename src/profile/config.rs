//! The resolved profile config that every render shares.

use crate::profile::AmigaModel;
use crate::profile::Profile;
use crate::profile::settings::ResolvedSettings;

/// The one playback configuration that every stem and reference render uses.
///
/// The resolver builds this value once, before any render starts. The value
/// records the profile, the output rate, and the resolved settings, so the
/// export metadata can record it without a second lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileConfig {
    profile: Profile,
    rate: i32,
    settings: ResolvedSettings,
}

impl ProfileConfig {
    /// Builds a config from a resolved profile, a validated rate, and its settings.
    pub(crate) fn new(profile: Profile, rate: i32, settings: ResolvedSettings) -> Self {
        Self {
            profile,
            rate,
            settings,
        }
    }

    /// Returns the profile that this config resolves.
    #[must_use]
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Returns the output sample rate, in Hz.
    #[must_use]
    pub fn rate(&self) -> i32 {
        self.rate
    }

    /// Returns the Amiga filter model, when the profile names one.
    #[must_use]
    pub fn amiga_model(&self) -> Option<AmigaModel> {
        self.settings.amiga_model
    }

    /// Reports whether the profile enables Amiga emulation.
    #[must_use]
    pub fn amiga_enabled(&self) -> bool {
        self.settings.amiga_model.is_some()
    }

    /// Returns the interpolation filter length, when the profile names one.
    #[must_use]
    pub fn interpolation_filter_length(&self) -> Option<i32> {
        self.settings.interpolation_filter_length
    }

    /// Returns the settings that this config carries.
    #[must_use]
    pub fn settings(&self) -> &ResolvedSettings {
        &self.settings
    }
}
