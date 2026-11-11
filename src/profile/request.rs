//! The caller's profile request, and its resolution to one config.

use openmpt_ext::SAMPLE_RATE_RANGE;

use crate::profile::AmigaModel;
use crate::profile::DEFAULT_OUTPUT_RATE;
use crate::profile::Profile;
use crate::profile::ProfileConfig;
use crate::profile::ProfileError;
use crate::profile::settings::ResolvedSettings;

/// A profile request, before resolution.
///
/// The request holds what the caller named. Resolution checks the combination
/// and finds the one config that every render shares. Exact CLI spelling can
/// settle later; this type is the seam between the command and the profiles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileRequest {
    profile: Profile,
    amiga_model: Option<AmigaModel>,
    rate: Option<i32>,
    interpolation_filter_length: Option<i32>,
}

impl ProfileRequest {
    /// Builds a `compatible` request that names no rate.
    #[must_use]
    pub fn compatible() -> Self {
        Self::default()
    }

    /// Builds a `clean` request that names no rate.
    #[must_use]
    pub fn clean() -> Self {
        Self {
            profile: Profile::Clean,
            ..Self::default()
        }
    }

    /// Builds an `amiga` request with an optional filter model.
    ///
    /// Resolution fails when the model is `None`.
    #[must_use]
    pub fn amiga(model: Option<AmigaModel>) -> Self {
        Self {
            profile: Profile::Amiga,
            amiga_model: model,
            ..Self::default()
        }
    }

    /// Returns the profile that the request names.
    #[must_use]
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Returns the request with the named profile.
    #[must_use]
    pub fn with_profile(mut self, profile: Profile) -> Self {
        self.profile = profile;
        self
    }

    /// Returns the request with the named Amiga model.
    #[must_use]
    pub fn with_amiga_model(mut self, model: AmigaModel) -> Self {
        self.amiga_model = Some(model);
        self
    }

    /// Returns the request with the named output rate.
    #[must_use]
    pub fn with_rate(mut self, rate: i32) -> Self {
        self.rate = Some(rate);
        self
    }

    /// Returns the request with an explicit interpolation filter length.
    ///
    /// The advanced interpolation controls take this path. A profile does not
    /// name one, and a request that does conflicts with Amiga emulation.
    #[must_use]
    pub fn with_interpolation_filter_length(mut self, length: i32) -> Self {
        self.interpolation_filter_length = Some(length);
        self
    }

    /// Returns the rate that the request resolves to.
    #[must_use]
    pub fn rate(&self) -> i32 {
        self.rate.unwrap_or(DEFAULT_OUTPUT_RATE)
    }

    /// Resolves the request to the one config that every render shares.
    ///
    /// # Errors
    ///
    /// Returns an error when the profile needs an Amiga model and the request
    /// names none, when the request names a model that its profile does not
    /// use, when it sets an interpolation length next to Amiga emulation, or
    /// when the rate lies outside [`openmpt_ext::SAMPLE_RATE_RANGE`].
    pub fn resolve(self) -> Result<ProfileConfig, ProfileError> {
        let rate = self.checked_rate()?;
        let settings = self.resolve_settings()?;
        Ok(settings.into_config(self.profile, rate))
    }

    /// Rejects an out-of-range rate before any render starts.
    fn checked_rate(&self) -> Result<i32, ProfileError> {
        let rate = self.rate();
        if SAMPLE_RATE_RANGE.contains(&rate) {
            return Ok(rate);
        }
        Err(ProfileError::RateOutOfRange {
            rate,
            lowest: *SAMPLE_RATE_RANGE.start(),
            highest: *SAMPLE_RATE_RANGE.end(),
        })
    }

    /// Resolves the settings for the requested profile.
    fn resolve_settings(&self) -> Result<ResolvedSettings, ProfileError> {
        match self.profile {
            Profile::Compatible => self.resolve_compatible(),
            Profile::Clean => self.resolve_clean(),
            Profile::Amiga => self.resolve_amiga(),
        }
    }

    /// Resolves the `compatible` settings.
    ///
    /// The profile changes no library default, so a model or an interpolation
    /// length in the request is a caller error, not a setting to apply.
    fn resolve_compatible(&self) -> Result<ResolvedSettings, ProfileError> {
        self.reject_model(Profile::Compatible)?;
        self.reject_interpolation(Profile::Compatible)?;
        Ok(ResolvedSettings::compatible())
    }

    /// Resolves the `clean` settings.
    fn resolve_clean(&self) -> Result<ResolvedSettings, ProfileError> {
        self.reject_model(Profile::Clean)?;
        if self.interpolation_filter_length.is_some() {
            return Err(ProfileError::ConflictingSettings {
                first: "the clean interpolation filter",
                second: "an explicit interpolation filter length",
            });
        }
        Ok(ResolvedSettings::clean())
    }

    /// Resolves the `amiga` settings.
    ///
    /// Amiga emulation overrides ordinary interpolation, and the library
    /// accepts both without a complaint. This resolver rejects the pair.
    fn resolve_amiga(&self) -> Result<ResolvedSettings, ProfileError> {
        let Some(model) = self.amiga_model else {
            return Err(ProfileError::MissingAmigaModel);
        };
        if self.interpolation_filter_length.is_some() {
            return Err(ProfileError::ConflictingSettings {
                first: "Amiga emulation",
                second: "an explicit interpolation filter length",
            });
        }
        Ok(ResolvedSettings::amiga(model))
    }

    /// Rejects an Amiga model that the named profile does not use.
    fn reject_model(&self, profile: Profile) -> Result<(), ProfileError> {
        let Some(model) = self.amiga_model else {
            return Ok(());
        };
        Err(ProfileError::UnexpectedAmigaModel {
            profile: profile.as_str(),
            model: model.as_str(),
        })
    }

    /// Rejects an explicit interpolation length for a profile that names one.
    fn reject_interpolation(&self, profile: Profile) -> Result<(), ProfileError> {
        if self.interpolation_filter_length.is_none() {
            return Ok(());
        }
        Err(ProfileError::ConflictingSettings {
            first: profile.as_str(),
            second: "an explicit interpolation filter length",
        })
    }
}
