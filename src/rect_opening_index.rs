//! Version-gated decoder for Revit 2024 ArcWallRectOpening 60 B index records (RE-15-09).
use crate::{Error, Result};

pub const ARC_WALL_RECT_OPENING_TAG_2024: u16 = 0x01a7;
pub const OPENING_INDEX_STRIDE: usize = 60;
pub const OPENING_INDEX_FAMILY_MARKER: u32 = 0x4008_8204;
pub const OPENING_INDEX_CONST_0546: u32 = 0x0000_0546;
pub const OPENING_INDEX_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArcWallRectOpeningIndex {
    pub tag: u16,
    pub index: u32,
    pub family_marker: u32,
    pub related_id_a: u32,
    pub const_0546: u32,
    pub related_id_b: u32,
}

impl ArcWallRectOpeningIndex {
    pub fn supports_revit_version(revit_version: u32) -> bool {
        OPENING_INDEX_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
    }

    pub fn decode(buf: &[u8], offset: usize) -> Result<Self> {
        let end = offset.checked_add(OPENING_INDEX_STRIDE).ok_or_else(|| {
            Error::Cfb(format!(
                "ArcWallRectOpeningIndex: offset overflow at {offset}"
            ))
        })?;
        if end > buf.len() {
            return Err(Error::Cfb(format!(
                "ArcWallRectOpeningIndex: buffer too short ({} < {end})",
                buf.len()
            )));
        }
        let tag = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        if tag != ARC_WALL_RECT_OPENING_TAG_2024 {
            return Err(Error::Cfb(format!(
                "ArcWallRectOpeningIndex: expected tag 0x{ARC_WALL_RECT_OPENING_TAG_2024:04x}, got 0x{tag:04x}"
            )));
        }
        let pad = u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]);
        if pad != 0 {
            return Err(Error::Cfb(format!(
                "ArcWallRectOpeningIndex: expected pad 0, got 0x{pad:04x}"
            )));
        }
        let family_marker = u32::from_le_bytes([
            buf[offset + 0x10],
            buf[offset + 0x11],
            buf[offset + 0x12],
            buf[offset + 0x13],
        ]);
        if family_marker != OPENING_INDEX_FAMILY_MARKER {
            return Err(Error::Cfb(format!(
                "ArcWallRectOpeningIndex: expected family marker 0x{OPENING_INDEX_FAMILY_MARKER:08x}, got 0x{family_marker:08x}"
            )));
        }
        Ok(Self {
            tag,
            index: u32::from_le_bytes([
                buf[offset + 0x08],
                buf[offset + 0x09],
                buf[offset + 0x0a],
                buf[offset + 0x0b],
            ]),
            family_marker,
            related_id_a: u32::from_le_bytes([
                buf[offset + 0x14],
                buf[offset + 0x15],
                buf[offset + 0x16],
                buf[offset + 0x17],
            ]),
            const_0546: u32::from_le_bytes([
                buf[offset + 0x18],
                buf[offset + 0x19],
                buf[offset + 0x1a],
                buf[offset + 0x1b],
            ]),
            related_id_b: u32::from_le_bytes([
                buf[offset + 0x36],
                buf[offset + 0x37],
                buf[offset + 0x38],
                buf[offset + 0x39],
            ]),
        })
    }

    pub fn find_all_for_revit_version(revit_version: u32, buf: &[u8]) -> Vec<usize> {
        if !Self::supports_revit_version(revit_version) {
            return Vec::new();
        }
        let mut filtered = Vec::new();
        for i in 0..buf.len().saturating_sub(3) {
            let tag = u16::from_le_bytes([buf[i], buf[i + 1]]);
            if tag == ARC_WALL_RECT_OPENING_TAG_2024 && buf[i + 2] == 0 && buf[i + 3] == 0 {
                filtered.push(i);
            }
        }
        let mut out = Vec::new();
        for (idx, &off) in filtered.iter().enumerate() {
            let next_delta = filtered.get(idx + 1).map(|n| n - off);
            let is_stride60 = next_delta == Some(OPENING_INDEX_STRIDE);
            let marker_ok = off + 0x14 <= buf.len()
                && u32::from_le_bytes([
                    buf[off + 0x10],
                    buf[off + 0x11],
                    buf[off + 0x12],
                    buf[off + 0x13],
                ]) == OPENING_INDEX_FAMILY_MARKER;
            if (is_stride60 || marker_ok) && marker_ok && Self::decode(buf, off).is_ok() {
                out.push(off);
            }
        }
        out
    }
}
