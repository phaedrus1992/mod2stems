//! Ownership of one libopenmpt module and its interactive extension.

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::CString;
use std::marker::PhantomData;
use std::os::raw::c_char;
use std::os::raw::c_int;
use std::os::raw::c_void;
use std::ptr;

use openmpt_ext_sys::LIBOPENMPT_EXT_C_INTERFACE_INTERACTIVE;
use openmpt_ext_sys::openmpt_error_func_store;
use openmpt_ext_sys::openmpt_module;
use openmpt_ext_sys::openmpt_module_ext;
use openmpt_ext_sys::openmpt_module_ext_interface_interactive;
use openmpt_ext_sys::openmpt_module_initial_ctl;

use crate::cstr::LibString;
use crate::error::OpenmptError;
use crate::error::Result;
use crate::initial_ctl::InitialCtl;

/// One loaded module, with the interactive extension interface.
///
/// libopenmpt keeps mutable state inside the module, so a `Module` is `Send` and
/// not `Sync`. Move it to a thread. Do not share it.
pub struct Module {
    ext: *mut openmpt_module_ext,
    core: *mut openmpt_module,
    interactive: openmpt_module_ext_interface_interactive,
    pub(crate) ctls: HashSet<String>,
    sink: Box<RefCell<Vec<String>>>,
    // The marker makes the type `!Sync` without a lock. The log callback runs on
    // the calling thread and writes through a `RefCell`.
    _not_sync: PhantomData<Cell<()>>,
}

impl Module {
    /// Loads a module from the bytes of a file.
    ///
    /// The bytes can hold any format that libopenmpt reads.
    ///
    /// # Errors
    ///
    /// Returns an error when libopenmpt cannot load the bytes, when libopenmpt
    /// does not give the interactive interface, or when a key in `ctls` is not a
    /// ctl of this module.
    pub fn from_bytes(bytes: &[u8], ctls: &[InitialCtl]) -> Result<Self> {
        let sink = Box::new(RefCell::new(Vec::new()));
        let sink_ptr = std::ptr::from_ref::<RefCell<Vec<String>>>(&sink)
            .cast_mut()
            .cast::<c_void>();
        let encoded = EncodedCtls::new(ctls)?;
        let mut error: c_int = 0;
        let mut message: *const c_char = ptr::null();
        // SAFETY: `bytes` is valid for `bytes.len()` reads. The sink and the
        // encoded ctls stay alive until the call returns. The error out-params
        // are valid for writes.
        let ext = unsafe {
            openmpt_ext_sys::openmpt_module_ext_create_from_memory(
                bytes.as_ptr().cast::<c_void>(),
                bytes.len(),
                Some(log_callback),
                sink_ptr,
                Some(openmpt_error_func_store),
                ptr::null_mut(),
                &raw mut error,
                &raw mut message,
                encoded.as_ptr(),
            )
        };
        if ext.is_null() {
            let message = LibString::take(message);
            return Err(load_error(&sink, error, message, bytes.len()));
        }
        Module::finish(ext, sink, ctls)
    }

