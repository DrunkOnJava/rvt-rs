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
//!       +0x0b past the flags (2024; on 2025 the plane's raise)
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
//!
//! Revit 2025 writes the same block (RE-199), but +0x3b is not the
//! rise/run there: it is how far the edge's plane is raised at the edge. On
//! the roofs of `R2025_Models.zip` (#356: sheds, a gable, hips, L-shaped hips)
//! every defining edge and its angle match Revit's own IFC export.

use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Releases this layout is measured on.
pub const ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// How far into a sketch line's data its slope block is looked for.
pub const EDGE_SLOPE_WINDOW: usize = 0x1000;

/// One roof edge's slope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeSlope {
    /// Whether the edge defines the roof's slope.
    pub defines_slope: bool,
    /// The edge's slope angle from horizontal, radians.
    pub angle_radians: f64,
    /// How far the edge's plane is raised at the edge, feet: the f64 at
    /// +0x3b on 2025 (RE-199), 0 on 2024, where that f64 is −rise/run.
    pub plate_offset_feet: f64,
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|s| f64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

/// The slope block of a roof sketch line, from its data, as Revit 2024
/// writes it. `None` when no block of this shape is found, or when an edge
/// that defines the slope does not also carry its rise over run.
pub fn edge_slope(data: &[u8]) -> Option<EdgeSlope> {
    edge_slope_in(data, 2024)
}

/// [`edge_slope`] as `revit_version` writes the block. On 2025 the f64 at
/// +0x3b is not the rise over run but how far the edge's plane is raised
/// (RE-199): 0.0 on most edges; 0.17713 ft (54 mm) on `03_2Slope_Rafter2SquareCut`
/// (#356's `R2025_Models.zip`), whose off-centre ridge it reproduces, with
/// 0.10827 ft (the edge's 33 mm offset from its wall) at +0x47; 0.98425 ft
/// on `09_LShapedHipRoof_1Edge300Offset`'s 300 mm edge.
pub fn edge_slope_in(data: &[u8], revit_version: u32) -> Option<EdgeSlope> {
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
            let recorded = revit_version == 2025 || (rise_over_run + angle.tan()).abs() <= 1e-9;
            if !recorded {
                return None;
            }
        }
        let plate_offset_feet = match revit_version {
            2025 => f64_at(data, at + 0x3b).filter(|v| v.is_finite())?,
            _ => 0.0,
        };
        Some(EdgeSlope {
            defines_slope,
            angle_radians: angle,
            plate_offset_feet,
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
            let Some(slope) = buf
                .get(id_at + 8..end)
                .and_then(|data| edge_slope_in(data, revit_version))
            else {
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

/// A slope arrow's heights (RE-199): the `SlopeCurveData` an arrow sketch
/// line carries, in its members' order.
///
/// ```text
/// -0x28  f64      height offset at the tail (m_startHeight), feet
/// -0x20  f64      height offset at the head (m_endHeight), feet
/// -0x18  f64 × 2  0.0, 0.0 (the tail's and head's level elevations)
/// -0x08  f64      slope angle, radians (m_angle)
///  0x00  ff × 16  the tail's and head's levels, none
/// +0x10  u32      m_useAngle: 0 set by "Height at Tail", 1 by "Slope"
/// ```
///
/// On `05_1Slope_ArrowDefined` (#356's `R2025_Models.zip`; "Height at Tail",
/// 22 and 3000 mm) the roof rises from the tail's height to the head's along
/// the arrow; on `06_1Slope_ArrowDefined_SetToSlope` (the same arrow set by
/// "Slope", `m_useAngle` 1) it rises at the angle from the tail's height,
/// and the head's height is unused. Each
/// reproduces its record box. `None` unless the word at +0x10 is 0 or 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowHeights {
    /// Height offset at the tail, feet.
    pub tail_feet: f64,
    /// Height offset at the head, feet.
    pub head_feet: f64,
    /// The arrow's slope angle, radians.
    pub angle_radians: f64,
    /// Whether the arrow is set by its angle (`m_useAngle`) rather than by
    /// its two heights.
    pub by_angle: bool,
}

impl ArrowHeights {
    /// The rise over run along the arrow, `run` its length in feet.
    pub fn rise_over_run(&self, run: f64) -> f64 {
        if self.by_angle {
            self.angle_radians.tan()
        } else {
            (self.head_feet - self.tail_feet) / run
        }
    }
}

/// The [`ArrowHeights`] in a slope arrow's data, from the first block of
/// this shape: a slope angle followed by 16 `ff`, not 32 (an edge's RE-56
/// block), then `u32 0` or `u32 1`.
pub fn arrow_heights(data: &[u8]) -> Option<ArrowHeights> {
    const RUN: [u8; 16] = [0xff; 16];
    memchr::memmem::find_iter(data, &RUN).find_map(|at| {
        let by_angle = match data.get(at + 0x10..at + 0x14)? {
            [0, 0, 0, 0] => false,
            [1, 0, 0, 0] => true,
            _ => return None,
        };
        let base = at.checked_sub(0x28)?;
        let angle = f64_at(data, base + 0x20)?;
        if !(angle.is_finite() && angle > 0.0 && angle < std::f64::consts::FRAC_PI_2) {
            return None;
        }
        let levels = [f64_at(data, base + 0x10)?, f64_at(data, base + 0x18)?];
        if levels != [0.0, 0.0] {
            return None;
        }
        let (tail_feet, head_feet) = (f64_at(data, base)?, f64_at(data, base + 0x08)?);
        (tail_feet.is_finite() && head_feet.is_finite() && (by_angle || head_feet != tail_feet))
            .then_some(ArrowHeights {
                tail_feet,
                head_feet,
                angle_radians: angle,
                by_angle,
            })
    })
}

/// Each listed sketch line's [`ArrowHeights`], read as [`scan_edge_slopes`]
/// reads edges; a line whose copies disagree is dropped.
pub fn scan_arrow_heights(
    rf: &mut RevitFile,
    revit_version: u32,
    lines: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, ArrowHeights>> {
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) => header,
        _ => return Ok(BTreeMap::new()),
    };
    let mut found: BTreeMap<u32, Option<ArrowHeights>> = BTreeMap::new();
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
            let Some(heights) = buf.get(id_at + 8..end).and_then(arrow_heights) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(heights));
                }
                Some(held) => {
                    if *held != Some(heights) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, heights)| heights.map(|h| (id, h)))
        .collect())
}

