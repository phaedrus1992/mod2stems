//! One position that a render visited.

/// One position that a render visited.
///
/// The pair is one order-list entry and one row in its pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// The order-list index.
    pub order: i32,
    /// The row inside the pattern that the order plays.
    pub row: i32,
}

impl Position {
    /// Builds a position.
    #[must_use]
    pub fn new(order: i32, row: i32) -> Self {
        Self { order, row }
    }
}
