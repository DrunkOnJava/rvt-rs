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
/// username segment is replaced by `<redacted>`; nothing else is touched. The
/// name ends at the next path separator, so a profile folder named
/// `Alice Smith` is one name. Where no separator follows, as in a path an error
/// echoes back, it ends at the first space, quote, closing bracket, `,`, `;` or
/// `:`, or at the end of the string, so a name with a space is then scrubbed
/// only up to the space. A home folder with nothing after it is left as it is.
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
/// // A path an error echoes back keeps no name either, though no separator follows it.
/// assert_eq!(
///     redact_path_str("No such file: /Users/alice"),
///     "No such file: /Users/<redacted>"
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

/// The length of the user name at the start of `rest`, the text after a home
/// folder; 0 where there is none. It ends at the next path separator. Where
/// none follows, it ends at the first character that ends a name in running
/// text, or at the end of the string.
fn user_name_len(rest: &str) -> usize {
    rest.find(['\\', '/']).unwrap_or_else(|| {
        rest.find(|c: char| {
            c.is_whitespace() || matches!(c, '"' | '\'' | '`' | ')' | ']' | '}' | ',' | ';' | ':')
        })
        .unwrap_or(rest.len())
    })
}

/// `text` with the user name after each `parent` replaced by `<redacted>`. The
/// scan moves past every replacement, so a string that is already redacted
/// comes back as it was.
fn redact_after(text: &str, parent: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(parent) {
        let name_at = at + parent.len();
        out.push_str(&rest[..name_at]);
        rest = &rest[name_at..];
        let len = user_name_len(rest);
        if len > 0 {
            out.push_str("<redacted>");
            rest = &rest[len..];
        }
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

/// Scrub every string value inside `value` with [`redact_sensitive`], in
/// place, however deep an array or an object holds it. The keys of an object,
/// numbers, booleans and nulls are left as they are.
///
/// ```
/// use rvt::redact::redact_json_strings;
///
/// let mut value = serde_json::json!({ "value": "/home/alice/a.rvt", "size": 4 });
/// redact_json_strings(&mut value);
/// assert_eq!(
///     value,
///     serde_json::json!({ "value": "/home/<redacted>/a.rvt", "size": 4 })
/// );
/// ```
pub fn redact_json_strings(value: &mut serde_json::Value) {
    use serde_json::Value;
    match value {
        Value::String(text) => *text = redact_sensitive(text.as_str()),
        Value::Array(items) => items.iter_mut().for_each(redact_json_strings),
        Value::Object(fields) => fields.values_mut().for_each(redact_json_strings),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
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
    use super::{redact_json_strings, redact_path_str};

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
    fn a_name_no_separator_follows_is_scrubbed_where_an_error_echoes_a_path() {
        for (text, want) in [
            ("/home/alice", "/home/<redacted>"),
            ("/Users/alice", "/Users/<redacted>"),
            ("C:\\Users\\alice", "C:\\Users\\<redacted>"),
            (
                "No such file: /Users/alice",
                "No such file: /Users/<redacted>",
            ),
            ("cwd /home/alice, then", "cwd /home/<redacted>, then"),
            (
                "opened '/home/alice' twice",
                "opened '/home/<redacted>' twice",
            ),
            ("(see /home/alice)", "(see /home/<redacted>)"),
            (
                "/home/alice: no such file",
                "/home/<redacted>: no such file",
            ),
            ("/home/alice; next", "/home/<redacted>; next"),
            // The cost of cutting at a space: the rest of a spaced name stays.
            ("C:\\Users\\Alice Smith", "C:\\Users\\<redacted> Smith"),
        ] {
            assert_eq!(redact_path_str(text), want);
        }
    }

    #[test]
    fn a_home_folder_with_no_name_after_it_is_left_alone() {
        for text in [
            "/home/",
            "/Users/",
            "C:\\Users\\",
            "/home/ and more",
            "see (/Users/) here",
            "/home//x",
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

    // The ways `redact_json_strings` can leave text behind, or change what is
    // not text, written before the walk.

    #[test]
    fn a_string_is_scrubbed_however_deep_an_array_or_object_holds_it() {
        let mut value = serde_json::json!({
            "value": "/home/alice/a.rvt",
            "items": [
                "C:\\Users\\bob\\b.rvt",
                { "inner": ["/Users/carol/c.rvt", ["C:\\Users\\dave\\OneDrive - Autodesk\\m.rfa"]] },
            ],
        });
        redact_json_strings(&mut value);
        let text = value.to_string();
        for leaked in ["alice", "bob", "carol", "dave", "OneDrive"] {
            assert!(!text.contains(leaked), "{leaked} survives in {text}");
        }
        assert!(text.contains("<redacted autodesk internal path>"));
    }

    #[test]
    fn what_is_not_text_with_a_path_in_it_is_left_as_it_was() {
        let before = serde_json::json!({
            "id": 12345,
            "flag": true,
            "none": null,
            "ratio": 0.5,
            "digits": "12345",
            "name": "Basic Wall:Generic - 200mm",
            "empty": "",
            "list": [],
        });
        let mut after = before.clone();
        redact_json_strings(&mut after);
        assert_eq!(after, before);
    }

    #[test]
    fn scrubbing_a_value_twice_gives_what_scrubbing_once_gives() {
        let mut once = serde_json::json!([
            "/home/alice/a.rvt",
            { "path": "C:\\Users\\bob\\OneDrive - Autodesk\\m.rfa" },
        ]);
        redact_json_strings(&mut once);
        let mut twice = once.clone();
        redact_json_strings(&mut twice);
        assert_eq!(twice, once);
    }
}
