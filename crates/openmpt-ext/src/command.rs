//! One field of a pattern cell.

use std::os::raw::c_int;

/// One field of a pattern cell, with the `OPENMPT_MODULE_COMMAND_*` id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// The note.
    Note,
    /// The instrument or sample number.
    Instrument,
    /// The volume column effect.
    VolumeEffect,
    /// The effect number.
    Effect,
    /// The volume column value.
    Volume,
    /// The effect parameter.
    Parameter,
}

impl Command {
    /// Returns the id that libopenmpt accepts.
    pub(crate) fn id(self) -> c_int {
        match self {
            Self::Note => openmpt_ext_sys::OPENMPT_MODULE_COMMAND_NOTE.cast_signed(),
            Self::Instrument => openmpt_ext_sys::OPENMPT_MODULE_COMMAND_INSTRUMENT.cast_signed(),
            Self::VolumeEffect => {
                openmpt_ext_sys::OPENMPT_MODULE_COMMAND_VOLUMEEFFECT.cast_signed()
            }
            Self::Effect => openmpt_ext_sys::OPENMPT_MODULE_COMMAND_EFFECT.cast_signed(),
            Self::Volume => openmpt_ext_sys::OPENMPT_MODULE_COMMAND_VOLUME.cast_signed(),
            Self::Parameter => openmpt_ext_sys::OPENMPT_MODULE_COMMAND_PARAMETER.cast_signed(),
        }
    }
}
