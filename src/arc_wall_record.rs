//! Raw-byte decoder for ArcWall (tag 0x0191) partition records.
//!
//! # Scope: **Revit 2023 only.**
//!
//! The 2023 record envelope used by this module (tag `0x0191`, variant
//! marker `0x07fa`, fixed header `0x00088004`, 6 f64 coords × 2) is
//! **not** present in Revit 2024 files. On 2024 files:
//!
//! - ArcWall's tag drifted to `0x019c` (not `0x0191`).
//! - The record variant distribution at +0x10 shifted entirely;
//!   zero records carry the 2023 `0x07fa` marker.
//! - 2024 needs a separate decoder implementation.
//!
//! See `reports/element-framing/RE-13-synthesis.md` for the cross-
//! version drift evidence and open questions (Q18-Q20, hypotheses
//! H17-H19, decisions D23-D27 scoping this module's coverage).
//!
//! Callers that need 2024 support should check the file's
//! `BasicFileInfo.version` before invoking this decoder, and route
//! 2024+ files to the future `arc_wall_record_2024` module (not yet
//! implemented — blocked on RE-14.4).
//!
//! # Origin (why the shape is what it is)
//!
//! This module decodes records directly from `Partitions/N` bytes,
//! bypassing the schema-driven `ElementDecoder` trait. The reason
//! for a separate module is that the partition-level wire format
//! is distinct from the schema-field-level wire format — partition
//! records are self-describing fixed-size structs keyed by
//! `(tag, variant)`, while schema-field decoders operate on already-
//! classified `FieldType` enums.
//!
//! See `reports/element-framing/RE-14.3-synthesis.md` for the
//! empirical evidence this implementation is based on:
//! 32 records on Einhoven `Partitions/5` (Revit 2023), 28 decodable
//! as standard walls, 2 as compound walls, 1 index, 3 metadata.
//!
//! # Wire format (standard variant)
//!
//! ```text
//! offset  size  field
//! +0x00   2     u16 tag              = 0x0191
//! +0x02   2     u16 filter_pad       = 0x0000
//! +0x04   4     u32 fixed_header_0   = 0x00088004 (schema-family marker)
//! +0x08   4     u32 count_version    = 1 for standard, 3 for compound
//! +0x0c   4     u32 type_code        = 0x00000003
//! +0x10   2     u16 variant_marker   = 0x07fa standard | 0x0821 compound
//! +0x12   48    f64 × 6              primary geometry
//! +0x42   48    f64 × 6              geometry duplicate
//! +0x72   1     u8  trailer_0x03     record terminator
//! ```
//!
//! Total fixed size: 115 B. Records pack at 292 or 568 B stride in the
//! partition stream; the remaining bytes after the fixed core are
//! padding or inter-record content (not yet decoded).

use crate::{Error, Result};

/// ArcWall tag on Revit 2023. On 2024 the tag drifted to `0x019c`
/// — this constant is for 2023 decode only. See RE-13 synthesis for
/// the cross-version drift analysis.
pub const ARC_WALL_TAG: u16 = 0x0191;
/// Record envelope marker for "standard" wall records.
pub const ARC_WALL_VARIANT_STANDARD: u16 = 0x07fa;
/// Record envelope marker for "compound" wall records (with embedded openings).
pub const ARC_WALL_VARIANT_COMPOUND: u16 = 0x0821;
/// Schema-family marker — the constant at +0x04 of every ArcWall record.
/// Cross-references to the `0x00088004` value that appears in HostObjAttr
/// records' shared-suffix block (see RE-14.1).
pub const SCHEMA_FAMILY_MARKER: u32 = 0x0008_8004;
/// Record terminator byte found at +0x72 of every standard record.
pub const RECORD_TRAILER: u8 = 0x03;

/// Minimum bytes required to decode a standard ArcWall record.
pub const STANDARD_RECORD_MIN_SIZE: usize = 0x73;

