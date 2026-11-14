//! The Amiga filter models that the `amiga` profile accepts.

use crate::profile::ProfileError;

/// One Amiga filter model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AmigaModel {
    /// The A500 low-pass filter.
    A500,
    /// The A1200 low-pass filter.
    A1200,
    /// The emulation with no filter.
    Unfiltered,
}

impl AmigaModel {
    /// Returns the CLI spelling of the model, which is also the ctl value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A500 => "a500",
            Self::A1200 => "a1200",
            Self::Unfiltered => "unfiltered",
        }
    }

    /// Reads a model from its CLI spelling.
    ///
    /// # Errors
    ///
    /// Returns [`ProfileError::UnknownModel`] when the text names no model.
    pub fn parse(text: &str) -> Result<Self, ProfileError> {
        match text {
            "a500" => Ok(Self::A500),
            "a1200" => Ok(Self::A1200),
            "unfiltered" => Ok(Self::Unfiltered),
            other => Err(ProfileError::UnknownModel {
                name: other.to_owned(),
            }),
        }
    }
}
