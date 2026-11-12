//! The one resolved playback configuration that every render shares.

use openmpt_ext::RenderParam;

use crate::profile::AmigaModel;

/// The playback settings that a profile names.
///
/// A `None` field keeps the library default for that setting. The struct holds
/// only the settings that a profile can change, so the resolved config stays
/// small and a new setting is one field, not a parallel record.
///
/// The fields are private. A caller reaches a config through
/// [`crate::ProfileRequest::resolve`], which rejects a conflicting combination
/// before a render starts. A public field would let a caller build the Amiga
/// model and an interpolation length together, and the library applies the
/// model and ignores the length for that pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSettings {
    amiga_model: Option<AmigaModel>,
    interpolation_filter_length: Option<i32>,
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

    /// Returns the Amiga filter model, when the profile names one.
    #[must_use]
    pub fn amiga_model(&self) -> Option<AmigaModel> {
        self.amiga_model
    }

    /// Returns the interpolation filter length, when the profile names one.
    #[must_use]
    pub fn interpolation_filter_length(&self) -> Option<i32> {
        self.interpolation_filter_length
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
}
