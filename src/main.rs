//! The mod2stems command.

#![forbid(unsafe_code)]

use openmpt_ext::LibraryInfo;
use openmpt_ext::library_info;

// The workspace denies stdout output for library code. A command writes its
// result to stdout.
#[expect(clippy::print_stdout, reason = "the command reports on stdout")]
fn main() {
    let version = env!("CARGO_PKG_VERSION");
    // The library version string carries the revision, as in `0.8.10+r25840`.
    let library = library_info(LibraryInfo::LibraryVersion);
    let url = library_info(LibraryInfo::SourceUrl);
    println!("mod2stems {version} (libopenmpt {library}, {url})");
}
