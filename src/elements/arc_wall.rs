//! Typed ArcWall partition decoder (Revit 2023 standard variant).
//!
//! ArcWall instance geometry lives in `Partitions/*` with a fixed
//! `(tag, variant)` envelope — see [`crate::arc_wall_record`] — rather
//! than the schema-field wire format used by [`crate::walker::ElementDecoder`].
//! This module is therefore **not** registered in
//! [`crate::elements::all_decoders`]; it sits beside that registry as the
//! typed partition path for Lane Five / M3-05.
//!
//! Downstream lanes (geometry, IFC) should call
//! [`decode_candidate`] / [`decode_at`] rather than inventing a parallel
//! scanner.
//!
//! # Rejection rules
//!
//! | Condition | Error |
//! |---|---|
//! | Candidate `class_name` is `Some` and not `"ArcWall"` | wrong-schema |
//! | Candidate `class_tag` ≠ [`ARC_WALL_TAG`] | wrong-schema |
//! | Buffer fails [`ArcWallRecord::decode_standard`] gates | tag / variant / short |
//! | Optional schema name gate fails | wrong-schema |

use crate::arc_wall_record::{
    ARC_WALL_TAG, ArcWallRecord, ArcWallTrailer, STANDARD_RECORD_MIN_SIZE,
};
use crate::partition_scanner::PartitionRecordCandidate;
use crate::{Error, Result};

/// Typed view of a standard ArcWall recovered from partition bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcWall {
    /// Fixed-size core record (tag, coords, envelope).
    pub record: ArcWallRecord,
    /// Optional singleton trailer fields (ElementId, type id, base Z).
    pub trailer: Option<ArcWallTrailer>,
    /// Convenience: trailer ElementId when validated.
    pub element_id: Option<u32>,
    /// Convenience: trailer WallType / symbol candidate.
    pub type_id: Option<u32>,
    /// Convenience: trailer base elevation in feet.
    pub base_elevation_feet: Option<f64>,
    /// Unconnected height from the core Z delta, when recoverable.
    pub height_feet: Option<f64>,
}

impl ArcWall {
    fn from_parts(record: ArcWallRecord, trailer: Option<ArcWallTrailer>) -> Self {
        let element_id = trailer.and_then(|t| t.element_id);
        let type_id = trailer.and_then(|t| t.type_id);
        let base_elevation_feet = trailer.and_then(|t| t.base_elevation_feet);
        let height_feet = record.height_feet();
        Self {
            record,
            trailer,
            element_id,
            type_id,
            base_elevation_feet,
            height_feet,
        }
    }

    /// Wall centerline start point (ft) under RE-14.3 H16.
    pub fn start_point(&self) -> (f64, f64, f64) {
        self.record.start_point()
    }

    /// Wall centerline end point (ft) under RE-14.3 H16.
    pub fn end_point(&self) -> (f64, f64, f64) {
        self.record.end_point()
    }
}

/// Reject a class name that is present but not ArcWall.
fn reject_wrong_class(class_name: Option<&str>) -> Result<()> {
    match class_name {
        None | Some("ArcWall") => Ok(()),
        Some(other) => Err(Error::BasicFileInfo(format!(
            "ArcWallDecoder received wrong schema: {other}"
        ))),
    }
}

/// Decode a standard ArcWall at `offset` inside a decompressed partition
/// buffer. When `expected_class` is `Some`, it must be `"ArcWall"`.
pub fn decode_at(buf: &[u8], offset: usize, expected_class: Option<&str>) -> Result<ArcWall> {
    reject_wrong_class(expected_class)?;
    if buf.len().saturating_sub(offset) < STANDARD_RECORD_MIN_SIZE {
        return Err(Error::Cfb(format!(
            "ArcWall decode: buffer too short ({} < {} at offset {})",
            buf.len().saturating_sub(offset),
            STANDARD_RECORD_MIN_SIZE,
            offset
        )));
    }
    let record = ArcWallRecord::decode_standard(buf, offset)?;
    let trailer = ArcWallRecord::decode_trailer(buf, offset);
    Ok(ArcWall::from_parts(record, trailer))
}