/// A standard (non-compound) ArcWall record decoded from raw partition bytes.
///
/// Variants other than `0x07fa` are not decoded by this type. Compound
/// wall records use a different envelope and are tracked as separate
/// reverse-engineering work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcWallRecord {
    /// Tag value read from the record. Always `0x0191` for valid records.
    pub tag: u16,
    /// Record-envelope variant marker. For this type always
    /// `ARC_WALL_VARIANT_STANDARD (0x07fa)`.
    pub variant: u16,
    /// `u32` at `+0x04` of the record. Always `0x00088004` on observed
    /// corpus; check equals `SCHEMA_FAMILY_MARKER` as a sanity gate.
    pub fixed_header_0: u32,
    /// `u32` at `+0x08`. `1` for standard records, `3` for compound.
    pub count_version: u32,
    /// `u32` at `+0x0c`. `0x03` on all observed standard records.
    pub type_code: u32,
    /// Six `f64`s at `+0x12`: the primary geometry (two 3D points for
    /// wall-centerline endpoints on observed corpus).
    pub coords: [f64; 6],
    /// Six `f64`s at `+0x42`: geometry duplicate, typically matching
    /// `coords` exactly. Divergences from `coords` have been observed
    /// on ~20% of corpus records — hypothesis H16 (RE-14.3) suggests
    /// these may be the same geometry in a different reference frame
    /// (base-line vs location-line).
    pub coords_dup: [f64; 6],
    /// Record terminator byte at `+0x72`. Always `0x03` on observed corpus.
    pub trailer: u8,
}

impl ArcWallRecord {
    /// Decode a standard ArcWall record starting at `buf[offset..]`.
    ///
    /// Returns `Err` when any of the following fail:
    /// - `buf` is shorter than `STANDARD_RECORD_MIN_SIZE`
    /// - `tag` is not `0x0191`
    /// - `filter_pad` is not zero
    /// - `variant` is not `0x07fa`
    ///
    /// Callers should use [`Self::find_all`] to locate valid offsets;
    /// this function does not scan, it only
    /// validates + decodes at the given position.
    pub fn decode_standard(buf: &[u8], offset: usize) -> Result<Self> {
        if offset + STANDARD_RECORD_MIN_SIZE > buf.len() {
            return Err(Error::Cfb(format!(
                "ArcWall decode: buffer too short ({} < {} at offset {})",
                buf.len(),
                offset + STANDARD_RECORD_MIN_SIZE,
                offset
            )));
        }
        let tag = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        if tag != ARC_WALL_TAG {
            return Err(Error::Cfb(format!(
                "ArcWall decode: expected tag 0x{ARC_WALL_TAG:04x}, got 0x{tag:04x} at offset {offset}"
            )));
        }
        let filter_pad = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);
        if filter_pad != 0 {
            return Err(Error::Cfb(format!(
                "ArcWall decode: expected filter_pad 0x0000, got 0x{filter_pad:04x} at offset {offset}"
            )));
        }
        let variant = u16::from_le_bytes([buf[offset + 0x10], buf[offset + 0x11]]);
        if variant != ARC_WALL_VARIANT_STANDARD {
            return Err(Error::Cfb(format!(
                "ArcWall decode: expected variant 0x{ARC_WALL_VARIANT_STANDARD:04x}, got 0x{variant:04x} at offset {offset}"
            )));
        }

        let fixed_header_0 = u32::from_le_bytes([
            buf[offset + 0x04],
            buf[offset + 0x05],
            buf[offset + 0x06],
            buf[offset + 0x07],
        ]);
        let count_version = u32::from_le_bytes([
            buf[offset + 0x08],
            buf[offset + 0x09],
            buf[offset + 0x0a],
            buf[offset + 0x0b],
        ]);
        let type_code = u32::from_le_bytes([
            buf[offset + 0x0c],
            buf[offset + 0x0d],
            buf[offset + 0x0e],
            buf[offset + 0x0f],
        ]);

