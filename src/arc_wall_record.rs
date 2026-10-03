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
//! Production callers should use
//! [`ArcWallRecord::scan_standard_for_revit_version`] instead of
//! calling the raw pattern scanner directly. Unsupported releases
//! return an empty result with a structured status so exporters do not
//! apply the 2023 byte pattern to 2024+ partition streams.
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
//! partition stream; the remaining bytes after the fixed core are the
//! singleton trailer (177 B) documented in
//! `reports/element-framing/RE-15-arcwall-trailer-synthesis.md`.

use crate::walker::{WalkerLimitHit, WalkerLimits};
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

/// Minimum bytes required to decode a standard ArcWall record core.
pub const STANDARD_RECORD_MIN_SIZE: usize = 0x73;

/// Singleton stride observed on Einhoven Partitions/5 (core + 177 B
/// trailer). Paired records use a 568 B stride; the trailer fields
/// decoded below still live in the first 177 B after the core.
pub const STANDARD_RECORD_SINGLETON_STRIDE: usize = 292;

/// First byte past the fixed core (`+0x73`).
pub const STANDARD_TRAILER_START: usize = 0x73;

/// Absolute record offset of the validated ElementId u32 (RE-15).
pub const TRAILER_ELEMENT_ID_OFFSET: usize = 0x10e;

/// Absolute record offset of the ElementId duplicate / echo u32.
pub const TRAILER_ELEMENT_ID_DUP_OFFSET: usize = 0x11c;

/// Absolute record offset of the shared type-symbol u32 candidate.
pub const TRAILER_TYPE_ID_OFFSET: usize = 0xfe;

/// Absolute record offset of the base-elevation f64 (matches start Z).
pub const TRAILER_BASE_ELEVATION_OFFSET: usize = 0xf6;

/// End of the last trailer field we decode (`+0x11c` + 4).
pub const STANDARD_TRAILER_DECODE_END: usize = TRAILER_ELEMENT_ID_DUP_OFFSET + 4;

/// Revit releases covered by this 2023 standard-variant decoder.
pub const ARC_WALL_STANDARD_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2023];

/// Optional fields recovered from the 177 B singleton trailer
/// (RE-15). Only confidently validated slots are exposed — thickness
/// and Level ElementId are **not** present as stable trailer fields
/// on the Einhoven 2023 corpus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcWallTrailer {
    /// ElementId at `+0x10e`, accepted only when it equals the echo
    /// at `+0x11c` and is not a null/max sentinel. 23/24 Einhoven
    /// standard walls validate; all 23 hit `Global/ElemTable`.
    pub element_id: Option<u32>,
    /// Shared u32 at `+0xfe`. Constant `0x217a` across all 24 Einhoven
    /// standard walls and present in ElemTable — interpreted as a
    /// WallType / symbol handle candidate (not yet joined to width).
    pub type_id: Option<u32>,
    /// f64 at `+0xf6`. Matches the core start-point Z on every
    /// observed record — the wall's base elevation in feet.
    pub base_elevation_feet: Option<f64>,
}

/// Status for a version-scoped standard ArcWall scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcWallScanStatus {
    /// The caller supplied a Revit release covered by this decoder.
    Supported { revit_version: u32 },
    /// The caller supplied a release whose ArcWall record envelope is
    /// known or assumed to differ from the 2023 standard layout.
    UnsupportedVersion { revit_version: u32 },
}

impl ArcWallScanStatus {
    /// Human-readable diagnostic suitable for CLI/export metadata.
    pub fn diagnostic_message(self) -> Option<String> {
        match self {
            ArcWallScanStatus::Supported { .. } => None,
            ArcWallScanStatus::UnsupportedVersion { revit_version } => Some(format!(
                "ArcWall standard decoder skipped: Revit {revit_version} is outside supported versions {ARC_WALL_STANDARD_SUPPORTED_REVIT_VERSIONS:?}"
            )),
        }
    }

