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
//!
//! # The `Key: value` text block
//!
//! After the binary header, every release 2016-2026 appends a CRLF-separated
//! block of `Key: value` lines — the same text Revit shows nowhere in its UI
//! but writes on every save. On the 11-release `phi-ag/rvt` family corpus and
//! both magnetar project files it carries, in order:
//!
//! ```text
//! Worksharing: Not enabled
//! Username:
//! Central Model Path:
//! Format: 2024                      (2019+; 2016-2018 write "Revit Build: Autodesk Revit 2016 (Build: …)")
//! Build: 20230509_0315(x64)
//! Last Save Path: B:\…\2024_Core_Interior.rvt
//! Open Workset Default: 3
//! Project Spark File: 0
//! Central Model Identity: 00000000-0000-0000-0000-000000000000
//! Locale when saved: ENU
//! All Local Changes Saved To Central: 0
//! Central model's version number corresponding to the last reload latest: 66
//! Central model's episode GUID corresponding to the last reload latest: 2a619b2e-…
//! Unique Document GUID: 2a619b2e-…
//! Unique Document Increments: 66
//! Model Identity: 00000000-0000-0000-0000-000000000000
//! IsSingleUserCloudModel: False      (2019+)
//! Author: Autodesk Revit             (2019+)
//! ClientAppName: RevitApplication    (2021+)
//! ```
//!
//! Two quirks make a plain UTF-16LE decode miss it:
//!
//! 1. The binary header before the block contains 1-byte fields, so the
//!    block starts at an **odd** byte offset on some files (2026, both
//!    project files) and an even one on others. [`parse_properties`] decodes
//!    both alignments and keeps the one that yields the known keys.
//! 2. `IsSingleUserCloudModel` is written as a *narrow* C string packed into
//!    the wide text (`"False\0"` reads back as `慆獬e`), and a 5-byte
//!    `"True\0"` would flip the alignment of everything after it. Values that
//!    unpack to `True` / `False` are restored; lines after a flip are
//!    recovered from the other alignment.
//!
//! [`BasicFileInfo::properties`] keeps every line verbatim; typed accessors
//! such as [`BasicFileInfo::worksharing`] and [`BasicFileInfo::username`]
//! read from it and return `None` for an absent or empty value.

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
    /// Every `Key: value` line of the trailing text block, in file order,
    /// values verbatim (see the module docs). Empty when the stream carries
    /// no such block.
    #[serde(default)]
    pub properties: Vec<FileProperty>,
    /// Raw UTF-16LE decoded text for debugging.
    pub raw_text: String,
}

/// One `Key: value` line from the `BasicFileInfo` text block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileProperty {
    pub key: String,
    pub value: String,
}

/// Keys observed on every release of the reference corpora. Used to pick
/// the byte alignment the text block was written at; unknown keys are still
/// kept in [`BasicFileInfo::properties`].
const KNOWN_KEYS: &[&str] = &[
    "Worksharing",
    "Username",
    "Central Model Path",
    "Format",
    "Build",
    "Revit Build",
    "Last Save Path",
    "Open Workset Default",
    "Project Spark File",
    "Central Model Identity",
    "Locale when saved",
    "All Local Changes Saved To Central",
    "Central model's version number corresponding to the last reload latest",
    "Central model's episode GUID corresponding to the last reload latest",
    "Unique Document GUID",
    "Unique Document Increments",
    "Model Identity",
    "IsSingleUserCloudModel",
    "Author",
    "ClientAppName",
];

impl BasicFileInfo {
    /// Parse the raw `BasicFileInfo` stream bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let (cow, _, had_errors) = UTF_16LE.decode(data);
        if had_errors {
            // Not fatal — some single-byte markers aren't valid UTF-16 pairs.
            // We still extract from whatever decoded cleanly.
        }
        let raw = cow.into_owned();
        let properties = parse_properties(data);
        let prop = |key: &str| lookup(&properties, key);

        let version = extract_version(&raw)
            .or_else(|| prop("Format").and_then(parse_release_year))
            .ok_or_else(|| Error::BasicFileInfo("no 4-digit Revit version found".into()))?;
        let build = extract_build(&raw).or_else(|| prop("Build").map(str::to_string));
        let original_path =
            extract_path(&raw).or_else(|| prop("Last Save Path").map(str::to_string));
        let guid = prop("Unique Document GUID")
            .filter(|g| is_guid(g))
            .map(str::to_string)
            .or_else(|| extract_guid(&raw));
        let locale = prop("Locale when saved")
            .map(str::to_string)
            .or_else(|| extract_locale(&raw));

