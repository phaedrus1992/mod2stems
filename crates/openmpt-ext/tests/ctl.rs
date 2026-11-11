//! Tests the ctls and the render params.

mod support;

use openmpt_ext::Module;
use openmpt_ext::RenderParam;

/// Loads the simple test file.
fn load() -> Result<Module, String> {
    Module::from_bytes(&support::simple_mod(), &[]).map_err(|error| error.to_string())
}

#[test]
fn ctl_list_has_amiga_keys() -> Result<(), String> {
    let module = load()?;
    for wanted in [
        "render.resampler.emulate_amiga",
        "render.resampler.emulate_amiga_type",
    ] {
        if !module.ctls().contains(wanted) {
            return Err(format!("the ctl list has no {wanted:?}"));
        }
    }
    Ok(())
}

#[test]
fn ctl_list_has_subsong_and_dither() -> Result<(), String> {
    let module = load()?;
    for wanted in ["subsong", "dither"] {
        if !module.ctls().contains(wanted) {
            return Err(format!("the ctl list has no {wanted:?}"));
        }
    }
    Ok(())
}

#[test]
fn ctl_table_matches_library() -> Result<(), String> {
    // The type table in `ctl.rs` is a copy of the library ctl list. A version
    // bump that adds a ctl must fail here, so the copy stays correct.
    let module = load()?;
    let unknown: Vec<&String> = module
        .ctls()
        .iter()
        .filter(|key| !module.ctl_type_is_known(key))
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "the library has ctls with no type in this crate: {unknown:?}"
        ));
    }
    Ok(())
}

#[test]
fn set_get_bool_round_trip() -> Result<(), String> {
    let mut module = load()?;
    module
        .ctl_set_bool("render.resampler.emulate_amiga", true)
        .map_err(|error| error.to_string())?;
    let value = module
        .ctl_get_bool("render.resampler.emulate_amiga")
        .map_err(|error| error.to_string())?;
    if !value {
        return Err("the boolean ctl read back as false".to_owned());
    }
    Ok(())
}

#[test]
fn set_get_text_round_trip() -> Result<(), String> {
    let mut module = load()?;
    module
        .ctl_set_text("render.resampler.emulate_amiga_type", "a500")
        .map_err(|error| error.to_string())?;
    let value = module
        .ctl_get_text("render.resampler.emulate_amiga_type")
        .map_err(|error| error.to_string())?;
    if value != "a500" {
        return Err(format!(
            "the text ctl read back as {value:?}, expected \"a500\""
        ));
    }
    Ok(())
}

#[test]
fn set_get_float_round_trip() -> Result<(), String> {
    // The library stores the tempo factor as a fixed-point value with 16 bits of
    // fraction (`m_nTempoFactor`), so the read is near the set value, not equal.
    let mut module = load()?;
    module
        .ctl_set_float("play.tempo_factor", 1.5)
        .map_err(|error| error.to_string())?;
    let value = module
        .ctl_get_float("play.tempo_factor")
        .map_err(|error| error.to_string())?;
    let tolerance = 1.0 / 65536.0;
    if (value - 1.5).abs() > tolerance {
        return Err(format!("the float ctl read back as {value}, expected 1.5"));
    }
    Ok(())
}

#[test]
fn set_get_int_round_trip() -> Result<(), String> {
    let mut module = load()?;
    module
        .ctl_set_int("dither", 0)
        .map_err(|error| error.to_string())?;
    let value = module
        .ctl_get_int("dither")
        .map_err(|error| error.to_string())?;
    if value != 0 {
        return Err(format!("the integer ctl read back as {value}, expected 0"));
    }
    Ok(())
}

#[test]
fn unknown_ctl_is_error_without_library_call() -> Result<(), String> {
    let mut module = load()?;
    let error = module.ctl_set_bool("no.such", true).err();
    let Some(error) = error else {
        return Err("an unknown ctl must be an error".to_owned());
    };
    if error.input != "no.such" {
        return Err(format!("input is {:?}, expected \"no.such\"", error.input));
    }
    Ok(())
}

