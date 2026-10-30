//! URL-based viewer state sharing.
//!
//! Ties the viewer primitives into a single `ViewerState`
//! struct that a URL fragment carries as base64-encoded JSON.
//! [`decode_from_fragment`] restores one from the fragment.
//!
//! The encoding is base64-of-JSON — no compression (the state is
//! small enough that the extra dep isn't worth it), no versioning (a
//! breaking change bumps the JSON schema's top-level keys so
//! `decode_from_fragment` returns `None` gracefully).
// Tracker: VW1-24.

use super::camera::CameraState;
use super::clipping::{SectionBox, ViewMode};
use super::scene_graph::CategoryFilter;
use serde::{Deserialize, Serialize};

/// Complete viewer state bundled for URL sharing.
// Tracker: VW1-24.
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

/// Decode a URL fragment back into a `ViewerState`.
/// Returns `None` when the fragment is invalid base64, produces
/// invalid UTF-8, or doesn't parse as the current `ViewerState`
/// JSON schema. Callers should fall back to `ViewerState::default()`
/// when this returns `None`.
// Tracker: VW1-24.
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
