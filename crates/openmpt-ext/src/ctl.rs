//! The ctls and the render params, with a type check before each call.

use std::collections::HashSet;
use std::ffi::CString;
use std::ops::RangeInclusive;
use std::os::raw::c_int;

use crate::cstr::take_required;
use crate::error::OpenmptError;
use crate::error::Result;
use crate::module::Module;

/// The stereo separation range that the library documents, in percent.
const STEREO_SEPARATION_PERCENT: RangeInclusive<i32> = 0..=200;
/// The volume ramping range that the library documents.
///
/// A value of -1 selects the library default.
const VOLUME_RAMPING_STRENGTH: RangeInclusive<i32> = -1..=10;

/// One render parameter, with the `OPENMPT_MODULE_RENDER_*` id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderParam {
    /// The master gain, in millibel.
    MasterGainMillibel,
    /// The stereo separation, in percent.
    StereoSeparationPercent,
    /// The interpolation filter length. The library snaps it to a resampler mode.
    InterpolationFilterLength,
    /// The volume ramping strength.
    VolumeRampingStrength,
}

impl RenderParam {
    /// Returns the id that libopenmpt accepts.
    fn id(self) -> c_int {
        match self {
            Self::MasterGainMillibel => {
                openmpt_ext_sys::OPENMPT_MODULE_RENDER_MASTERGAIN_MILLIBEL.cast_signed()
            }
            Self::StereoSeparationPercent => {
                openmpt_ext_sys::OPENMPT_MODULE_RENDER_STEREOSEPARATION_PERCENT.cast_signed()
            }
            Self::InterpolationFilterLength => {
                openmpt_ext_sys::OPENMPT_MODULE_RENDER_INTERPOLATIONFILTER_LENGTH.cast_signed()
            }
            Self::VolumeRampingStrength => {
                openmpt_ext_sys::OPENMPT_MODULE_RENDER_VOLUMERAMPING_STRENGTH.cast_signed()
            }
        }
    }

    /// Returns the value range that the library documents, when it has one.
    ///
    /// The library clamps a filter length and accepts any gain, so only two
    /// params need a check. The other two return `None`.
    fn range(self) -> Option<RangeInclusive<i32>> {
        match self {
            Self::StereoSeparationPercent => Some(STEREO_SEPARATION_PERCENT),
            Self::VolumeRampingStrength => Some(VOLUME_RAMPING_STRENGTH),
            Self::MasterGainMillibel | Self::InterpolationFilterLength => None,
        }
    }

    /// Returns the name that an error message uses.
    fn name(self) -> &'static str {
        match self {
            Self::MasterGainMillibel => "master_gain_millibel",
            Self::StereoSeparationPercent => "stereo_separation_percent",
            Self::InterpolationFilterLength => "interpolation_filter_length",
            Self::VolumeRampingStrength => "volume_ramping_strength",
        }
    }
}

/// The value type of one ctl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CtlType {
    /// A boolean.
    Bool,
    /// A 64-bit integer.
    Int,
    /// A 64-bit float.
    Float,
    /// Text.
    Text,
}

/// Returns the type of a ctl, or `None` for a key that this crate does not know.
///
/// The table copies `get_ctl_infos` from `libopenmpt_impl.cpp`. The library does
/// not check the type in a release build: a setter of the wrong type returns
/// success and changes nothing. This table makes that case an error.
fn ctl_type(key: &str) -> Option<CtlType> {
    let kind = match key {
        "load.skip_samples"
        | "load.skip_patterns"
        | "load.skip_plugins"
        | "load.skip_subsongs_init"
        | "seek.sync_samples"
        | "render.resampler.emulate_amiga" => CtlType::Bool,
        "subsong" | "dither" => CtlType::Int,
        "play.tempo_factor" | "play.pitch_factor" | "render.opl.volume_factor" => CtlType::Float,
        "play.at_end" | "render.resampler.emulate_amiga_type" => CtlType::Text,
        _ => return None,
    };
    Some(kind)
}

