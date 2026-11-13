//! Reads one song and records the positions that it visits.
//!
//! Every isolation render uses the profile read loop, so it sets the same
//! `play.at_end` value and the same duration limit. This module adds the row
//! trace, which the isolation guard compares with the unmuted render.

use openmpt_ext::Module;

use crate::profile::ProfileError;
use crate::profile::render::read_song;
use crate::profile::render::trace_block_frames;
use crate::stems::position::Position;
use crate::stems::trace::RowTrace;

/// The result of one isolation render.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardedRender {
    /// The interleaved stereo samples that the render wrote.
    pub samples: Vec<f32>,
    /// The positions that the render visited.
    pub trace: RowTrace,
}

/// Reads one whole song, and records the song flow.
///
/// The module must be at the start of the song, which it is after a load. The
/// caller sets the silencing first, because the read does not touch a channel.
///
/// # Errors
///
/// Returns an error when the module lacks the `play.at_end` control, when a
/// library call fails, or when the render reaches `max_seconds`.
pub fn read_guarded(
    module: &mut Module,
    rate: i32,
    max_seconds: u64,
) -> Result<GuardedRender, ProfileError> {
    read_guarded_with(module, rate, max_seconds, &mut |_| {})
}

/// Reads one whole song, records the song flow, and calls `observe` first.
///
/// `observe` runs before each block, so a caller can inspect the module during
/// the song. The isolation render uses it to prove that a silenced channel
/// stays silenced. A caller that fails a check stores the error and returns it
/// after the read, because the read loop cannot fail on a check.
///
/// # Errors
///
/// Returns an error when the module lacks the `play.at_end` control, when a
/// library call fails, or when the render reaches `max_seconds`.
pub(crate) fn read_guarded_with(
    module: &mut Module,
    rate: i32,
    max_seconds: u64,
    observe: &mut dyn FnMut(&mut Module),
) -> Result<GuardedRender, ProfileError> {
    let mut trace = RowTrace::default();
    // The trace samples only between reads, so one read must not span a row.
    // `trace_block_frames` sets the longest window that a command cannot hide
    // in, and the guard needs the same window.
    let samples = read_song(
        module,
        rate,
        max_seconds,
        trace_block_frames(rate),
        &mut |module| {
            trace.record(Position::new(module.current_order(), module.current_row()));
            observe(module);
        },
    )?;
    Ok(GuardedRender { samples, trace })
}
