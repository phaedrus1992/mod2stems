//! The progress reporting of one export.
//!
//! The caller shows the render count before any work starts, and one line per
//! finished stage after that. A sink writes the lines, so the library stays
//! free of a direct stdout write and a test can capture what the export
//! reported. Design: `docs/design.md` §6.

use std::io::Write;

use crate::export::stage::Stage;

/// The sink that one export reports its progress to.
///
/// The sink writes to any [`Write`], so the command passes stdout and a test
/// passes a buffer. The export calls [`Self::begin`] once before any work
/// starts, so the cost of the run is visible first.
pub struct Progress<W: Write> {
    out: W,
    total: usize,
    done: usize,
}

impl<W: Write> Progress<W> {
    /// Returns a sink that writes to `out`.
    #[must_use]
    pub fn new(out: W) -> Self {
        Self {
            out,
            total: 0,
            done: 0,
        }
    }

    /// Reports the size of the run before any work starts.
    ///
    /// `stages` counts every line that [`Self::stage`] prints, so the counter
    /// never passes its total. `renders` and `files` state the work itself.
    ///
    /// # Errors
    ///
    /// Returns the reason when the sink cannot write.
    pub fn begin(&mut self, stages: usize, renders: usize, files: usize) -> std::io::Result<()> {
        self.total = stages;
        writeln!(
            self.out,
            "The export runs {renders} render(s) and writes {files} audio file(s), in {stages} \
             step(s)."
        )
    }

    /// Reports one finished stage.
    ///
    /// # Errors
    ///
    /// Returns the reason when the sink cannot write.
    pub fn stage(&mut self, stage: Stage) -> std::io::Result<()> {
        self.done = self.done.saturating_add(1);
        writeln!(
            self.out,
            "[{}/{}] {}",
            self.done,
            self.total,
            stage.describe()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Progress;
    use crate::export::stage::Stage;

    #[test]
    fn the_report_names_the_count_before_the_stages() -> Result<(), String> {
        let mut buffer: Vec<u8> = Vec::new();
        {
            let mut progress = Progress::new(&mut buffer);
            progress.begin(3, 3, 2).map_err(|e| e.to_string())?;
            progress
                .stage(Stage::Reference)
                .map_err(|e| e.to_string())?;
        }
        let text = String::from_utf8(buffer).map_err(|e| e.to_string())?;
        let first = text.lines().next().unwrap_or_default();
        if !first.contains('3') {
            return Err(format!("the count does not come first, got {text:?}"));
        }
        if !text.contains("1/3") {
            return Err(format!("the stage does not count, got {text:?}"));
        }
        Ok(())
    }

    #[test]
    fn a_stage_names_the_channel_it_covers() {
        assert!(Stage::Channel(4).describe().contains('4'));
    }
}
