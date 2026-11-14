//! The one resolved configuration that an export shares.
//!
//! The resolver checks everything that needs no render: the destination, the
//! input, the rate against the renderer and the encoder, and the module itself.
//! Every render of the export then shares this value. Design:
//! `docs/design.md` §2, §5.1, §5.2, §6.

use std::path::Path;
use std::path::PathBuf;

use openmpt_ext::Module;
use openmpt_ext::SAMPLE_RATE_RANGE;

use crate::encode::Encoding;
use crate::encode::Format;
use crate::encode::Gain;
use crate::export::ExportError;
use crate::export::ExportRequest;
use crate::export::hash::hash;
use crate::export::record::StagedFiles;
use crate::manifest::ExportDetails;
use crate::manifest::file_name;
use crate::profile::ProfileConfig;
use crate::profile::ProfileError;
use crate::stems::ChannelLayout;
use crate::stems::PlaybackPolicy;
use crate::stems::StemError;

/// The encoder rate limit that every FLAC stream shares.
///
/// A FLAC frame states its rate as a sample rate code, and the code covers a
/// rate up to 655350 Hz. The renderer accepts fewer rates, so the check only
/// matters when the library range widens.
const FLAC_HIGHEST_RATE: i64 = 655_350;

/// The largest input file that the export reads, in bytes.
///
/// The export holds the whole module in memory, and libopenmpt supports samples
/// up to 4 GiB in theory. A 2 GiB cap covers every real module by a wide margin
/// and keeps a wrong path from a memory exhaustion.
const INPUT_LIMIT: usize = 2 * 1024 * 1024 * 1024;

/// Everything that one export resolved before its first render.
///
/// The value holds one config, one policy, and one encoding, and every render
/// and every file of the export shares them. The caller builds it before any
/// work starts, so a bad rate or an unusable module fails with no render and no
/// file.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportPlan {
    config: ProfileConfig,
    policy: PlaybackPolicy,
    layout: ChannelLayout,
    encoding: Encoding,
    destination: PathBuf,
    bytes: Vec<u8>,
    channels: i32,
    names: Vec<String>,
    metadata: Vec<(String, String)>,
}

impl ExportPlan {
    /// Reads the module, and resolves everything that the export shares.
    ///
    /// The call reads the whole input into memory, because the export loads a
    /// fresh module from those bytes for every channel and records their digest.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::DestinationExists`] when the destination holds an
    /// entry, [`ExportError::Input`] when the input cannot be read,
    /// [`ExportError::Stem`] when the profile does not resolve,
    /// [`ExportError::RateUnsupported`] when the format cannot store the rate,
    /// and [`ExportError::Encode`] when the gain is not usable.
    pub fn resolve(request: &ExportRequest) -> Result<Self, ExportError> {
        request.refuse_existing()?;
        let bytes = read_input(request.input())?;
        let config = request.config()?;
        check_rate(request.format, config.rate())?;
        let encoding = request.encoding(config.rate())?;
        let module = load(&bytes)?;
        let channels = module.num_channels();
        let names = channel_names(&module, channels)?;
        let metadata = module_metadata(&module)?;
        let policy = PlaybackPolicy::default_policy().with_subsong(request.subsong);
        Ok(Self {
            config,
            policy,
            layout: request.layout,
            encoding,
            destination: request.destination().to_path_buf(),
            bytes,
            channels,
            names,
            metadata,
        })
    }

    /// Returns the module bytes that every render loads.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the resolved config that every render shares.
    #[must_use]
    pub fn config(&self) -> &ProfileConfig {
        &self.config
    }

    /// Returns the playback policy that every render shares.
    #[must_use]
    pub fn policy(&self) -> PlaybackPolicy {
        self.policy
    }

    /// Returns the encoding that every file shares.
    #[must_use]
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Returns the channel count of the module.
    #[must_use]
    pub fn channels(&self) -> i32 {
        self.channels
    }

    /// Returns the resolved output rate, in Hz.
    #[must_use]
    pub fn rate(&self) -> i32 {
        self.config.rate()
    }

    /// Returns the channel layout of the output.
    #[must_use]
    pub fn layout(&self) -> ChannelLayout {
        self.layout
    }

