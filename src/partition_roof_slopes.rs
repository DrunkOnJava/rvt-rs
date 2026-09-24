//! Roof edges' slopes (RE-56).
//!
//! Each sketch line of a footprint roof carries, in its element data, the
//! slope Revit's roof tools set per edge. After the edge's slope angle come
//! 16 zero bytes and 32 `ff` bytes, then:
//!
//! ```text
//! f64   slope angle, radians (9:12 when the edge sets no slope)
//! 00 × 16
//! ff × 32
//! i64   a small negative value (−9, −6 on the files measured)
//! u32   0 or 1
//! u32   7
//! u8    1 when the edge defines the roof's slope
//! u8    1 when the edge defines the roof's slope
//! u8    1
//! u8    0
//! ...   on an edge that defines the slope, the slope as −rise/run at
//!       +0x0b past the flags
//! ```
//!
//! On the Autodesk tutorial house (Revit 2024) three roofs have no edge
//! that defines a slope and one has exactly one, at 1:12. That shed roof,
//! rising from its defining edge across its outline at 1:12 with its type's
//! 1.25 ft thickness measured square to the slope, reproduces its record
//! box's height, 2.959193880 ft, to 1e-9 ft. No edge of Snowdon Towers'
//! roofs defines a slope. One of them, 2112673, is sloped in Revit's own
//! export all the same: a roof's slope can be set some other way, which is
//! not read, so a roof with no defining edge is not known to be flat.

use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Releases this layout is measured on.
pub const ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// How far into a sketch line's data its slope block is looked for.
pub const EDGE_SLOPE_WINDOW: usize = 0x1000;

/// One roof edge's slope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeSlope {
    /// Whether the edge defines the roof's slope.
    pub defines_slope: bool,
    /// The edge's slope angle from horizontal, radians.
    pub angle_radians: f64,
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|s| f64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

/// The slope block of a roof sketch line, from its data. `None` when no
/// block of this shape is found, or when an edge that defines the slope
/// does not also carry its rise over run.
pub fn edge_slope(data: &[u8]) -> Option<EdgeSlope> {
    const RUN: [u8; 32] = [0xff; 32];
    memchr::memmem::find_iter(data, &RUN).find_map(|at| {
        let zeros = data.get(at.checked_sub(0x10)?..at)?;
        if zeros.iter().any(|&b| b != 0) {
            return None;
        }
        let angle = f64_at(data, at.checked_sub(0x18)?)?;
        if !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::FRAC_PI_2) {
            return None;
        }
        let small = i64::from_le_bytes(data.get(at + 0x20..at + 0x28)?.try_into().ok()?);
        if !(-64..=-1).contains(&small) || u32_at(data, at + 0x2c)? != 7 {
            return None;
        }
        let flags = data.get(at + 0x30..at + 0x34)?;
        let defines_slope = match flags {
            [0, 0, 1, 0] => false,
            [1, 1, 1, 0] => true,
            _ => return None,
        };
        if defines_slope {
            let rise_over_run = f64_at(data, at + 0x3b)?;
            if (rise_over_run + angle.tan()).abs() > 1e-9 {
                return None;
            }
        }
        Some(EdgeSlope {
            defines_slope,
            angle_radians: angle,
        })
    })
}

/// Each sketch line's slope, by ElementId, from its own data; a line whose
/// copies disagree is dropped. Empty on a release this layout is not
/// measured on.
pub fn scan_edge_slopes(
    rf: &mut RevitFile,
    revit_version: u32,
    lines: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, EdgeSlope>> {
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) => header,
        _ => return Ok(BTreeMap::new()),
    };
    let mut found: BTreeMap<u32, Option<EdgeSlope>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = buf
                .get(id_at..id_at + 8)
                .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| lines.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(EDGE_SLOPE_WINDOW))
                .min(buf.len());
            let Some(slope) = buf.get(id_at + 8..end).and_then(edge_slope) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(slope));
                }
                Some(held) => {
                    if *held != Some(slope) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, slope)| slope.map(|s| (id, s)))
        .collect())
}
