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
    // A Startrekker AM file with no sample data makes the loader write a warning.
    let mut module = load(&support::startrekker_am(), &[])?;
    let first = module.take_log();
    if first.is_empty() {
        return Err("a truncated sample must log a message".to_owned());
    }
    let second = module.take_log();
    if !second.is_empty() {
        return Err(format!("the log must drain, but holds {second:?}"));
    }
    Ok(())
}
