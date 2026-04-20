//! Parse the `BasicFileInfo` OLE stream — UTF-16LE metadata every Revit file carries.
//!
//! Content is a loose structure of UTF-16LE strings interleaved with binary
//! markers. The fields we reliably extract across 2016-2026:
//!
//! - Version (4-digit year) — preceded by `\x04\x00` in the raw bytes
//! - Build string — either `(Build: YYYYMMDD_HHMM(x64))` or `YYYYMMDD_HHMM(x64)`
//! - Original local file path (embedded by the creator's filesystem)
//! - File GUID — standard UUID notation, sometimes repeated
//! - Locale — e.g. `ENU` appears as `E·N·U` in UTF-16LE
//!
//! The regex-driven approach matches Apache Tika and chuongmep/revit-extractor
//! (Python) — this is the "easy layer" that's been public since 2008.

use crate::{Error, Result};
use encoding_rs::UTF_16LE;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicFileInfo {
    /// 4-digit Revit release year (e.g. 2024).
    pub version: u32,
    /// Build tag — either `YYYYMMDD_HHMM(x64)` or a free-form string like
    /// `"Development Build"`.
    pub build: Option<String>,
    /// Original file path recorded at save time on the creator's system.
    pub original_path: Option<String>,
    /// File GUID (UUIDv4) if recoverable.
    pub guid: Option<String>,
    /// Locale code if present (e.g. `ENU`, `FRA`).
    pub locale: Option<String>,
    /// Raw UTF-16LE decoded text for debugging.
    pub raw_text: String,
}

impl BasicFileInfo {
    /// Parse the raw `BasicFileInfo` stream bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let (cow, _, had_errors) = UTF_16LE.decode(data);
        if had_errors {
            // Not fatal — some single-byte markers aren't valid UTF-16 pairs.
            // We still extract from whatever decoded cleanly.
        }
        let raw = cow.into_owned();

        let version = extract_version(&raw)
            .ok_or_else(|| Error::BasicFileInfo("no 4-digit Revit version found".into()))?;
        let build = extract_build(&raw);
        let original_path = extract_path(&raw);
        let guid = extract_guid(&raw);
        let locale = extract_locale(&raw);

        Ok(Self {
            version,
            build,
            original_path,
            guid,
            locale,
            raw_text: raw,
        })
    }
}