        Ok(Self {
            version,
            build,
            original_path,
            guid,
            locale,
            properties,
            raw_text: raw,
        })
    }

    /// Value of the text-block line `key` (exact, case-sensitive match),
    /// or `None` when the line is absent or its value is empty.
    pub fn property(&self, key: &str) -> Option<&str> {
        lookup(&self.properties, key)
    }

    /// The `Worksharing` line verbatim: `Not enabled` on a file without
    /// worksharing; other values (`Central`, `Local`, …) name the role the
    /// saved copy had in a workshared project.
    pub fn worksharing(&self) -> Option<&str> {
        self.property("Worksharing")
    }

    /// `Some(false)` when `Worksharing` reads `Not enabled`, `Some(true)`
    /// for any other recorded value, `None` when the line is missing.
    pub fn is_workshared(&self) -> Option<bool> {
        self.worksharing()
            .map(|w| !w.eq_ignore_ascii_case("Not enabled"))
    }

    /// Windows / Autodesk account name recorded by the last save
    /// (empty on non-workshared files, which yields `None`).
    pub fn username(&self) -> Option<&str> {
        self.property("Username")
    }

    /// Path of the central model a workshared file syncs with — a UNC or
    /// drive path, an `RSN://` Revit Server path, or a cloud path.
    pub fn central_model_path(&self) -> Option<&str> {
        self.property("Central Model Path")
    }

    /// Full path the file was last saved to.
    pub fn last_save_path(&self) -> Option<&str> {
        self.property("Last Save Path")
    }

    /// `Unique Document GUID`: the document's identity, stable across saves.
    pub fn document_guid(&self) -> Option<&str> {
        self.property("Unique Document GUID")
    }

    /// `Unique Document Increments`: Revit's per-document counter. It grows
    /// as the document is saved (59 → 70 across the eleven yearly re-saves
    /// of the reference sample family); the exact increment rule is not
    /// documented.
    pub fn document_increments(&self) -> Option<u32> {
        self.property("Unique Document Increments")
            .and_then(|v| v.trim().parse().ok())
    }

    /// `IsSingleUserCloudModel` (Revit 2019+).
    pub fn is_single_user_cloud_model(&self) -> Option<bool> {
        self.property("IsSingleUserCloudModel").and_then(parse_bool)
    }

    /// `All Local Changes Saved To Central` (`0` / `1` on disk).
    pub fn all_local_changes_saved_to_central(&self) -> Option<bool> {
        self.property("All Local Changes Saved To Central")
            .and_then(parse_bool)
    }

    /// `Open Workset Default`: the worksets-to-open choice stored with the file.
    pub fn open_workset_default(&self) -> Option<u32> {
        self.property("Open Workset Default")
            .and_then(|v| v.trim().parse().ok())
    }

    /// `Author` (Revit 2019+; `Autodesk Revit` on every observed file).
    pub fn author(&self) -> Option<&str> {
        self.property("Author")
    }

    /// `ClientAppName` (Revit 2021+; `RevitApplication` on every observed file).
    pub fn client_app_name(&self) -> Option<&str> {
        self.property("ClientAppName")
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

fn lookup<'a>(properties: &'a [FileProperty], key: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|p| p.key == key)
        .map(|p| p.value.as_str())
        .filter(|v| !v.is_empty())
}

/// Recover the `Key: value` text block (see the module docs).
///
/// Decodes the stream at both byte alignments. The alignment that yields
/// more of the keys observed on the reference corpora supplies the lines in
/// file order; known keys only the other alignment decodes (the tail after a
/// narrow `True\0` flip) are appended.
pub fn parse_properties(data: &[u8]) -> Vec<FileProperty> {
    let decode = |align: usize| -> Vec<FileProperty> {
        let Some(body) = data.get(align..) else {
            return Vec::new();
        };
        let units = body
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
        let text: String = char::decode_utf16(units)
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
        text.split(['\r', '\n', '\u{0A0D}'])
            .filter_map(parse_line)
            .collect()
    };
    let score = |props: &[FileProperty]| {
        props
            .iter()
            .filter(|p| KNOWN_KEYS.contains(&p.key.as_str()))
            .count()
    };
    let even = decode(0);
    let odd = decode(1);
    let (mut primary, secondary) = if score(&odd) > score(&even) {
        (odd, even)
    } else {
        (even, odd)
    };
    if score(&primary) == 0 {
        return Vec::new();
    }
    for prop in secondary {
        if KNOWN_KEYS.contains(&prop.key.as_str()) && !primary.iter().any(|p| p.key == prop.key) {
            primary.push(prop);
        }
    }
    primary
}

