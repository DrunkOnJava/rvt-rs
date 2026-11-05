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
use crate::le_bytes;
use crate::partition_element_records::supports_revit_version;
use std::collections::{BTreeMap, BTreeSet};

/// The type parameters read, by BuiltInParameter id, with the name Revit
/// shows for each.
pub const TYPE_TEXT_PARAMETERS: &[(i64, &str)] = &[
    (-1001405, "Type Mark"),
    (-1010103, "Description"),
    (-1001206, "Fire Rating"),
];

/// The type parameters read in the form of a family type table's entries,
/// `u32 n · UTF-16 × n · (u32 0 | ff ff ff ff · u16) · f64 · i64 -1 · i64
/// BuiltInParameter`, with the text before the parameter it belongs to
/// (RE-185 section 3.3, RE-354). The BIM4LCA office's door types hold their
/// Fire Rating this way and in no other: `EI30`, `EI60` and `-` on the six
/// door types Revit's export gives a `FireRating`, and its family types hold
/// their Manufacturer so: the six texts of the six family types Revit's
/// export gives a `Manufacturer`, which RE-289 looked for in RE-77's form
/// alone.
pub const TABLE_TEXT_PARAMETERS: &[(i64, &str)] =
    &[(-1001206, "Fire Rating"), (-1010108, "Manufacturer")];

/// Longest value read, in UTF-16 units.
const MAX_VALUE_UNITS: usize = 400;

/// Longest text of a table entry read, in UTF-16 units.
const MAX_TABLE_TEXT_UNITS: usize = 60;

