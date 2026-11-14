//! Checks the progress lines of one export.
//!
//! The lines say "n/m". A numerator above the denominator tells the person
//! that the export is over budget and still running.

use mod2stems::export::Cancel;
use mod2stems::export::ExportRequest;
use mod2stems::export::Progress;
use mod2stems::export::run;

#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/header.rs"]
mod header;

use fixture::require;
use fixture::require_eq;
use fixture::temporary;
use fixture::write_fixture;

/// Returns the done and total counts of one progress line.
fn counts(line: &str) -> Option<(usize, usize)> {
    let body = line.strip_prefix('[')?.split(']').next()?;
    let (done, total) = body.split_once('/')?;
    Some((done.parse().ok()?, total.parse().ok()?))
}

#[test]
fn the_progress_never_counts_past_its_total() -> Result<(), String> {
    let workspace = temporary("export-progress")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination);
    request.write_mix = true;
    request.write_reference_mix = true;
    let mut buffer: Vec<u8> = Vec::new();
    {
        let mut progress = Progress::new(&mut buffer);
        let cancel = Cancel::new();
        run(&request, &cancel, &mut progress).map_err(|error| error.to_string())?;
    }
    let text = String::from_utf8_lossy(&buffer).into_owned();
    let mut last = 0;
    let mut total = 0;
    let mut seen = false;
    for line in text.lines() {
        let Some((done, stated)) = counts(line) else {
            continue;
        };
        require(
            done <= stated,
            &format!("the line {line} counts past its total"),
        )?;
        if seen {
            require_eq(&stated, &total, "every line states one total")?;
        }
        seen = true;
        last = done;
        total = stated;
    }
    require(
        seen,
        &format!("the export reports its stages, got {text:?}"),
    )?;
    require_eq(&last, &total, "the last stage reaches the total")
}
