//! A duct's or pipe's size and type, from its curve object (RE-134, #96, #35).
//!
//! A duct is an `RbsDuctCurve` and a pipe an `RbsPipeCurve`, both `RbsCurve`s,
//! whose first serialized field, `m_pConnectorManager`, is written as the tag
//! of `RbsCurveConnectorManager`, `0xFF` × 4 and the next tag: the anchor.
//! The schema's next fields follow it:
//!
//! ```text
//! +8    f64   m_dWidthOrDiameter
//! +16   f64   m_dHeight
//! +64   u64   m_idType
//! ```
//!
//! The anchors are one per curve, and a curve's connector entries (RE-131,
//! [`crate::partition_pipe_axes`]) follow its anchor within [`ANCHOR_WINDOW`]
//! bytes: an anchor is the element's whose box alone holds the points of the
//! first two entries after it. A neighbour joined at one end holds only that
//! end, so the first entry in the stream that falls in a box is not enough. On
//! the RE1 Mechanical and Plumbing models (Revit 2025, MIT), against Revit's
//! own IFC: every duct and pipe has one (25, 6 and 63); the type id is
//! Revit's type `Tag` on all 94; a duct's width and height are two of the
//! three dimensions of Revit's extrusion on 25 of 25, and a pipe's are equal.
//! A round duct is stored as width = height = diameter, as a pipe is, so the
//! fields do not tell a round duct from a square one.

use crate::{Result, RevitFile};
use std::collections::BTreeMap;

/// Releases where this layout is measured: the RE1 models.
pub const CURVE_FIELDS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2025];

/// Whether `revit_version` is a release this layout is measured on.
pub fn supports_revit_version(revit_version: u32) -> bool {
    CURVE_FIELDS_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

/// Bytes after an anchor its curve's connector entries are looked for in. The
/// first lies 1,253 to 1,324 bytes after the anchor on RE1.
pub const ANCHOR_WINDOW: usize = 2_000;

/// Offset of `m_dWidthOrDiameter` from the anchor.
pub const WIDTH_OFFSET: usize = 8;
/// Offset of `m_dHeight` from the anchor.
pub const HEIGHT_OFFSET: usize = 16;
/// Offset of `m_idType` from the anchor.
pub const TYPE_OFFSET: usize = 64;

/// How far outside an element's box a connector entry's point may fall, feet.
const POINT_TOLERANCE_FEET: f64 = 1e-3;
/// Largest connector index an entry is taken to have.
const MAX_CONNECTOR_INDEX: u32 = 255;

/// The fields of one duct's or pipe's curve object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveFields {
    /// `m_dWidthOrDiameter`, feet.
    pub width_feet: f64,
    /// `m_dHeight`, feet.
    pub height_feet: f64,
    /// `m_idType`: the ElementId of the element's type.
    pub type_id: u32,
}

impl CurveFields {
    /// The element's length along its axis, from its record box `[min x, min
    /// y, min z, max x, max y, max z]`: the one dimension of the box left
    /// when the width and the height are matched to the other two, each to
    /// `tolerance` feet. `None` when no dimension is left that way, or when
    /// two are and their lengths differ. Only an element along one of the
    /// model's axes has its section in its box like this.
    pub fn length_in_box(&self, bbox: [f64; 6], tolerance: f64) -> Option<f64> {
        let dims = [bbox[3] - bbox[0], bbox[4] - bbox[1], bbox[5] - bbox[2]];
        let near = |a: f64, b: f64| (a - b).abs() <= tolerance;
        let mut length: Option<f64> = None;
        for axis in 0..3 {
            let (a, b) = (dims[(axis + 1) % 3], dims[(axis + 2) % 3]);
            let section = (near(a, self.width_feet) && near(b, self.height_feet))
                || (near(a, self.height_feet) && near(b, self.width_feet));
            if !section {
                continue;
            }
            match length {
                None => length = Some(dims[axis]),
                Some(held) if near(held, dims[axis]) => {}
                Some(_) => return None,
            }
        }
        length.filter(|l| l.is_finite() && *l > 0.0)
    }
}

