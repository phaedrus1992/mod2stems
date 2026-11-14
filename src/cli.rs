//! The command line of mod2stems.
//!
//! The arguments are the caller's choices only. Every default is applied once,
//! when the export resolves its plan, so the manifest records the value that
//! produced the audio. Design: `docs/design.md` §6.

use std::path::PathBuf;

use clap::Parser;

use mod2stems::AmigaModel;
use mod2stems::Profile;
use mod2stems::encode::Dither;
use mod2stems::encode::Format;
use mod2stems::export::ExportRequest;
use mod2stems::stems::ChannelLayout;

/// The arguments of one `mod2stems` run.
///
/// The two paths are optional at the parser level, because a run with no
/// argument prints the version and exits. A run that names one path alone
/// fails, because a partial request names no work.
#[derive(Debug, Parser)]
#[command(
    name = "mod2stems",
    version,
    about = "Split a tracker module into one audio file per channel",
    long_about = None
)]
// Each bool is one switch of the command line, and the §6 surface names them.
// An enum would hide the flag name that the caller types.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each field is one switch of the command line surface"
)]
pub struct Args {
    /// The tracker module to export.
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// The directory to write the export into. It must not exist.
    #[arg(value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// The rendering profile.
    #[arg(long, value_parser = parse_profile, default_value = "compatible")]
    pub profile: Profile,

    /// The Amiga filter model. It applies only to the `amiga` profile.
    #[arg(long, value_name = "MODEL", value_parser = parse_model)]
    pub amiga_model: Option<AmigaModel>,

    /// The output rate in Hz. The default is 48000.
    #[arg(long, value_name = "HZ", value_parser = parse_signed)]
    pub sample_rate: Option<i32>,

    /// The subsong to render.
    #[arg(long, value_name = "N", default_value_t = 0, value_parser = parse_signed)]
    pub subsong: i32,

    /// Write one mono file per output instead of a stereo file.
    #[arg(long, conflicts_with = "stereo")]
    pub mono: bool,

    /// Write a stereo file per output. This is the default.
    #[arg(long)]
    pub stereo: bool,

    /// The output container and bit depth.
    #[arg(long, value_parser = parse_format, default_value = "float")]
    pub format: Format,

    /// Also write the sum of the channel stems.
    #[arg(long)]
    pub mix: bool,

    /// Also write the unmuted render of the whole song.
    #[arg(long)]
    pub reference_mix: bool,

    /// Also write one file per instrument and channel pair.
    #[arg(long)]
    pub instrument_channel_stems: bool,

    /// Do not write instrument and channel pair files. This is the default.
    #[arg(long, conflicts_with = "instrument_channel_stems")]
    pub no_instrument_channel_stems: bool,

    /// The common gain that every output receives.
    #[arg(long, value_name = "FACTOR", default_value_t = 1.0)]
    pub gain: f32,

    /// The dither of the final integer conversion.
    #[arg(long, value_parser = parse_dither, default_value = "none")]
    pub dither: Dither,
}

impl Args {
    /// Returns the export request that these arguments name.
    ///
    /// Returns `None` when the run names no work, which is the version report.
    ///
    /// # Errors
    ///
    /// Returns a message when the run names one path alone.
    pub fn request(&self) -> Result<Option<ExportRequest>, String> {
        let (input, destination) = match (&self.input, &self.output) {
            (Some(input), Some(destination)) => (input.clone(), destination.clone()),
            (None, None) => return Ok(None),
            _ => {
                return Err(
                    "Cannot start the export: it needs an input module and an output \
                     directory. Name both paths, as in `mod2stems song.mod export`."
                        .to_owned(),
                );
            }
        };
        let mut request = ExportRequest::new(input, destination);
        request.profile = self.profile;
        request.amiga_model = self.amiga_model;
        request.rate = self.sample_rate;
        request.subsong = self.subsong;
        request.layout = self.layout();
        request.format = self.format;
        request.gain = self.gain;
        request.dither = self.dither;
        request.write_mix = self.mix;
        request.write_reference_mix = self.reference_mix;
        request.instrument_channel_stems =
            self.instrument_channel_stems && !self.no_instrument_channel_stems;
        Ok(Some(request))
    }

    /// Returns the output layout that these arguments name.
    #[must_use]
    pub fn layout(&self) -> ChannelLayout {
        if self.mono {
            ChannelLayout::Mono
        } else {
            ChannelLayout::Stereo
        }
    }
}

