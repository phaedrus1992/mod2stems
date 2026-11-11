//! The library info strings and the module metadata.

use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;

use crate::cstr::take_required;
use crate::error::OpenmptError;
use crate::error::Result;
use crate::module::Module;

/// One key that `openmpt_get_string` accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryInfo {
    /// The release version, such as `"0.8.10"`.
    LibraryVersion,
    /// The URL of the source tree.
    SourceUrl,
    /// The revision of the source tree.
    SourceRevision,
}

impl LibraryInfo {
    /// Returns the key that `openmpt_get_string` reads.
    fn key(self) -> &'static CStr {
        match self {
            Self::LibraryVersion => c"library_version",
            Self::SourceUrl => c"source_url",
            Self::SourceRevision => c"source_revision",
        }
    }
}

/// Reads one info string about the linked libopenmpt.
///
/// The value is a property of the linked library, not of a module.
///
/// # Errors
///
/// Returns an error when libopenmpt cannot give the string. An unknown key is
/// impossible here, because every [`LibraryInfo`] variant names a real key.
pub fn library_info(key: LibraryInfo) -> Result<String> {
    let name = key.key();
    // SAFETY: `name` is a NUL-terminated string. libopenmpt owns the result.
    let ptr = unsafe { openmpt_ext_sys::openmpt_get_string(name.as_ptr()) };
    take_required(ptr, "library info", &name.to_string_lossy())
}

impl Module {
    /// Returns the metadata keys of this module.
    ///
    /// # Errors
    ///
    /// Returns an error when libopenmpt cannot give the key list.
    pub fn metadata_keys(&self) -> Result<Vec<String>> {
        let ptr = unsafe { openmpt_ext_sys::openmpt_module_get_metadata_keys(self.core()) };
        let list = take_required(ptr, "metadata keys", "this module")?;
        Ok(list.split(';').map(str::to_owned).collect())
    }

    /// Returns the value of one metadata key.
    ///
    /// An unknown key gives an empty string, because that is what libopenmpt
    /// returns for it.
    ///
    /// # Errors
    ///
    /// Returns an error when the key holds a NUL byte, or when libopenmpt cannot
    /// read the value. A read failure is an error, not an empty string, because
    /// an empty string is also the answer for a key that the module does not
    /// carry.
    pub fn metadata(&self, key: &str) -> Result<String> {
        let name = CString::new(key).map_err(|_| {
            OpenmptError::new(
                "metadata",
                key,
                "the key holds a NUL byte",
                Some("a metadata key must be text with no NUL"),
            )
        })?;
        let ptr =
            unsafe { openmpt_ext_sys::openmpt_module_get_metadata(self.core(), name.as_ptr()) };
        take_required(ptr, "metadata", key)
    }

    /// Returns the number of channels.
    #[must_use]
    pub fn num_channels(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_num_channels(self.core()) }
    }

    /// Returns the number of instruments.
    #[must_use]
    pub fn num_instruments(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_num_instruments(self.core()) }
    }

    /// Returns the number of samples.
    #[must_use]
    pub fn num_samples(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_num_samples(self.core()) }
    }

    /// Returns the number of subsongs.
    #[must_use]
    pub fn num_subsongs(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_num_subsongs(self.core()) }
    }

    /// Returns the number of orders.
    #[must_use]
    pub fn num_orders(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_num_orders(self.core()) }
    }

    /// Returns the number of patterns.
    #[must_use]
    pub fn num_patterns(&self) -> i32 {
        // SAFETY: the handle is live. The call only reads a counter.
        unsafe { openmpt_ext_sys::openmpt_module_get_num_patterns(self.core()) }
    }

    /// Returns the name of one channel.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is outside `0..num_channels`.
    pub fn channel_name(&self, index: i32) -> Result<String> {
        self.name(index, Kind::Channel)
    }

    /// Returns the name of one instrument.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is outside `0..num_instruments`.
    pub fn instrument_name(&self, index: i32) -> Result<String> {
        self.name(index, Kind::Instrument)
    }

    /// Returns the name of one sample.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is outside `0..num_samples`.
    pub fn sample_name(&self, index: i32) -> Result<String> {
        self.name(index, Kind::Sample)
    }

    /// Returns the name of one subsong.
    ///
    /// # Errors
    ///
    /// Returns an error when `index` is outside `0..num_subsongs`.
    pub fn subsong_name(&self, index: i32) -> Result<String> {
        self.name(index, Kind::Subsong)
    }

    /// Reads one name, after a check on the index.
    fn name(&self, index: i32, kind: Kind) -> Result<String> {
        check_index("name", kind.name(), index, kind.count(self))?;
        let ptr = kind.get(self.core(), index);
        take_required(ptr, "name", &format!("{} {index}", kind.name()))
    }
}

/// The kinds of index that a module has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A channel.
    Channel,
    /// An instrument.
    Instrument,
    /// A sample.
    Sample,
    /// A subsong.
    Subsong,
}

impl Kind {
    /// Returns the word that names this kind in an error message.
    fn name(self) -> &'static str {
        match self {
            Self::Channel => "channel",
            Self::Instrument => "instrument",
            Self::Sample => "sample",
            Self::Subsong => "subsong",
        }
    }

    /// Returns the number of items of this kind.
    fn count(self, module: &Module) -> i32 {
        match self {
            Self::Channel => module.num_channels(),
            Self::Instrument => module.num_instruments(),
            Self::Sample => module.num_samples(),
            Self::Subsong => module.num_subsongs(),
        }
    }

    /// Calls the getter of this kind.
    fn get(self, core: *mut openmpt_ext_sys::openmpt_module, index: i32) -> *const c_char {
        match self {
            Self::Channel => unsafe {
                openmpt_ext_sys::openmpt_module_get_channel_name(core, index)
            },
            Self::Instrument => unsafe {
                openmpt_ext_sys::openmpt_module_get_instrument_name(core, index)
            },
            Self::Sample => unsafe { openmpt_ext_sys::openmpt_module_get_sample_name(core, index) },
            Self::Subsong => unsafe {
                openmpt_ext_sys::openmpt_module_get_subsong_name(core, index)
            },
        }
    }
}

/// Rejects an index that is outside `0..count`.
///
/// # Errors
///
/// Returns an error that names the kind, the index, and the valid range.
pub(crate) fn check_index(op: &'static str, kind: &str, index: i32, count: i32) -> Result<()> {
    if (0..count).contains(&index) {
        return Ok(());
    }
    Err(OpenmptError::new(
        op,
        format!("{kind} {index}"),
        format!("this module has no such {kind}: the valid range is 0..{count}"),
        Some("read the count first, and use an index below it"),
    ))
}
