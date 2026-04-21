//! URL-based viewer state sharing (VW1-24).
//!
//! Ties the VW1 viewer primitives into a single `ViewerState`
//! struct that can be base64-serialized to fit in a URL fragment.
//! Typical flow:
//!
//! 1. Viewer loads with default state.
//! 2. User orbits, hides layers, draws a section box.
//! 3. Frontend calls `encode_to_fragment(&state)` and writes it
//!    to `window.location.hash`.
//! 4. Anyone opening the URL calls `decode_from_fragment(hash)`
//!    and passes the restored state to the viewer.
//!
//! The encoding is deterministic base64-of-JSON — no compression
//! (the state is small enough that the extra dep isn't worth
//! it), no versioning (a breaking change bumps the JSON schema's
//! top-level keys so `decode_from_fragment` returns `None`
//! gracefully).

use super::camera::CameraState;
use super::clipping::{SectionBox, ViewMode};
use super::scene_graph::CategoryFilter;
use serde::{Deserialize, Serialize};

/// Complete viewer state bundled for URL sharing (VW1-24).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewerState {
    /// Optional file hash / identifier — when present, frontends
    /// can refuse to restore state that belongs to a different
    /// file than what's currently loaded. Empty when the
    /// caller isn't tracking file identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,
    /// Current orbit camera pose.
    pub camera: CameraState,
    /// Current view mode (plan / 3D / section).
    pub view_mode: ViewMode,
    /// Current section box. `None` when the user hasn't drawn
    /// one (viewer uses the view mode's default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_box: Option<SectionBox>,
    /// Hidden IFC types for the category layer toggles.
    #[serde(default, skip_serializing_if = "CategoryFilter::is_empty")]
    pub category_filter: CategoryFilter,
    /// Name of the selected element (from SceneNode.name) for
    /// auto-focus on restore. Empty when nothing is selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_name: Option<String>,
}

/// Base64-encode a `ViewerState` to a URL-safe fragment (VW1-24).
/// The returned string is ready to drop into
/// `window.location.hash = "#v=" + returned` on the frontend.
///
/// Uses the standard base64 alphabet with `=` padding.
pub fn encode_to_fragment(state: &ViewerState) -> String {
    let json =
        serde_json::to_string(state).expect("ViewerState serialization can't fail — pure data");
    base64_encode(json.as_bytes())
}

/// Decode a URL fragment back into a `ViewerState` (VW1-24).
/// Returns `None` when the fragment is invalid base64, produces
/// invalid UTF-8, or doesn't parse as the current `ViewerState`
/// JSON schema. Callers should fall back to `ViewerState::default()`
/// when this returns `None`.
pub fn decode_from_fragment(fragment: &str) -> Option<ViewerState> {
    let trimmed = fragment
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("v=");
    let bytes = base64_decode(trimmed)?;
    let text = std::str::from_utf8(&bytes).ok()?;
    serde_json::from_str(text).ok()
}

impl CategoryFilter {
    /// `true` when the filter has no hidden types (everything
    /// visible). Used to skip-serialize the default case and
    /// keep the URL fragment short.
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty()
    }
}

