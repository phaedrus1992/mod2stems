//! The render loop that the profile code reads.
//!
//! Every profile render uses the same timeline. `docs/design.md` §4.2 sets
//! `play.at_end` to `stop`, and §4.2 caps the duration, so a pathological
//! module fails the export instead of allocating without a limit.

use openmpt_ext::Module;
use openmpt_ext::SAMPLE_RATE_RANGE;

use crate::profile::AmigaModel;
use crate::profile::ProfileError;
use crate::profile::apply::disable_amiga;
use crate::profile::apply::enable_amiga;

/// The ctl that selects what the library does at the song end.
pub const AT_END_CTL: &str = "play.at_end";
/// The value that ends the render at the song end, with no fade.
pub const AT_END_STOP: &str = "stop";

/// The frames that one profile read requests.
///
/// A larger block lowers the call count. A smaller block lets the song end
/// land closer to the true frame count, which keeps the two Amiga passes equal
/// when the song has a fractional block. The profile read records no trace, so
/// a block may span a row.
pub(crate) const PROFILE_BLOCK_FRAMES: usize = 4800;

/// The milliseconds that one trace read covers.
///
/// A player tick is the shortest window in which a command can run, so a read
/// longer than one tick can hide a row and a channel-volume change. Classic
/// mode plays a 2.5 ms tick, and Alternative mode plays a 1 ms tick at tempo
/// 1000, the highest tempo that the library accepts. Both hold at every output
/// rate (`Sndfile.cpp:1853,1861`). Modern mode can play a shorter tick, so a
/// command that starts and ends inside one millisecond still escapes the check.
/// The window is a guard, not a proof.
/// Design: `docs/design.md` §4.5.
const TRACE_MILLIS: i32 = 1;

/// The milliseconds in one second, so a rate in Hz gives a frame count.
const MILLIS_PER_SECOND: i32 = 1_000;

/// Returns the frames that one trace read requests at `rate`.
///
/// The division truncates, so the window is never longer than one millisecond.
/// The result is at least one frame, because a read of zero frames never ends.
pub(crate) fn trace_block_frames(rate: i32) -> usize {
    let frames = rate / MILLIS_PER_SECOND * TRACE_MILLIS;
    usize::try_from(frames).unwrap_or(1).max(1)
}

/// The maximum duration that a profile render reads.
///
/// `docs/design.md` §4.2 requires a maximum duration, because
/// `get_duration_seconds` is an estimate and not an allocation bound. This
/// profile render is not the export writer, so the limit only stops the
/// comparison from a runaway allocation.
pub const DEFAULT_MAX_RENDER_SECONDS: u64 = 3600;

/// Reads one whole song with the named Amiga setting.
///
/// `model` is `None` for a plain pass, and `Some(model)` for an emulated pass.
/// The call sets `play.at_end` to `stop` on both passes, so the same settings
/// reach both. The module must be at the start of the song, which it is after
/// a load.
///
/// # Errors
///
/// Returns an error when the module lacks a control that the pass needs, when
/// a library call fails, or when the render reaches `max_seconds`.
pub fn read_pass(
    module: &mut Module,
    rate: i32,
    model: Option<AmigaModel>,
    max_seconds: u64,
) -> Result<Vec<f32>, ProfileError> {
    match model {
        Some(model) => enable_amiga(module, model)?,
        None => disable_amiga(module)?,
    }
    read_song(module, rate, max_seconds, PROFILE_BLOCK_FRAMES, &mut |_| {})
}

