//! Tests the interactive channel and instrument controls.

mod support;

use openmpt_ext::Module;

/// Loads the simple test file.
fn load() -> Result<Module, String> {
    Module::from_bytes(&support::simple_mod(), &[]).map_err(|error| error.to_string())
}

/// Renders 4096 frames, which is more than the first row holds.
fn render_all(module: &mut Module) -> Result<Vec<f32>, String> {
    let mut buf = vec![0.0_f32; 8192];
    module
        .read_float(48_000, &mut buf)
        .map_err(|error| error.to_string())?;
    Ok(buf)
}

#[test]
fn mute_all_is_silent() -> Result<(), String> {
    let mut module = load()?;
    for channel in 0..module.num_channels() {
        module
            .set_channel_mute(channel, true)
            .map_err(|error| error.to_string())?;
    }
    let buf = render_all(&mut module)?;
    if buf.iter().any(|value| *value != 0.0) {
        return Err("the render is not silent after every channel is muted".to_owned());
    }
    Ok(())
}

#[test]
fn solo_channel_differs_from_full() -> Result<(), String> {
    let mut full_module = load()?;
    let full = render_all(&mut full_module)?;
    let mut module = load()?;
    for channel in 1..module.num_channels() {
        module
            .set_channel_mute(channel, true)
            .map_err(|error| error.to_string())?;
    }
    let solo = render_all(&mut module)?;
    if solo.iter().all(|value| *value == 0.0) {
        return Err("the solo render is silent, expected channel 0".to_owned());
    }
    if solo == full {
        return Err("the solo render equals the full render".to_owned());
    }
    Ok(())
}

#[test]
fn zero_volume_acts_like_mute() -> Result<(), String> {
    let mut full_module = load()?;
    let full = render_all(&mut full_module)?;
    let mut module = load()?;
    for channel in 1..module.num_channels() {
        module
            .set_channel_volume(channel, 0.0)
            .map_err(|error| error.to_string())?;
    }
    let solo = render_all(&mut module)?;
    if solo.iter().all(|value| *value == 0.0) {
        return Err("the solo render is silent, expected channel 0".to_owned());
    }
    if solo == full {
        return Err("the solo render equals the full render".to_owned());
    }
    Ok(())
}

#[test]
fn mute_getter_round_trip() -> Result<(), String> {
    let mut module = load()?;
    module
        .set_channel_mute(0, true)
        .map_err(|error| error.to_string())?;
    if !module.channel_mute(0).map_err(|error| error.to_string())? {
        return Err("channel 0 must read back as muted".to_owned());
    }
    module
        .set_channel_mute(0, false)
        .map_err(|error| error.to_string())?;
    if module.channel_mute(0).map_err(|error| error.to_string())? {
        return Err("channel 0 must read back as not muted".to_owned());
    }
    Ok(())
}

#[test]
fn volume_is_quantized() -> Result<(), String> {
    let mut module = load()?;
    module
        .set_channel_volume(0, 0.5)
        .map_err(|error| error.to_string())?;
    let half = module
        .channel_volume(0)
        .map_err(|error| error.to_string())?;
    if (half - 0.5).abs() > f64::EPSILON {
        return Err(format!("a volume of 0.5 read back as {half}"));
    }
    module
        .set_channel_volume(0, 0.3)
        .map_err(|error| error.to_string())?;
    let level = module
        .channel_volume(0)
        .map_err(|error| error.to_string())?;
    // The library stores the volume as a step count out of 64.
    let expected = (0.3_f64 * 64.0).round() / 64.0;
    if (level - expected).abs() > f64::EPSILON {
        return Err(format!(
            "a volume of 0.3 read back as {level}, expected {expected}"
        ));
    }
    Ok(())
}

#[test]
fn volume_out_of_range() -> Result<(), String> {
    let cases = [1.5, -0.1, f64::NAN];
    for value in cases {
        let mut module = load()?;
        let Some(error) = module.set_channel_volume(0, value).err() else {
            return Err(format!("the volume {value} must be an error"));
        };
        if !error.message.contains(&format!("{value}")) {
            return Err(format!(
                "message is {:?}, expected it to name the volume {value}",
                error.message
            ));
        }
    }
    Ok(())
}

#[test]
fn instrument_mute_uses_sample_count() -> Result<(), String> {
    let mut module = load()?;
    let count = module.num_samples();
    for index in [0, 1] {
        module
            .set_instrument_mute(index, true)
            .map_err(|error| error.to_string())?;
        if !module
            .instrument_mute(index)
            .map_err(|error| error.to_string())?
        {
            return Err(format!("instrument {index} must read back as muted"));
        }
    }
    for index in [count, count + 1] {
        let Some(error) = module.set_instrument_mute(index, true).err() else {
            return Err(format!("instrument {index} must be an error"));
        };
        if !error.message.contains(&format!("0..{count}")) {
            return Err(format!(
                "message is {:?}, expected it to name the range 0..{count}",
                error.message
            ));
        }
    }
    Ok(())
}

#[test]
fn channel_out_of_range() -> Result<(), String> {
    let mut module = load()?;
    let channel = module.num_channels();
    let results = [
        module.set_channel_mute(channel, true).err().is_some(),
        module.channel_mute(channel).err().is_some(),
        module.set_channel_volume(channel, 0.5).err().is_some(),
        module.channel_volume(channel).err().is_some(),
    ];
    if let Some(index) = results.iter().position(|ok| !ok) {
        return Err(format!("call {index} accepted channel {channel}"));
    }
    Ok(())
}

#[test]
fn instrument_out_of_range_is_not_a_channel_index() -> Result<(), String> {
    let mut module = load()?;
    let channel = module.num_channels();
    // The instrument methods index sample slots in a module with no
    // instruments, so a channel count is no limit for them.
    module
        .set_instrument_mute(channel, true)
        .map_err(|error| format!("sample slot {channel} must be valid: {error}"))?;
    Ok(())
}
