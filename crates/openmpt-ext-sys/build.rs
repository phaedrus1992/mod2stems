//! Builds the vendored libopenmpt as a static library.
//!
//! The vendored tree is unmodified upstream code. The source list, the include
//! paths, and the defines come from the upstream `Makefile`.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Git tag of the vendored libopenmpt tree.
const PIN_TAG: &str = "libopenmpt-0.8.10";
/// Git commit of the vendored libopenmpt tree.
const PIN_COMMIT: &str = "dc4ddcf682763d55c33dcf6c874a3153dea7b304";
/// SVN revision in the vendor commit message.
const PIN_SVN_REVISION: &str = "25840";
/// SVN branch that the vendor commit comes from.
const PIN_SVN_URL: &str = "https://source.openmpt.org/svn/openmpt/branches/OpenMPT-1.32";
/// Commit date of the vendor tree, in ISO 8601.
const PIN_SVN_DATE: &str = "2026-10-31T18:27:30Z";

/// Directories that hold C++ sources, relative to the vendor root.
///
/// The upstream `Makefile` lists these directories. Some hold only headers at
/// this tag, and the glob returns nothing for them.
const CXX_DIRS: [&str; 13] = [
    "src/openmpt/all",
    "src/openmpt/base",
    "src/openmpt/logging",
    "src/openmpt/random",
    "src/openmpt/fileformat_base",
    "src/openmpt/soundbase",
    "src/openmpt/soundfile_data",
    "common",
    "soundlib",
    "soundlib/plugins",
    "soundlib/plugins/dmo",
    "sounddsp",
    "libopenmpt",
];

/// C sources of the bundled decoders, relative to the vendor root.
const C_SOURCES: [&str; 3] = [
    "include/miniz/miniz.c",
    "include/minimp3/minimp3.c",
    "include/stb_vorbis/stb_vorbis.c",
];

/// Include paths, relative to the vendor root.
const INCLUDE_DIRS: [&str; 4] = [".", "src", "common", "include"];

/// Builds the static library.
fn main() -> Result<(), Box<dyn Error>> {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let vendor = manifest.join("vendor/openmpt");

    if !vendor.join("libopenmpt/libopenmpt.h").is_file() {
        return Err("crates/openmpt-ext-sys/vendor/openmpt is empty. \
             Run: git submodule update --init --depth 1"
            .into());
    }

    emit_pin(&manifest.join("wrapper.h"));

    let mut cpp = cc::Build::new();
    configure(&mut cpp, &vendor);
    cpp.cpp(true).std("c++17");
    cpp.files(cpp_sources(&vendor)?);

    let mut c = cc::Build::new();
    configure(&mut c, &vendor);
    for source in C_SOURCES {
        c.file(vendor.join(source));
    }

    cpp.compile("openmpt");
    c.compile("openmpt_decoders");
    Ok(())
}

/// Applies the include paths and the defines that both builds share.
fn configure(build: &mut cc::Build, vendor: &Path) {
    for dir in INCLUDE_DIRS {
        build.include(vendor.join(dir));
    }
    for (name, value) in defines() {
        build.define(name, value.as_deref());
    }
    // The tree is unmodified upstream code. Its warnings are not ours to fix,
    // and a warning that the local compiler grew must not fail the build.
    build.warnings(false);
}

/// Returns the defines that the upstream `Makefile` passes to the static build.
///
/// The quoted values become C string literals.
fn defines() -> [(&'static str, Option<String>); 9] {
    [
        ("LIBOPENMPT_BUILD", None),
        ("MPT_WITH_MINIZ", None),
        ("MPT_WITH_MINIMP3", None),
        ("MPT_WITH_STBVORBIS", None),
        ("STB_VORBIS_NO_PULLDATA_API", None),
        ("STB_VORBIS_NO_STDIO", None),
        ("MPT_SVNURL", Some(quoted(PIN_SVN_URL))),
        ("MPT_SVNVERSION", Some(quoted(PIN_SVN_REVISION))),
        ("MPT_SVNDATE", Some(quoted(PIN_SVN_DATE))),
    ]
}

/// Wraps a value in double quotes, so the compiler sees a string literal.
fn quoted(value: &str) -> String {
    format!("\"{value}\"")
}

/// Returns every `.cpp` file in `CXX_DIRS`, sorted by path.
fn cpp_sources(vendor: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for dir in CXX_DIRS {
        let mut found = Vec::new();
        for entry in fs::read_dir(vendor.join(dir))? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "cpp") {
                found.push(path);
            }
        }
        found.sort();
        files.extend(found);
    }
    Ok(files)
}

/// Exports the pin constants to the crate and sets the rebuild triggers.
fn emit_pin(wrapper: &Path) {
    println!("cargo:rustc-env=OPENMPT_EXT_SYS_PIN_TAG={PIN_TAG}");
    println!("cargo:rustc-env=OPENMPT_EXT_SYS_PIN_COMMIT={PIN_COMMIT}");
    println!("cargo:rustc-env=OPENMPT_EXT_SYS_PIN_SVN_REVISION={PIN_SVN_REVISION}");
    println!("cargo:rustc-env=OPENMPT_EXT_SYS_PIN_SVN_URL={PIN_SVN_URL}");
    println!("cargo:rustc-env=OPENMPT_EXT_SYS_PIN_SVN_DATE={PIN_SVN_DATE}");
    // The vendored sources only change with a pin bump, and `scripts/bump-libopenmpt.sh`
    // rewrites this file in the same step. Cargo always reruns a build script when
    // the script itself changes, so only the wrapper needs its own watch.
    println!("cargo:rerun-if-changed={}", wrapper.display());
}
