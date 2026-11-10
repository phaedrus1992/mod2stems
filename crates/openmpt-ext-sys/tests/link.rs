//! Checks that the linked library reports the version and revision of the pin.

// The test calls the raw C API, because this crate is the raw layer.
#![expect(unsafe_code, reason = "the test exercises the raw C API")]

use std::ffi::CStr;
use std::ffi::CString;

/// Reads one key with `openmpt_get_string` and frees the result.
fn get_string(key: &str) -> Result<String, String> {
    let key = CString::new(key).map_err(|error| format!("bad key {key:?}: {error}"))?;
    // SAFETY: `key` is a valid NUL-terminated C string. The returned pointer is
    // owned by libopenmpt and stays valid until `openmpt_free_string` runs.
    let raw = unsafe { openmpt_ext_sys::openmpt_get_string(key.as_ptr()) };
    if raw.is_null() {
        return Err(format!(
            "openmpt_get_string({key:?}) returned a null pointer"
        ));
    }
    // SAFETY: the pointer is not null, and libopenmpt terminates the string.
    let value = unsafe { CStr::from_ptr(raw) }
        .to_str()
        .map_err(|error| format!("openmpt_get_string({key:?}) is not UTF-8: {error}"))?
        .to_owned();
    // SAFETY: this pointer came from `openmpt_get_string` and is used no more.
    unsafe { openmpt_ext_sys::openmpt_free_string(raw) };
    Ok(value)
}

/// Compares one value and returns the mismatch as an error.
fn check(name: &str, actual: &str, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name} is {actual:?}, expected {expected:?}"))
    }
}

#[test]
fn library_version_string_is_pinned() -> Result<(), String> {
    // libopenmpt appends the source revision to the release version, so the
    // string reads "<major>.<minor>.<patch>+r<revision>".
    check(
        "library_version",
        &get_string("library_version")?,
        "0.8.10+r25840",
    )
}

#[test]
fn source_revision_comes_from_pin() -> Result<(), String> {
    check(
        "source_revision",
        &get_string("source_revision")?,
        openmpt_ext_sys::PIN_SVN_REVISION,
    )
}
