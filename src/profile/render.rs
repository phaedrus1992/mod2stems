//! The render loop that the profile code reads.
//!
//! Every profile render uses the same timeline. `docs/design.md` §4.2 sets
//! `play.at_end` to `stop`, and §4.2 caps the duration, so a pathological
//! module fails the export instead of allocating without a limit.

use openmpt_ext::Module;

use crate::profile::AmigaModel;
use crate::profile::ProfileError;
use crate::profile::apply::enable_amiga;

/// The ctl that selects what the library does at the song end.
pub const AT_END_CTL: &str = "play.at_end";
/// The value that ends the render at the song end, with no fade.
pub const AT_END_STOP: &str = "stop";

/// The frames that one profile read requests.
///
/// A larger block lowers the call count. A smaller block lets the song end
/// land closer to the true frame count, which keeps the two Amiga passes equal
/// when the song has a fractional block.
const BLOCK_FRAMES: usize = 4800;

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
        Some(model) => enable_amiga(module, model, "the Amiga render control set")?,
        None => module
            .ctl_set_bool(crate::profile::apply::AMIGA_CTL, false)
            .map_err(|error| ProfileError::library(&error, "the plain render control set"))?,
    }
    module
        .ctl_set_text(AT_END_CTL, AT_END_STOP)
        .map_err(|error| ProfileError::library(&error, "the render end control set"))?;
    let limit = frame_limit(rate, max_seconds);
    let mut out = Vec::new();
    let mut buf = vec![0.0_f32; BLOCK_FRAMES * 2];
    loop {
        // The limit is checked before the read, so a reached limit stops the
        // loop even when the library would return more samples.
        if out.len() / 2 >= limit {
            return Err(ProfileError::TooLong {
                limit_seconds: max_seconds,
            });
        }
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
/// `rate` is one of the rates that the renderer accepts, so the product is
/// small. `saturating_mul` keeps the limit defined for any rate.
fn frame_limit(rate: i32, max_seconds: u64) -> usize {
    let seconds = usize::try_from(max_seconds).unwrap_or(usize::MAX);
    let rate = usize::try_from(rate).unwrap_or(0);
    seconds.saturating_mul(rate)
}