    /// Returns the log lines that libopenmpt wrote, and clears them.
    pub fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.sink.borrow_mut())
    }

    /// Returns the core module handle.
    pub(crate) fn core(&self) -> *mut openmpt_module {
        self.core
    }

    /// Returns the interactive extension interface.
    #[expect(dead_code, reason = "Task 9 reads the interface")]
    pub(crate) fn interactive(&self) -> &openmpt_module_ext_interface_interactive {
        &self.interactive
    }

    /// Clears the stored error, so a later call reports only its own failure.
    ///
    /// libopenmpt does not clear the stored error on its own.
    pub(crate) fn clear_error(&mut self) {
        // SAFETY: the handle is live. The call only writes the error state.
        unsafe { openmpt_ext_sys::openmpt_module_error_clear(self.core) };
    }

    /// Reads the stored error, and returns one when a call set it.
    pub(crate) fn check_last_error(
        &mut self,
        op: &'static str,
        input: impl Into<String>,
    ) -> Result<()> {
        // SAFETY: the handle is live. The call only reads the stored error.
        let code = unsafe { openmpt_ext_sys::openmpt_module_error_get_last(self.core) };
        if code == 0 {
            return Ok(());
        }
        // SAFETY: the handle is live. The module owns the message.
        let ptr = unsafe { openmpt_ext_sys::openmpt_module_error_get_last_message(self.core) };
        let message = LibString::take(ptr).unwrap_or_else(|| format!("libopenmpt error {code}"));
        Err(OpenmptError {
            op,
            input: input.into(),
            code: Some(code),
            message,
            hint: None,
            log: self.take_log(),
        })
    }

    /// Gets the interface, checks the ctl keys, and builds the value.
    fn finish(
        ext: *mut openmpt_module_ext,
        sink: Box<RefCell<Vec<String>>>,
        ctls: &[InitialCtl],
    ) -> Result<Self> {
        // SAFETY: the create call returned a live handle.
        let core = unsafe { openmpt_ext_sys::openmpt_module_ext_get_module(ext) };
        // Zeroes are the valid empty value for a struct of function pointers.
        // SAFETY: every field is a nullable function pointer.
        let mut interactive: openmpt_module_ext_interface_interactive =
            unsafe { std::mem::zeroed() };
        // SAFETY: `ext` is live, and the out-parameter points to a struct of the
        // size that is passed.
        let ok = unsafe {
            openmpt_ext_sys::openmpt_module_ext_get_interface(
                ext,
                LIBOPENMPT_EXT_C_INTERFACE_INTERACTIVE
                    .as_ptr()
                    .cast::<c_char>(),
                ptr::addr_of_mut!(interactive).cast::<c_void>(),
                size_of::<openmpt_module_ext_interface_interactive>(),
            )
        };
        if ok == 0 {
            // SAFETY: the handle is live and is not used after this call.
            unsafe { openmpt_ext_sys::openmpt_module_ext_destroy(ext) };
            return Err(OpenmptError {
                op: "get interactive interface",
                input: "interactive".to_owned(),
                code: None,
                message: "libopenmpt did not give the interactive interface".to_owned(),
                hint: Some("the vendored libopenmpt must build with the extension interface"),
                log: std::mem::take(&mut *sink.borrow_mut()),
            });
        }
        let known = match check_ctl_keys(core, ctls, &sink) {
            Ok(known) => known,
            Err(error) => {
                // SAFETY: the handle is live and is not used after this call.
                unsafe { openmpt_ext_sys::openmpt_module_ext_destroy(ext) };
                return Err(error);
            }
        };
        Ok(Self {
            ext,
            core,
            interactive,
            ctls: known,
            sink,
            _not_sync: PhantomData,
        })
    }
}

// The `!Sync` marker keeps two threads off one module. A move to one thread is
// safe: libopenmpt keeps no thread-local state for a module, and the log sink
// lives in a `RefCell` that the move carries with it.
// SAFETY: the handle and the sink move together and are used by one thread.
unsafe impl Send for Module {}

impl Drop for Module {
    fn drop(&mut self) {
        // SAFETY: the handle came from the create call and is destroyed once.
        unsafe { openmpt_ext_sys::openmpt_module_ext_destroy(self.ext) };
        // The sink drops after this, so no callback can write to it again.
        self.ext = ptr::null_mut();
    }
}

impl std::fmt::Debug for Module {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Module")
            .field("core", &self.core)
            .field("interactive", &"openmpt_module_ext_interface_interactive")
            .field("log_lines", &self.sink.borrow().len())
            .finish_non_exhaustive()
    }
}

/// Builds the error for a failed create call.
fn load_error(
    sink: &RefCell<Vec<String>>,
    code: c_int,
    message: Option<String>,
    len: usize,
) -> OpenmptError {
    let log = std::mem::take(&mut *sink.borrow_mut());
    let message = match message {
        Some(message) if !message.is_empty() => message,
        _ => format!("libopenmpt cannot read these bytes (error code {code})"),
    };
    OpenmptError {
        op: "load",
        input: format!("{len} bytes"),
        code: Some(code),
        message,
        hint: Some("the bytes are not a module format that libopenmpt reads"),
        log,
    }
}

