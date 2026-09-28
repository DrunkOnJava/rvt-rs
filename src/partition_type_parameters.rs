//! Text parameters of element types, stored under their BuiltInParameter
//! (#35, RE-77).
//!
//! An element's parameter values sit in value blocks that open with the
//! element's ElementId followed by 56 bytes of `0xff` and 3 zero bytes
//! (Discussion #112). Within a type's block, some text parameters are
//! stored as `i64 BuiltInParameter · u32 n · UTF-16 × n`. An entry belongs to
//! the block it follows, up to the next block, and the block's ElementId
//! must be declared in `Global/ElemTable`. The partition record chain
//! (RE-35) cannot attribute these: it covers only each partition's leading
//! records, while most type records sit in the loaded families' documents
//! after it (4% of Snowdon Towers' inflated partitions are in the chain).
//!
//! Measured on Autodesk's Snowdon Towers (Revit 2024) against the VIM export
//! of its 2027 edition, per type that both hold (see
//! `reports/element-framing/RE-77-type-text-parameters.md`). Instance
//! parameters such as Mark and Comments are not stored this way and are not
//! read. A type whose blocks carry two different values for one parameter
//! gets neither (fail closed).
//!
//! A type's length parameters sit in the same blocks as `f64 feet · ff × 8 ·
//! i64 parameter` (RE-93): a BuiltInParameter's negative id, or the declared
//! ElementId of a family parameter. [`type_window_openings`] and
//! [`type_door_openings`] read the values a window's and a door's opening
//! are built from, and [`type_i_sections`] a steel framing type's I section
//! (RE-103).

use crate::RevitFile;
use crate::partition_element_records::bbox_marker;
use std::collections::{BTreeMap, BTreeSet};

/// The type parameters read, by BuiltInParameter id, with the name Revit
/// shows for each.
pub const TYPE_TEXT_PARAMETERS: &[(i64, &str)] = &[
    (-1001405, "Type Mark"),
    (-1010103, "Description"),
    (-1001206, "Fire Rating"),
];

/// Longest value read, in UTF-16 units.
const MAX_VALUE_UNITS: usize = 400;

/// The bytes that follow an ElementId to open its value block.
const VALUE_BLOCK_MARK: [u8; 59] = {
    let mut mark = [0xffu8; 59];
    mark[56] = 0;
    mark[57] = 0;
    mark[58] = 0;
    mark
};

/// Every type's text parameters in one inflated partition, by the declared
/// ElementId of the value block each entry follows.
pub fn scan_partition(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
) -> BTreeMap<u32, BTreeMap<&'static str, String>> {
    let blocks = value_blocks(buf, declared_ids);
    let mut values: BTreeMap<(u32, &'static str), Option<String>> = BTreeMap::new();
    for (parameter, name) in TYPE_TEXT_PARAMETERS {
        for at in memchr::memmem::find_iter(buf, &parameter.to_le_bytes()) {
            let Some(text) = text_at(buf, at + 8) else {
                continue;
            };
            let index = blocks.partition_point(|(start, _)| *start < at);
            let Some(&(_, owner)) = index.checked_sub(1).and_then(|i| blocks.get(i)) else {
                continue;
            };
            values
                .entry((owner, name))
                .and_modify(|seen| {
                    if seen.as_deref() != Some(text.as_str()) {
                        *seen = None;
                    }
                })
                .or_insert(Some(text));
        }
    }
    let mut out: BTreeMap<u32, BTreeMap<&'static str, String>> = BTreeMap::new();
    for ((owner, name), value) in values {
        if let Some(value) = value {
            out.entry(owner).or_default().insert(name, value);
        }
    }
    out
}

/// Every type's text parameters in the file, by type ElementId. Empty for a
/// release whose element records are not decoded.
pub fn type_text_parameters(
    rf: &mut RevitFile,
    revit_version: u32,
) -> BTreeMap<u32, BTreeMap<&'static str, String>> {
    if bbox_marker(revit_version).is_none() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let mut out: BTreeMap<u32, BTreeMap<&'static str, String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, parameters) in scan_partition(inflated.bytes(), &declared) {
            out.entry(owner).or_default().extend(parameters);
        }
    }
    out
}