/// Reads one signed integer, including a negative value.
///
/// The parser reads a leading `-` as an option name, so a negative rate or
/// subsong needs `--flag=-1`. The error states that form.
fn parse_signed(text: &str) -> Result<i32, String> {
    text.parse::<i32>().map_err(|_| {
        format!(
            "cannot read {text:?} as an integer. An integer holds an optional sign and \
             digits, as in `-1` or `44100`."
        )
    })
}

/// Reads one profile name.
fn parse_profile(text: &str) -> Result<Profile, String> {
    Profile::parse(text).map_err(|error| error.to_string())
}

/// Reads one Amiga filter model name.
fn parse_model(text: &str) -> Result<AmigaModel, String> {
    AmigaModel::parse(text).map_err(|error| error.to_string())
}

/// Reads one output format name.
fn parse_format(text: &str) -> Result<Format, String> {
    match text {
        "float" => Ok(Format::WavF32),
        "flac16" => Ok(Format::FlacI16),
        "flac24" => Ok(Format::FlacI24),
        other => Err(format!(
            "cannot read {other:?} as an output format. Use `float`, `flac16`, or `flac24`."
        )),
    }
}

/// Reads one dither policy name.
fn parse_dither(text: &str) -> Result<Dither, String> {
    match text {
        "none" => Ok(Dither::None),
        "tpdf" => Ok(Dither::Tpdf),
        other => Err(format!(
            "cannot read {other:?} as a dither policy. Use `none` or `tpdf`."
        )),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Args;
    use mod2stems::Profile;
    use mod2stems::encode::Format;
    use mod2stems::export::ExportRequest;
    use mod2stems::stems::ChannelLayout;

    /// Returns the arguments that one command line names.
    fn args(line: &[&str]) -> Result<Args, String> {
        Args::try_parse_from(line).map_err(|error| error.to_string())
    }

    /// Returns the export request that one command line names.
    fn request(line: &[&str]) -> Result<ExportRequest, String> {
        args(line)?
            .request()?
            .ok_or_else(|| "the command line names no export".to_owned())
    }

    /// Returns an error that names `what` when `condition` is false.
    fn require(condition: bool, what: &str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(what.to_owned())
        }
    }

    #[test]
    fn a_run_with_no_path_names_no_work() -> Result<(), String> {
        let parsed = args(&["mod2stems"])?;
        require(parsed.request()?.is_none(), "no path means no export")
    }

    #[test]
    fn one_path_alone_fails() -> Result<(), String> {
        let parsed = args(&["mod2stems", "song.mod"])?;
        require(parsed.request().is_err(), "one path names no work")
    }

    #[test]
    fn the_defaults_match_the_design() -> Result<(), String> {
        let request = request(&["mod2stems", "song.mod", "out"])?;
        require(
            request.profile == Profile::Compatible,
            "the profile defaults to compatible",
        )?;
        require(
            request.format == Format::WavF32,
            "the format defaults to float",
        )?;
        require(
            request.layout == ChannelLayout::Stereo,
            "the layout defaults to stereo",
        )?;
        require(
            (request.gain - 1.0).abs() < f32::EPSILON,
            "the gain defaults to unity",
        )?;
        require(!request.write_mix, "the mix is off by default")
    }

    #[test]
    fn the_switches_reach_the_request() -> Result<(), String> {
        let line = [
            "mod2stems",
            "song.mod",
            "out",
            "--profile",
            "clean",
            "--format",
            "flac24",
            "--mono",
            "--mix",
            "--reference-mix",
            "--sample-rate",
            "44100",
            "--gain",
            "0.5",
        ];
        let request = request(&line)?;
        require(request.profile == Profile::Clean, "the profile switch")?;
        require(request.format == Format::FlacI24, "the format switch")?;
        require(request.layout == ChannelLayout::Mono, "the mono switch")?;
        require(request.write_mix, "the mix switch")?;
        require(request.write_reference_mix, "the reference mix switch")?;
        require(request.rate == Some(44_100), "the rate switch")?;
        require((request.gain - 0.5).abs() < f32::EPSILON, "the gain switch")
    }

    #[test]
    fn an_unreadable_format_fails() {
        assert!(args(&["mod2stems", "song.mod", "out", "--format", "mp3"]).is_err());
    }

    #[test]
    fn mono_and_stereo_together_fail() {
        assert!(args(&["mod2stems", "song.mod", "out", "--mono", "--stereo"]).is_err());
    }
}
