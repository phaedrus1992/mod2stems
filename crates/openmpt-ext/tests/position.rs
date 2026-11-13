//! Tests the subsong selection, the playback position, and the pattern data.

mod support;

use openmpt_ext::Command;
use openmpt_ext::Module;
use openmpt_ext::OrderEntry;

/// Loads the simple test file.
fn load() -> Result<Module, String> {
    Module::from_bytes(&support::simple_mod(), &[]).map_err(|error| error.to_string())
}

#[test]
fn select_subsong_out_of_range() -> Result<(), String> {
    let mut module = load()?;
    let error = module.select_subsong(1).err();
    let Some(error) = error else {
        return Err("subsong 1 must be out of range".to_owned());
    };
    if !error.message.contains("0..1") {
        return Err(format!(
            "message is {:?}, expected it to name 0..1",
            error.message
        ));
    }
    Ok(())
}

#[test]
fn select_subsong_minus_one_is_rejected() -> Result<(), String> {
    // The library treats -1 as "all subsongs". This crate renders one subsong at
    // a time, so it does not accept the shorthand.
    let mut module = load()?;
    let error = module.select_subsong(-1).err();
    let Some(error) = error else {
        return Err("subsong -1 must be rejected".to_owned());
    };
    if error.input != "subsong -1" {
        return Err(format!(
            "input is {:?}, expected \"subsong -1\"",
            error.input
        ));
    }
    Ok(())
}

#[test]
fn select_subsong_zero_is_accepted() -> Result<(), String> {
    let mut module = load()?;
    module
        .select_subsong(0)
        .map_err(|error| error.to_string())?;
    if module.selected_subsong() != 0 {
        return Err(format!(
            "the selected subsong is {}, expected 0",
            module.selected_subsong()
        ));
    }
    Ok(())
}

#[test]
fn pattern_cell_reads_generator_values() -> Result<(), String> {
    let module = load()?;
    let instrument = module
        .pattern_cell(0, 0, 2, Command::Instrument)
        .map_err(|error| error.to_string())?;
    if instrument != 3 {
        return Err(format!("channel 2 has instrument {instrument}, expected 3"));
    }
    let note = module
        .pattern_cell(0, 0, 2, Command::Note)
        .map_err(|error| error.to_string())?;
    if note == 0 {
        return Err("channel 2 has no note".to_owned());
    }
    Ok(())
}

#[test]
fn pattern_cell_out_of_range_row() -> Result<(), String> {
    let module = load()?;
    let error = module.pattern_cell(0, 64, 0, Command::Note).err();
    let Some(error) = error else {
        return Err("row 64 must be out of range".to_owned());
    };
    if !error.message.contains("0..64") {
        return Err(format!(
            "message is {:?}, expected it to name 0..64",
            error.message
        ));
    }
    Ok(())
}

#[test]
fn pattern_cell_out_of_range_channel() -> Result<(), String> {
    let module = load()?;
    let error = module.pattern_cell(0, 0, 4, Command::Note).err();
    let Some(error) = error else {
        return Err("channel 4 must be out of range".to_owned());
    };
    if !error.message.contains("0..4") {
        return Err(format!(
            "message is {:?}, expected it to name 0..4",
            error.message
        ));
    }
    Ok(())
}

#[test]
fn pattern_cell_out_of_range_pattern() -> Result<(), String> {
    let module = load()?;
    let error = module.pattern_cell(9, 0, 0, Command::Note).err();
    let Some(error) = error else {
        return Err("pattern 9 must be out of range".to_owned());
    };
    if error.input != "pattern 9" {
        return Err(format!(
            "input is {:?}, expected \"pattern 9\"",
            error.input
        ));
    }
    Ok(())
}

#[test]
fn pattern_num_rows_is_64() -> Result<(), String> {
    let module = load()?;
    let rows = module
        .pattern_num_rows(0)
        .map_err(|error| error.to_string())?;
    if rows != 64 {
        return Err(format!("pattern 0 has {rows} rows, expected 64"));
    }
    Ok(())
}

