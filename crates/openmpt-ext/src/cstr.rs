//! Strings that libopenmpt gives out and expects the caller to free.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::OpenmptError;
use crate::error::Result;

/// Copies a string that libopenmpt allocated, and frees it at once.
///
/// Returns `None` for a null pointer. A string that is not UTF-8 is copied with
/// replacement characters, because a module file controls the bytes.
pub(crate) fn take(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: the pointer is not null, and every libopenmpt string function
    // returns a NUL-terminated string.
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    let owned = String::from_utf8_lossy(bytes).into_owned();
    // SAFETY: the pointer came from a libopenmpt string function, which
    // transfers ownership to the caller. It is freed one time only, here.
    unsafe { openmpt_ext_sys::openmpt_free_string(ptr) };
    Some(owned)
}

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
    take(ptr).ok_or_else(|| {
        OpenmptError::new(
            op,
            input,
            "libopenmpt gave no string",
            Some("the module may hold text that libopenmpt cannot convert"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::take;
    use std::ffi::CString;

    #[test]
    fn take_copies_a_string() -> Result<(), String> {
        let source = CString::new("hello").map_err(|error| error.to_string())?;
        let text = take(source.into_raw());
        if text.as_deref() != Some("hello") {
            return Err(format!("got {text:?}, expected Some(\"hello\")"));
        }
        Ok(())
    }

    #[test]
    fn take_rejects_null() -> Result<(), String> {
        match take(std::ptr::null()) {
            None => Ok(()),
            Some(text) => Err(format!("a null pointer gave {text:?}")),
        }
    }
}

#[cfg(test)]
mod properties {
    use super::take;
    use proptest::collection::vec;
    use proptest::prelude::any;
    use proptest::prelude::prop_assert;
    use proptest::prelude::prop_assert_eq;
    use proptest::proptest;
    use std::ffi::CString;

    proptest! {
        /// A module file controls these bytes, so the decoder must agree with
        /// `from_utf8_lossy` at every byte value, not only for ASCII.
        #[test]
        fn take_decodes_like_from_utf8_lossy(bytes in vec(1_u8..=255, 0..64)) {
            // `CString::new` rejects a NUL, so the draw starts at 1.
            let source = CString::new(bytes.clone()).unwrap_or_default();
            let taken = take(source.into_raw());
            let expected = String::from_utf8_lossy(&bytes);
            prop_assert_eq!(taken.as_deref(), Some(expected.as_ref()));
        }

        /// A NUL byte anywhere in the input is the only value that a C string
        /// cannot hold.
        #[test]
        fn take_handles_every_byte_value(bytes in vec(any::<u8>(), 0..64)) {
            let Ok(source) = CString::new(bytes) else {
                return Ok(());
            };
            prop_assert!(take(source.into_raw()).is_some());
        }
    }
}
