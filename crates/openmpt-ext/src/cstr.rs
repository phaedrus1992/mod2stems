//! A guard for a string that libopenmpt gives out and expects the caller to free.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::OpenmptError;
use crate::error::Result;

/// Owns a string from libopenmpt, and frees it on drop.
pub(crate) struct LibString(*const c_char);

/// Takes one string that libopenmpt allocated, and frees it.
///
/// A null pointer is an error and not an empty string, because the C functions
/// return null when they fail, and an empty string means "no value" for several
/// of them.
///
/// # Errors
///
/// Returns an error when libopenmpt gave no string.
pub(crate) fn take_required(ptr: *const c_char, op: &'static str, input: &str) -> Result<String> {
    LibString::take(ptr).ok_or_else(|| {
        OpenmptError::new(
            op,
            input,
            "libopenmpt gave no string",
            Some("the module may hold text that libopenmpt cannot convert"),
        )
    })
}

impl LibString {
    /// Copies the string into a `String` and frees it at once.
    ///
    /// Returns `None` for a null pointer. A string that is not UTF-8 is copied
    /// with replacement characters, because a module file controls the bytes.
    pub(crate) fn take(ptr: *const c_char) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        // SAFETY: the pointer is not null, and every libopenmpt string function
        // returns a NUL-terminated string.
        let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
        let owned = String::from_utf8_lossy(bytes).into_owned();
        drop(Self(ptr));
        Some(owned)
    }
}

impl Drop for LibString {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        // SAFETY: the pointer came from a libopenmpt string function, which
        // transfers ownership to the caller. It is freed one time only, here.
        unsafe { openmpt_ext_sys::openmpt_free_string(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::LibString;
    use std::ffi::CString;

    #[test]
    fn take_copies_a_string() -> Result<(), String> {
        let source = CString::new("hello").map_err(|error| error.to_string())?;
        let text = LibString::take(source.into_raw());
        if text.as_deref() != Some("hello") {
            return Err(format!("got {text:?}, expected Some(\"hello\")"));
        }
        Ok(())
    }

    #[test]
    fn take_rejects_null() -> Result<(), String> {
        match LibString::take(std::ptr::null()) {
            None => Ok(()),
            Some(text) => Err(format!("a null pointer gave {text:?}")),
        }
    }
}