    /// True when the scan may apply the 2023 standard record pattern.
    pub fn is_supported(self) -> bool {
        matches!(self, ArcWallScanStatus::Supported { .. })
    }
}

/// Result of a bounded ArcWall byte scan ([`ArcWallRecord::find_all_with_limits`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcWallFindResult {
    /// Byte offsets of candidate standard ArcWall records.
    pub offsets: Vec<usize>,
    /// Number of input bytes actually inspected (may be capped).
    pub scanned_bytes: usize,
    /// Present when a [`WalkerLimits`] cap stopped or truncated the scan.
    pub limit_hit: Option<WalkerLimitHit>,
}

/// Result of a version-scoped ArcWall scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcWallScanReport {
    /// Whether the scan ran or was suppressed by the release guard.
    pub status: ArcWallScanStatus,
    /// Byte offsets where [`ArcWallRecord::decode_standard`] succeeds.
    pub offsets: Vec<usize>,
    /// Number of input bytes actually inspected (may be capped).
    pub scanned_bytes: usize,
    /// Present when a [`WalkerLimits`] cap stopped or truncated the scan.
    pub limit_hit: Option<WalkerLimitHit>,
}

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
    /// Returns the version-scope status for this standard decoder.
    pub fn standard_decoder_status(revit_version: u32) -> ArcWallScanStatus {
        if ARC_WALL_STANDARD_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
            ArcWallScanStatus::Supported { revit_version }
        } else {
            ArcWallScanStatus::UnsupportedVersion { revit_version }
        }
    }

    /// True when this decoder covers the supplied Revit release.
    pub fn supports_revit_version(revit_version: u32) -> bool {
        Self::standard_decoder_status(revit_version).is_supported()
    }

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
        let record_end = offset
            .checked_add(STANDARD_RECORD_MIN_SIZE)
            .ok_or_else(|| {
                Error::Cfb(format!(
                    "ArcWall decode: offset overflow at offset {offset}"
                ))
            })?;
        if record_end > buf.len() {
            return Err(Error::Cfb(format!(
                "ArcWall decode: buffer too short ({} < {} at offset {})",
                buf.len(),
                record_end,
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
        Self::find_all_with_limits(buf, WalkerLimits::default()).offsets
    }

    /// Bounded variant of [`Self::find_all`] for production scanning.
    ///
    /// Caps the scanned byte window at [`WalkerLimits::max_scan_bytes`]
    /// and stops after [`WalkerLimits::max_candidates`] hits so a
    /// hostile partition stream cannot force unbounded candidate
    /// materialisation.
    pub fn find_all_with_limits(buf: &[u8], limits: WalkerLimits) -> ArcWallFindResult {
        let mut out = Vec::new();
        let mut limit_hit = None;
        let scan_len = buf.len().min(limits.max_scan_bytes);
        if buf.len() > limits.max_scan_bytes {
            limit_hit = Some(WalkerLimitHit::MaxScanBytes);
        }
        if scan_len < STANDARD_RECORD_MIN_SIZE {
            return ArcWallFindResult {
                offsets: out,
                scanned_bytes: scan_len,
                limit_hit,
            };
        }
        for i in 0..=(scan_len - STANDARD_RECORD_MIN_SIZE) {
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
            if out.len() >= limits.max_candidates {
                limit_hit.get_or_insert(WalkerLimitHit::MaxCandidates);
                break;
            }
        }
        ArcWallFindResult {
            offsets: out,
            scanned_bytes: scan_len,
            limit_hit,
        }
    }

    /// Version-gated scanner for production use.
    ///
    /// Unsupported Revit releases return an empty offset list even if
    /// the byte buffer happens to contain the 2023 tag/variant pattern.
    /// This prevents false-positive IFC walls on releases whose ArcWall
    /// record envelope has not been proven compatible.
    pub fn scan_standard_for_revit_version(revit_version: u32, buf: &[u8]) -> ArcWallScanReport {
        Self::scan_standard_for_revit_version_with_limits(
            revit_version,
            buf,
            WalkerLimits::default(),
        )
    }

    /// Same as [`Self::scan_standard_for_revit_version`], with explicit
    /// [`WalkerLimits`] applied to the underlying byte scan.
    pub fn scan_standard_for_revit_version_with_limits(
        revit_version: u32,
        buf: &[u8],
        limits: WalkerLimits,
    ) -> ArcWallScanReport {
        let status = Self::standard_decoder_status(revit_version);
        if !status.is_supported() {
            return ArcWallScanReport {
                status,
                offsets: Vec::new(),
                scanned_bytes: 0,
                limit_hit: None,
            };
        }
        let found = Self::find_all_with_limits(buf, limits);
        ArcWallScanReport {
            status,
            offsets: found.offsets,
            scanned_bytes: found.scanned_bytes,
            limit_hit: found.limit_hit,
        }
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

    /// Unconnected wall height from the core Z delta when both Z
    /// values are finite and the absolute delta is at least 0.1 ft.
    ///
    /// On Einhoven every standard wall carries a 2 m (≈6.562 ft) Z
    /// rise; there is no separate height f64 in the singleton trailer.
    pub fn height_feet(&self) -> Option<f64> {
        let (_, _, sz) = self.start_point();
        let (_, _, ez) = self.end_point();
        if !sz.is_finite() || !ez.is_finite() {
            return None;
        }
        let height = (ez - sz).abs();
        (height >= 0.1).then_some(height)
    }

    /// Decode optional singleton-trailer fields starting at the same
    /// record offset used by [`Self::decode_standard`].
    ///
    /// Returns `None` when the buffer does not cover
    /// [`STANDARD_TRAILER_DECODE_END`]. Partial validation is allowed:
    /// individual fields may still be `None` inside a present trailer.
    pub fn decode_trailer(buf: &[u8], offset: usize) -> Option<ArcWallTrailer> {
        let end = offset.checked_add(STANDARD_TRAILER_DECODE_END)?;
        if end > buf.len() {
            return None;
        }

        let element_id_raw = u32::from_le_bytes([
            buf[offset + TRAILER_ELEMENT_ID_OFFSET],
            buf[offset + TRAILER_ELEMENT_ID_OFFSET + 1],
            buf[offset + TRAILER_ELEMENT_ID_OFFSET + 2],
            buf[offset + TRAILER_ELEMENT_ID_OFFSET + 3],
        ]);
        let element_id_dup = u32::from_le_bytes([
            buf[offset + TRAILER_ELEMENT_ID_DUP_OFFSET],
            buf[offset + TRAILER_ELEMENT_ID_DUP_OFFSET + 1],
            buf[offset + TRAILER_ELEMENT_ID_DUP_OFFSET + 2],
            buf[offset + TRAILER_ELEMENT_ID_DUP_OFFSET + 3],
        ]);
        let element_id =
            (element_id_raw == element_id_dup && element_id_raw != 0 && element_id_raw != u32::MAX)
                .then_some(element_id_raw);

        let type_id_raw = u32::from_le_bytes([
            buf[offset + TRAILER_TYPE_ID_OFFSET],
            buf[offset + TRAILER_TYPE_ID_OFFSET + 1],
            buf[offset + TRAILER_TYPE_ID_OFFSET + 2],
            buf[offset + TRAILER_TYPE_ID_OFFSET + 3],
        ]);
        let type_id = (type_id_raw != 0 && type_id_raw != u32::MAX).then_some(type_id_raw);

        let base_elevation = f64::from_le_bytes([
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET],
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET + 1],
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET + 2],
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET + 3],
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET + 4],
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET + 5],
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET + 6],
            buf[offset + TRAILER_BASE_ELEVATION_OFFSET + 7],
        ]);
        let base_elevation_feet = base_elevation.is_finite().then_some(base_elevation);

        Some(ArcWallTrailer {
            element_id,
            type_id,
            base_elevation_feet,
        })
    }
}
