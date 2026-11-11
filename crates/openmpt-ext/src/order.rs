//! One entry of the order list.

/// One entry of the order list.
///
/// The library gives a skip and a stop entry as the sentinel values `65534` and
/// `65535`. Neither is a pattern index, so this crate reports them as their own
/// variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderEntry {
    /// The entry plays this pattern.
    Pattern(i32),
    /// The entry is a skip ("+++"). Playback continues at the next entry.
    Skip,
    /// The entry is a stop ("---"). Playback continues at the restart position.
    Stop,
}

impl OrderEntry {
    /// Builds an entry from the raw order-list value and the two predicates.
    pub(crate) fn new(raw: i32, skip: bool, stop: bool) -> Self {
        match (skip, stop) {
            (true, _) => Self::Skip,
            (_, true) => Self::Stop,
            _ => Self::Pattern(raw),
        }
    }
}
