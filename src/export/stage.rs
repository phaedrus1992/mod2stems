//! The stages that an export reports, and the line that each one prints.

/// One stage of an export that the progress sink reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The unmuted reference render.
    Reference,
    /// The render of one tracker channel.
    Channel(i32),
    /// The write of the file of one tracker channel.
    File(i32),
    /// The write of the mix of the channel stems.
    Mix,
    /// The write of the unmuted render as audio.
    ReferenceMix,
    /// The publication of the finished export.
    Publish,
}

impl Stage {
    /// Returns the line that the sink prints for this stage.
    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Self::Reference => "rendered the unmuted reference".to_owned(),
            Self::Channel(channel) => format!("rendered channel {channel}"),
            Self::File(channel) => format!("wrote the file of channel {channel}"),
            Self::Mix => "wrote the mix of the channel stems".to_owned(),
            Self::ReferenceMix => "wrote the unmuted render as an audio file".to_owned(),
            Self::Publish => "published the export".to_owned(),
        }
    }
}
