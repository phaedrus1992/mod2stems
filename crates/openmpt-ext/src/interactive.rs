//! The interactive channel and instrument controls.

use std::os::raw::c_int;

use crate::error::OpenmptError;
use crate::error::Result;
use crate::info::check_index;
use crate::module::Module;

impl Module {
    /// Mutes or unmutes one channel.
    ///
    /// # Errors
    ///
    /// Returns an error when `channel` is outside `0..num_channels`, when the
    /// library gave no mute function, or when libopenmpt rejects the call.
    pub fn set_channel_mute(&mut self, channel: i32, mute: bool) -> Result<()> {
        check_index("channel mute", "channel", channel, self.num_channels())?;
        let Some(set) = self.interactive().set_channel_mute_status else {
            return Err(missing("set_channel_mute_status"));
        };
        let ext = self.ext();
        self.clear_error();
        // SAFETY: the handle is live, and the channel is in range.
        let ok = unsafe { set(ext, channel, c_int::from(mute)) };
        self.check_ok("channel mute", &format!("channel {channel}"), ok)
    }

    /// Reports whether one channel is muted.
    ///
    /// # Errors
    ///
    /// Returns an error when `channel` is outside `0..num_channels`, when the
    /// library gave no mute function, or when libopenmpt rejects the read.
    pub fn channel_mute(&mut self, channel: i32) -> Result<bool> {
        check_index("channel mute", "channel", channel, self.num_channels())?;
        let Some(get) = self.interactive().get_channel_mute_status else {
            return Err(missing("get_channel_mute_status"));
        };
        let ext = self.ext();
        self.clear_error();
        // SAFETY: the handle is live, and the channel is in range.
        let value = unsafe { get(ext, channel) };
        let input = format!("channel {channel}");
        self.check_last_error("channel mute", &input)?;
        check_mute(self, "channel mute", &input, value)
    }

    /// Sets the volume of one channel.
    ///
    /// libopenmpt stores the volume as a step count out of 64, so a value
    /// between two steps reads back as the nearer step.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is not finite or outside `0.0..=1.0`,
    /// when `channel` is outside `0..num_channels`, when the library gave no
    /// volume function, or when libopenmpt rejects the call.
    pub fn set_channel_volume(&mut self, channel: i32, volume: f64) -> Result<()> {
        check_index("channel volume", "channel", channel, self.num_channels())?;
        check_volume(volume)?;
        let Some(set) = self.interactive().set_channel_volume else {
            return Err(missing("set_channel_volume"));
        };
        let ext = self.ext();
        self.clear_error();
        // SAFETY: the handle is live, and the channel and volume are valid.
        let ok = unsafe { set(ext, channel, volume) };
        self.check_ok("channel volume", &format!("channel {channel}"), ok)
    }

    /// Returns the volume of one channel.
    ///
    /// # Errors
    ///
    /// Returns an error when `channel` is outside `0..num_channels`, when the
    /// library gave no volume function, or when libopenmpt rejects the read.
    pub fn channel_volume(&mut self, channel: i32) -> Result<f64> {
        check_index("channel volume", "channel", channel, self.num_channels())?;
        let Some(get) = self.interactive().get_channel_volume else {
            return Err(missing("get_channel_volume"));
        };
        let ext = self.ext();
        self.clear_error();
        // SAFETY: the handle is live, and the channel is in range.
        let value = unsafe { get(ext, channel) };
        // A volume of 0.0 is a valid value and also the failure value, so the
        // stored error is the only way to tell them apart.
        self.check_last_error("channel volume", format!("channel {channel}"))?;
        Ok(value)
    }

    /// Mutes or unmutes one instrument.
    ///
    /// A module with no instruments indexes its samples instead. The index is
    /// then a sample slot, in `0..num_samples`.
    ///
    /// # Errors
    ///
    /// Returns an error when the index is outside the valid slots, when the
    /// library gave no mute function, or when libopenmpt rejects the call.
    pub fn set_instrument_mute(&mut self, index: i32, mute: bool) -> Result<()> {
        let (count, kind) = self.instrument_slots();
        check_index("instrument mute", kind, index, count)?;
        let Some(set) = self.interactive().set_instrument_mute_status else {
            return Err(missing("set_instrument_mute_status"));
        };
        let ext = self.ext();
        self.clear_error();
        // SAFETY: the handle is live, and the index is in range.
        let ok = unsafe { set(ext, index, c_int::from(mute)) };
        self.check_ok("instrument mute", &format!("{kind} {index}"), ok)
    }