    /// Returns the container and bit depth of the output.
    #[must_use]
    pub fn format(&self) -> Format {
        self.encoding.format()
    }

    /// Returns the destination directory.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// Returns the number of renders that a plain export runs.
    ///
    /// The count is the unmuted reference render plus one render per channel.
    /// An `amiga` export runs three more, because the profile proves that the
    /// module uses the resampler by comparison (`docs/design.md` §2.1).
    #[must_use]
    pub fn render_count(&self) -> usize {
        let channels = usize::try_from(self.channels).unwrap_or(0);
        let extra = if self.config.amiga_enabled() { 3 } else { 0 };
        channels.saturating_add(1).saturating_add(extra)
    }

    /// Returns the name that the module gave one channel.
    ///
    /// The name is text for a person. The manifest keeps it, because the file
    /// name is sanitized and the channel index is the identity.
    #[must_use]
    pub fn channel_name(&self, channel: i32) -> Option<String> {
        usize::try_from(channel)
            .ok()
            .and_then(|index| self.names.get(index))
            .filter(|name| !name.is_empty())
            .cloned()
    }

    /// Returns the file name of one channel stem, relative to the export root.
    #[must_use]
    pub fn stem_name(&self, channel: i32) -> String {
        let name = usize::try_from(channel)
            .ok()
            .and_then(|index| self.names.get(index))
            .map(String::as_str)
            .filter(|name| !name.is_empty());
        file_name(channel, name, self.encoding.format().extension())
    }

    /// Returns the export-level facts that the manifest records.
    ///
    /// `files` names the optional mix files, because a mix is a sum and not a
    /// channel, and the manifest must name every file of the export.
    ///
    /// # Errors
    ///
    /// Returns [`ExportError::Manifest`] when the libopenmpt version cannot be
    /// read.
    pub fn details(
        &self,
        frames: usize,
        files: &StagedFiles,
    ) -> Result<ExportDetails, ExportError> {
        Ok(ExportDetails {
            input_hash: hash(&self.bytes),
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            libopenmpt_version: libopenmpt_version()?,
            profile: self.config.profile().as_str().to_owned(),
            settings: self.resolved_settings(),
            sample_rate: self.rate(),
            layout: self.layout.as_str().to_owned(),
            frame_count: frames,
            end_policy: "song end, no fade".to_owned(),
            gain: self.encoding.gain().value(),
            mix: files.mix_name(),
            reference_mix: files.reference_mix_name(),
            format: self.encoding.format().to_string(),
            dither: dither_name(self.encoding),
            quantization: quantization_name(self.encoding),
            subsong: self.policy.subsong(),
            module_metadata: self.metadata.clone(),
            warnings: Vec::new(),
            validation: "unverified".to_owned(),
        })
    }

    /// Returns true when the caller's gain brings one measured peak inside the
    /// format range.
    ///
    /// A float format has no range, so every finite peak fits it. An integer
    /// format holds a magnitude of at most one, so the peak times the gain must
    /// stay at or below that.
    #[must_use]
    pub fn gain_fits(&self, peak: f32, gain: Gain) -> bool {
        let target = self.format().peak();
        let scaled = peak * gain.value();
        scaled <= target || !scaled.is_finite()
    }

    /// Returns the resolved settings of this export, library defaults included.
    ///
    /// The list names the library-owned values too, because the manifest must
    /// state what produced the audio and not only what the caller set.
    fn resolved_settings(&self) -> Vec<(String, String)> {
        let mut settings = vec![
            ("play.at_end".to_owned(), "stop".to_owned()),
            ("repeat_count".to_owned(), "0".to_owned()),
            ("subsong".to_owned(), self.policy.subsong().to_string()),
            (
                "max_seconds".to_owned(),
                self.policy.max_seconds().to_string(),
            ),
        ];
        if let Some(model) = self.config.amiga_model() {
            settings.push((
                "render.resampler.emulate_amiga_type".to_owned(),
                model.as_str().to_owned(),
            ));
        }
        if let Some(length) = self.config.settings().interpolation_filter_length() {
            let key = "render.interpolation_filter_length".to_owned();
            settings.push((key, length.to_string()));
        }
        settings
    }
}