/// The parameter id the roof's fascia depth precedes in its element data
/// (RE-199).
pub const FASCIA_DEPTH_NEXT_ID: i64 = -1_001_711;

/// A footprint roof's fascia depth, feet (RE-199): in the roof's own
/// element data, the f64 just before the first `i64` [`FASCIA_DEPTH_NEXT_ID`].
/// On the roofs of #356's `R2025_Models.zip` it is 0.0 on every roof but
/// `07_1Slope_ArrowDefined_2Cut50mmFascia`, where it is 0.164042 ft
/// (50 mm); a two-cut rafter's plumb face at the eave is that tall.
pub fn fascia_depth(data: &[u8]) -> Option<f64> {
    let at = memchr::memmem::find(data, &FASCIA_DEPTH_NEXT_ID.to_le_bytes())?;
    f64_at(data, at.checked_sub(8)?).filter(|v| v.is_finite() && (0.0..100.0).contains(v))
}

/// Each roof's [`fascia_depth`], by ElementId, from its element data; a
/// roof whose copies disagree is dropped. Empty on a release this layout is
/// not measured on (2025 only).
pub fn scan_fascia_depths(
    rf: &mut RevitFile,
    revit_version: u32,
    roofs: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, f64>> {
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if revit_version == 2025 => header,
        _ => return Ok(BTreeMap::new()),
    };
    let mut found: BTreeMap<u32, Option<f64>> = BTreeMap::new();
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
                .filter(|id| roofs.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(EDGE_SLOPE_WINDOW))
                .min(buf.len());
            let Some(depth) = buf.get(id_at + 8..end).and_then(fascia_depth) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(depth));
                }
                Some(held) => {
                    if *held != Some(depth) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, depth)| depth.map(|d| (id, d)))
        .collect())
}