/// The bytes that follow an ElementId to open its value block.
pub const VALUE_BLOCK_MARK: [u8; 59] = {
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
    let mut note = |at: usize, name: &'static str, text: String| {
        let index = blocks.partition_point(|(start, _)| *start < at);
        let Some(&(_, Some(owner))) = index.checked_sub(1).and_then(|i| blocks.get(i)) else {
            return;
        };
        values
            .entry((owner, name))
            .and_modify(|seen| {
                if seen.as_deref() != Some(text.as_str()) {
                    *seen = None;
                }
            })
            .or_insert(Some(text));
    };
    for (parameter, name) in TYPE_TEXT_PARAMETERS {
        for at in memchr::memmem::find_iter(buf, &parameter.to_le_bytes()) {
            if let Some(text) = text_at(buf, at + 8) {
                note(at, name, text);
            }
        }
    }
    for (parameter, name) in TABLE_TEXT_PARAMETERS {
        let mut mark = [0xffu8; 16];
        mark[8..].copy_from_slice(&parameter.to_le_bytes());
        for at in memchr::memmem::find_iter(buf, &mark) {
            if let Some(text) = table_text_before(buf, at) {
                note(at, name, text);
            }
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
    if !supports_revit_version(revit_version) {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
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

/// The text of the table entry whose element id (`-1`) is at `at`: the
/// `u32`-counted UTF-16 string before the entry's flag and `f64`, the flag
/// being a zero `u32` or `ff ff ff ff` and a `u16`.
fn table_text_before(buf: &[u8], at: usize) -> Option<String> {
    let flag_ends = at.checked_sub(8)?;
    let text_ends = if le_bytes::u32_at(buf, flag_ends.checked_sub(4)?)? == 0 {
        flag_ends - 4
    } else if buf.get(flag_ends.checked_sub(6)?..flag_ends.checked_sub(2)?)? == [0xff; 4] {
        flag_ends - 6
    } else {
        return None;
    };
    (1..=MAX_TABLE_TEXT_UNITS).find_map(|units| {
        let start = text_ends.checked_sub(4 + 2 * units)?;
        if le_bytes::u32_at(buf, start)? as usize != units {
            return None;
        }
        text_at(buf, start)
    })
}

/// A `u32`-counted UTF-16 string at `at` of printable characters.
fn text_at(buf: &[u8], at: usize) -> Option<String> {
    let len = le_bytes::u32_at(buf, at)? as usize;
    if !(1..=MAX_VALUE_UNITS).contains(&len) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + len * 2)?;
    let units: Vec<u16> = le_bytes::u16_units(bytes).collect();
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
            let id = le_bytes::u64_at(buf, id_at)?;
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
    scan_partition_length_places(buf, declared_ids, parameters, false)
        .into_iter()
        .map(|(owner, held)| {
            let held = held
                .into_iter()
                .map(|(parameter, (value, _))| (parameter, value))
                .collect();
            (owner, held)
        })
        .collect()
}

/// [`scan_partition_lengths`], also giving, when `want_places` asks, where the
/// first copy of each value is: the offset of its `f64` in `buf` (0 when it
/// does not ask) (#1368).
pub fn scan_partition_length_places(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    parameters: &BTreeSet<i64>,
    want_places: bool,
) -> BTreeMap<u32, BTreeMap<i64, (f64, usize)>> {
    let blocks = value_blocks(buf, declared_ids);
    let mut first_at: BTreeMap<(u32, i64), usize> = BTreeMap::new();
    let mut values: BTreeMap<(u32, i64), Option<f64>> = BTreeMap::new();
    for &parameter in parameters {
        for at in memchr::memmem::find_iter(buf, &parameter.to_le_bytes()) {
            let Some(value) = at
                .checked_sub(16)
                .and_then(|start| buf.get(start..at))
                .filter(|entry| entry[8..] == [0xff; 8])
                .and_then(|entry| le_bytes::f64_at(entry, 0))
                .filter(|value| value.is_finite() && value.abs() <= MAX_LENGTH_FEET)
            else {
                continue;
            };
            let index = blocks.partition_point(|(start, _)| *start < at);
            let Some(&(_, Some(owner))) = index.checked_sub(1).and_then(|i| blocks.get(i)) else {
                continue;
            };
            if want_places {
                first_at.entry((owner, parameter)).or_insert(at - 16);
            }
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
    let mut out: BTreeMap<u32, BTreeMap<i64, (f64, usize)>> = BTreeMap::new();
    for ((owner, parameter), value) in values {
        if let Some(value) = value {
            let at = first_at.get(&(owner, parameter)).copied().unwrap_or(0);
            out.entry(owner).or_default().insert(parameter, (value, at));
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
    if !supports_revit_version(revit_version) || types.is_empty() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
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
    if !supports_revit_version(revit_version) || types.is_empty() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
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

/// A door type's Width and Height (RE-187), feet: the BuiltInParameters a
/// window type holds them in too ([`WINDOW_WIDTH_PARAMETER`],
/// [`WINDOW_HEIGHT_PARAMETER`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorTypeSize {
    /// The type's Width.
    pub width_feet: f64,
    /// The type's Height.
    pub height_feet: f64,
}

/// The size of each type in `types` whose Width and Height are read, and
/// agree across every partition (RE-187). Empty for a release whose element
/// records are not decoded.
pub fn type_door_sizes(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
) -> BTreeMap<u32, DoorTypeSize> {
    if !supports_revit_version(revit_version) || types.is_empty() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
    let parameters = BTreeSet::from([WINDOW_WIDTH_PARAMETER, WINDOW_HEIGHT_PARAMETER]);
    type_lengths(rf, &declared, types, &parameters)
        .into_iter()
        .filter_map(|(owner, held)| {
            let size = DoorTypeSize {
                width_feet: *held.get(&WINDOW_WIDTH_PARAMETER)?,
                height_feet: *held.get(&WINDOW_HEIGHT_PARAMETER)?,
            };
            (size.width_feet > 0.0 && size.height_feet > 0.0).then_some((owner, size))
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
    type_i_sections_at(rf, revit_version, types, false).0
}

/// The parameters an I section is read from (RE-103).
pub const I_SECTION_PARAMETERS: [i64; 6] = [
    SECTION_WIDTH_PARAMETER,
    SECTION_HEIGHT_PARAMETER,
    SECTION_WEB_THICKNESS_PARAMETER,
    SECTION_FLANGE_THICKNESS_PARAMETER,
    SECTION_CENTROID_HORIZONTAL_PARAMETER,
    SECTION_CENTROID_VERTICAL_PARAMETER,
];

/// Where the first copy of a type's parameter entry is: its partition and the
/// offset of its `f64` in that partition inflated (#1368).
pub type ParameterPlaces<K> = BTreeMap<(u32, K), (String, usize)>;

/// Note, for `element`, the value source `reader` made of the first copies of
/// `keys`, the parameters of the type `type_id` that `element` took values
/// from (#1368). Nothing is noted unless every key has a place.
pub(crate) fn note_parameter_source<K: Ord>(
    rf: &mut RevitFile,
    reader: &str,
    (element, type_id): (u32, u32),
    keys: impl IntoIterator<Item = K>,
    places: &ParameterPlaces<K>,
) {
    use crate::ifc::element_provenance::{CopyRule, SourcePart, ValueSource};
    let mut parts = Vec::new();
    for key in keys {
        let Some((stream, offset)) = places.get(&(type_id, key)) else {
            return;
        };
        parts.push(SourcePart {
            element_id: type_id,
            stream: stream.clone(),
            offset: *offset,
        });
    }
    let copies = parts.len();
    rf.note_value_source(
        element,
        ValueSource::from_parts(reader, CopyRule::Agreeing, copies, parts),
    );
}

/// [`type_i_sections`], also giving, when `want_places` asks, where the first
/// copy of each parameter an I section is read from is (#1368).
pub fn type_i_sections_at(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
    want_places: bool,
) -> (BTreeMap<u32, ISection>, ParameterPlaces<i64>) {
    if !supports_revit_version(revit_version) || types.is_empty() {
        return (BTreeMap::new(), ParameterPlaces::new());
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return (BTreeMap::new(), ParameterPlaces::new());
    };
    let declared = crate::elem_table::declared_ids(&records);
    let parameters = BTreeSet::from(I_SECTION_PARAMETERS);
    let (lengths, places) = type_lengths_at(rf, &declared, types, &parameters, want_places);
    let sections = lengths
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
        .collect();
    (sections, places)
}

/// Every `f64 · ff×8 · i64` entry in `buf` (a length value and the
/// parameter it is stored under, BuiltInParameter or not), by the declared
/// ElementId of the value block it follows: what [`scan_partition_lengths`]
/// reads for a given set of parameters, here for all of them, for probes.
pub fn partition_length_entries(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
) -> BTreeMap<u32, Vec<(i64, f64)>> {
    length_entries_at(buf, declared_ids)
        .into_iter()
        .map(|(owner, entries)| {
            let entries = entries
                .into_iter()
                .map(|(parameter, value, _)| (parameter, value))
                .collect();
            (owner, entries)
        })
        .collect()
}

/// [`partition_length_entries`], with the offset of each entry's `f64`.
fn length_entries_at(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
) -> BTreeMap<u32, Vec<(i64, f64, usize)>> {
    let blocks = value_blocks(buf, declared_ids);
    let mut out: BTreeMap<u32, Vec<(i64, f64, usize)>> = BTreeMap::new();
    for at in memchr::memmem::find_iter(buf, &[0xff; 8]) {
        if at < 8 || buf[at - 1] == 0xff {
            continue;
        }
        let (Some(value), Some(parameter)) = (
            le_bytes::f64_at(buf, at - 8).filter(|v| v.is_finite() && v.abs() <= MAX_LENGTH_FEET),
            le_bytes::i64_at(buf, at + 8),
        ) else {
            continue;
        };
        let index = blocks.partition_point(|(start, _)| *start < at);
        if let Some(&(_, Some(owner))) = index.checked_sub(1).and_then(|i| blocks.get(i)) {
            out.entry(owner)
                .or_default()
                .push((parameter, value, at - 8));
        }
    }
    out
}

/// A rectangular hollow structural section with rounded corners (W6), feet:
/// its outside width and height, its wall thickness, and its outside
/// corner radius; the inside corners' radius is the outside's less the wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HollowSection {
    pub width_feet: f64,
    pub height_feet: f64,
    pub wall_feet: f64,
    pub outer_radius_feet: f64,
}

/// The family parameters a hollow section type holds its width, height,
/// wall thickness and outside corner radius under, by the names
/// (`ParamDefValue.m_caption`) their `ParamElemFamily` elements carry: the
/// BIM4LCA office's `Rectangular and Square Hollow Sections-Column` family.
pub const HOLLOW_SECTION_PARAMETER_NAMES: [&str; 4] = ["b", "Ht", "t", "kr"];

/// The length parameters each type in `types` holds, by the caption
/// (`ParamDefValue.m_caption`) their `ParamElemFamily` elements carry, every
/// partition's value agreeing (RE-221, RE-291). A parameter two partitions
/// give different values, or whose element is not read, is left out.
pub fn type_named_lengths(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
) -> BTreeMap<u32, BTreeMap<String, f64>> {
    type_named_lengths_at(rf, revit_version, types, false).0
}

/// [`type_named_lengths`], also giving, when `want_places` asks, where the
/// first copy of each parameter's value is (#1368).
pub fn type_named_lengths_at(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
    want_places: bool,
) -> (
    BTreeMap<u32, BTreeMap<String, f64>>,
    ParameterPlaces<String>,
) {
    let none = || (BTreeMap::new(), ParameterPlaces::new());
    if !supports_revit_version(revit_version) || types.is_empty() {
        return none();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return none();
    };
    let declared = crate::elem_table::declared_ids(&records);
    let mut values: BTreeMap<(u32, i64), Option<f64>> = BTreeMap::new();
    let mut first_at: BTreeMap<(u32, i64), (String, usize)> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, entries) in length_entries_at(inflated.bytes(), &declared) {
            if !types.contains(&owner) {
                continue;
            }
            for (parameter, value, at) in entries {
                if want_places {
                    first_at
                        .entry((owner, parameter))
                        .or_insert_with(|| (stream.clone(), at));
                }
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
    }
    let parameters: BTreeSet<u64> = values
        .keys()
        .filter_map(|(_, parameter)| u64::try_from(*parameter).ok())
        .filter(|id| *id > 0 && *id < u64::from(u32::MAX))
        .collect();
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    if !parameters.is_empty() {
        let options = crate::native_document::Options {
            selected_ids: parameters,
            ..crate::native_document::Options::default()
        };
        let read = crate::native_document::extract_graphs(rf, &options, |record| {
            let (Some("ParamElemFamily"), Some(graph)) =
                (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let caption = graph
                .objects
                .iter()
                .find(|object| object.class_name == "ParamDefValue")
                .and_then(|object| object.fields.get("m_caption")?.as_str());
            if let (Some(caption), Ok(id)) = (caption, i64::try_from(record.identity.element_id)) {
                names.insert(id, caption.to_string());
            }
            Ok(())
        });
        if read.is_err() {
            return none();
        }
    }
    let mut held: BTreeMap<u32, BTreeMap<String, f64>> = BTreeMap::new();
    let mut places = ParameterPlaces::new();
    for ((owner, parameter), value) in &values {
        let (Some(value), Some(name)) = (value, names.get(parameter)) else {
            continue;
        };
        held.entry(*owner).or_default().insert(name.clone(), *value);
        if let Some(place) = first_at.get(&(*owner, *parameter)) {
            places.insert((*owner, name.clone()), place.clone());
        }
    }
    (held, places)
}

/// The hollow section of each type in `types` whose family parameters
/// named [`HOLLOW_SECTION_PARAMETER_NAMES`] it holds, every partition's
/// value agreeing, where they make a section with walls and rounded corners
/// ([`hollow_section`]). On the BIM4LCA office the three such column types'
/// values are Revit's profiles exactly: outside `b` by `Ht` with corners of
/// radius `kr`, inside `t` in with corners of radius `kr - t`.
pub fn type_hollow_sections(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
) -> BTreeMap<u32, HollowSection> {
    type_named_lengths(rf, revit_version, types)
        .into_iter()
        .filter_map(|(owner, by_name)| hollow_section(&by_name).map(|section| (owner, section)))
        .collect()
}

/// The hollow section `by_name` (a type's [`type_named_lengths`]) makes:
/// `t` positive and under half of `b` and of `Ht`, `kr` over `t` and at most
/// half of each.
pub fn hollow_section(by_name: &BTreeMap<String, f64>) -> Option<HollowSection> {
    let [b, h, t, r] = HOLLOW_SECTION_PARAMETER_NAMES.map(|name| by_name.get(name).copied());
    let section = HollowSection {
        width_feet: b?,
        height_feet: h?,
        wall_feet: t?,
        outer_radius_feet: r?,
    };
    let narrow = section.width_feet.min(section.height_feet);
    (section.wall_feet > 0.0
        && 2.0 * section.wall_feet < narrow
        && section.outer_radius_feet > section.wall_feet
        && 2.0 * section.outer_radius_feet <= narrow)
        .then_some(section)
}

/// The two sides, feet, of the plain rectangular section a column type holds
/// as its parameters `b` and `h` (`b` alone for a square one), from `by_name`
/// (a type's [`type_named_lengths`]). On the BIM4LCA office the square and
/// rectangular concrete column types hold exactly these, equal to their
/// columns' record boxes, and the one round column's type holds a `Diameter`
/// and no `b` (RE-291). `None` unless the type's named length parameters are
/// exactly `b`, or `b` and `h` (a hollow section's `Ht`, `t` and `kr`, a
/// round column's `Diameter`, an I, T or channel section's flange and web
/// thicknesses, or any other named length leave it out), and each side is a
/// positive number. A length parameter whose element is not read is not in
/// `by_name`, so it is not seen.
pub fn rectangular_section(by_name: &BTreeMap<String, f64>) -> Option<[f64; 2]> {
    let b = *by_name.get("b")?;
    let h = match by_name.len() {
        1 => b,
        2 => *by_name.get("h")?,
        _ => return None,
    };
    [b, h]
        .iter()
        .all(|side| side.is_finite() && *side > 0.0)
        .then_some([b, h])
}

/// The values of `parameters` held by each type in `types`, where every
/// partition's value agrees.
fn type_lengths(
    rf: &mut RevitFile,
    declared: &BTreeSet<u32>,
    types: &BTreeSet<u32>,
    parameters: &BTreeSet<i64>,
) -> BTreeMap<u32, BTreeMap<i64, f64>> {
    type_lengths_at(rf, declared, types, parameters, false).0
}

/// [`type_lengths`], also giving, when `want_places` asks, where the first
/// copy of each value is (#1368).
fn type_lengths_at(
    rf: &mut RevitFile,
    declared: &BTreeSet<u32>,
    types: &BTreeSet<u32>,
    parameters: &BTreeSet<i64>,
    want_places: bool,
) -> (BTreeMap<u32, BTreeMap<i64, f64>>, ParameterPlaces<i64>) {
    let mut values: BTreeMap<u32, BTreeMap<i64, Option<f64>>> = BTreeMap::new();
    let mut places = ParameterPlaces::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, found) in
            scan_partition_length_places(inflated.bytes(), declared, parameters, want_places)
        {
            if !types.contains(&owner) {
                continue;
            }
            let held = values.entry(owner).or_default();
            for (parameter, (value, at)) in found {
                if want_places {
                    places
                        .entry((owner, parameter))
                        .or_insert_with(|| (stream.clone(), at));
                }
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
    let agreed = values
        .into_iter()
        .map(|(owner, held)| {
            let agreed = held
                .into_iter()
                .filter_map(|(parameter, value)| Some((parameter, value?)))
                .collect();
            (owner, agreed)
        })
        .collect();
    (agreed, places)
}

/// Value block starts in `buf`: the mark, not preceded by a further 0xff (a
/// longer run of 0xff is padding, not a block), with a declared ElementId
/// before it, the block's owner. A mark with 0 before it also starts a
/// block, owned by element 0 only when the table declares one: it ends the
/// block before it either way. (Until #152 every 2024 and later file
/// declared a false id 0, read from the bytes after its ElemTable, and the
/// blocks were cut there; without the cut, RE1 Architecture's curtain wall
/// type took the next block's `Type Mark`.)
fn value_blocks(buf: &[u8], declared_ids: &BTreeSet<u32>) -> Vec<(usize, Option<u32>)> {
    memchr::memmem::find_iter(buf, &VALUE_BLOCK_MARK)
        .filter(|&at| at >= 8 && buf[at - 1] != 0xff)
        .filter_map(|at| {
            let id = le_bytes::u64_at(buf, at - 8)?;
            let id = u32::try_from(id).ok()?;
            if declared_ids.contains(&id) {
                Some((at, Some(id)))
            } else {
                (id == 0).then_some((at, None))
            }
        })
        .collect()
}
