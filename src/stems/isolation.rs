//! Renders one channel in isolation and proves the isolation held.
//!
//! Every channel render starts from a fresh module, applies the shared profile,
//! silences every other channel, and reads the whole song. Two checks follow
//! the read: the silenced channels still read as silenced, and the song flow
//! matches the unmuted render. Design: `docs/design.md` §4.1.

use openmpt_ext::Module;

use crate::profile::DEFAULT_MAX_RENDER_SECONDS;
use crate::profile::ProfileConfig;
use crate::profile::ProfileError;
use crate::profile::apply::apply_profile;
use crate::stems::StemError;
use crate::stems::render::GuardedRender;
use crate::stems::render::read_guarded;
use crate::stems::silencing::Silencing;
use crate::stems::silencing::silencing_for;
use crate::stems::trace::RowTrace;

/// Reads the whole song with no channel silenced.
///
/// The trace that this call returns is the flow that every channel render must
/// match. The samples stay available, because the common timeline comes from
/// this render (`docs/design.md` §4.2).
///
/// # Errors
///
/// Returns an error when the profile does not apply or when the render fails.
pub fn reference_render(
    module: &mut Module,
    config: &ProfileConfig,
) -> Result<GuardedRender, StemError> {
    apply_profile(config, module)?;
    Ok(read_guarded(
        module,
        config.rate(),
        DEFAULT_MAX_RENDER_SECONDS,
    )?)
}

/// Renders one channel, with every other channel silenced.
///
/// The module must be a fresh instance at the start of the song. The caller
/// discards it after the call, because a failed check leaves it partly
/// silenced.
///
/// # Errors
///
/// Returns an error when the channel is outside the module, when the profile
/// does not apply, when the silencing fails, when a silenced channel does not
/// stay silenced, or when the song flow differs from `reference`.
pub fn export_channel(
    module: &mut Module,
    config: &ProfileConfig,
    channel: i32,
    reference: &RowTrace,
) -> Result<GuardedRender, StemError> {
    let channels = module.num_channels();
    if !(0..channels).contains(&channel) {
        return Err(StemError::ChannelOutOfRange { channel, channels });
    }
    apply_profile(config, module)?;
    let method = silencing_for(&module_type(module)?)?;
    let silenced = silence(module, method, channel)?;
    let render = read_guarded(module, config.rate(), DEFAULT_MAX_RENDER_SECONDS)?;
    verify_silence(module, method, channel, &silenced)?;
    require_same_flow(channel, &render.trace, reference)?;
    Ok(render)
}

/// Returns the type token that selects the silencing method.
fn module_type(module: &Module) -> Result<String, ProfileError> {
    module
        .metadata("type")
        .map_err(|error| ProfileError::library(&error, "the module type read"))
}

/// Silences every channel except `exported`, and returns those channels.
///
/// The call collects the whole list before it silences any channel, so a
/// channel that the library rejects stops the render before it starts.
fn silence(module: &mut Module, method: Silencing, exported: i32) -> Result<Vec<i32>, StemError> {
    let silenced: Vec<i32> = (0..module.num_channels())
        .filter(|channel| *channel != exported)
        .collect();
    for channel in &silenced {
        match method {
            Silencing::ZeroVolume => {
                map_library(
                    module.set_channel_volume(*channel, 0.0),
                    "the channel volume set",
                )?;
            }
            Silencing::ChannelMute => {
                map_library(
                    module.set_channel_mute(*channel, true),
                    "the channel mute set",
                )?;
            }
        }
    }
    Ok(silenced)
}

/// Wraps one library error with the operation that the render ran.
fn map_library<T>(
    result: Result<T, openmpt_ext::OpenmptError>,
    operation: &'static str,
) -> Result<T, StemError> {
    result.map_err(|error| StemError::Profile(ProfileError::library(&error, operation)))
}

/// Reads the silenced channels back, and fails when one has audio again.
///
/// A channel-volume command, a channel-volume slide, or a song restart raises
/// a zeroed channel volume again (`docs/design.md` §4.1).
fn verify_silence(
    module: &mut Module,
    method: Silencing,
    exported: i32,
    silenced: &[i32],
) -> Result<(), StemError> {
    for channel in silenced {
        match method {
            Silencing::ZeroVolume => {
                let volume =
                    map_library(module.channel_volume(*channel), "the channel volume read")?;
                if volume != 0.0 {
                    return Err(StemError::VolumeDrifted {
                        exported,
                        silenced: *channel,
                        volume,
                    });
                }
            }
            Silencing::ChannelMute => {
                let muted = map_library(module.channel_mute(*channel), "the channel mute read")?;
                if !muted {
                    return Err(StemError::MuteDrifted {
                        exported,
                        silenced: *channel,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Fails when the isolated render visits a different song flow.
fn require_same_flow(
    exported: i32,
    trace: &RowTrace,
    reference: &RowTrace,
) -> Result<(), StemError> {
    match trace.first_divergence(reference) {
        Some(index) => Err(StemError::TraceDiverged { exported, index }),
        None => Ok(()),
    }
}