/// A footprint roof's "Rafter Cut" (RE-199).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RafterCut {
    /// "Plumb Cut", the default.
    Plumb,
    /// "Two Cut - Plumb".
    TwoCutPlumb,
    /// "Two Cut - Square".
    TwoCutSquare,
}

impl RafterCut {
    /// The name [`RAFTER_CUT_FIELD`](crate::partition_schema_mvp::ROOF_RAFTER_CUT_FIELD)
    /// records.
    pub fn name(self) -> &'static str {
        match self {
            Self::Plumb => "plumb",
            Self::TwoCutPlumb => "two_cut_plumb",
            Self::TwoCutSquare => "two_cut_square",
        }
    }

    /// The cut a [`RafterCut::name`] names.
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Plumb, Self::TwoCutPlumb, Self::TwoCutSquare]
            .into_iter()
            .find(|cut| cut.name() == name)
    }
}

/// The parameter in a `ProfileRoof`'s `m_paramSet` that holds its rafter
/// cut (RE-199).
pub const RAFTER_CUT_PARAMETER: i64 = -1_001_710;

/// The rafter cut a [`RAFTER_CUT_PARAMETER`] value names. On the 13
/// footprint roofs of #356's `R2025_Models.zip`: 33615 on the eight with the
/// default plumb cut, 33619 on the two two-cut plumb, 33618 on the three
/// two-cut square. `02_1Slope`, `02_1Slope_Rafter2PlumbCut` and
/// `02_1Slope_Rafter2SquareCut` are one roof with each cut.
pub fn rafter_cut_from_value(value: i64) -> Option<RafterCut> {
    match value {
        33615 => Some(RafterCut::Plumb),
        33619 => Some(RafterCut::TwoCutPlumb),
        33618 => Some(RafterCut::TwoCutSquare),
        _ => None,
    }
}

/// Each of `roofs` that is a `ProfileRoof` (a footprint roof) with a known
/// [`RAFTER_CUT_PARAMETER`], read from its native record's object graph,
/// which needs no parameter definitions
/// ([`crate::native_document::extract_graphs`]). Empty where the native
/// record path does not read the file, or on a release this value is not
/// measured on (2025 only).
pub fn scan_rafter_cuts(
    rf: &mut RevitFile,
    revit_version: u32,
    roofs: &BTreeSet<u32>,
) -> BTreeMap<u32, RafterCut> {
    let mut found = BTreeMap::new();
    if revit_version != 2025 || roofs.is_empty() {
        return found;
    }
    let options = crate::native_document::Options {
        selected_ids: roofs.iter().map(|&id| u64::from(id)).collect(),
        ..crate::native_document::Options::default()
    };
    let id64 = |value: &serde_json::Value| value.get("m_id")?.get("m_id64")?.as_i64();
    let _ = crate::native_document::extract_graphs(rf, &options, |record| {
        if record.class_name.as_deref() != Some("ProfileRoof") {
            return Ok(());
        }
        let Ok(id) = u32::try_from(record.identity.element_id) else {
            return Ok(());
        };
        let cut = record
            .graph
            .iter()
            .flat_map(|graph| graph.objects.iter())
            .filter_map(|object| object.fields.get("m_paramSet")?.as_array())
            .flatten()
            .find(|pair| pair.get("m_paramId").and_then(id64) == Some(RAFTER_CUT_PARAMETER))
            .and_then(|pair| pair.get("m_value")?.as_i64())
            .and_then(rafter_cut_from_value);
        if let Some(cut) = cut {
            found.insert(id, cut);
        }
        Ok(())
    });
    found
}