/// Decode from a generic partition scanner candidate.
///
/// Rejects when the candidate names a different class or carries a
/// non-ArcWall tag, even if the bytes at `offset` happen to look like
/// an ArcWall envelope.
pub fn decode_candidate(candidate: &PartitionRecordCandidate, buf: &[u8]) -> Result<ArcWall> {
    reject_wrong_class(candidate.class_name.as_deref())?;
    if candidate.class_tag != ARC_WALL_TAG {
        return Err(Error::BasicFileInfo(format!(
            "ArcWallDecoder received wrong schema tag: 0x{:04x} (expected 0x{ARC_WALL_TAG:04x})",
            candidate.class_tag
        )));
    }
    decode_at(buf, candidate.offset, candidate.class_name.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partition_scanner::{PartitionEnvelope, PartitionRecordCandidate};

    /// Same embedded fixture as `arc_wall_record` record #4 (Einhoven).
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

    fn candidate(
        class_name: Option<&str>,
        class_tag: u16,
        offset: usize,
    ) -> PartitionRecordCandidate {
        PartitionRecordCandidate {
            stream: "Partitions/5".into(),
            chunk_index: None,
            offset,
            class_tag,
            class_name: class_name.map(str::to_string),
            envelope: PartitionEnvelope {
                filter_pad: 0,
                fixed_header_0: None,
                count_version: None,
                type_code: None,
                variant_marker: None,
            },
            confidence: 0.85,
            consumed_start: offset,
            consumed_end: offset + STANDARD_RECORD_MIN_SIZE,
            excerpt_hash: "00".into(),
            element_id: None,
        }
    }

    #[test]
    fn decode_at_happy_path() {
        let wall = decode_at(RECORD_4_HEX, 0, Some("ArcWall")).expect("decode");
        assert_eq!(wall.record.tag, ARC_WALL_TAG);
        assert!(wall.height_feet.is_some());
        let (sx, _, _) = wall.start_point();
        assert!(sx > 0.0);
    }

    #[test]
    fn rejects_wrong_schema_name() {
        let err = decode_at(RECORD_4_HEX, 0, Some("Wall")).unwrap_err();
        assert!(err.to_string().contains("wrong schema"), "err={err}");
    }

    #[test]
    fn rejects_short_buffer() {
        let err = decode_at(&RECORD_4_HEX[..20], 0, None).unwrap_err();
        assert!(err.to_string().contains("too short"), "err={err}");
    }

    #[test]
    fn decode_candidate_rejects_wrong_class_name() {
        let c = candidate(Some("Floor"), ARC_WALL_TAG, 0);
        let err = decode_candidate(&c, RECORD_4_HEX).unwrap_err();
        assert!(err.to_string().contains("wrong schema"), "err={err}");
    }

    #[test]
    fn decode_candidate_rejects_wrong_tag() {
        let c = candidate(Some("ArcWall"), 0x0001, 0);
        let err = decode_candidate(&c, RECORD_4_HEX).unwrap_err();
        assert!(err.to_string().contains("wrong schema tag"), "err={err}");
    }

    #[test]
    fn decode_candidate_accepts_matching() {
        let c = candidate(Some("ArcWall"), ARC_WALL_TAG, 0);
        let wall = decode_candidate(&c, RECORD_4_HEX).expect("decode");
        assert_eq!(wall.record.tag, ARC_WALL_TAG);
    }

    #[test]
    fn decode_candidate_allows_unnamed_matching_tag() {
        // Scanner may leave class_name unset when the schema lacks the tag.
        let c = candidate(None, ARC_WALL_TAG, 0);
        let wall = decode_candidate(&c, RECORD_4_HEX).expect("decode");
        assert_eq!(wall.record.tag, ARC_WALL_TAG);
    }
}