fn read_f64(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// The point of the connector entry whose marker `u32 1` starts at `at`.
fn entry_point(buf: &[u8], at: usize) -> Option<[f64; 3]> {
    if read_u32(buf, at.checked_sub(4)?)? > MAX_CONNECTOR_INDEX {
        return None;
    }
    let mut point = [0.0; 3];
    for (axis, slot) in point.iter_mut().enumerate() {
        *slot = read_f64(buf, at + 4 + 8 * axis)?;
        if !slot.is_finite() {
            return None;
        }
    }
    Some(point)
}

/// The fields at the anchor at `anchor`, when they are a size and a type.
fn fields_at(buf: &[u8], anchor: usize) -> Option<CurveFields> {
    let width = read_f64(buf, anchor + WIDTH_OFFSET)?;
    let height = read_f64(buf, anchor + HEIGHT_OFFSET)?;
    let type_id = u32::try_from(read_u64(buf, anchor + TYPE_OFFSET)?).ok()?;
    let size = |v: f64| v.is_finite() && v > 0.0 && v < 1_000.0;
    (size(width) && size(height) && type_id != 0).then_some(CurveFields {
        width_feet: width,
        height_feet: height,
        type_id,
    })
}

/// Bytes from the header of a pipe's `RbsPipeCurve` data object to its inner
/// diameter; its outer diameter follows (RE-157).
pub const PIPE_INNER_DIAMETER_OFFSET: usize = 549;
/// Largest diameter, feet, a pipe's two diameters are taken to be under.
const MAX_PIPE_DIAMETER_FEET: f64 = 10.0;

/// Each pipe of `pipes`' inner and outer diameters, feet, by ElementId
/// (RE-157): the two `f64`s at [`PIPE_INNER_DIAMETER_OFFSET`] of the verified
/// data object whose header carries the pipe's id and `RbsPipeCurve`'s tag.
/// On RE1 Mechanical and Plumbing the inner diameter is the one Revit's
/// `InvertElevation` implies on every pipe size. A pipe whose objects
/// disagree, or whose two values are not 0 < inner < outer, is absent. Empty
/// for a release this layout is not measured on.
pub fn scan_pipe_diameters(
    rf: &mut RevitFile,
    revit_version: u32,
    pipes: &std::collections::BTreeSet<u32>,
) -> Result<BTreeMap<u32, (f64, f64)>> {
    if !supports_revit_version(revit_version) || pipes.is_empty() {
        return Ok(BTreeMap::new());
    }
    let classes = rf.schema_classes()?;
    let Some(tag) = classes
        .classes
        .iter()
        .find(|class| class.name == "RbsPipeCurve")
        .map(|class| u32::from(class.tag))
    else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<(f64, f64)>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for &id in pipes {
            let mut header = id.to_le_bytes().to_vec();
            header.extend_from_slice(&0u32.to_le_bytes());
            for p in memchr::memmem::find_iter(buf, header.as_slice()) {
                let Some(object) = crate::partition_room_parameters::verified_data_object(buf, p)
                else {
                    continue;
                };
                if object.class & 0xffff != tag {
                    continue;
                }
                let at = p + PIPE_INNER_DIAMETER_OFFSET;
                let (Some(inner), Some(outer)) = (read_f64(buf, at), read_f64(buf, at + 8)) else {
                    continue;
                };
                if !(inner > 0.0 && inner < outer && outer < MAX_PIPE_DIAMETER_FEET) {
                    continue;
                }
                match found.get(&id) {
                    None => {
                        found.insert(id, Some((inner, outer)));
                    }
                    Some(Some(held)) if *held != (inner, outer) => {
                        found.insert(id, None);
                    }
                    Some(_) => {}
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, diameters)| Some((id, diameters?)))
        .collect())
}

/// The curve fields of each duct and pipe of `curves` (their record boxes, by
/// the partition stream their records are in), by ElementId. An element whose
/// anchor is not found, or whose fields are not a size and a type, is absent;
/// so is one whose ElementId gives different fields in two streams. Empty for
/// a release this layout is not measured on, or a schema with no
/// `RbsCurveConnectorManager`.
pub fn scan_curve_fields(
    rf: &mut RevitFile,
    revit_version: u32,
    curves: &BTreeMap<String, Vec<(u32, [f64; 6])>>,
) -> Result<BTreeMap<u32, CurveFields>> {
    if !supports_revit_version(revit_version) || curves.is_empty() {
        return Ok(BTreeMap::new());
    }
    let classes = rf.schema_classes()?;
    let Some(manager) = classes
        .classes
        .iter()
        .find(|class| class.name == "RbsCurveConnectorManager")
        .map(|class| class.tag)
    else {
        return Ok(BTreeMap::new());
    };
    let mut anchor = manager.to_le_bytes().to_vec();
    anchor.extend_from_slice(&[0xff; 4]);
    anchor.extend_from_slice(&manager.wrapping_add(1).to_le_bytes());
    let mut found: BTreeMap<u32, Option<CurveFields>> = BTreeMap::new();
    for (stream, boxes) in curves {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let anchors: Vec<usize> = memchr::memmem::find_iter(buf, anchor.as_slice()).collect();
        if anchors.is_empty() {
            continue;
        }
        let inside = |point: [f64; 3], bbox: &[f64; 6]| {
            (0..3).all(|axis| {
                point[axis] >= bbox[axis] - POINT_TOLERANCE_FEET
                    && point[axis] <= bbox[axis + 3] + POINT_TOLERANCE_FEET
            })
        };
        // The connector entries whose points lie in some element's box, in
        // the order they are written.
        let entries: Vec<(usize, [f64; 3])> = memchr::memmem::find_iter(buf, &1u32.to_le_bytes())
            .filter_map(|at| Some((at, entry_point(buf, at)?)))
            .filter(|(_, point)| boxes.iter().any(|(_, bbox)| inside(*point, bbox)))
            .collect();
        // An anchor belongs to the one element whose box holds the points of
        // the first two entries after it within the window: a curve's own two
        // ends, of which a neighbour joined at one end holds only that one.
        for &at in &anchors {
            let from = entries.partition_point(|(offset, _)| *offset <= at);
            let points: Vec<[f64; 3]> = entries[from..]
                .iter()
                .take_while(|(offset, _)| offset - at <= ANCHOR_WINDOW)
                .take(2)
                .map(|(_, point)| *point)
                .collect();
            if points.is_empty() {
                continue;
            }
            let mut owners = boxes
                .iter()
                .filter(|(_, bbox)| points.iter().all(|point| inside(*point, bbox)));
            let (Some((id, _)), None) = (owners.next(), owners.next()) else {
                continue;
            };
            let Some(fields) = fields_at(buf, at) else {
                continue;
            };
            match found.get(id) {
                None => {
                    found.insert(*id, Some(fields));
                }
                Some(Some(held)) if *held != fields => {
                    found.insert(*id, None);
                }
                Some(_) => {}
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, fields)| Some((id, fields?)))
        .collect())
}
