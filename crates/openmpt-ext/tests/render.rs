//! Tests the float stereo render.

mod support;

use openmpt_ext::Module;

/// Loads the simple test file.
fn load() -> Result<Module, String> {
    Module::from_bytes(&support::simple_mod(), &[]).map_err(|error| error.to_string())
}

#[test]
fn renders_frames() -> Result<(), String> {
    let mut module = load()?;
    let mut buf = vec![0.0_f32; 4096];
    let frames = module
        .read_float(48_000, &mut buf)
        .map_err(|error| error.to_string())?;
    if frames != 2048 {
        return Err(format!(
            "a 4096-sample buffer gave {frames} frames, expected 2048"
        ));
    }
    Ok(())
}

#[test]
fn samples_are_finite() -> Result<(), String> {
    let mut module = load()?;
    let mut buf = vec![0.0_f32; 4096];
    module
        .read_float(48_000, &mut buf)
        .map_err(|error| error.to_string())?;
    if let Some(index) = buf.iter().position(|value| !value.is_finite()) {
        return Err(format!(
            "sample {index} is {}, expected a finite value",
            buf[index]
        ));
    }
    Ok(())
}

#[test]
fn render_is_not_silent() -> Result<(), String> {
    let mut module = load()?;
    let mut buf = vec![0.0_f32; 4096];
    module
        .read_float(48_000, &mut buf)
        .map_err(|error| error.to_string())?;
    if buf.iter().all(|value| *value == 0.0) {
        return Err("the render is silent, expected a signal".to_owned());
    }
    Ok(())
}

#[test]
fn odd_buffer_is_error() -> Result<(), String> {
    let mut module = load()?;
    let mut buf = [0.0_f32; 5];
    let error = module.read_float(48_000, &mut buf).err();
    let Some(error) = error else {
        return Err("an odd buffer length must be an error".to_owned());
    };
    if !error.message.contains("even") {
        return Err(format!(
            "message is {:?}, expected it to name the even-length rule",
            error.message
        ));
    }
    Ok(())
}

#[test]
fn song_end_returns_zero() -> Result<(), String> {
    let mut module = load()?;
    module
        .set_repeat_count(0)
        .map_err(|error| error.to_string())?;
    let mut buf = vec![0.0_f32; 4096];
    // The test file is near one second long, so this bound ends after the song.
    for _ in 0..2000 {
        let frames = module
            .read_float(48_000, &mut buf)
            .map_err(|error| error.to_string())?;
        if frames == 0 {
            let after = module
                .read_float(48_000, &mut buf)
                .map_err(|error| error.to_string())?;
            if after != 0 {
                return Err(format!(
                    "a read after the end gave {after} frames, expected 0"
                ));
            }
            // The position stays readable at the end of the song.
            let _ = module.current_row();
            return Ok(());
        }
    }
    Err("the song did not end in 2000 reads".to_owned())
}

#[test]
fn bad_rate_is_error() -> Result<(), String> {
    for rate in [0, 1_000_000] {
        let mut module = load()?;
        let mut buf = vec![0.0_f32; 1024];
        let result = module.read_float(rate, &mut buf);
        let Some(error) = result.err() else {
            return Err(format!("rate {rate} must be an error"));
        };
        if !error.message.contains(&rate.to_string()) {
            return Err(format!(
                "message is {:?}, expected it to name the rate {rate}",
                error.message
            ));
        }
    }
    Ok(())
}

#[test]
fn the_range_ends_are_inclusive() -> Result<(), String> {
    let mut buf = vec![0.0_f32; 1024];
    for rate in [7999, 8000, 192_000, 192_001] {
        let mut module = load()?;
        let inside = (8000..=192_000).contains(&rate);
        let result = module.read_float(rate, &mut buf);
        if result.is_ok() != inside {
            return Err(format!(
                "rate {rate} gave {result:?}, expected {inside} from the range 8000..=192000"
            ));
        }
    }
    Ok(())
}

#[test]
fn position_advances_by_row() -> Result<(), String> {
    let mut module = load()?;
    module
        .set_repeat_count(0)
        .map_err(|error| error.to_string())?;
    let mut buf = vec![0.0_f32; 128];
    let mut order_rows = Vec::new();
    for _ in 0..200 {
        let frames = module
            .read_float(48_000, &mut buf)
            .map_err(|error| error.to_string())?;
        if frames == 0 {
            break;
        }
        let row = (module.current_order(), module.current_row());
        if order_rows.last() != Some(&row) {
            order_rows.push(row);
        }
    }
    let wanted = [(0, 0), (0, 1), (0, 2)];
    if order_rows.len() < wanted.len() || order_rows[..wanted.len()] != wanted {
        return Err(format!(
            "the row sequence starts {order_rows:?}, expected it to start {wanted:?}"
        ));
    }
    Ok(())
}
