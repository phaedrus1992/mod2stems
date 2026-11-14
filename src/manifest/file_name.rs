//! The file names of one export.
//!
//! The channel index is authoritative and the name is only text, so two
//! channels that share a name still get two files. Every name from a module
//! goes through [`sanitize`], because a module is untrusted input. A name that
//! gives nothing usable falls back to the index alone, so one bad name never
//! fails an export. Design: `docs/design.md` §5.2.

/// The longest name that one component of a file name holds.
///
/// A component holds 255 bytes on ext4, and the index, the marker, and the
/// extension take part of that. This cap keeps one name far below the smallest
/// platform limit, so a long channel name cannot fail the export.
const MAX_NAME_BYTES: usize = 64;

/// Returns the file name of one channel stem.
///
/// The name holds the index, so two channels never share a file. The sanitized
/// name follows the index, because it is text for a person and not identity. A
/// name that gives nothing usable is dropped, and the index alone names the
/// file.
#[must_use]
pub fn file_name(channel: i32, name: Option<&str>, extension: &str) -> String {
    let index = index_text(channel);
    match name.and_then(sanitize) {
        Some(clean) => format!("ch_{index}_{clean}.{extension}"),
        None => format!("ch_{index}.{extension}"),
    }
}

/// Returns the file name of one instrument pair stem.
#[must_use]
pub fn pair_file_name(
    channel: i32,
    instrument: i32,
    name: Option<&str>,
    extension: &str,
) -> String {
    pair_name("ins", channel, instrument, name, extension)
}

/// Returns the file name of one sample pair stem.
///
/// A sample-only module has no instrument, so the marker says that the number
/// names a sample slot instead (`docs/design.md` §5.2).
#[must_use]
pub fn sample_file_name(channel: i32, sample: i32, name: Option<&str>, extension: &str) -> String {
    pair_name("smp", channel, sample, name, extension)
}

/// Returns the name of one pair stem under the named marker.
fn pair_name(marker: &str, channel: i32, slot: i32, name: Option<&str>, extension: &str) -> String {
    let channel = index_text(channel);
    let slot = format!("{:03}", slot.max(0));
    match name.and_then(sanitize) {
        Some(clean) => format!("ch_{channel}_{marker}_{slot}_{clean}.{extension}"),
        None => format!("ch_{channel}_{marker}_{slot}.{extension}"),
    }
}

/// Returns the two-digit index text of one channel.
fn index_text(channel: i32) -> String {
    format!("{:02}", channel.max(0))
}

/// Returns the text part of a file name that a module supplies, or `None`.
///
/// The result is never a whole file name. The caller prefixes the index, so
/// this text can never name a directory or a traversal on its own.
///
/// Letters, digits, and `-` stay. Every other character becomes an underscore,
/// and a run of underscores collapses to one. The name is lowercased, so a
/// case-insensitive filesystem cannot collide two names that differ only by
/// case. Leading and trailing underscores drop.
///
/// A name that gives no letter or digit returns `None`, because such a name
/// would give an empty or punctuation-only part that carries no information.
#[must_use]
pub fn sanitize(name: &str) -> Option<String> {
    let mut out = String::with_capacity(name.len().min(MAX_NAME_BYTES));
    for character in name.chars() {
        if out.len() >= MAX_NAME_BYTES {
            break;
        }
        let next = match character {
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            '-' => '-',
            _ => '_',
        };
        // A run of underscores names nothing, so it collapses to one.
        if next == '_' && out.ends_with('_') {
            continue;
        }
        out.push(next);
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::file_name;
    use super::pair_file_name;
    use super::sanitize;

    #[test]
    fn a_name_is_lowercased() {
        // A case-insensitive filesystem would collide two names that differ
        // only by case, and the index alone must decide the path.
        assert_eq!(sanitize("Lead").as_deref(), Some("lead"));
    }

    #[test]
    fn a_space_run_collapses_to_one_underscore() {
        assert_eq!(sanitize("a   b").as_deref(), Some("a_b"));
    }

    #[test]
    fn a_dot_never_survives() {
        // A dot can form a traversal or a hidden file, and a name gains
        // nothing from one.
        assert_eq!(sanitize("a...b").as_deref(), Some("a_b"));
        assert_eq!(sanitize("../../etc/passwd").as_deref(), Some("etc_passwd"));
    }

    #[test]
    fn a_name_at_the_cap_keeps_its_head() -> Result<(), String> {
        let long = "a".repeat(100);
        let name = sanitize(&long).ok_or("a run of letters gives a name")?;
        if name.len() != 64 {
            return Err(format!("the name is {} characters", name.len()));
        }
        Ok(())
    }

    #[test]
    fn a_name_with_no_usable_character_gives_none() {
        for name in ["///", "..", ".", "   "] {
            assert_eq!(sanitize(name), None, "the name {name:?} must give None");
        }
    }

    #[test]
    fn an_unusable_name_falls_back_to_the_index() {
        // One bad channel name must not fail a whole export.
        assert_eq!(file_name(3, Some(".."), "wav"), "ch_03.wav");
    }

    #[test]
    fn a_pair_name_holds_both_indices() {
        assert_eq!(
            pair_file_name(3, 7, Some("Kick"), "flac"),
            "ch_03_ins_007_kick.flac"
        );
    }
}