/// A `u32`-counted UTF-16 string at `at` of printable characters.
fn text_at(buf: &[u8], at: usize) -> Option<String> {
    let len = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=MAX_VALUE_UNITS).contains(&len) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + len * 2)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    text.chars().all(|c| !c.is_control()).then_some(text)
}

/// A window type's Width and Height, by BuiltInParameter (RE-93).
pub const WINDOW_WIDTH_PARAMETER: i64 = -1001301;
/// See [`WINDOW_WIDTH_PARAMETER`].
pub const WINDOW_HEIGHT_PARAMETER: i64 = -1001300;
/// The name of the window family parameter holding a type's default sill
/// height (RE-93). It is a family parameter, not a BuiltInParameter, so it
/// is found by its name, as Revit's English templates name it.
pub const DEFAULT_SILL_HEIGHT_NAME: &str = "Default Sill Height";
/// From a family parameter's ElementId (after `01 00 00 00`) to its name,
/// `u32 n · UTF-16 × n`, which a `u32`-counted `revit.local.family:` id
/// follows (RE-93).
pub const PARAMETER_NAME_OFFSET: usize = 0x56;
const FAMILY_PARAMETER_ID_PREFIX: &str = "revit.local.family:";
/// Largest length value read, feet.
const MAX_LENGTH_FEET: f64 = 1000.0;

/// The declared family parameters in `buf` named `name` (RE-93).
pub fn family_parameters_named(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    name: &str,
) -> BTreeSet<u32> {
    let units: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let Ok(count) = u32::try_from(name.encode_utf16().count()) else {
        return BTreeSet::new();
    };
    let mut pattern = count.to_le_bytes().to_vec();
    pattern.extend_from_slice(&units);
    memchr::memmem::find_iter(buf, &pattern)
        .filter(|&at| {
            text_at(buf, at + pattern.len())
                .is_some_and(|id| id.starts_with(FAMILY_PARAMETER_ID_PREFIX))
        })
        .filter_map(|at| {
            let id_at = at.checked_sub(PARAMETER_NAME_OFFSET)?;
            if buf.get(id_at.checked_sub(4)?..id_at)? != [1, 0, 0, 0] {
                return None;
            }
            let id = u64::from_le_bytes(buf.get(id_at..id_at + 8)?.try_into().ok()?);
            u32::try_from(id)
                .ok()
                .filter(|id| declared_ids.contains(id))
        })
        .collect()
}

/// The length values in `buf` of each parameter in `parameters`, by the
/// declared ElementId of the value block each entry follows. A block that
/// holds two values for one parameter gets neither.
pub fn scan_partition_lengths(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    parameters: &BTreeSet<i64>,
) -> BTreeMap<u32, BTreeMap<i64, f64>> {
    let blocks = value_blocks(buf, declared_ids);
    let mut values: BTreeMap<(u32, i64), Option<f64>> = BTreeMap::new();
    for &parameter in parameters {
        for at in memchr::memmem::find_iter(buf, &parameter.to_le_bytes()) {
            let Some(value) = at
                .checked_sub(16)
                .and_then(|start| buf.get(start..at))
                .filter(|entry| entry[8..] == [0xff; 8])
                .map(|entry| f64::from_le_bytes(entry[..8].try_into().expect("8 bytes")))
                .filter(|value| value.is_finite() && value.abs() <= MAX_LENGTH_FEET)
            else {
                continue;
            };
            let index = blocks.partition_point(|(start, _)| *start < at);
            let Some(&(_, owner)) = index.checked_sub(1).and_then(|i| blocks.get(i)) else {
                continue;
            };
            values
                .entry((owner, parameter))
                .and_modify(|seen| {
                    if *seen != Some(value) {
                        *seen = None;
                    }
                })
                .or_insert(Some(value));
        }
    }
    let mut out: BTreeMap<u32, BTreeMap<i64, f64>> = BTreeMap::new();
    for ((owner, parameter), value) in values {
        if let Some(value) = value {
            out.entry(owner).or_default().insert(parameter, value);
        }
    }
    out
}

/// What a window's opening is built from, from its type (RE-93), feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowTypeOpening {
    /// The type's Width.
    pub width_feet: f64,
    /// The type's Height.
    pub height_feet: f64,
    /// The type's Default Sill Height.
    pub default_sill_feet: f64,
}