        let mut coords = [0f64; 6];
        for (i, slot) in coords.iter_mut().enumerate() {
            let p = offset + 0x12 + i * 8;
            *slot = f64::from_le_bytes([
                buf[p],
                buf[p + 1],
                buf[p + 2],
                buf[p + 3],
                buf[p + 4],
                buf[p + 5],
                buf[p + 6],
                buf[p + 7],
            ]);
        }
        let mut coords_dup = [0f64; 6];
        for (i, slot) in coords_dup.iter_mut().enumerate() {
            let p = offset + 0x42 + i * 8;
            *slot = f64::from_le_bytes([
                buf[p],
                buf[p + 1],
                buf[p + 2],
                buf[p + 3],
                buf[p + 4],
                buf[p + 5],
                buf[p + 6],
                buf[p + 7],
            ]);
        }
        let trailer = buf[offset + 0x72];

        Ok(ArcWallRecord {
            tag,
            variant,
            fixed_header_0,
            count_version,
            type_code,
            coords,
            coords_dup,
            trailer,
        })
    }

    /// Scan a partition buffer for all records matching ArcWall tag +
    /// filter-pad + standard-variant. Returns the byte offsets where
    /// `decode_standard` succeeds.
    ///
    /// This is a convenience scanner — for use in walker integration
    /// and probe CLIs. It does not deduplicate overlapping matches
    /// (the filter pattern is strict enough that overlap is vanishingly
    /// unlikely).
    pub fn find_all(buf: &[u8]) -> Vec<usize> {
        let mut out = Vec::new();
        if buf.len() < STANDARD_RECORD_MIN_SIZE {
            return out;
        }
        for i in 0..=(buf.len() - STANDARD_RECORD_MIN_SIZE) {
            let tag = u16::from_le_bytes([buf[i], buf[i + 1]]);
            if tag != ARC_WALL_TAG {
                continue;
            }
            if buf[i + 2] != 0 || buf[i + 3] != 0 {
                continue;
            }
            let variant = u16::from_le_bytes([buf[i + 0x10], buf[i + 0x11]]);
            if variant != ARC_WALL_VARIANT_STANDARD {
                continue;
            }
            out.push(i);
        }
        out
    }

    /// Convenience: returns the first 3 coordinates as a 3-tuple.
    /// Hypothesis H16 (conf 0.75) says these are the first 3D point
    /// (wall start).
    pub fn start_point(&self) -> (f64, f64, f64) {
        (self.coords[0], self.coords[1], self.coords[2])
    }

    /// Convenience: last 3 coordinates — the second 3D point (wall end
    /// under H16).
    pub fn end_point(&self) -> (f64, f64, f64) {
        (self.coords[3], self.coords[4], self.coords[5])
    }

    /// Whether the record's coord duplicate matches the primary block.
    /// ~80% of observed records have an exact match (tight tolerance).
    pub fn coords_match(&self) -> bool {
        self.coords == self.coords_dup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact hex bytes from record #4 on Einhoven Partitions/5
    /// (offset 87198 in the decompressed partition buffer). Captured
    /// from `probe_arcwall_records` output on 2026-04-21.
    const RECORD_4_HEX: &[u8] = &[
        0x91, 0x01, 0x00, 0x00, 0x04, 0x80, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
        0x00, 0xfa, 0x07, 0x63, 0x7f, 0x48, 0x57, 0x8a, 0x77, 0x22, 0x40, 0x9c, 0xd5, 0xb6, 0x13,
        0x76, 0xaa, 0x39, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x13, 0x7a,
        0x96, 0x54, 0x07, 0x29, 0x40, 0x32, 0xf5, 0x9b, 0x5b, 0x6f, 0x7c, 0x3a, 0x40, 0x8f, 0xf2,
        0xa3, 0xfc, 0x28, 0x3f, 0x1a, 0x40, 0x63, 0x7f, 0x48, 0x57, 0x8a, 0x77, 0x22, 0x40, 0x9c,
        0xd5, 0xb6, 0x13, 0x76, 0xaa, 0x39, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x0e, 0x13, 0x7a, 0x96, 0x54, 0x07, 0x29, 0x40, 0x32, 0xf5, 0x9b, 0x5b, 0x6f, 0x7c, 0x3a,
        0x40, 0x8f, 0xf2, 0xa3, 0xfc, 0x28, 0x3f, 0x1a, 0x40, 0x03,
    ];

    #[test]
    fn decodes_fixture_record_4() {
        let rec = ArcWallRecord::decode_standard(RECORD_4_HEX, 0).expect("record 4 must decode");
        assert_eq!(rec.tag, ARC_WALL_TAG);
        assert_eq!(rec.variant, ARC_WALL_VARIANT_STANDARD);
        assert_eq!(rec.fixed_header_0, SCHEMA_FAMILY_MARKER);
        assert_eq!(rec.count_version, 1);
        assert_eq!(rec.type_code, 3);
        assert_eq!(rec.trailer, RECORD_TRAILER);
    }

    #[test]
    fn record_4_coords_are_finite() {
        let rec = ArcWallRecord::decode_standard(RECORD_4_HEX, 0).unwrap();
        for c in &rec.coords {
            assert!(c.is_finite(), "coord must be finite, got {c}");
        }
        // Specifically the third coordinate on record #4 is 0.0 (Z=0
        // confirmed via hex); the others are positive ft values.
        assert_eq!(rec.coords[2], 0.0);
        assert!(rec.coords[0] > 0.0);
        assert!(rec.coords[1] > 0.0);
    }

    #[test]
    fn record_4_coords_match_duplicate() {
        let rec = ArcWallRecord::decode_standard(RECORD_4_HEX, 0).unwrap();
        assert!(
            rec.coords_match(),
            "record 4 has identical coords/coords_dup per RE-14.3"
        );
    }

    #[test]
    fn start_and_end_point_helpers() {
        let rec = ArcWallRecord::decode_standard(RECORD_4_HEX, 0).unwrap();
        let (sx, sy, sz) = rec.start_point();
        let (ex, ey, ez) = rec.end_point();
        assert_eq!((sx, sy, sz), (rec.coords[0], rec.coords[1], rec.coords[2]));
        assert_eq!((ex, ey, ez), (rec.coords[3], rec.coords[4], rec.coords[5]));
    }

    #[test]
    fn rejects_wrong_tag() {
        let mut buf = RECORD_4_HEX.to_vec();
        buf[0] = 0xff; // corrupt tag
        let err = ArcWallRecord::decode_standard(&buf, 0).unwrap_err();
        assert!(err.to_string().contains("tag"));
    }

    #[test]
    fn rejects_nonzero_filter_pad() {
        let mut buf = RECORD_4_HEX.to_vec();
        buf[2] = 0x01; // break filter_pad
        let err = ArcWallRecord::decode_standard(&buf, 0).unwrap_err();
        assert!(err.to_string().contains("filter_pad"));
    }

    #[test]
    fn rejects_wrong_variant() {
        let mut buf = RECORD_4_HEX.to_vec();
        buf[0x10] = 0x21; // variant becomes 0x0021 — wrong
        buf[0x11] = 0x00;
        let err = ArcWallRecord::decode_standard(&buf, 0).unwrap_err();
        assert!(err.to_string().contains("variant"));
    }

    #[test]
    fn rejects_buffer_too_short() {
        let err = ArcWallRecord::decode_standard(&RECORD_4_HEX[..20], 0).unwrap_err();
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn find_all_finds_embedded_record() {
        // Embed RECORD_4_HEX in a larger buffer at offset 100.
        let mut buf = vec![0u8; 100];
        buf.extend_from_slice(RECORD_4_HEX);
        buf.extend_from_slice(&[0u8; 100]);
        let found = ArcWallRecord::find_all(&buf);
        assert_eq!(found, vec![100]);
    }

    #[test]
    fn find_all_empty_on_noise() {
        let buf = vec![0xff_u8; 10_000];
        let found = ArcWallRecord::find_all(&buf);
        assert!(found.is_empty());
    }
}
