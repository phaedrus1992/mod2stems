//! Tests the mix that the export writes against the stems it holds.
//!
//! The mix is the sum of the channel stems with no normalization. The checks
//! read the published float WAV files, so they prove what a DAW would import.

use std::path::Path;

use mod2stems::export::ExportRequest;
use mod2stems::manifest::README_FILE;
use mod2stems::stems::ChannelLayout;

#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/header.rs"]
mod header;
use fixture::export;
use fixture::manifest_of;
use fixture::require;
use fixture::require_val;
use fixture::temporary;
use fixture::write_fixture;

/// Reads one float WAV file, as interleaved samples.
fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    reader
        .samples::<f32>()
        .collect::<Result<Vec<f32>, _>>()
        .map_err(|error| error.to_string())
}

/// Adds `samples` into `sum`, and seeds `sum` from the first buffer.
fn add_into(sum: &mut Vec<f32>, samples: &[f32]) {
    if sum.is_empty() {
        *sum = samples.to_vec();
        return;
    }
    for (total, sample) in sum.iter_mut().zip(samples) {
        *total += sample;
    }
}

/// Returns the sum of every stem of one published export.
fn stem_sum(destination: &Path, mix_name: &str) -> Result<(Vec<f32>, Vec<f32>), String> {
    let manifest = manifest_of(destination)?;
    let mut sum: Vec<f32> = Vec::new();
    for entry in manifest.stems() {
        let samples = read_wav(&destination.join(entry.path()))?;
        add_into(&mut sum, &samples);
    }
    let mix = read_wav(&destination.join(mix_name))?;
    Ok((sum, mix))
}

#[test]
fn a_mono_mix_sums_to_the_mono_stems() -> Result<(), String> {
    // A mono stem holds one sample per frame. A mix built by summing the mono
    // stems and then downmixing that sum reads each pair of frames as one
    // stereo frame, so the mix spans half the timeline.
    let workspace = temporary("mix-mono-sum")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.layout = ChannelLayout::Mono;
    request.write_mix = true;
    export(&request)?;
    let name = manifest_of(&destination)?
        .details()
        .mix
        .clone()
        .ok_or("the manifest names no mix")?;
    let (sum, mix) = stem_sum(&destination, &name)?;
    require_val(mix.len(), sum.len(), "the mono mix spans the mono stems")?;
    for (index, (mix_sample, sum_sample)) in mix.iter().zip(&sum).enumerate() {
        require(
            (mix_sample - sum_sample).abs() <= 1e-5,
            &format!("sample {index} differs: mix {mix_sample}, sum {sum_sample}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_mix_sums_to_the_stems_within_the_float_tolerance() -> Result<(), String> {
    // The check proves the mix is not a separate render and not a scaled copy.
    let workspace = temporary("mix-sum")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.write_mix = true;
    export(&request)?;
    let name = manifest_of(&destination)?
        .details()
        .mix
        .clone()
        .ok_or("the manifest names no mix")?;
    let (sum, mix) = stem_sum(&destination, &name)?;
    require_val(mix.len(), sum.len(), "the mix spans the stems")?;
    for (index, (mix_sample, sum_sample)) in mix.iter().zip(&sum).enumerate() {
        require(
            (mix_sample - sum_sample).abs() <= 1e-5,
            &format!("sample {index} differs: mix {mix_sample}, sum {sum_sample}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_readme_names_the_frames_of_the_output_layout() -> Result<(), String> {
    // A mono file holds one sample per frame, so the sample count of a stereo
    // render is not the frame count of the export.
    let workspace = temporary("mix-readme-frames")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.layout = ChannelLayout::Mono;
    export(&request)?;
    let manifest = manifest_of(&destination)?;
    let frames = manifest.details().frame_count;
    let text = std::fs::read_to_string(destination.join(README_FILE)).map_err(|e| e.to_string())?;
    require(
        text.contains(&format!("{frames} frames")),
        &format!("the README names {frames} frames, got {text}"),
    )?;
    for entry in manifest.stems() {
        let samples = read_wav(&destination.join(entry.path()))?;
        require_val(
            samples.len(),
            frames,
            "the mono file holds one sample per frame",
        )?;
    }
    Ok(())
}
