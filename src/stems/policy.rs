//! The playback policy that every render of one export shares.
//!
//! The policy names one subsong, one repeat count, and one duration limit. It
//! is resolved before any render starts, so every stem and the unmuted render
//! play the same song under the same limit (`docs/design.md` §4.2).
//!
//! The limit is a guard against pathological input, and a render that reaches
//! it fails as incomplete. The limit never shortens a render to a duration
//! estimate (`docs/design.md` §4.2).

use openmpt_ext::Module;

use crate::profile::DEFAULT_MAX_RENDER_SECONDS;
use crate::profile::ProfileError;
use crate::stems::StemError;

/// The subsong and the rendering limits that one export uses.
///
/// The default plays the first subsong, which is the design default. The
/// library also defaults to subsong 0, but it accepts `-1` as "all subsongs",
/// so the policy names its value instead of inheriting one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackPolicy {
    subsong: i32,
    max_seconds: u64,
}

impl PlaybackPolicy {
    /// Returns the policy that plays the first subsong under the default limit.
    #[must_use]
    pub fn default_policy() -> Self {
        Self {
            subsong: 0,
            max_seconds: DEFAULT_MAX_RENDER_SECONDS,
        }
    }

    /// Returns the policy with another subsong.
    ///
    /// The value is checked against the module during [`Self::apply`], because
    /// only a loaded module knows its subsong count.
    #[must_use]
    pub fn with_subsong(self, subsong: i32) -> Self {
        Self { subsong, ..self }
    }

    /// Returns the policy with another duration limit, in seconds.
    ///
    /// A render that reaches the limit fails as incomplete. That failure is
    /// separate from a render that reaches the song end, so the caller never
    /// reads a truncated song as a finished one.
    #[must_use]
    pub fn with_max_seconds(self, max_seconds: u64) -> Self {
        Self {
            max_seconds,
            ..self
        }
    }

    /// Returns the subsong that every render plays.
    #[must_use]
    pub fn subsong(self) -> i32 {
        self.subsong
    }

    /// Returns the maximum duration of one render, in seconds.
    #[must_use]
    pub fn max_seconds(self) -> u64 {
        self.max_seconds
    }

    /// Applies the subsong and the repeat count, and reads both back.
    ///
    /// A module can arrive with a repeat count that replays the order list, so
    /// the policy sets the count on every call instead of relying on the
    /// library default.
    ///
    /// # Errors
    ///
    /// Returns an error when the module does not hold the named subsong, when
    /// the library rejects a call, or when the module reads back a value other
    /// than the one that the policy set.
    pub fn apply(self, module: &mut Module) -> Result<(), StemError> {
        map_library(module.select_subsong(self.subsong), "the subsong selection")?;
        let selected = module.selected_subsong();
        if selected != self.subsong {
            return Err(ProfileError::SettingMismatch {
                setting: "subsong",
                wanted: self.subsong,
                actual: selected,
            }
            .into());
        }
        // `docs/design.md` §4.2 plays one pass of the order list. The library
        // reads a negative count as "repeat forever", which never ends.
        map_library(module.set_repeat_count(0), "the repeat count set")?;
        let count = module.repeat_count();
        if count != 0 {
            return Err(ProfileError::SettingMismatch {
                setting: "repeat count",
                wanted: 0,
                actual: count,
            }
            .into());
        }
        Ok(())
    }
}

/// Wraps one library error with the operation that the policy ran.
fn map_library<T>(
    result: Result<T, openmpt_ext::OpenmptError>,
    operation: &'static str,
) -> Result<T, StemError> {
    result.map_err(|error| ProfileError::library(&error, operation).into())
}

#[cfg(test)]
mod tests {
    use super::PlaybackPolicy;

    #[test]
    fn the_policy_keeps_the_values_that_it_holds() {
        let policy = PlaybackPolicy::default_policy().with_subsong(2);
        assert_eq!(policy.subsong(), 2);
        assert_eq!(
            policy.max_seconds(),
            PlaybackPolicy::default_policy().max_seconds()
        );
    }

    #[test]
    fn with_subsong_keeps_every_other_field() {
        let default = PlaybackPolicy::default_policy();
        let changed = default.with_subsong(1);
        assert_eq!(changed.max_seconds(), default.max_seconds());
    }
}
