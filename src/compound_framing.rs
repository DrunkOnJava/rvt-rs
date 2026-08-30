//! Compound ArcWall (`0x0821`) stamp / marker investigation harness.
//!
//! Research-only helpers for Lane C framing work described in
//! `reports/element-framing/RE-14.3-synthesis.md` and
//! `reports/element-framing/RE-15-synthesis.md`.
//!
//! This module **tokenizes and classifies markers**. It does **not** decode
//! compound openings, Door/Window fills, or host joins. Do not name APIs
//! `decode_compound_openings`.

use serde::{Deserialize, Serialize};

/// ArcWall compound variant marker (LE `21 08`).
pub const COMPOUND_VARIANT_MARKER: u16 = 0x0821;
/// Sub-marker observed inside compound bodies (LE `70 08`).
pub const COMPOUND_SUB_MARKER_0870: u16 = 0x0870;
/// Standard (non-compound) ArcWall variant for contrast.
pub const STANDARD_VARIANT_MARKER: u16 = 0x07fa;

/// Classification of a u16 stamp found while scanning synthetic/hex bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundStampKind {
    /// Envelope variant `0x0821`.
    CompoundVariant,
    /// Sub-marker `0x0870`.
    SubMarker0870,
    /// Standard wall variant `0x07fa` (contrast / collision notes).
    StandardVariant,
    /// Other `0x08xx`-shaped stamp — unclassified research noise.
    Unknown08xx(u16),
}

impl CompoundStampKind {
    pub fn classify(marker: u16) -> Option<Self> {
        match marker {
            COMPOUND_VARIANT_MARKER => Some(Self::CompoundVariant),
            COMPOUND_SUB_MARKER_0870 => Some(Self::SubMarker0870),
            STANDARD_VARIANT_MARKER => Some(Self::StandardVariant),
            m if (0x0800..=0x08ff).contains(&m) => Some(Self::Unknown08xx(m)),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompoundVariant => "compound_variant_0x0821",
            Self::SubMarker0870 => "sub_marker_0x0870",
            Self::StandardVariant => "standard_variant_0x07fa",
            Self::Unknown08xx(_) => "unknown_0x08xx",
        }
    }
}

/// One marker hit at a byte offset (little-endian u16 aligned or unaligned scan).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundStampHit {
    pub offset: usize,
    pub marker: u16,
    pub kind: CompoundStampKind,
}

/// Tokenize little-endian u16 markers of interest in a byte window.
///
/// Scans every offset (unaligned) so adversarial mid-f64 collisions can be
/// noted. This is a **research harness**, not a record parser.
pub fn tokenize_compound_markers(bytes: &[u8]) -> Vec<CompoundStampHit> {
    let mut hits = Vec::new();
    if bytes.len() < 2 {
        return hits;
    }
    for offset in 0..bytes.len() - 1 {
        let marker = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        if let Some(kind) = CompoundStampKind::classify(marker) {
            // Skip dense unknown_08xx spam except explicit known stamps —
            // still record Unknown08xx when caller wants collision notes via
            // `tokenize_all_08xx`.
            match kind {
                CompoundStampKind::Unknown08xx(_) => {}
                _ => hits.push(CompoundStampHit {
                    offset,
                    marker,
                    kind,
                }),
            }
        }
    }
    hits
}

/// Like [`tokenize_compound_markers`] but also keeps unknown `0x08xx` stamps.
pub fn tokenize_all_08xx(bytes: &[u8]) -> Vec<CompoundStampHit> {
    let mut hits = Vec::new();
    if bytes.len() < 2 {
        return hits;
    }
    for offset in 0..bytes.len() - 1 {
        let marker = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        if let Some(kind) = CompoundStampKind::classify(marker) {
            hits.push(CompoundStampHit {
                offset,
                marker,
                kind,
            });
        }
    }
    hits
}

/// Summary counts for a scanned window (honest investigation report row).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompoundStampSummary {
    pub compound_variant_hits: usize,
    pub sub_marker_0870_hits: usize,
    pub standard_variant_hits: usize,
    pub unknown_08xx_hits: usize,
    pub notes: Vec<String>,
}

impl CompoundStampSummary {
    pub fn from_hits(hits: &[CompoundStampHit]) -> Self {
        let mut s = Self {
            notes: vec![
                "Marker tokenization only — compound openings are not decoded.".into(),
                "Empty hit list does not prove absence of compound walls.".into(),
            ],
            ..Self::default()
        };
        for h in hits {
            match h.kind {
                CompoundStampKind::CompoundVariant => s.compound_variant_hits += 1,
                CompoundStampKind::SubMarker0870 => s.sub_marker_0870_hits += 1,
                CompoundStampKind::StandardVariant => s.standard_variant_hits += 1,
                CompoundStampKind::Unknown08xx(_) => s.unknown_08xx_hits += 1,
            }
        }
        s
    }
}

/// Adversarial note: f64 payloads can synthesize `21 08` / `70 08` by chance.
///
/// Unit tests seed a synthetic f64 byte pattern that collides with
/// `COMPOUND_VARIANT_MARKER` so harness consumers treat mid-float hits as
/// **candidates**, not proven record envelopes.
pub fn adversarial_f64_collision_seed() -> [u8; 8] {
    // Craft an f64 whose little-endian bytes contain `21 08` at offset 2.
    // Bytes: 00 00 21 08 00 00 00 00 — not a valid ArcWall envelope.
    [0x00, 0x00, 0x21, 0x08, 0x00, 0x00, 0x00, 0x00]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_synthetic_compound_envelope_hex() {
        // Minimal synthetic: tag pad + headers + 0x0821 + body with 0x0870
        // (does not claim this is a valid ArcWall record).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0191u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0x0008_8004u32.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&COMPOUND_VARIANT_MARKER.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 8]);
        bytes.extend_from_slice(&COMPOUND_SUB_MARKER_0870.to_le_bytes());
        let hits = tokenize_compound_markers(&bytes);
        assert!(
            hits.iter()
                .any(|h| h.kind == CompoundStampKind::CompoundVariant)
        );
        assert!(
            hits.iter()
                .any(|h| h.kind == CompoundStampKind::SubMarker0870)
        );
        let summary = CompoundStampSummary::from_hits(&hits);
        assert_eq!(summary.compound_variant_hits, 1);
        assert_eq!(summary.sub_marker_0870_hits, 1);
        assert!(summary.notes.iter().any(|n| n.contains("not decoded")));
    }

    #[test]
    fn adversarial_f64_seed_collides_with_compound_marker() {
        let seed = adversarial_f64_collision_seed();
        let hits = tokenize_compound_markers(&seed);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].offset, 2);
        assert_eq!(hits[0].kind, CompoundStampKind::CompoundVariant);
        // Honesty: a lone mid-buffer hit is not an envelope.
        assert_ne!(hits[0].offset, 0);
    }

    #[test]
    fn classify_round_trips_json() {
        let hit = CompoundStampHit {
            offset: 16,
            marker: COMPOUND_VARIANT_MARKER,
            kind: CompoundStampKind::CompoundVariant,
        };
        let json = serde_json::to_value(&hit).expect("ser");
        let back: CompoundStampHit = serde_json::from_value(json).expect("de");
        assert_eq!(back.marker, 0x0821);
    }
}