/// The opening of each type in `types` whose Width, Height and Default Sill
/// Height are read, and agree across every partition (RE-93). Empty for a
/// release whose element records are not decoded.
pub fn type_window_openings(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
) -> BTreeMap<u32, WindowTypeOpening> {
    if bbox_marker(revit_version).is_none() || types.is_empty() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let streams = rf.partition_stream_names();
    let mut sills: BTreeSet<i64> = BTreeSet::new();
    for stream in &streams {
        if let Ok(inflated) = rf.inflated_partition(stream) {
            sills.extend(
                family_parameters_named(inflated.bytes(), &declared, DEFAULT_SILL_HEIGHT_NAME)
                    .into_iter()
                    .map(i64::from),
            );
        }
    }
    let mut parameters = sills.clone();
    parameters.extend([WINDOW_WIDTH_PARAMETER, WINDOW_HEIGHT_PARAMETER]);
    type_lengths(rf, &declared, types, &parameters)
        .into_iter()
        .filter_map(|(owner, held)| {
            let value = |parameter: i64| held.get(&parameter).copied();
            let mut sill = sills.iter().filter_map(|&parameter| value(parameter));
            let default_sill_feet = sill.next()?;
            if sill.any(|other| other != default_sill_feet) {
                return None;
            }
            let opening = WindowTypeOpening {
                width_feet: value(WINDOW_WIDTH_PARAMETER)?,
                height_feet: value(WINDOW_HEIGHT_PARAMETER)?,
                default_sill_feet,
            };
            (opening.width_feet > 0.0
                && opening.height_feet > 0.0
                && opening.default_sill_feet >= 0.0)
                .then_some((owner, opening))
        })
        .collect()
}

/// A door type's Rough Width, by BuiltInParameter (RE-94).
pub const DOOR_ROUGH_WIDTH_PARAMETER: i64 = -1001305;
/// A door type's Rough Height, by BuiltInParameter (RE-94).
pub const DOOR_ROUGH_HEIGHT_PARAMETER: i64 = -1001304;

/// A door type's rough opening (RE-94), feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorTypeOpening {
    /// The type's Rough Width.
    pub rough_width_feet: f64,
    /// The type's Rough Height.
    pub rough_height_feet: f64,
}

/// The rough opening of each type in `types` whose Rough Width and Rough
/// Height are read, and agree across every partition (RE-94). Empty for a
/// release whose element records are not decoded.
pub fn type_door_openings(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
) -> BTreeMap<u32, DoorTypeOpening> {
    if bbox_marker(revit_version).is_none() || types.is_empty() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let parameters = BTreeSet::from([DOOR_ROUGH_WIDTH_PARAMETER, DOOR_ROUGH_HEIGHT_PARAMETER]);
    type_lengths(rf, &declared, types, &parameters)
        .into_iter()
        .filter_map(|(owner, held)| {
            let opening = DoorTypeOpening {
                rough_width_feet: *held.get(&DOOR_ROUGH_WIDTH_PARAMETER)?,
                rough_height_feet: *held.get(&DOOR_ROUGH_HEIGHT_PARAMETER)?,
            };
            (opening.rough_width_feet > 0.0 && opening.rough_height_feet > 0.0)
                .then_some((owner, opening))
        })
        .collect()
}

/// A structural section type's Width, by BuiltInParameter (RE-103).
pub const SECTION_WIDTH_PARAMETER: i64 = -1_005_502;
/// A structural section type's Height (RE-103).
pub const SECTION_HEIGHT_PARAMETER: i64 = -1_005_503;
/// An I-shaped section type's web thickness (RE-103).
pub const SECTION_WEB_THICKNESS_PARAMETER: i64 = -1_005_525;
/// An I-shaped section type's flange thickness (RE-103).
pub const SECTION_FLANGE_THICKNESS_PARAMETER: i64 = -1_005_524;
/// A section type's centroid, from its left edge (RE-103).
pub const SECTION_CENTROID_HORIZONTAL_PARAMETER: i64 = -1_005_508;
/// A section type's centroid, from its bottom edge (RE-103).
pub const SECTION_CENTROID_VERTICAL_PARAMETER: i64 = -1_005_509;

/// How closely an I section's centroid must sit at its centre, feet.
pub const SECTION_CENTROID_TOLERANCE_FEET: f64 = 1e-4;

