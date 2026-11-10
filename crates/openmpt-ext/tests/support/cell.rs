//! One pattern cell.

/// One pattern cell.
pub struct Cell {
    /// Note period in `ProTracker` units. Zero means no note.
    pub period: u16,
    /// Sample number, 1 to 31. Zero means no sample.
    pub sample: u8,
    /// Effect number.
    pub effect: u8,
    /// Effect parameter.
    pub param: u8,
}