impl Module {
    /// Returns the ctls of this module.
    #[must_use]
    pub fn ctls(&self) -> &HashSet<String> {
        &self.ctls
    }

    /// Reports whether this crate knows the value type of a ctl.
    #[must_use]
    pub fn ctl_type_is_known(&self, key: &str) -> bool {
        ctl_type(key).is_some()
    }

    /// Sets a boolean ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not a boolean ctl, or when libopenmpt rejects the value.
    pub fn ctl_set_bool(&mut self, key: &str, value: bool) -> Result<()> {
        let name = Self::checked_ctl(key, CtlType::Bool)?;
        self.clear_error();
        // SAFETY: the handle is live, and `name` is a valid C string.
        let ok = unsafe {
            openmpt_ext_sys::openmpt_module_ctl_set_boolean(
                self.core(),
                name.as_ptr(),
                c_int::from(value),
            )
        };
        self.check_ok("ctl set", key, ok)
    }

    /// Sets an integer ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not an integer ctl, or when libopenmpt rejects the value.
    pub fn ctl_set_int(&mut self, key: &str, value: i64) -> Result<()> {
        let name = Self::checked_ctl(key, CtlType::Int)?;
        self.clear_error();
        // SAFETY: the handle is live, and `name` is a valid C string.
        let ok = unsafe {
            openmpt_ext_sys::openmpt_module_ctl_set_integer(self.core(), name.as_ptr(), value)
        };
        self.check_ok("ctl set", key, ok)
    }

    /// Sets a float ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not a float ctl, or when libopenmpt rejects the value.
    pub fn ctl_set_float(&mut self, key: &str, value: f64) -> Result<()> {
        let name = Self::checked_ctl(key, CtlType::Float)?;
        self.clear_error();
        // SAFETY: the handle is live, and `name` is a valid C string.
        let ok = unsafe {
            openmpt_ext_sys::openmpt_module_ctl_set_floatingpoint(self.core(), name.as_ptr(), value)
        };
        self.check_ok("ctl set", key, ok)
    }

    /// Sets a text ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not a text ctl, when the value holds a NUL byte, or when libopenmpt
    /// rejects the value.
    pub fn ctl_set_text(&mut self, key: &str, value: &str) -> Result<()> {
        let name = Self::checked_ctl(key, CtlType::Text)?;
        let text = CString::new(value).map_err(|_| {
            OpenmptError::new(
                "ctl set",
                key,
                "the value holds a NUL byte",
                Some("a ctl value must be text with no NUL"),
            )
        })?;
        self.clear_error();
        // SAFETY: the handle is live, and both strings are valid C strings.
        let ok = unsafe {
            openmpt_ext_sys::openmpt_module_ctl_set_text(self.core(), name.as_ptr(), text.as_ptr())
        };
        self.check_ok("ctl set", key, ok)
    }

    /// Reads a boolean ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not a boolean ctl, or when libopenmpt rejects the read.
    pub fn ctl_get_bool(&mut self, key: &str) -> Result<bool> {
        let name = Self::checked_ctl(key, CtlType::Bool)?;
        self.clear_error();
        // SAFETY: the handle is live, and `name` is a valid C string.
        let value =
            unsafe { openmpt_ext_sys::openmpt_module_ctl_get_boolean(self.core(), name.as_ptr()) };
        self.check_last_error("ctl get", key)?;
        Ok(value != 0)
    }

    /// Reads an integer ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not an integer ctl, or when libopenmpt rejects the read.
    pub fn ctl_get_int(&mut self, key: &str) -> Result<i64> {
        let name = Self::checked_ctl(key, CtlType::Int)?;
        self.clear_error();
        // SAFETY: the handle is live, and `name` is a valid C string.
        let value =
            unsafe { openmpt_ext_sys::openmpt_module_ctl_get_integer(self.core(), name.as_ptr()) };
        self.check_last_error("ctl get", key)?;
        Ok(value)
    }

