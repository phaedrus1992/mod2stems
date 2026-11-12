//! The order and row sequence that one render passes through.
//!
//! The type token that selects the silencing method is only a proxy for the
//! playback rules. A broken isolation shows itself as a different song flow, so
//! each render records the position that it visits, and the caller compares
//! that trace with the trace of the unmuted render
//! (`docs/design.md` §4.1).

use crate::stems::position::Position;

/// The positions that one render visited, in order.
///
/// A position lands in the trace when the render enters it, so two renders of
/// the same song give the same trace however many frames each read covers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowTrace {
    positions: Vec<Position>,
}

impl RowTrace {
    /// Returns the positions that the render visited.
    #[must_use]
    pub fn positions(&self) -> &[Position] {
        &self.positions
    }

    /// Appends the current position when it differs from the last one.
    ///
    /// The library reports the current position between reads, and reads
    /// happen many times per row. The trace keeps only the transitions, so a
    /// smaller read block does not change it.
    pub fn record(&mut self, position: Position) {
        if self.positions.last() != Some(&position) {
            self.positions.push(position);
        }
    }

    /// Returns the first position that one trace visits and another does not.
    ///
    /// Returns `None` when the two traces agree. The index is the position in
    /// the sequence, so a caller can name where the flows diverge.
    #[must_use]
    pub fn first_divergence(&self, other: &Self) -> Option<usize> {
        let mine = self.positions();
        let theirs = other.positions();
        let shared = mine.len().min(theirs.len());
        for index in 0..shared {
            if mine.get(index) != theirs.get(index) {
                return Some(index);
            }
        }
        if mine.len() == theirs.len() {
            return None;
        }
        Some(shared)
    }
}
