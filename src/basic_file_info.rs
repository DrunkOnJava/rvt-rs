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

    /// Encode a `BasicFileInfo` back to UTF-16LE bytes (WRT-07).
    /// Inverse of [`Self::from_bytes`].
    ///
    /// Uses the canonical 2019+ pattern:
    ///
    /// ```text
    /// {year}  {build} {path} {locale} {guid}
    /// ```
    ///
    /// Any missing optional field is omitted (alongside its leading
    /// space) so the extract-\* parsers see the same empty-state
    /// they would in a real file without that field. Callers who
    /// want a specific older (2016-2018) pattern should use
    /// [`Self::encode_with_build_wrapper`] instead.
    ///
    /// Round-trip guarantee: `BasicFileInfo::from_bytes(&bfi.encode())`
    /// yields a `BasicFileInfo` whose `version`, `build`,
    /// `original_path`, `guid`, and `locale` equal the original's.
    /// `raw_text` may differ (it's the decoder's reconstruction,
    /// not a byte-level echo).
    pub fn encode(&self) -> Vec<u8> {
        let mut text = format!("{}", self.version);
        if let Some(build) = self.build.as_deref() {
            text.push_str("  ");
            text.push_str(build);
        }
        if let Some(path) = self.original_path.as_deref() {
            text.push(' ');
            text.push_str(path);
        }
        if let Some(locale) = self.locale.as_deref() {
            text.push(' ');
            text.push_str(locale);
        }
        if let Some(guid) = self.guid.as_deref() {
            text.push(' ');
            text.push_str(guid);
        }
        // Trailing space so the reader's GUID scanner (which uses an
        // exclusive upper bound) can find a GUID sitting at what
        // would otherwise be the string's exact final position.
        text.push(' ');
        utf16le(&text)
    }

    /// Encode using the 2016-2018 pattern where `build` is wrapped
    /// in `(Build: …)`. Useful when a downstream tool expects the
    /// pre-2019 on-disk format exactly.
    pub fn encode_with_build_wrapper(&self) -> Vec<u8> {
        let mut text = format!("Autodesk Revit {}", self.version);
        if let Some(build) = self.build.as_deref() {
            // Strip any (x64) suffix — the wrapper wants the raw
            // YYYYMMDD_HHMM token. Keep it verbatim if already
            // quoted.
            text.push_str(&format!(" (Build: {build})"));
        }
        if let Some(path) = self.original_path.as_deref() {
            text.push(' ');
            text.push_str(path);
        }
        if let Some(locale) = self.locale.as_deref() {
            text.push(' ');
            text.push_str(locale);
        }
        if let Some(guid) = self.guid.as_deref() {
            text.push(' ');
            text.push_str(guid);
        }
        // Trailing space — see `encode` for the rationale.
        text.push(' ');
        utf16le(&text)
    }
}

/// Encode a UTF-8 string as UTF-16LE bytes. Helper shared by the
/// [`BasicFileInfo::encode`] family.
fn utf16le(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 2);
    for unit in s.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
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
        // `colon_backslash` points to the ':' byte. Back up one byte
        // to include the drive letter, but land on a char boundary —
        // the saturating_sub can point into the middle of a multi-byte
        // UTF-8 character when text before ":\\" is non-ASCII (caught
        // by libFuzzer fuzz_basic_file_info 2026-04-21 on input
        // containing UTF-8 BOMs + Arabic bytes before a ":\\" literal).
        let s = colon_backslash.saturating_sub(1);
        let tail = text.get(s..)?;
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

    // ---- WRT-07: BasicFileInfo writer round-trip ----

    fn make_info() -> BasicFileInfo {
        BasicFileInfo {
            version: 2024,
            build: Some("20230308_1635(x64)".into()),
            original_path: Some("C:\\Users\\testuser\\Desktop\\sample.rfa".into()),
            guid: Some("d713e470-abcd-4321-9876-123456789012".into()),
            locale: Some("ENU".into()),
            raw_text: String::new(),
        }
    }

    #[test]
    fn encode_round_trips_all_fields() {
        let original = make_info();
        let bytes = original.encode();
        let decoded = BasicFileInfo::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.version, original.version);
        assert_eq!(decoded.build, original.build);
        assert_eq!(decoded.original_path, original.original_path);
        assert_eq!(decoded.guid, original.guid);
        assert_eq!(decoded.locale, original.locale);
    }

    #[test]
    fn encode_produces_utf16le_bytes() {
        let info = make_info();
        let bytes = info.encode();
        // UTF-16LE always has an even byte count.
        assert_eq!(bytes.len() % 2, 0);
        // First two bytes decode as '2' (0x0032) LE = [0x32, 0x00].
        assert_eq!(bytes[..2], [0x32, 0x00]);
    }

    #[test]
    fn encode_with_build_wrapper_produces_pre_2019_pattern() {
        let info = BasicFileInfo {
            version: 2018,
            build: Some("20170130_1515(x64)".into()),
            original_path: None,
            guid: None,
            locale: None,
            raw_text: String::new(),
        };
        let bytes = info.encode_with_build_wrapper();
        // Decode back as UTF-16LE and look for the wrapper.
        let (cow, _, _) = UTF_16LE.decode(&bytes);
        assert!(
            cow.contains("(Build: 20170130_1515(x64))"),
            "expected Build: wrapper, got {cow}"
        );
        // Round-trip still recovers the build tag.
        let re = BasicFileInfo::from_bytes(&bytes).unwrap();
        assert_eq!(re.build.as_deref(), Some("20170130_1515(x64)"));
        assert_eq!(re.version, 2018);
    }

    #[test]
    fn encode_omits_missing_optional_fields() {
        let info = BasicFileInfo {
            version: 2020,
            build: None,
            original_path: None,
            guid: None,
            locale: None,
            raw_text: String::new(),
        };
        let bytes = info.encode();
        let decoded = BasicFileInfo::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.version, 2020);
        assert!(decoded.build.is_none());
        assert!(decoded.original_path.is_none());
        assert!(decoded.guid.is_none());
        assert!(decoded.locale.is_none());
    }

    #[test]
    fn encode_round_trip_survives_only_version() {
        let info = BasicFileInfo {
            version: 2026,
            build: None,
            original_path: None,
            guid: None,
            locale: None,
            raw_text: String::new(),
        };
        let bytes = info.encode();
        let re = BasicFileInfo::from_bytes(&bytes).unwrap();
        assert_eq!(re.version, 2026);
    }
}