/// An I-shaped structural section (RE-103), feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ISection {
    /// The flanges' width.
    pub width_feet: f64,
    /// The section's overall depth.
    pub depth_feet: f64,
    /// The web's thickness.
    pub web_feet: f64,
    /// Each flange's thickness.
    pub flange_feet: f64,
}

/// The I section of each type in `types` (RE-103): a type whose Width,
/// Height, web thickness and flange thickness are read and whose centroid
/// sits at its centre both ways, as only a doubly symmetric section's does.
/// A channel or a tee stores the same four values with its centroid off
/// centre, and an angle with neither thickness, so neither is read as an I.
pub fn type_i_sections(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
) -> BTreeMap<u32, ISection> {
    if bbox_marker(revit_version).is_none() || types.is_empty() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let parameters = BTreeSet::from([
        SECTION_WIDTH_PARAMETER,
        SECTION_HEIGHT_PARAMETER,
        SECTION_WEB_THICKNESS_PARAMETER,
        SECTION_FLANGE_THICKNESS_PARAMETER,
        SECTION_CENTROID_HORIZONTAL_PARAMETER,
        SECTION_CENTROID_VERTICAL_PARAMETER,
    ]);
    type_lengths(rf, &declared, types, &parameters)
        .into_iter()
        .filter_map(|(owner, held)| {
            let value = |parameter: i64| held.get(&parameter).copied();
            let section = ISection {
                width_feet: value(SECTION_WIDTH_PARAMETER)?,
                depth_feet: value(SECTION_HEIGHT_PARAMETER)?,
                web_feet: value(SECTION_WEB_THICKNESS_PARAMETER)?,
                flange_feet: value(SECTION_FLANGE_THICKNESS_PARAMETER)?,
            };
            let centred = |centroid: f64, extent: f64| {
                (centroid - extent / 2.0).abs() <= SECTION_CENTROID_TOLERANCE_FEET
            };
            (section.web_feet > 0.0
                && section.flange_feet > 0.0
                && section.web_feet < section.width_feet
                && 2.0 * section.flange_feet < section.depth_feet
                && centred(
                    value(SECTION_CENTROID_HORIZONTAL_PARAMETER)?,
                    section.width_feet,
                )
                && centred(
                    value(SECTION_CENTROID_VERTICAL_PARAMETER)?,
                    section.depth_feet,
                ))
            .then_some((owner, section))
        })
        .collect()
}

/// The values of `parameters` held by each type in `types`, where every
/// partition's value agrees.
fn type_lengths(
    rf: &mut RevitFile,
    declared: &BTreeSet<u32>,
    types: &BTreeSet<u32>,
    parameters: &BTreeSet<i64>,
) -> BTreeMap<u32, BTreeMap<i64, f64>> {
    let mut values: BTreeMap<u32, BTreeMap<i64, Option<f64>>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, found) in scan_partition_lengths(inflated.bytes(), declared, parameters) {
            if !types.contains(&owner) {
                continue;
            }
            let held = values.entry(owner).or_default();
            for (parameter, value) in found {
                held.entry(parameter)
                    .and_modify(|seen| {
                        if *seen != Some(value) {
                            *seen = None;
                        }
                    })
                    .or_insert(Some(value));
            }
        }
    }
    values
        .into_iter()
        .map(|(owner, held)| {
            let agreed = held
                .into_iter()
                .filter_map(|(parameter, value)| Some((parameter, value?)))
                .collect();
            (owner, agreed)
        })
        .collect()
}

/// Value block starts in `buf`: the mark, not preceded by a further 0xff (a
/// longer run of 0xff is padding, not a block), with a declared ElementId
/// before it.
fn value_blocks(buf: &[u8], declared_ids: &BTreeSet<u32>) -> Vec<(usize, u32)> {
    memchr::memmem::find_iter(buf, &VALUE_BLOCK_MARK)
        .filter(|&at| at >= 8 && buf[at - 1] != 0xff)
        .filter_map(|at| {
            let id = u64::from_le_bytes(buf[at - 8..at].try_into().ok()?);
            let id = u32::try_from(id).ok()?;
            declared_ids.contains(&id).then_some((at, id))
        })
        .collect()
}