fn extract_version(text: &str) -> Option<u32> {
    // Two patterns seen:
    //   "Autodesk Revit 2018 (Build: 20170130_1515(x64))"   <- 2016-2018
    //   "2019  20180123_1515(x64)"                          <- 2019+
    // Strategy: scan for the first 4-digit number in 2014..2030 range.
    let mut chars = text.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if !c.is_ascii_digit() {
            continue;
        }
        // tentative start
        let start = chars.peek().map(|(i, _)| *i).unwrap_or(text.len()) - c.len_utf8();
        let slice: String = text[start..].chars().take(4).collect();
        if slice.len() == 4 && slice.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = slice.parse::<u32>() {
                if (2014..=2030).contains(&n) {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn extract_build(text: &str) -> Option<String> {
    // Pattern 1: "(Build: 20170130_1515(x64))" — most common wrapper
    // on files saved by Revit 2016–2018.
    if let Some(p) = text.find("Build: ") {
        let tail = &text[p + 7..];
        if let Some(end) = tail.find(')') {
            return Some(tail[..end + 1].to_string());
        }
    }
    // Pattern 2: plain "YYYYMMDD_HHMM(x64)" format — Revit 2019+ and
    // later releases embed the build tag without the "Build:" wrapper.
    // The HHMM component varies (1515, 1635, 1200, …); we scan for
    // the full YYYYMMDD_HHMM(x64) shape directly rather than literal
    // substring matches so all build tags survive this path.
    //
    // Wire format: 8 digits, '_', 4 digits, literal "(x64)".
    // Total length: 18 chars.
    let suffix = "(x64)";
    let bytes = text.as_bytes();
    let n = bytes.len();
    let tag_len = 8 + 1 + 4 + suffix.len();
    if n >= tag_len {
        for i in 0..=n - tag_len {
            let window = &bytes[i..i + tag_len];
            let ymd = &window[0..8];
            let us = window[8];
            let hm = &window[9..13];
            let sfx = &window[13..];
            let ok = ymd.iter().all(u8::is_ascii_digit)
                && us == b'_'
                && hm.iter().all(u8::is_ascii_digit)
                && sfx == suffix.as_bytes();
            if ok {
                // Safe because window is all ASCII.
                return Some(std::str::from_utf8(window).unwrap().to_string());
            }
        }
    }
    // Pattern 3: "Development Build" — Revit dev releases ship
    // without a build tag; callers want SOME string.
    if text.contains("Development Build") {
        return Some("Development Build".to_string());
    }
    None
}

fn extract_path(text: &str) -> Option<String> {
    // Windows paths like C:\Users\...\...rfa
    // UTF-16LE-decoded text has these as plain chars.
    let needle_start = text.find(['C', 'D']);
    let start = needle_start?;
    let tail = &text[start..];
    // Must be "C:\" or "D:\" pattern
    if !tail.starts_with("C:\\") && !tail.starts_with("D:\\") {
        // fallback: first occurrence of ":\\" anywhere
        let colon_backslash = text.find(":\\")?;
        let s = colon_backslash.saturating_sub(1);
        let tail = &text[s..];
        return take_until_rfa(tail);
    }
    take_until_rfa(tail)
}

fn take_until_rfa(tail: &str) -> Option<String> {
    for ext in &[".rvt", ".rfa", ".rte", ".rft"] {
        if let Some(end) = tail.find(ext) {
            return Some(tail[..end + ext.len()].to_string());
        }
    }
    None
}

fn extract_guid(text: &str) -> Option<String> {
    // UUIDv4: 8-4-4-4-12 hex chars
    let bytes = text.as_bytes();
    for i in 0..bytes.len().saturating_sub(36) {
        let slice = &text.get(i..i + 36)?;
        if is_guid(slice) {
            return Some(slice.to_string());
        }
    }
    None
}

fn is_guid(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    let dash_positions = [8, 13, 18, 23];
    for (i, b) in bytes.iter().enumerate() {
        if dash_positions.contains(&i) {
            if *b != b'-' {
                return false;
            }
        } else if !b.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn extract_locale(text: &str) -> Option<String> {
    // Locale codes appear as 3-letter ASCII blocks like ENU, FRA, DEU, ESP, RUS, JPN, CHS
    for code in &[
        "ENU", "FRA", "DEU", "ESP", "ITA", "RUS", "JPN", "CHS", "CHT", "KOR", "PLK", "PTB", "CSY",
    ] {
        if text.contains(*code) {
            return Some(code.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_version_2024_pattern() {
        let text =
            "2024  20230308_1635(x64)Z C:\\Users\\testuser\\Desktop\\racbasicsamplefamily.rfa";
        let v = extract_version(text).unwrap();
        assert_eq!(v, 2024);
    }

    #[test]
    fn extracts_version_2018_pattern() {
        let text = "Autodesk Revit 2018 (Build: 20170130_1515(x64))";
        assert_eq!(extract_version(text), Some(2018));
    }

    #[test]
    fn extract_build_matches_wrapped_form() {
        // Revit 2016-2018 uses "(Build: ...)" wrapper.
        let text = "Autodesk Revit 2018 (Build: 20170130_1515(x64))";
        assert_eq!(extract_build(text).as_deref(), Some("20170130_1515(x64)"));
    }

    #[test]
    fn extract_build_matches_plain_1515_form() {
        // Revit 2019+ uses plain "YYYYMMDD_HHMM(x64)" after year.
        let text = "2019  20180123_1515(x64)Z C:\\Users\\testuser\\Desktop\\x.rfa";
        assert_eq!(extract_build(text).as_deref(), Some("20180123_1515(x64)"));
    }

    #[test]
    fn extract_build_matches_non_1515_time_component() {
        // Regression for the 2024 sample which has "_1635(x64)", not
        // "_1515(x64)". Previous implementation only matched literal
        // "_1515(x64)" and silently dropped every non-1515 build tag.
        let text = "2024  20230308_1635(x64)Z C:\\Users\\testuser\\Desktop\\x.rfa";
        assert_eq!(extract_build(text).as_deref(), Some("20230308_1635(x64)"));
    }

    #[test]
    fn extract_build_returns_none_on_missing_tag() {
        let text = "2024 some other content with no build tag at all";
        assert!(extract_build(text).is_none());
    }

    #[test]
    fn extract_build_development_build() {
        let text = "Autodesk Revit 2024 (Development Build)";
        assert_eq!(extract_build(text).as_deref(), Some("Development Build"));
    }

    #[test]
    fn guid_detection() {
        assert!(is_guid("d713e470-abcd-4321-9876-123456789012"));
        assert!(!is_guid("not-a-guid"));
    }
}
