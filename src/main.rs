//! The mod2stems command.

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use openmpt_ext::LibraryInfo;
use openmpt_ext::library_info;

use mod2stems::export::Cancel;
use mod2stems::export::Progress;
use mod2stems::export::run;

mod cli;

/// The signal that stops an export.
///
/// The command reports the staging files and exits with a failure code, so a
/// cancelled export is never mistaken for a complete one.
const INTERRUPT_SIGNAL: i32 = signal_hook::consts::SIGINT;

// The workspace denies terminal output for library code. A command writes its
// result to stdout.
#[expect(clippy::print_stdout, reason = "the command reports on stdout")]
#[expect(
    clippy::print_stderr,
    reason = "the command reports an error on stderr"
)]
fn main() -> ExitCode {
    let args = cli::Args::parse();
    let request = match args.request() {
        Ok(Some(request)) => request,
        Ok(None) => return report_version(),
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let cancel = Cancel::new();
    if let Err(error) = register_interrupt(&cancel) {
        eprintln!("Cannot install the interrupt handler: {error}. The export did not start.");
        return ExitCode::FAILURE;
    }
    let stdout = std::io::stdout();
    let mut progress = Progress::new(stdout.lock());
    match run(&request, &cancel, &mut progress) {
        Ok(report) => {
            println!(
                "Wrote {} audio file(s) of {} frame(s) into {}.",
                report.files,
                report.frames,
                report.destination.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Registers the interrupt handler that sets the cancellation flag.
fn register_interrupt(cancel: &Cancel) -> Result<(), std::io::Error> {
    signal_hook::flag::register(INTERRUPT_SIGNAL, cancel.shared()).map(|_| ())
}

/// Prints the version of the tool and the linked library.
#[expect(clippy::print_stdout, reason = "the command reports on stdout")]
#[expect(
    clippy::print_stderr,
    reason = "the command reports an error on stderr"
)]
fn report_version() -> ExitCode {
    let version = env!("CARGO_PKG_VERSION");
    let library = match library_info(LibraryInfo::LibraryVersion) {
        Ok(library) => library,
        Err(error) => {
            eprintln!("Cannot read the libopenmpt version: {error}. Check the linked library.");
            return ExitCode::FAILURE;
        }
    };
    let url = match library_info(LibraryInfo::SourceUrl) {
        Ok(url) => url,
        Err(error) => {
            eprintln!("Cannot read the libopenmpt source URL: {error}. Check the linked library.");
            return ExitCode::FAILURE;
        }
    };
    println!("mod2stems {version} (libopenmpt {library}, {url})");
    ExitCode::SUCCESS
}