fn parse_line(line: &str) -> Option<FileProperty> {
    let line = line.trim_matches(is_padding);
    let colon = line.find(':')?;
    let key = line[..colon].trim();
    if !is_label(key) {
        return None;
    }
    let rest = &line[colon + 1..];
    let value = if rest.is_empty() {
        ""
    } else {
        // `Key: value` — a colon glued to a non-space is a drive letter or a
        // URL scheme inside some other string, not a label.
        rest.strip_prefix(' ')?
    };
    Some(FileProperty {
        key: key.to_string(),
        value: clean_value(value),
    })
}

fn is_padding(c: char) -> bool {
    c == '\0' || c == char::REPLACEMENT_CHARACTER || c.is_whitespace()
}

fn is_label(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 96
        && key.starts_with(|c: char| c.is_ascii_alphabetic())
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '\'' | '-' | '_'))
}

fn clean_value(value: &str) -> String {
    let value = value.trim_matches(is_padding);
    unpack_narrow_bool(value).unwrap_or_else(|| value.to_string())
}

/// Undo Revit writing a narrow `"True\0"` / `"False\0"` into the wide text:
/// each UTF-16 unit then carries two ASCII bytes, read up to the NUL.
fn unpack_narrow_bool(value: &str) -> Option<String> {
    if value.is_ascii() {
        return None;
    }
    let mut narrow = Vec::new();
    'units: for unit in value.encode_utf16() {
        for byte in unit.to_le_bytes() {
            if byte == 0 {
                break 'units;
            }
            narrow.push(byte);
        }
    }
    let narrow = String::from_utf8(narrow).ok()?;
    parse_bool(&narrow).map(|b| if b { "True" } else { "False" }.to_string())
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        v if v.eq_ignore_ascii_case("true") || v == "1" => Some(true),
        v if v.eq_ignore_ascii_case("false") || v == "0" => Some(false),
        _ => None,
    }
}

/// Plausible Revit release years. The lower bound is the one the version
/// scan has always used; the upper bound leaves room for releases newer
/// than the reference corpus (Revit 2027 shipped in 2026) without accepting
/// arbitrary 4-digit numbers.
const RELEASE_YEARS: std::ops::RangeInclusive<u32> = 2014..=2040;

