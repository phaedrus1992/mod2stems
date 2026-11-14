//! The export-level facts that one manifest records.
//!
//! The values are resolved before the first render, so the manifest states what
//! the export used and not what a default would have used. The struct is a
//! record: every field is public, because the caller builds it from the
//! resolved configuration. Design: `docs/design.md` §5.2.

use serde::Deserialize;
use serde::Serialize;

/// The facts of one export that do not belong to a single file.
///
/// A field that the library owns still appears here. The export records the
/// resolved value, such as the `play.at_end` setting, and not the requested
/// value, because the resolved value is what produced the audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportDetails {
    /// The SHA-256 digest of the input module bytes, with a `sha256:` prefix.
    pub input_hash: String,
    /// The version of this tool.
    pub tool_version: String,
    /// The libopenmpt version that rendered the audio.
    pub libopenmpt_version: String,
    /// The resolved rendering profile.
    pub profile: String,
    /// Every resolved playback setting, as a name and a value.
    pub settings: Vec<(String, String)>,
    /// The output sample rate, in Hz.
    pub sample_rate: i32,
    /// The channel layout of the output.
    pub layout: String,
    /// The frame count that every file spans.
    pub frame_count: usize,
    /// The rule that ended the render.
    pub end_policy: String,
    /// The common gain that every file received.
    pub gain: f32,
    /// The file that holds the sum of the channel stems, when the export wrote
    /// one, as a path relative to the export root.
    ///
    /// The mix is a sum and not a channel, so it has no stem entry. The field
    /// still names it, because publication checks every file that the manifest
    /// names, and the mix must pass that check too.
    pub mix: Option<String>,
    /// The file that holds the unmuted render, when the export wrote one, as a
    /// path relative to the export root.
    pub reference_mix: Option<String>,
    /// The output container and bit depth.
    pub format: String,
    /// The dither policy of the integer path.
    pub dither: String,
    /// The quantization policy of the integer path.
    pub quantization: String,
    /// The subsong that the export rendered.
    pub subsong: i32,
    /// The module metadata, as a name and a value.
    pub module_metadata: Vec<(String, String)>,
    /// Every warning that the library reported during the export.
    pub warnings: Vec<String>,
    /// The reconstruction measurement and its status.
    pub validation: String,
}
