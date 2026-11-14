//! The README that labels the sets of one export.
//!
//! The channel stems and the pair stems are two decompositions of the same
//! audio. A DAW that imports both sums the song twice, so the export labels
//! them as alternatives. Design: `docs/design.md` §5.2.

use std::fmt::Write;

use crate::export::ExportPlan;

/// Returns the text of the README of one export.
///
/// The writes to a `String` cannot fail, so the result of each one is ignored.
#[must_use]
pub fn text(plan: &ExportPlan) -> String {
    let mut text = String::new();
    text.push_str("mod2stems export\n\n");
    let _ = writeln!(
        text,
        "The files in this directory are channel stems: one file per tracker channel, in {} at {} Hz.",
        plan.format(),
        plan.rate()
    );
    let _ = writeln!(
        text,
        "Every file spans {} frames. The common gain is {}.\n",
        plan.layout().as_str(),
        plan.encoding().gain().value()
    );
    text.push_str("The channel stems sum to the song. Do not normalize them on their own.\n");
    text.push_str("The manifest.json records what produced the audio.\n");
    text
}