#[test]
fn pattern_num_rows_out_of_range() -> Result<(), String> {
    let module = load()?;
    let error = module.pattern_num_rows(9).err();
    if error.is_none() {
        return Err("pattern 9 must be out of range".to_owned());
    }
    Ok(())
}

#[test]
fn order_entry_reads_the_order_list() -> Result<(), String> {
    let module = load()?;
    let entry = module.order_entry(0).map_err(|error| error.to_string())?;
    if entry != OrderEntry::Pattern(0) {
        return Err(format!("order 0 gave {entry:?}, expected Pattern(0)"));
    }
    Ok(())
}

#[test]
fn order_entry_rejects_out_of_range_order() -> Result<(), String> {
    let module = load()?;
    let error = module.order_entry(9).err();
    let Some(error) = error else {
        return Err("order 9 must be out of range".to_owned());
    };
    if error.input != "order 9" {
        return Err(format!("input is {:?}, expected \"order 9\"", error.input));
    }
    Ok(())
}

#[test]
fn repeat_count_rejects_negative() -> Result<(), String> {
    // The library reads -1, and every other negative value, as "repeat
    // forever", so a render loop that waits for the song end never stops.
    let mut module = load()?;
    for count in [-1, i32::MIN] {
        let error = module.set_repeat_count(count).err();
        let Some(error) = error else {
            return Err(format!("a repeat count of {count} must be an error"));
        };
        if !error.message.contains("one pass") {
            return Err(format!(
                "message is {:?}, expected it to name one pass",
                error.message
            ));
        }
    }
    Ok(())
}

#[test]
fn repeat_count_accepts_zero_and_positive() -> Result<(), String> {
    let mut module = load()?;
    for count in [0, 3] {
        module
            .set_repeat_count(count)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[test]
fn repeat_count_reads_back_the_value_that_it_set() -> Result<(), String> {
    // A render that sets one pass must prove that the library kept it. A build
    // that ignored the setter would repeat the song with no error.
    let mut module = load()?;
    module
        .set_repeat_count(0)
        .map_err(|error| error.to_string())?;
    if module.repeat_count() != 0 {
        return Err(format!(
            "the repeat count reads {}, expected 0",
            module.repeat_count()
        ));
    }
    Ok(())
}

#[test]
fn the_default_repeat_count_plays_one_pass() -> Result<(), String> {
    // `docs/design.md` §4.2 renders one pass of the order list. The library
    // default is 0, and this check pins that the wrapper reads it as 0 rather
    // than as the "repeat forever" value of -1.
    let module = load()?;
    if module.repeat_count() != 0 {
        return Err(format!(
            "the default repeat count is {}, expected 0",
            module.repeat_count()
        ));
    }
    Ok(())
}

#[test]
fn duration_is_positive() -> Result<(), String> {
    let mut module = load()?;
    let duration = module
        .duration_seconds()
        .map_err(|error| error.to_string())?;
    if duration <= 0.0 {
        return Err(format!(
            "the duration is {duration}, expected a positive value"
        ));
    }
    Ok(())
}

#[test]
fn position_starts_at_zero() -> Result<(), String> {
    let mut module = load()?;
    let position = module
        .position_seconds()
        .map_err(|error| error.to_string())?;
    if position != 0.0 {
        return Err(format!("the position is {position}, expected 0"));
    }
    Ok(())
}

#[test]
fn position_after_load_is_first_row() -> Result<(), String> {
    let module = load()?;
    let same = [
        ("order", module.current_order(), 0),
        ("row", module.current_row(), 0),
        ("pattern", module.current_pattern(), 0),
    ];
    for (name, actual, expected) in same {
        if actual != expected {
            return Err(format!("the {name} is {actual}, expected {expected}"));
        }
    }
    if module.current_speed() < 1 || module.current_tempo() < 32.0 {
        return Err(format!(
            "the speed is {} and the tempo is {}, expected both to be in range",
            module.current_speed(),
            module.current_tempo()
        ));
    }
    Ok(())
}

#[test]
fn repeat_count_zero_is_accepted() -> Result<(), String> {
    // A repeat count of 0 stops the render at the end of the order list.
    let mut module = load()?;
    module
        .set_repeat_count(0)
        .map_err(|error| error.to_string())?;
    Ok(())
}