/// Reads the ctl list, and rejects a key that libopenmpt does not report.
fn check_ctl_keys(
    core: *mut openmpt_module,
    ctls: &[InitialCtl],
    sink: &RefCell<Vec<String>>,
) -> Result<HashSet<String>> {
    // SAFETY: the handle is live. The list is owned by the caller.
    let list = LibString::take(unsafe { openmpt_ext_sys::openmpt_module_get_ctls(core) })
        .unwrap_or_default();
    let known: HashSet<String> = list.split(';').map(str::to_owned).collect();
    for ctl in ctls {
        if known.contains(&ctl.key) {
            continue;
        }
        return Err(OpenmptError {
            op: "set initial ctl",
            input: ctl.key.clone(),
            code: None,
            message: format!("this module has no ctl named {:?}", ctl.key),
            hint: Some(
                "libopenmpt ignores an unknown ctl without an error, so this crate rejects it",
            ),
            log: std::mem::take(&mut *sink.borrow_mut()),
        });
    }
    Ok(known)
}

/// The ctl list in the C form, with the strings that the pointers refer to.
///
/// The pointers in `pairs` refer to the strings in `strings`. Keep this value
/// alive until the create call returns.
struct EncodedCtls {
    // Held only so the strings that `pairs` points at stay alive. The field is
    // never read, and it must not be dropped before the create call runs.
    _strings: Vec<CString>,
    pairs: Vec<openmpt_module_initial_ctl>,
}

impl EncodedCtls {
    /// Converts the ctls, and appends the null pair that ends the array.
    fn new(ctls: &[InitialCtl]) -> Result<Self> {
        let mut strings = Vec::with_capacity(ctls.len() * 2);
        for ctl in ctls {
            strings.push(to_cstring(&ctl.key)?);
            strings.push(to_cstring(&ctl.value)?);
        }
        let mut pairs = Vec::with_capacity(ctls.len() + 1);
        for index in 0..ctls.len() {
            let (Some(key), Some(value)) = (strings.get(index * 2), strings.get(index * 2 + 1))
            else {
                continue;
            };
            pairs.push(openmpt_module_initial_ctl {
                ctl: key.as_ptr(),
                value: value.as_ptr(),
            });
        }
        pairs.push(openmpt_module_initial_ctl {
            ctl: ptr::null(),
            value: ptr::null(),
        });
        Ok(Self {
            _strings: strings,
            pairs,
        })
    }

    /// Returns the array pointer for the create call.
    fn as_ptr(&self) -> *const openmpt_module_initial_ctl {
        self.pairs.as_ptr()
    }
}

/// Converts one text field to a `CString`.
fn to_cstring(text: &str) -> Result<CString> {
    CString::new(text).map_err(|_| OpenmptError {
        op: "set initial ctl",
        input: text.to_owned(),
        code: None,
        message: "this text holds a NUL byte".to_owned(),
        hint: Some("a ctl key and a ctl value must be text with no NUL"),
        log: Vec::new(),
    })
}

/// The libopenmpt log callback. It appends one message to the sink.
///
/// A failed borrow does nothing. This callback runs inside the C library, and an
/// unwind across a C frame is undefined behavior.
extern "C" fn log_callback(message: *const c_char, user: *mut c_void) {
    if message.is_null() || user.is_null() {
        return;
    }
    // SAFETY: the pointer is not null, and libopenmpt terminates the string.
    let text = unsafe { std::ffi::CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: `user` is the sink pointer that `from_bytes` passed to the create
    // call. The sink stays live until the module drops.
    let sink = unsafe { &*user.cast::<RefCell<Vec<String>>>() };
    if let Ok(mut log) = sink.try_borrow_mut() {
        log.push(text);
    }
}
