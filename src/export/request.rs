//! What one export request names, before anything resolves.
//!
//! The command builds this value from its arguments. The request holds the
//! caller's choices only, and resolution turns them into the one config and the
//! one encoding that every render and every file shares. Design:
//! `docs/design.md` §2, §5.2, §6.

use std::path::Path;
use std::path::PathBuf;

use crate::encode::Dither;
use crate::encode::Encoding;
use crate::encode::Format;
use crate::manifest::ManifestError;
use crate::manifest::StagingReport;
use crate::profile::AmigaModel;
use crate::profile::Profile;
use crate::profile::ProfileConfig;
use crate::profile::ProfileRequest;
use crate::stems::ChannelLayout;

use crate::export::ExportError;

/// One export request, as the caller named it.
///
/// A field holds what the caller chose, and no default is applied here. The
/// resolver applies the defaults once, so the export records the values that
/// produced the audio and not the values that the command printed.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportRequest {
    /// The input module.
    pub input: PathBuf,
    /// The export directory.
    pub destination: PathBuf,
    /// The rendering profile.
    pub profile: Profile,
    /// The Amiga filter model, when the caller names one.
    pub amiga_model: Option<AmigaModel>,
    /// The output rate, in Hz. `None` takes the default rate.
    pub rate: Option<i32>,
    /// The subsong to render.
    pub subsong: i32,
    /// The output channel layout.
    pub layout: ChannelLayout,
    /// The output container and bit depth.
    pub format: Format,
    /// The common gain that every output receives.
    pub gain: f32,
    /// The dither policy of the final integer encode.
    pub dither: Dither,
    /// Whether the export writes the sum of the channel stems.
    pub write_mix: bool,
    /// Whether the export writes the unmuted render as audio.
    pub write_reference_mix: bool,
    /// Whether the caller asked for pair stems.
    pub instrument_channel_stems: bool,
}

impl ExportRequest {
    /// Returns the default request of one input and one destination.
    ///
    /// The defaults match the design: `compatible`, stereo, float WAV, the
    /// default rate, the first subsong, unity gain, no dither, no mix, no
    /// reference mix, and no pair stems.
    #[must_use]
    pub fn new(input: PathBuf, destination: PathBuf) -> Self {
        Self {
            input,
            destination,
            profile: Profile::default(),
            amiga_model: None,
            rate: None,
            subsong: 0,
            layout: ChannelLayout::default(),
            format: Format::WavF32,
            gain: 1.0,
            dither: Dither::None,
            write_mix: false,
            write_reference_mix: false,
            instrument_channel_stems: false,
        }
    }

    /// Returns the input module path.
    #[must_use]
    pub fn input(&self) -> &Path {
        &self.input
    }

    /// Returns the export directory.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// Resolves the profile request of this export.
    ///
    /// The call resolves the request once, and every render of the export
    /// shares the result.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::Stem`] when the profile does not resolve, and
    /// [`ExportError::PairsUnavailable`] when the caller asked for pair stems
    /// that this release does not build.
    pub fn config(&self) -> Result<ProfileConfig, ExportError> {
        if self.instrument_channel_stems {
            return Err(ExportError::PairsUnavailable);
        }
        let request = self.profile_request();
        request
            .resolve()
            .map_err(|error| ExportError::Stem(error.into()))
    }

    /// Returns the profile request that this export resolves.
    #[must_use]
    pub fn profile_request(&self) -> ProfileRequest {
        let request = ProfileRequest::default().with_profile(self.profile);
        let request = match self.amiga_model {
            Some(model) => request.with_amiga_model(model),
            None => request,
        };
        match self.rate {
            Some(rate) => request.with_rate(rate),
            None => request,
        }
    }

    /// Returns the encoding of one output of this export.
    ///
    /// The channel count follows the layout, so a mono export writes one sample
    /// per frame.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::Encode`] when the gain is not finite and positive.
    pub fn encoding(&self, rate: i32) -> Result<Encoding, ExportError> {
        let encoding = Encoding::new(self.format, rate)
            .with_channels(self.layout.channels())
            .with_gain(self.gain)?
            .with_dither(self.dither);
        Ok(encoding)
    }

    /// Rejects a destination that already holds an entry.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::DestinationExists`] when the path holds an entry,
    /// and [`ExportError::Manifest`] when the path cannot be read.
    pub fn refuse_existing(&self) -> Result<(), ExportError> {
        match StagingReport::refuse_existing(&self.destination) {
            Ok(()) => Ok(()),
            Err(ManifestError::DestinationExists { path }) => {
                Err(ExportError::DestinationExists { path })
            }
            Err(error) => Err(ExportError::Manifest(error)),
        }
    }
}
