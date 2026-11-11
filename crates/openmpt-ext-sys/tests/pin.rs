//! Checks that the vendored checkout matches the pin constants and that the
//! license files are present.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// Prints a skip notice, so a run without the submodule is not silent.
// A test that skips must say so. The workspace bans `println!` in library code.
#[expect(clippy::print_stdout, reason = "a skipped test must name its reason")]
fn skip(reason: &str) {
    println!("skip: {reason}");
}

/// Runs `git -C <repo> <args>` and returns its trimmed standard output.
///
/// Returns `None` when the command exits non-zero.
fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Returns the path to the vendored libopenmpt checkout.
fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/openmpt")
}

/// Returns true when the vendored checkout is a git repository.
fn has_git(vendor: &Path) -> bool {
    vendor.join(".git").exists()
}

/// Compares one value and returns the mismatch as an error.
fn check(name: &str, actual: &str, expected: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name} is {actual:?}, expected {expected:?}"))
    }
}

#[test]
fn pin_commit_matches_submodule_head() -> Result<(), String> {
    let vendor = vendor();
    if !has_git(&vendor) {
        skip("vendor/openmpt is not a git checkout");
        return Ok(());
    }
    let head = git(&vendor, &["rev-parse", "HEAD"])
        .ok_or_else(|| "cannot run git rev-parse in vendor/openmpt".to_owned())?;
    check("vendor HEAD", &head, openmpt_ext_sys::PIN_COMMIT)
}

#[test]
fn pin_revision_matches_git_svn_id() -> Result<(), String> {
    let vendor = vendor();
    if !has_git(&vendor) {
        skip("vendor/openmpt is not a git checkout");
        return Ok(());
    }
    let message = git(&vendor, &["log", "-1", "--format=%B"])
        .ok_or_else(|| "cannot run git log in vendor/openmpt".to_owned())?;
    let marker = format!("@{}\u{20}", openmpt_ext_sys::PIN_SVN_REVISION);
    if message.contains(&marker) {
        Ok(())
    } else {
        Err(format!(
            "the vendor commit message does not name revision {}",
            openmpt_ext_sys::PIN_SVN_REVISION
        ))
    }
}

#[test]
fn pin_url_and_date_match_the_vendor_commit() -> Result<(), String> {
    let vendor = vendor();
    if !has_git(&vendor) {
        skip("vendor/openmpt is not a git checkout");
        return Ok(());
    }
    let message = git(&vendor, &["log", "-1", "--format=%B"])
        .ok_or_else(|| "cannot run git log in vendor/openmpt".to_owned())?;
    // The build passes this URL to the library as `MPT_SVNURL`, so a stale pin
    // makes the library report the wrong source.
    if !message.contains(openmpt_ext_sys::PIN_SVN_URL) {
        return Err(format!(
            "the vendor commit message does not name the URL {}",
            openmpt_ext_sys::PIN_SVN_URL
        ));
    }
    let date = git(&vendor, &["log", "-1", "--format=%cI"])
        .ok_or_else(|| "cannot run git log in vendor/openmpt".to_owned())?;
    check("vendor commit date", &date, openmpt_ext_sys::PIN_SVN_DATE)
}

#[test]
fn pin_tag_names_the_vendor_commit() -> Result<(), String> {
    let vendor = vendor();
    if !has_git(&vendor) {
        skip("vendor/openmpt is not a git checkout");
        return Ok(());
    }
    let tags = git(&vendor, &["tag", "--points-at", "HEAD"])
        .ok_or_else(|| "cannot run git tag in vendor/openmpt".to_owned())?;
    // A shallow submodule clone can hold no tags. The check is only possible
    // when the clone holds them.
    if tags.is_empty() {
        skip("vendor/openmpt holds no tags");
        return Ok(());
    }
    if tags.lines().any(|tag| tag == openmpt_ext_sys::PIN_TAG) {
        Ok(())
    } else {
        Err(format!(
            "the vendor commit is tagged {tags:?}, not {:?}",
            openmpt_ext_sys::PIN_TAG
        ))
    }
}

#[test]
fn license_files_exist() -> Result<(), String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSES");
    for name in [
        "libopenmpt.txt",
        "miniz.txt",
        "minimp3.txt",
        "stb_vorbis.txt",
    ] {
        let path = dir.join(name);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if text.trim().is_empty() {
            return Err(format!("{} is empty", path.display()));
        }
    }
    Ok(())
}