/// Reads one whole song, and calls `observe` before each block.
///
/// The call sets `play.at_end` to `stop` and caps the duration, so a caller
/// never starts a read without either. The isolation renders use the observer
/// to record the song flow and to check the silenced channels during the song.
/// The profile renders pass an observer that does nothing.
///
/// `block_frames` sets how much audio one read covers, and so how often
/// `observe` runs. A caller that checks a mid-song state passes no more than
/// [`trace_block_frames`] for its rate.
///
/// # Errors
///
/// Returns an error when the rate is outside [`SAMPLE_RATE_RANGE`], when the
/// module lacks the `play.at_end` control, when a library call fails, or when
/// the render reaches `max_seconds`.
pub(crate) fn read_song(
    module: &mut Module,
    rate: i32,
    max_seconds: u64,
    block_frames: usize,
    observe: &mut dyn FnMut(&mut Module),
) -> Result<Vec<f32>, ProfileError> {
    // The rate limit is computed before the first read, so a rate outside the
    // renderer range must fail here. The read would report it later, and the
    // caller would read a `TooLong` error for a bad rate instead.
    if !SAMPLE_RATE_RANGE.contains(&rate) {
        return Err(ProfileError::RateOutOfRange {
            rate,
            lowest: *SAMPLE_RATE_RANGE.start(),
            highest: *SAMPLE_RATE_RANGE.end(),
        });
    }
    if !module.ctls().contains(AT_END_CTL) {
        return Err(ProfileError::MissingControl { key: AT_END_CTL });
    }
    module
        .ctl_set_text(AT_END_CTL, AT_END_STOP)
        .map_err(|error| ProfileError::library(&error, "the render end control set"))?;
    // A build can accept the value and keep another one, and a render that
    // fades at the song end would report a stem of another length.
    let at_end = module
        .ctl_get_text(AT_END_CTL)
        .map_err(|error| ProfileError::library(&error, "the render end control read"))?;
    if at_end != AT_END_STOP {
        return Err(ProfileError::SettingTextMismatch {
            setting: AT_END_CTL,
            wanted: AT_END_STOP,
            actual: at_end,
        });
    }
    let limit = frame_limit(rate, max_seconds);
    let mut out = Vec::new();
    let mut buf = vec![0.0_f32; block_frames.saturating_mul(2)];
    loop {
        // The limit is checked before the read, so a reached limit stops the
        // loop even when the library would return more samples.
        if out.len() / 2 >= limit {
            return Err(ProfileError::TooLong {
                limit_seconds: max_seconds,
            });
        }
        observe(module);
        let frames = module
            .read_float(rate, &mut buf)
            .map_err(|error| ProfileError::library(&error, "the profile render"))?;
        if frames == 0 {
            break;
        }
        // The library writes only `frames` frames. The tail of the buffer
        // still holds the previous block, so a copy of the whole buffer would
        // compare stale samples.
        let written = frames.saturating_mul(2);
        out.extend_from_slice(&buf[..written]);
    }
    if out.len() / 2 > limit {
        return Err(ProfileError::TooLong {
            limit_seconds: max_seconds,
        });
    }
    Ok(out)
}

/// Returns the sample count that `max_seconds` allows at `rate`.
///
/// `read_song` rejects a rate outside [`SAMPLE_RATE_RANGE`] before this call,
/// so `rate` is positive and small. `saturating_mul` keeps the limit defined
/// when `max_seconds` is huge.
fn frame_limit(rate: i32, max_seconds: u64) -> usize {
    let seconds = usize::try_from(max_seconds).unwrap_or(usize::MAX);
    let rate = usize::try_from(rate).unwrap_or(0);
    seconds.saturating_mul(rate)
}

#[cfg(test)]
mod tests {
    use super::trace_block_frames;

    #[test]
    fn the_trace_window_follows_the_rate() {
        // The window is a time, so the frame count comes from the rate.
        for rate in [8_000, 44_100, 48_000, 192_000] {
            assert_eq!(
                trace_block_frames(rate),
                usize::try_from(rate / 1_000).unwrap_or(1),
                "the rate {rate} gives another window"
            );
        }
    }

    #[test]
    fn a_trace_window_is_never_zero_frames() {
        // A read of zero frames never ends, so the floor is one frame.
        assert_eq!(trace_block_frames(0), 1);
        assert_eq!(trace_block_frames(999), 1);
    }
}
