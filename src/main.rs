//! The mod2stems command.

#![forbid(unsafe_code)]

use openmpt_ext::LibraryInfo;
use openmpt_ext::library_info;

// The workspace denies stdout output for library code. A command writes its
// result to stdout.
#[expect(clippy::print_stdout, reason = "the command reports on stdout")]
#[expect(
    clippy::print_stderr,
    reason = "the command reports an error on stderr"
)]
fn main() -> std::process::ExitCode {
    let version = env!("CARGO_PKG_VERSION");
    // The library version string carries the revision, as in `0.8.10+r25840`.
    let Ok(library) = library_info(LibraryInfo::LibraryVersion) else {
        eprintln!("cannot read the libopenmpt version");
        return std::process::ExitCode::FAILURE;
    };
    let Ok(url) = library_info(LibraryInfo::SourceUrl) else {
        eprintln!("cannot read the libopenmpt source URL");
        return std::process::ExitCode::FAILURE;
    };
    println!("mod2stems {version} (libopenmpt {library}, {url})");
    std::process::ExitCode::SUCCESS
}
