//! Shared redaction helpers — strip PII from free-form strings while
//! preserving surrounding structure so the shape of a finding remains
//! verifiable. Used by every CLI that surfaces user-authored paths or
//! Autodesk-internal identifiers.
//!
//! Contract: never silently remove data. Always replace the sensitive
//! region with an explicit `<redacted...>` marker that indicates what
//! class of info was scrubbed.

/// The folders that hold one folder per user, each written with the separators
/// of the platform that has it. The segment after one is a user name.
const HOME_PARENTS: [&str; 3] = ["\\Users\\", "/Users/", "/home/"];

/// Scrub `\Users\<name>\`, `/Users/<name>/` and `/home/<name>/` usernames
/// while preserving the path shape, in every path the string holds. The
/// username segment is replaced by `<redacted>`; nothing else is touched. A
/// name no separator follows, because the string ends at it, is left as it is.
///
/// ```
/// use rvt::redact::redact_path_str;
///
/// let input  = "C:\\Users\\alice\\Documents\\model.rfa";
/// let output = redact_path_str(input);
/// assert_eq!(output, "C:\\Users\\<redacted>\\Documents\\model.rfa");
///
/// // A Linux home folder is scrubbed the same way, and so is each path of a string.
/// assert_eq!(
///     redact_path_str("/home/alice/a.rvt and /home/bob/b.rvt"),
///     "/home/<redacted>/a.rvt and /home/<redacted>/b.rvt"
/// );
///
/// // Paths without a \Users\, /Users/ or /home/ segment pass through unchanged.
/// assert_eq!(
///     redact_path_str("C:\\ProgramData\\Autodesk\\RVT 2024"),
///     "C:\\ProgramData\\Autodesk\\RVT 2024"
/// );
/// ```
pub fn redact_path_str(p: &str) -> String {
    HOME_PARENTS
        .iter()
        .fold(p.to_string(), |text, parent| redact_after(&text, parent))
}

/// `text` with the segment after each `parent` that a path separator ends
/// replaced by `<redacted>`. The scan moves past every replacement, so a
/// string that is already redacted comes back as it was.
fn redact_after(text: &str, parent: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(parent) {
        let name_at = at + parent.len();
        out.push_str(&rest[..name_at]);
        rest = &rest[name_at..];
        let Some(end) = rest.find(['\\', '/']) else {
            break;
        };
        out.push_str("<redacted>");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Scrub a free-form string for every known PII pattern:
///
/// - Home-folder usernames in `\Users\`, `/Users/` and `/home/` paths →
///   `<redacted>`
/// - Autodesk OneDrive authoring paths → `<redacted autodesk internal path>`
/// - Build-server paths (`F:\Ship`, `D:\build`) → `<redacted build-server path>`
/// - Autodesk internal project-ID folders (`Revit - <digits>`) →
///   `<redacted project id>`
pub fn redact_sensitive(s: &str) -> String {
    let mut out = redact_path_str(s);
    if let Some(idx) = out.find("OneDrive - Autodesk") {
        let tail = &out[idx..];
        if let Some(end) = tail
            .find(".txt")
            .or_else(|| tail.find(".rfa"))
            .or_else(|| tail.find(".rvt"))
        {
            let before = &out[..idx];
            let ext_end = idx + end + 4;
            let after = &out[ext_end..];
            out = format!("{before}<redacted autodesk internal path>{after}");
        } else {
            let before = &out[..idx];
            out = format!("{before}<redacted autodesk internal path>");
        }
    }
    for prefix in &["F:\\Ship", "F:/Ship", "D:\\build", "D:/build"] {
        if let Some(idx) = out.find(prefix) {
            let before = &out[..idx];
            out = format!("{before}<redacted build-server path>");
        }
    }
    out = redact_autodesk_project_ids(&out);
    out
}

fn redact_autodesk_project_ids(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find("Revit - ") {
        out.push_str(&rest[..idx]);
        let after_prefix = &rest[idx + "Revit - ".len()..];
        let digit_len: usize = after_prefix
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if digit_len < 3 {
            out.push_str("Revit - ");
            rest = after_prefix;
            continue;
        }
        let tail = after_prefix;
        let seg_end = tail.find(['\\', '/']).unwrap_or(tail.len());
        out.push_str("Revit - <redacted project id>");
        rest = &tail[seg_end..];
    }
    out.push_str(rest);
    out
}

/// What an anonymized report puts in place of a name it takes from the
/// project: a file, the project, a family or a type.
pub const ANONYMIZED: &str = "<redacted>";

/// `path` as an anonymized file name: its folders and stem replaced by
/// [`ANONYMIZED`], its extension kept, so the kind of file still shows.
///
/// ```
/// use rvt::redact::anonymize_file_name;
///
/// assert_eq!(anonymize_file_name("C:\\Projects\\Client\\Tower.rvt"), "<redacted>.rvt");
/// assert_eq!(anonymize_file_name("model"), "<redacted>");
/// ```
pub fn anonymize_file_name(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => {
            format!("{ANONYMIZED}.{extension}")
        }
        _ => ANONYMIZED.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::redact_path_str;

    // The ways `redact_path_str` can leave a user name behind, or take more
    // than a name, written before the loop that scrubs every path.

    #[test]
    fn a_linux_home_folder_loses_its_user_name() {
        assert_eq!(
            redact_path_str("/home/alice/models/a.rvt"),
            "/home/<redacted>/models/a.rvt"
        );
    }

    #[test]
    fn every_path_of_a_string_is_scrubbed_not_the_first_of_each_kind() {
        let text = "C:\\Users\\alice\\a.rvt, /Users/bob/b.rvt, /home/carol/c.rvt, \
                    C:\\Users\\dave\\d.rvt and /home/erin/e.rvt";
        let out = redact_path_str(text);
        for name in ["alice", "bob", "carol", "dave", "erin"] {
            assert!(!out.contains(name), "{name} survives in {out}");
        }
        assert_eq!(out.matches("<redacted>").count(), 5);
    }

    #[test]
    fn a_name_the_string_ends_at_is_left_as_it_is() {
        for text in [
            "/home/",
            "/home/alice",
            "/Users/",
            "/Users/alice",
            "C:\\Users\\alice",
        ] {
            assert_eq!(redact_path_str(text), text);
        }
    }

    #[test]
    fn a_folder_that_only_starts_like_a_home_folder_is_left_alone() {
        for text in [
            "/srv/homework/alice/x",
            "/opt/rvt/data.rvt",
            "C:\\ProgramData\\Autodesk\\RVT 2024",
        ] {
            assert_eq!(redact_path_str(text), text);
        }
    }

    #[test]
    fn a_name_with_spaces_dots_or_accents_is_scrubbed_whole() {
        assert_eq!(
            redact_path_str("C:\\Users\\Alice Smith\\m.rvt"),
            "C:\\Users\\<redacted>\\m.rvt"
        );
        assert_eq!(
            redact_path_str("/home/j.doe-ñ/m.rvt"),
            "/home/<redacted>/m.rvt"
        );
    }

    #[test]
    fn scrubbing_twice_gives_what_scrubbing_once_gives() {
        let once = redact_path_str("/home/alice/a.rvt and C:\\Users\\bob\\b.rvt");
        assert_eq!(redact_path_str(&once), once);
    }
}