    /// Reads a float ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not a float ctl, or when libopenmpt rejects the read.
    pub fn ctl_get_float(&mut self, key: &str) -> Result<f64> {
        let name = Self::checked_ctl(key, CtlType::Float)?;
        self.clear_error();
        // SAFETY: the handle is live, and `name` is a valid C string.
        let value = unsafe {
            openmpt_ext_sys::openmpt_module_ctl_get_floatingpoint(self.core(), name.as_ptr())
        };
        self.check_last_error("ctl get", key)?;
        Ok(value)
    }

    /// Reads a text ctl.
    ///
    /// # Errors
    ///
    /// Returns an error when the key is not a ctl of this module, when the key
    /// is not a text ctl, or when libopenmpt rejects the read.
    pub fn ctl_get_text(&mut self, key: &str) -> Result<String> {
        let name = Self::checked_ctl(key, CtlType::Text)?;
        self.clear_error();
        // SAFETY: the handle is live, and `name` is a valid C string.
        let ptr =
            unsafe { openmpt_ext_sys::openmpt_module_ctl_get_text(self.core(), name.as_ptr()) };
        self.check_last_error("ctl get", key)?;
        take_required(ptr, "ctl get", key)
    }

    /// Sets one render parameter.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is outside the range that libopenmpt
    /// documents for `param`, when libopenmpt rejects the value, or when the
    /// filter length is negative.
    pub fn set_render_param(&mut self, param: RenderParam, value: i32) -> Result<()> {
        check_render_param(param, value)?;
        self.clear_error();
        // SAFETY: the handle is live. The other arguments are plain values.
        let ok = unsafe {
            openmpt_ext_sys::openmpt_module_set_render_param(self.core(), param.id(), value)
        };
        self.check_ok("set render param", &format!("{} {value}", param.name()), ok)
    }

    /// Reads one render parameter.
    ///
    /// # Errors
    ///
    /// Returns an error when libopenmpt rejects the read.
    pub fn render_param(&mut self, param: RenderParam) -> Result<i32> {
        self.clear_error();
        let mut value: i32 = 0;
        // SAFETY: the handle is live, and the out-parameter points to an `i32`.
        let ok = unsafe {
            openmpt_ext_sys::openmpt_module_get_render_param(
                self.core(),
                param.id(),
                &raw mut value,
            )
        };
        self.check_ok("render param", param.name(), ok)?;
        Ok(value)
    }

    /// Checks the key and the type, and returns the key as a C string.
    fn checked_ctl(key: &str, wanted: CtlType) -> Result<CString> {
        let Some(found) = ctl_type(key) else {
            return Err(OpenmptError::new(
                "ctl",
                key,
                format!("this module has no ctl named {key:?}"),
                Some("read `Module::ctls` for the keys that this module accepts"),
            ));
        };
        if found != wanted {
            return Err(OpenmptError::new(
                "ctl",
                key,
                format!("the ctl {key:?} does not hold this value type"),
                Some("libopenmpt ignores a value of the wrong type, so this crate rejects it"),
            ));
        }
        CString::new(key).map_err(|_| {
            OpenmptError::new(
                "ctl",
                key,
                "the key holds a NUL byte",
                Some("a ctl key must be text with no NUL"),
            )
        })
    }
}

/// Rejects a render-param value outside the range that the library documents.
///
/// The C++ mixer multiplies the stereo separation by 128 and the ramping by
/// 1000 in an `i32`, so a value near the `i32` limit is a signed overflow in
/// the library. The library clamps neither value.
fn check_render_param(param: RenderParam, value: i32) -> Result<()> {
    let Some(range) = param.range() else {
        return Ok(());
    };
    if range.contains(&value) {
        return Ok(());
    }
    Err(OpenmptError::new(
        "set render param",
        format!("{} {value}", param.name()),
        format!(
            "the value {value} is outside {}..={} for {}",
            range.start(),
            range.end(),
            param.name()
        ),
        Some("use a value in the range that libopenmpt documents"),
    ))
}
