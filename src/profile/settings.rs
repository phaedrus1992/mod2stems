//! The one resolved playback configuration that every render shares.

use openmpt_ext::RenderParam;

use crate::profile::AmigaModel;
use crate::profile::Profile;
use crate::profile::config::ProfileConfig;

/// The playback settings that a profile names.
///
/// A `None` field keeps the library default for that setting. The struct holds
/// only the settings that a profile can change, so the resolved config stays
/// small and a new setting is one field, not a parallel record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSettings {
    /// The Amiga filter model, when the `amiga` profile names one.
    pub amiga_model: Option<AmigaModel>,
    /// The interpolation filter length, when a profile names one.
    pub interpolation_filter_length: Option<i32>,
}

impl ResolvedSettings {
    /// Returns the settings that the `compatible` profile resolves to.
    ///
    /// The profile changes none of the library defaults, so the struct holds
    /// no value.
    #[must_use]
    pub fn compatible() -> Self {
        Self {
            amiga_model: None,
            interpolation_filter_length: None,
        }
    }

    /// Returns the settings that the `clean` profile resolves to.
    #[must_use]
    pub fn clean() -> Self {
        Self {
            amiga_model: None,
            interpolation_filter_length: Some(crate::profile::WINDOWED_SINC_FILTER_LENGTH),
        }
    }

    /// Returns the settings that the `amiga` profile resolves to.
    #[must_use]
    pub fn amiga(model: AmigaModel) -> Self {
        Self {
            amiga_model: Some(model),
            interpolation_filter_length: None,
        }
    }

    /// Returns the interpolated render params, in the order to set them.
    ///
    /// The list is empty for a profile that changes no render param.
    #[must_use]
    pub fn render_params(&self) -> Vec<(RenderParam, i32)> {
        let Some(length) = self.interpolation_filter_length else {
            return Vec::new();
        };
        vec![(RenderParam::InterpolationFilterLength, length)]
    }

    /// Builds the config that carries these settings.
    #[must_use]
    pub fn into_config(self, profile: Profile, rate: i32) -> ProfileConfig {
        ProfileConfig::new(profile, rate, self)
    }
}
