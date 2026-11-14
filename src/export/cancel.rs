//! The cancellation flag of one export.
//!
//! A caller cancels an export by setting one shared flag. The export checks the
//! flag between renders, so a cancelled export starts no more work and never
//! publishes a partial set (`docs/design.md` §5.2).

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// The cancellation flag of one export.
///
/// The value is cheap to clone, so the command and the signal handler hold one
/// handle each. The flag never clears, because a cancelled export does not
/// resume.
#[derive(Debug, Clone, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
}

impl Cancel {
    /// Returns a flag that is not set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the shared flag, for a signal handler.
    ///
    /// A signal handler needs a value that owns no lock and allocates nothing.
    /// The shared atomic is that value, and the command registers it once.
    #[must_use]
    pub fn shared(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    /// Sets the flag.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Returns true when the caller cancelled the export.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::Cancel;

    #[test]
    fn a_shared_handle_sets_the_flag_of_its_export() {
        // The command registers the shared flag with the signal handler, so a
        // copy of the shared value must reach the export.
        let cancel = Cancel::new();
        assert!(!cancel.is_cancelled());
        let shared = cancel.shared();
        shared.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(cancel.is_cancelled());
    }

    #[test]
    fn a_cancelled_export_stays_cancelled() {
        let cancel = Cancel::new();
        cancel.cancel();
        assert!(cancel.clone().is_cancelled());
    }
}