#[test]
fn wrong_type_ctl_is_error() -> Result<(), String> {
    // libopenmpt does not check the type in a release build: the C setter for a
    // boolean accepts the key of a float ctl and writes nothing. The wrapper
    // checks the type itself, so the caller sees an error and not a silent no-op.
    let mut module = load()?;
    let error = module.ctl_set_bool("play.tempo_factor", true).err();
    let Some(error) = error else {
        return Err("a boolean set on a float ctl must be an error".to_owned());
    };
    if error.input != "play.tempo_factor" {
        return Err(format!(
            "input is {:?}, expected \"play.tempo_factor\"",
            error.input
        ));
    }
    Ok(())
}

#[test]
fn render_param_round_trip() -> Result<(), String> {
    let mut module = load()?;
    module
        .set_render_param(RenderParam::InterpolationFilterLength, 8)
        .map_err(|error| error.to_string())?;
    let value = module
        .render_param(RenderParam::InterpolationFilterLength)
        .map_err(|error| error.to_string())?;
    if value != 8 {
        return Err(format!(
            "the filter length read back as {value}, expected 8"
        ));
    }
    Ok(())
}

#[test]
fn render_param_snaps_filter_length() -> Result<(), String> {
    // The library maps a length to a resampler mode, and the mode reads back as
    // its own length: 3 becomes cubic, which reports 4.
    let mut module = load()?;
    module
        .set_render_param(RenderParam::InterpolationFilterLength, 3)
        .map_err(|error| error.to_string())?;
    let value = module
        .render_param(RenderParam::InterpolationFilterLength)
        .map_err(|error| error.to_string())?;
    if value != 4 {
        return Err(format!("a length of 3 read back as {value}, expected 4"));
    }
    Ok(())
}

#[test]
fn render_param_rejects_out_of_range_separation() -> Result<(), String> {
    // The C++ mixer computes `value * 128` in an `i32`, so a large value is a
    // signed overflow inside the library.
    let mut module = load()?;
    for value in [201, -1, i32::MAX] {
        let error = module
            .set_render_param(RenderParam::StereoSeparationPercent, value)
            .err();
        let Some(error) = error else {
            return Err(format!("a stereo separation of {value} must be an error"));
        };
        if !error.message.contains("0..=200") {
            return Err(format!(
                "message is {:?}, expected it to name 0..=200",
                error.message
            ));
        }
    }
    Ok(())
}

#[test]
fn render_param_rejects_out_of_range_ramping() -> Result<(), String> {
    let mut module = load()?;
    for value in [11, -2] {
        let error = module
            .set_render_param(RenderParam::VolumeRampingStrength, value)
            .err();
        let Some(error) = error else {
            return Err(format!("a ramping strength of {value} must be an error"));
        };
        if !error.message.contains("-1..=10") {
            return Err(format!(
                "message is {:?}, expected it to name -1..=10",
                error.message
            ));
        }
    }
    Ok(())
}

#[test]
fn render_param_accepts_the_documented_edges() -> Result<(), String> {
    let mut module = load()?;
    let checks = [
        (RenderParam::StereoSeparationPercent, 0),
        (RenderParam::StereoSeparationPercent, 200),
        (RenderParam::VolumeRampingStrength, -1),
        (RenderParam::VolumeRampingStrength, 10),
    ];
    for (param, value) in checks {
        module
            .set_render_param(param, value)
            .map_err(|error| error.to_string())?;
        let read = module
            .render_param(param)
            .map_err(|error| error.to_string())?;
        if read != value {
            return Err(format!("{param:?} {value} read back as {read}"));
        }
    }
    Ok(())
}

#[test]
fn render_param_rejects_negative() -> Result<(), String> {
    let mut module = load()?;
    let error = module
        .set_render_param(RenderParam::InterpolationFilterLength, -1)
        .err();
    let Some(error) = error else {
        return Err("a negative filter length must be an error".to_owned());
    };
    if !error.input.contains("-1") {
        return Err(format!(
            "input is {:?}, expected it to name -1",
            error.input
        ));
    }
    Ok(())
}
