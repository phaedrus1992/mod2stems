//! One initial ctl that the caller sets when the module loads.

/// One initial ctl, such as `render.resampler.emulate_amiga_type` set to
/// `"a500"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialCtl {
    /// The ctl name.
    pub key: String,
    /// The value to set.
    pub value: String,
}
