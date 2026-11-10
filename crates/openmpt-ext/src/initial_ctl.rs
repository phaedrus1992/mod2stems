//! One initial ctl that the caller sets when the module loads.

/// One initial ctl, such as `render.resampler.internal.fir8tap` set to `"0"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialCtl {
    /// The ctl name.
    pub key: String,
    /// The value to set.
    pub value: String,
}
