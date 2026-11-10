//! Tests the module lifecycle: load, error reporting, log capture, and drop.

mod support;

use openmpt_ext::InitialCtl;
use openmpt_ext::Module;

/// Loads bytes and turns any error into a message.
fn load(bytes: &[u8], ctls: &[InitialCtl]) -> Result<Module, String> {
    Module::from_bytes(bytes, ctls).map_err(|error| error.to_string())
}

#[test]
fn loads_simple_mod() -> Result<(), String> {
    let module = load(&support::simple_mod(), &[])?;
    drop(module);
    Ok(())
}

#[test]
fn empty_bytes_is_error() -> Result<(), String> {
    let error = Module::from_bytes(&[], &[]).err();
    let Some(error) = error else {
        return Err("empty bytes must fail to load".to_owned());
    };
    if error.op != "load" {
        return Err(format!("op is {:?}, expected \"load\"", error.op));
    }
    if error.message.is_empty() {
        return Err("the error message is empty".to_owned());
    }
    Ok(())
}

#[test]
fn garbage_bytes_is_error() -> Result<(), String> {
    let error = Module::from_bytes(&[0x5A; 4096], &[]).err();
    let Some(error) = error else {
        return Err("garbage bytes must fail to load".to_owned());
    };
    if error.op != "load" {
        return Err(format!("op is {:?}, expected \"load\"", error.op));
    }
    if error.message.is_empty() {
        return Err("the error message is empty".to_owned());
    }
    Ok(())
}

#[test]
fn unknown_initial_ctl_is_error() -> Result<(), String> {
    let ctls = [InitialCtl {
        key: "no.such.ctl".to_owned(),
        value: "1".to_owned(),
    }];
    let error = Module::from_bytes(&support::simple_mod(), &ctls).err();
    let Some(error) = error else {
        return Err("an unknown initial ctl must fail to load".to_owned());
    };
    if error.input != "no.such.ctl" {
        return Err(format!(
            "input is {:?}, expected \"no.such.ctl\"",
            error.input
        ));
    }
    Ok(())
}

#[test]
fn drop_many_modules() -> Result<(), String> {
    let bytes = support::simple_mod();
    for _ in 0..1000 {
        let module = load(&bytes, &[])?;
        drop(module);
    }
    Ok(())
}

#[test]
fn module_is_send() -> Result<(), String> {
    let module = load(&support::simple_mod(), &[])?;
    let handle = std::thread::spawn(move || {
        let mut module = module;
        module.take_log().len()
    });
    handle
        .join()
        .map_err(|_| "the module thread panicked".to_owned())?;
    Ok(())
}

#[test]
fn load_warning_is_logged() -> Result<(), String> {
    let mut module = load(&startrekker_am(), &[])?;
    let first = module.take_log();
    if first.is_empty() {
        return Err("a Startrekker file must log a message".to_owned());
    }
    let second = module.take_log();
    if !second.is_empty() {
        return Err(format!("the log must drain, but holds {second:?}"));
    }
    Ok(())
}

/// Builds an eight-channel Startrekker FLT8 file that starts a warning.
///
/// The file names four samples in the pattern but gives no sample data, which the
/// loader reports as a Startrekker AM file that misses its companion file
/// (`soundlib/Load_mod.cpp`).
fn startrekker_am() -> Vec<u8> {
    let names: [&[u8]; 4] = [b"one", b"two", b"three", b"four"];
    let mut samples = names
        .iter()
        .map(|name| support::sample_with_name(name))
        .collect::<Vec<_>>();
    // The loader counts a sample as referenced only when the pattern holds a
    // sample number for it. Lengths stay zero, so no data follows.
    for sample in &mut samples {
        sample.data.clear();
    }
    let row = [0, 1, 2, 3].map(|channel| support::Cell {
        period: 428,
        sample: channel + 1,
        effect: 0,
        param: 0,
    });
    support::build_with_magic(
        &support::ModSpec {
            rows: vec![row],
            samples,
            orders: vec![0],
        },
        *b"FLT8",
    )
}