fn parse_release_year(value: &str) -> Option<u32> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|y| RELEASE_YEARS.contains(y))
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
                if RELEASE_YEARS.contains(&n) {
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

    // ---- `Key: value` text block ----

    const DOC_GUID: &str = "d713e470-abcd-4321-9876-123456789012";

    /// A stream shaped like the observed ones: a wide header, an optional
    /// 1-byte field that shifts the text block to an odd offset, then the
    /// block itself with `IsSingleUserCloudModel` written as a narrow string.
    fn synth_stream(odd: bool, worksharing: &str, username: &str, cloud_narrow: &[u8]) -> Vec<u8> {
        let mut bytes = utf16le("2024  20230509_0315(x64) ");
        if odd {
            bytes.push(0x01);
        }
        bytes.extend(utf16le("\u{0A0D}"));
        bytes.extend(utf16le(&format!(
            "Worksharing: {worksharing}\r\n\
             Username: {username}\r\n\
             Central Model Path: \\\\fileserver\\projects\\Tower_Central.rvt\r\n\
             Format: 2024\r\n\
             Build: 20230509_0315(x64)\r\n\
             Last Save Path: C:\\Users\\testuser\\Documents\\Tower_testuser.rvt\r\n\
             Open Workset Default: 3\r\n\
             Locale when saved: ENU\r\n\
             All Local Changes Saved To Central: 1\r\n\
             Unique Document GUID: {DOC_GUID}\r\n\
             Unique Document Increments: 66\r\n\
             IsSingleUserCloudModel: "
        )));
        bytes.extend_from_slice(cloud_narrow);
        bytes.extend(utf16le(
            "\r\nAuthor: Autodesk Revit\r\nClientAppName: RevitApplication\u{0A0D}",
        ));
        bytes
    }

    fn assert_block(info: &BasicFileInfo) {
        assert_eq!(info.version, 2024);
        assert_eq!(info.property("Format"), Some("2024"));
        assert_eq!(info.document_guid(), Some(DOC_GUID));
        assert_eq!(info.guid.as_deref(), Some(DOC_GUID));
        assert_eq!(info.document_increments(), Some(66));
        assert_eq!(info.open_workset_default(), Some(3));
        assert_eq!(info.locale.as_deref(), Some("ENU"));
        assert_eq!(info.all_local_changes_saved_to_central(), Some(true));
        assert_eq!(
            info.last_save_path(),
            Some("C:\\Users\\testuser\\Documents\\Tower_testuser.rvt")
        );
        assert_eq!(info.author(), Some("Autodesk Revit"));
        assert_eq!(info.client_app_name(), Some("RevitApplication"));
    }

    #[test]
    fn properties_parse_at_even_alignment() {
        let info =
            BasicFileInfo::from_bytes(&synth_stream(false, "Not enabled", "", b"False\0")).unwrap();
        assert_block(&info);
        assert_eq!(info.worksharing(), Some("Not enabled"));
        assert_eq!(info.is_workshared(), Some(false));
        assert_eq!(info.username(), None);
        assert_eq!(info.is_single_user_cloud_model(), Some(false));
    }

    #[test]
    fn properties_parse_at_odd_alignment() {
        let info = BasicFileInfo::from_bytes(&synth_stream(true, "Local", "testuser", b"False\0"))
            .unwrap();
        assert_block(&info);
        assert_eq!(info.worksharing(), Some("Local"));
        assert_eq!(info.is_workshared(), Some(true));
        assert_eq!(info.username(), Some("testuser"));
        assert_eq!(
            info.central_model_path(),
            Some("\\\\fileserver\\projects\\Tower_Central.rvt")
        );
        assert_eq!(info.is_single_user_cloud_model(), Some(false));
    }

    #[test]
    fn narrow_true_flips_alignment_and_the_tail_is_recovered() {
        for odd in [false, true] {
            let info =
                BasicFileInfo::from_bytes(&synth_stream(odd, "Central", "testuser", b"True\0"))
                    .unwrap();
            assert_block(&info);
            assert_eq!(info.is_single_user_cloud_model(), Some(true), "odd={odd}");
            assert_eq!(info.worksharing(), Some("Central"));
        }
    }

    #[test]
    fn properties_keep_file_order_and_unknown_keys() {
        let mut bytes = utf16le("2025  Development Build ");
        bytes.extend(utf16le(
            "\u{0A0D}Worksharing: Not enabled\r\nSome Future Key: 42\r\nFormat: 2025\u{0A0D}",
        ));
        let info = BasicFileInfo::from_bytes(&bytes).unwrap();
        let keys: Vec<&str> = info.properties.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, ["Worksharing", "Some Future Key", "Format"]);
        assert_eq!(info.property("Some Future Key"), Some("42"));
    }

    #[test]
    fn header_strings_are_not_mistaken_for_properties() {
        // 2016-2018 headers carry `Autodesk Revit 2016 (Build: …)` and a
        // drive-letter path before the block; neither is a `Key: value` line.
        let mut bytes =
            utf16le("Autodesk Revit 2016 (Build: 20150110_1515(x64))d C:\\Samples\\family.rfa ");
        bytes.extend(utf16le(
            "\u{0A0D}Worksharing: Not enabled\r\nRevit Build: Autodesk Revit 2016 (Build: 20150110_1515(x64))\u{0A0D}",
        ));
        let info = BasicFileInfo::from_bytes(&bytes).unwrap();
        let keys: Vec<&str> = info.properties.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, ["Worksharing", "Revit Build"]);
        assert_eq!(
            info.property("Revit Build"),
            Some("Autodesk Revit 2016 (Build: 20150110_1515(x64))")
        );
        assert_eq!(info.version, 2016);
    }

    #[test]
    fn stream_without_block_has_no_properties() {
        let info = BasicFileInfo::from_bytes(&utf16le("2024  20230308_1635(x64) ")).unwrap();
        assert!(info.properties.is_empty());
        assert_eq!(info.worksharing(), None);
        assert_eq!(info.document_increments(), None);
    }

    #[test]
    fn parse_properties_never_panics_on_arbitrary_bytes() {
        for len in 0..64 {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let _ = parse_properties(&bytes);
        }
        let _ = parse_properties(&[0x3a, 0x00, 0x20, 0x00]);
    }

    // ---- WRT-07: BasicFileInfo writer round-trip ----

    fn make_info() -> BasicFileInfo {
        BasicFileInfo {
            version: 2024,
            build: Some("20230308_1635(x64)".into()),
            original_path: Some("C:\\Users\\testuser\\Desktop\\sample.rfa".into()),
            guid: Some("d713e470-abcd-4321-9876-123456789012".into()),
            locale: Some("ENU".into()),
            properties: Vec::new(),
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
            properties: Vec::new(),
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
            properties: Vec::new(),
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
            properties: Vec::new(),
            raw_text: String::new(),
        };
        let bytes = info.encode();
        let re = BasicFileInfo::from_bytes(&bytes).unwrap();
        assert_eq!(re.version, 2026);
    }
}