// ---- Internal: dep-free base64 ----

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() >= 2 {
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() == 3 {
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn lookup(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.iter().all(|c| *c == b'=') {
            break;
        }
        let mut buf = [0u8; 4];
        let mut pad = 0;
        for (i, c) in chunk.iter().enumerate() {
            if *c == b'=' {
                pad += 1;
                buf[i] = 0;
            } else {
                buf[i] = lookup(*c)?;
            }
        }
        let triple = ((buf[0] as u32) << 18)
            | ((buf[1] as u32) << 12)
            | ((buf[2] as u32) << 6)
            | buf[3] as u32;
        if pad <= 2 {
            out.push((triple >> 16) as u8);
        }
        if pad <= 1 {
            out.push((triple >> 8) as u8);
        }
        if pad == 0 {
            out.push(triple as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_round_trips_through_url() {
        let state = ViewerState::default();
        let fragment = encode_to_fragment(&state);
        let back = decode_from_fragment(&fragment).unwrap();
        assert_eq!(back.view_mode, state.view_mode);
        assert!(back.section_box.is_none());
        assert!(back.category_filter.hidden.is_empty());
    }

    #[test]
    fn state_with_filter_and_section_box_survives_round_trip() {
        let mut filter = CategoryFilter::default();
        filter.hide("IFCWALL");
        filter.hide("IFCCOLUMN");
        let state = ViewerState {
            file_hash: Some("abc123".into()),
            camera: CameraState::default(),
            view_mode: ViewMode::Plan,
            section_box: Some(SectionBox::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0])),
            category_filter: filter,
            selected_name: Some("Wall-5".into()),
        };
        let fragment = encode_to_fragment(&state);
        let back = decode_from_fragment(&fragment).unwrap();
        assert_eq!(back.file_hash.as_deref(), Some("abc123"));
        assert_eq!(back.view_mode, ViewMode::Plan);
        assert!(back.section_box.is_some());
        assert!(back.category_filter.is_hidden("IFCWALL"));
        assert!(back.category_filter.is_hidden("IFCCOLUMN"));
        assert_eq!(back.selected_name.as_deref(), Some("Wall-5"));
    }

    #[test]
    fn decode_strips_hash_prefix() {
        let state = ViewerState::default();
        let fragment = encode_to_fragment(&state);
        let with_hash = format!("#{}", fragment);
        let back = decode_from_fragment(&with_hash).unwrap();
        assert_eq!(back.view_mode, state.view_mode);
    }

    #[test]
    fn decode_strips_v_equals_prefix() {
        let state = ViewerState::default();
        let fragment = encode_to_fragment(&state);
        let with_v = format!("v={}", fragment);
        let back = decode_from_fragment(&with_v).unwrap();
        assert_eq!(back.view_mode, state.view_mode);
    }

    #[test]
    fn decode_strips_full_hash_v_prefix() {
        let state = ViewerState::default();
        let fragment = encode_to_fragment(&state);
        let full = format!("#v={}", fragment);
        let back = decode_from_fragment(&full).unwrap();
        assert_eq!(back.view_mode, state.view_mode);
    }

    #[test]
    fn decode_returns_none_for_garbage() {
        assert!(decode_from_fragment("not-base64!@#$%").is_none());
    }

    #[test]
    fn decode_returns_none_for_valid_base64_but_non_json() {
        let not_json = base64_encode(b"this is not JSON at all");
        assert!(decode_from_fragment(&not_json).is_none());
    }

    #[test]
    fn decode_returns_none_for_empty_fragment() {
        assert!(decode_from_fragment("").is_none());
    }

    #[test]
    fn base64_encode_decode_cycles_cleanly() {
        for input in [
            &b""[..],
            &b"a"[..],
            &b"ab"[..],
            &b"abc"[..],
            &b"abcd"[..],
            &b"abcdefghijklmnopqrstuvwxyz0123456789"[..],
        ] {
            let encoded = base64_encode(input);
            let decoded = base64_decode(&encoded).unwrap();
            assert_eq!(decoded, input);
        }
    }

    #[test]
    fn base64_decode_ignores_whitespace() {
        let encoded = base64_encode(b"hello");
        let with_ws = format!("\n {}\t \n", encoded);
        assert_eq!(base64_decode(&with_ws).unwrap(), b"hello");
    }

    #[test]
    fn default_state_fragment_is_compact() {
        // Default state should encode to a small fragment — the
        // skip_serializing_if attributes mean empty-filter /
        // None-file-hash / None-section-box don't bloat the URL.
        let fragment = encode_to_fragment(&ViewerState::default());
        // Should fit comfortably in a URL hash (< 500 chars).
        assert!(fragment.len() < 500);
    }

    #[test]
    fn file_hash_optional_omitted_when_none() {
        let state = ViewerState::default();
        let fragment = encode_to_fragment(&state);
        let decoded = base64_decode(&fragment).unwrap();
        let json = std::str::from_utf8(&decoded).unwrap();
        assert!(!json.contains("file_hash"));
    }
}
