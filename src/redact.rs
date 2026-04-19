//! Shared redaction helpers — strip PII from free-form strings while
//! preserving surrounding structure so the shape of a finding remains
//! verifiable. Used by every CLI that surfaces user-authored paths or
//! Autodesk-internal identifiers.
//!
//! Contract: never silently remove data. Always replace the sensitive
//! region with an explicit `<redacted...>` marker that indicates what
//! class of info was scrubbed. Tests below are the canonical examples.

/// Scrub `\Users\<name>\` and `/Users/<name>/` usernames while
/// preserving the path shape. The username segment is replaced by
/// `<redacted>`; nothing else is touched.
///
/// ```
/// use rvt::redact::redact_path_str;
///
/// let input  = "C:\\Users\\alice\\Documents\\model.rfa";
/// let output = redact_path_str(input);
/// assert_eq!(output, "C:\\Users\\<redacted>\\Documents\\model.rfa");
///
/// // Paths without a \Users\ or /Users/ segment pass through unchanged.
/// assert_eq!(
///     redact_path_str("C:\\ProgramData\\Autodesk\\RVT 2024"),
///     "C:\\ProgramData\\Autodesk\\RVT 2024"
/// );
/// ```
pub fn redact_path_str(p: &str) -> String {
    let mut out = p.to_string();
    for n in ["\\Users\\", "/Users/"] {
        if let Some(idx) = out.find(n) {
            let tail_start = idx + n.len();
            let tail = &out[tail_start..];
            if let Some(end) = tail.find(|c: char| c == '\\' || c == '/') {
                let before = &out[..tail_start];
                let after = &tail[end..];
                out = format!("{before}<redacted>{after}");
            }
        }
    }
    out
}

/// Scrub a free-form string for every known PII pattern:
///
/// - Windows usernames in `\Users\...` paths → `<redacted>`
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
        let seg_end = tail
            .find(|c: char| c == '\\' || c == '/')
            .unwrap_or(tail.len());
        out.push_str("Revit - <redacted project id>");
        rest = &tail[seg_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_windows_username() {
        let input = "C:\\Users\\alice\\Documents\\file.rfa";
        assert_eq!(
            redact_path_str(input),
            "C:\\Users\\<redacted>\\Documents\\file.rfa"
        );
    }

    #[test]
    fn redacts_unix_username() {
        let input = "/Users/bob/Documents/file.rfa";
        assert_eq!(
            redact_path_str(input),
            "/Users/<redacted>/Documents/file.rfa"
        );
    }

    #[test]
    fn preserves_non_user_paths() {
        let input = "C:\\ProgramData\\Autodesk\\RVT 2024\\libraries";
        assert_eq!(redact_path_str(input), input);
    }

    // Test fixtures use synthetic username + project-id values. The
    // real-world patterns these match (seen in Autodesk-shipped reference
    // content) are deliberately NOT reproduced here.
    #[test]
    fn redacts_onedrive_autodesk_path() {
        let input = "C:\\Users\\testuser\\OneDrive - Autodesk\\FY-20XX Projects\\Revit - 111111 Update\\20XX\\UniformatClassifications.txt";
        let out = redact_sensitive(input);
        assert!(out.contains("<redacted>"));
        assert!(out.contains("<redacted autodesk internal path>"));
        assert!(!out.contains("testuser"));
        assert!(!out.contains("OneDrive - Autodesk"));
    }

    #[test]
    fn redacts_build_server_path() {
        let input = "F:\\Ship\\2026_px64\\Source\\API\\RevitAPI";
        assert!(redact_sensitive(input).contains("<redacted build-server path>"));
    }

    #[test]
    fn redacts_project_id_folder() {
        let input = "C:\\sample\\Revit - 111111 Update\\file.rfa";
        let out = redact_sensitive(input);
        assert!(out.contains("<redacted project id>"));
        assert!(!out.contains("111111"));
    }
}
