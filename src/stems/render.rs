//! Reads one song and records the positions that it visits.
//!
//! Every isolation render uses the profile read loop, so it sets the same
//! `play.at_end` value and the same duration limit. This module adds the row
//! trace, which the isolation guard compares with the unmuted render.

use openmpt_ext::Module;

use crate::profile::ProfileError;
use crate::profile::render::read_song;
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
    let mut trace = RowTrace::default();
    let samples = read_song(module, rate, max_seconds, &mut |order, row| {
        trace.record(Position::new(order, row));
    })?;
    Ok(GuardedRender { samples, trace })
}