/// Returns the bytes of the input module.
///
/// The call reads the whole input into memory, because the export loads a fresh
/// module from those bytes for every channel and records their digest. It
/// refuses a file above [`INPUT_LIMIT`] before it allocates, so a wrong path to
/// a large file cannot exhaust memory.
///
/// # Errors
///
/// Returns [`ExportError::Input`] when the input is too large, and when the
/// filesystem refuses the read.
fn read_input(path: &Path) -> Result<Vec<u8>, ExportError> {
    let metadata = std::fs::metadata(path).map_err(|error| ExportError::Input {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    let limit = u64::try_from(INPUT_LIMIT).unwrap_or(u64::MAX);
    if metadata.len() > limit {
        return Err(ExportError::Input {
            path: path.to_path_buf(),
            detail: format!(
                "the file holds {} bytes and this tool reads at most {INPUT_LIMIT}",
                metadata.len()
            ),
        });
    }
    std::fs::read(path).map_err(|error| ExportError::Input {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })
}

/// Loads one module from bytes.
///
/// # Errors
///
/// Returns [`ExportError::Library`] when libopenmpt cannot load the bytes.
pub fn load(bytes: &[u8]) -> Result<Module, ExportError> {
    Module::from_bytes(bytes, &[]).map_err(|error| {
        ExportError::library(
            "the module load",
            &error,
            "Check that the input is a module that this libopenmpt build accepts.",
        )
    })
}

/// Returns the names that the module gave its channels.
fn channel_names(module: &Module, channels: i32) -> Result<Vec<String>, ExportError> {
    let mut names = Vec::with_capacity(usize::try_from(channels).unwrap_or(0));
    for channel in 0..channels {
        let name = module
            .channel_name(channel)
            .map_err(|error| library_error("the channel name read", &error))?;
        names.push(name);
    }
    Ok(names)
}

/// Returns every metadata key and value of the module.
///
/// A key whose value cannot be read keeps an empty value, because one broken
/// key must not fail an export that the audio does not depend on.
fn module_metadata(module: &Module) -> Result<Vec<(String, String)>, ExportError> {
    let keys = module
        .metadata_keys()
        .map_err(|error| library_error("the metadata key read", &error))?;
    let mut metadata = Vec::with_capacity(keys.len());
    for key in keys {
        let value = module.metadata(&key).unwrap_or_default();
        metadata.push((key, value));
    }
    Ok(metadata)
}

/// Rejects a rate that the encoder cannot store.
fn check_rate(format: Format, rate: i32) -> Result<(), ExportError> {
    if !SAMPLE_RATE_RANGE.contains(&rate) {
        // The profile request rejects this rate too. The check stays, because a
        // config can arrive from another caller.
        return Err(ExportError::RateUnsupported {
            rate,
            format: "the renderer",
        });
    }
    if !format.is_integer() || i64::from(rate) <= FLAC_HIGHEST_RATE {
        return Ok(());
    }
    Err(ExportError::RateUnsupported {
        rate,
        format: "FLAC",
    })
}

/// Returns the dither name that the manifest records.
fn dither_name(encoding: Encoding) -> String {
    match encoding.dither() {
        crate::encode::Dither::None => "none".to_owned(),
        crate::encode::Dither::Tpdf => "tpdf".to_owned(),
    }
}

/// Returns the quantization name that the manifest records.
fn quantization_name(encoding: Encoding) -> String {
    if encoding.format().is_integer() {
        "final integer conversion".to_owned()
    } else {
        "none (float)".to_owned()
    }
}

/// Returns the libopenmpt version string of the linked library.
fn libopenmpt_version() -> Result<String, ExportError> {
    openmpt_ext::library_info(openmpt_ext::LibraryInfo::LibraryVersion)
        .map_err(|error| library_error("the libopenmpt version read", &error))
}

/// Returns the export error of one failed library call.
pub(crate) fn library_error(
    operation: &'static str,
    error: &openmpt_ext::OpenmptError,
) -> ExportError {
    ExportError::from(StemError::from(ProfileError::library(error, operation)))
}