    /// Reports whether one instrument is muted.
    ///
    /// # Errors
    ///
    /// Returns an error when the index is outside the valid slots, when the
    /// library gave no mute function, or when libopenmpt rejects the read.
    pub fn instrument_mute(&mut self, index: i32) -> Result<bool> {
        let (count, kind) = self.instrument_slots();
        check_index("instrument mute", kind, index, count)?;
        let Some(get) = self.interactive().get_instrument_mute_status else {
            return Err(missing("get_instrument_mute_status"));
        };
        let ext = self.ext();
        self.clear_error();
        // SAFETY: the handle is live, and the index is in range.
        let value = unsafe { get(ext, index) };
        let input = format!("{kind} {index}");
        self.check_last_error("instrument mute", &input)?;
        check_mute(self, "instrument mute", &input, value)
    }

    /// Returns the number of mute slots, and the word that names one.
    ///
    /// The library reads the index as an instrument slot when the module has
    /// instruments, and as a sample slot when it has none.
    fn instrument_slots(&self) -> (i32, &'static str) {
        let instruments = self.num_instruments();
        if instruments > 0 {
            (instruments, "instrument")
        } else {
            (self.num_samples(), "sample")
        }
    }
}

/// Builds the error for an interface function that the library did not give.
fn missing(function: &'static str) -> OpenmptError {
    OpenmptError::new(
        "interactive",
        function,
        format!("libopenmpt gave no {function} function"),
        Some("the vendored libopenmpt must build with the interactive interface"),
    )
}

/// Turns the `-1` that a mute getter returns on failure into an error.
fn check_mute(module: &mut Module, op: &'static str, input: &str, value: c_int) -> Result<bool> {
    if value < 0 {
        // The C wrapper returns -1 after it stores an error, so this branch
        // normally follows a stored error. It stays as a guard because a
        // failure with no stored error must not read as "not muted".
        return Err(module.check_last_error(op, input).err().unwrap_or_else(|| {
            OpenmptError::new(
                op,
                input,
                "libopenmpt gave no mute state and stored no error",
                None,
            )
        }));
    }
    Ok(value != 0)
}

/// Rejects a volume that is not finite or not in `0.0..=1.0`.
fn check_volume(volume: f64) -> Result<()> {
    if volume.is_finite() && (0.0..=1.0).contains(&volume) {
        return Ok(());
    }
    Err(OpenmptError::new(
        "channel volume",
        format!("volume {volume}"),
        format!("the volume {volume} is not a finite value in 0.0..=1.0"),
        Some("use a volume in 0.0..=1.0, where 0.0 is silent"),
    ))
}

#[cfg(test)]
mod properties {
    use super::check_volume;
    use proptest::prelude::any;
    use proptest::prelude::prop_assert;
    use proptest::prelude::prop_assert_eq;
    use proptest::proptest;

    proptest! {
        /// The gate decides which volumes reach the mixer. It must accept
        /// exactly the finite values in the closed range that the library
        /// documents.
        #[test]
        fn check_volume_accepts_exactly_the_finite_closed_range(volume in any::<f64>()) {
            let wanted = volume.is_finite() && (0.0..=1.0).contains(&volume);
            prop_assert_eq!(check_volume(volume).is_ok(), wanted);
        }

        /// The ends of the range are valid values, and the values just outside
        /// it are not.
        #[test]
        fn check_volume_pins_the_range_ends(offset in -1.0_f64..1.0) {
            let inside = |value: f64| (0.0..=1.0).contains(&value);
            for edge in [0.0_f64, 1.0] {
                prop_assert_eq!(check_volume(edge).is_ok(), true);
                let outside = edge + offset.signum() * 0.5;
                if !inside(outside) {
                    prop_assert!(check_volume(outside).is_err());
                }
            }
        }
    }
}
