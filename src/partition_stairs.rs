//! Stair and stair-run dimensions from their serialised element data
//! (RE-47).
//!
//! A stair's data (its element-data header and ElementId, see
//! [`crate::partition_names::element_data_header`]) carries, a fixed
//! distance past the bytes [`STAIR_DIMENSIONS_ANCHOR`]:
//!
//! ```text
//! +16  f64  riser height, feet
//! +24  f64  tread depth, feet
//! +88  u32  number of risers
//! ```
//!
//! The anchor sits `0x63` or `0x6b` bytes into the data on Autodesk's
//! Snowdon Towers 2024 architectural sample: an optional field before it
//! moves it, so it is found rather than assumed. All three values equal the
//! `Pset_StairCommon` `RiserHeight`, `TreadLength` and `NumberOfRiser`
//! Revit's own IFC4 export writes for the same stair, on 29 of 29 stairs.
//!
//! A stair run's data carries its own number of risers right after
//! [`STAIR_RUN_RISERS_ANCHOR`], at `0x42` on every Snowdon run. The runs of
//! a stair add up to the stair's count on 26 of its 29 stairs. The other
//! three have one run each, which reads one more than the stair, so a
//! single-run stair's flight takes the stair's count
//! ([`flight_riser_counts`]).
//!
//! # Treads and risers (RE-52)
//!
//! A straight run's data also carries its plan sketch as bounded lines
//! ([`crate::partition_beam_axes::BoundedLine`]), all at the run's base
//! elevation: first its two boundary lines, from the first riser to the
//! last, then one line across the run at each riser ([`run_sketch`]). The
//! run's reference list names its run type, and that type's own serialised
//! object (`01 00 00 00 · u64 id`, often held inside another element's data)
//! carries the construction: tread, riser and nosing thicknesses and which
//! parts the run has ([`run_type_at`]). With the stair's riser height, those
//! give the run's side view, treads and risers ([`run_side_profile`]).
//!
//! On Snowdon Towers 34 of the 43 runs are drawn. The 30 steel-pan runs,
//! with slanted risers, equal Revit's own geometry of the flight to 1e-5
//! ft. The 4 with upright risers hold every point of Revit's, but their
//! nosing is drawn square where Revit shapes it with the type's nosing
//! profile, and one that starts on a landing has its first riser 2 in
//! short at the bottom. Monolithic runs, runs without risers, curved runs
//! and runs whose riser lines outnumber their risers are not drawn.
//!
//! Nothing here reads a release other than 2024.

use crate::partition_beam_axes::BoundedLine;
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where these layouts are measured.
pub const STAIR_DIMENSIONS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// The bytes the stair dimensions follow: two unset `0x03fb` field frames
/// and a zero `u32`.
pub const STAIR_DIMENSIONS_ANCHOR: [u8; 16] = [
    0xff, 0xff, 0xff, 0xff, 0xfb, 0x03, 0xff, 0xff, 0xff, 0xff, 0xfb, 0x03, 0x00, 0x00, 0x00, 0x00,
];

/// Offsets past the start of [`STAIR_DIMENSIONS_ANCHOR`].
pub const RISER_HEIGHT_OFFSET: usize = 16;
/// See [`RISER_HEIGHT_OFFSET`].
pub const TREAD_DEPTH_OFFSET: usize = 24;
/// See [`RISER_HEIGHT_OFFSET`].
pub const RISER_COUNT_OFFSET: usize = 88;

/// The bytes a run's number of risers follows.
pub const STAIR_RUN_RISERS_ANCHOR: [u8; 15] = [
    0xfc, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
];

/// How far into an element's data an anchor is looked for.
pub const ANCHOR_WINDOW: usize = 0x100;

/// Largest number of risers accepted; a stair of this many would rise
/// hundreds of feet.
pub const MAX_RISERS: u32 = 400;

/// A stair's riser and tread dimensions (RE-47).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairDimensions {
    /// Height of one riser, feet.
    pub riser_height_feet: f64,
    /// Depth of one tread, feet.
    pub tread_depth_feet: f64,
    /// Number of risers of the whole stair.
    pub riser_count: u32,
}

/// Whether these layouts are measured for `revit_version`.
pub fn supports_revit_version(revit_version: u32) -> bool {
    STAIR_DIMENSIONS_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_f64(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|s| f64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// Where `anchor` first starts within [`ANCHOR_WINDOW`] of `data_offset`.
fn anchor_at(buf: &[u8], data_offset: usize, anchor: &[u8]) -> Option<usize> {
    let end = data_offset
        .checked_add(ANCHOR_WINDOW + anchor.len())?
        .min(buf.len());
    let window = buf.get(data_offset..end)?;
    memchr::memmem::find(window, anchor).map(|at| data_offset + at)
}

/// A length a stair can have: finite, positive and under `limit_feet`.
fn plausible(value: f64, limit_feet: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0 && value < limit_feet).then_some(value)
}

/// The dimensions in the stair data that starts at `data_offset` (its
/// element-data header), or `None` when the anchor is missing or a value
/// is out of range.
pub fn stair_dimensions_at(buf: &[u8], data_offset: usize) -> Option<StairDimensions> {
    let anchor = anchor_at(buf, data_offset, &STAIR_DIMENSIONS_ANCHOR)?;
    let riser_height_feet = plausible(read_f64(buf, anchor + RISER_HEIGHT_OFFSET)?, 2.0)?;
    let tread_depth_feet = plausible(read_f64(buf, anchor + TREAD_DEPTH_OFFSET)?, 10.0)?;
    let riser_count = read_u32(buf, anchor + RISER_COUNT_OFFSET)?;
    if !(1..=MAX_RISERS).contains(&riser_count) {
        return None;
    }
    Some(StairDimensions {
        riser_height_feet,
        tread_depth_feet,
        riser_count,
    })
}

/// The number of risers in the run data that starts at `data_offset`.
pub fn run_riser_count_at(buf: &[u8], data_offset: usize) -> Option<u32> {
    let anchor = anchor_at(buf, data_offset, &STAIR_RUN_RISERS_ANCHOR)?;
    let count = read_u32(buf, anchor + STAIR_RUN_RISERS_ANCHOR.len())?;
    (1..=MAX_RISERS).contains(&count).then_some(count)
}

/// Where each id in `ids` has its element data in `buf`, first occurrence.
fn data_offsets(buf: &[u8], header: &[u8; 10], ids: &[u32]) -> BTreeMap<u32, usize> {
    let mut out = BTreeMap::new();
    for hit in memchr::memmem::find_iter(buf, header) {
        let Some(id) = buf
            .get(hit + header.len()..hit + header.len() + 8)
            .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
            .and_then(|id| u32::try_from(id).ok())
        else {
            continue;
        };
        if ids.contains(&id) {
            out.entry(id).or_insert(hit);
        }
    }
    out
}

/// The dimensions of each stair in `stairs` and the riser count of each run
/// in `runs`, read from every partition. An id whose copies disagree is
/// dropped. Both maps are empty for a release these layouts are not
/// measured on.
pub fn scan_stair_dimensions(
    rf: &mut RevitFile,
    revit_version: u32,
    stairs: &[u32],
    runs: &[u32],
) -> Result<(BTreeMap<u32, StairDimensions>, BTreeMap<u32, u32>)> {
    let mut stair_out: BTreeMap<u32, Option<StairDimensions>> = BTreeMap::new();
    let mut run_out: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if supports_revit_version(revit_version) => header,
        _ => return Ok((BTreeMap::new(), BTreeMap::new())),
    };
    let wanted: Vec<u32> = stairs.iter().chain(runs).copied().collect();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (id, offset) in data_offsets(buf, &header, &wanted) {
            if stairs.contains(&id) {
                if let Some(found) = stair_dimensions_at(buf, offset) {
                    merge(&mut stair_out, id, found);
                }
            }
            if runs.contains(&id) {
                if let Some(found) = run_riser_count_at(buf, offset) {
                    merge(&mut run_out, id, found);
                }
            }
        }
    }
    Ok((settled(stair_out), settled(run_out)))
}

fn merge<T: PartialEq>(out: &mut BTreeMap<u32, Option<T>>, id: u32, found: T) {
    match out.get_mut(&id) {
        None => {
            out.insert(id, Some(found));
        }
        Some(held) => {
            if held.as_ref() != Some(&found) {
                *held = None;
            }
        }
    }
}

fn settled<T>(map: BTreeMap<u32, Option<T>>) -> BTreeMap<u32, T> {
    map.into_iter()
        .filter_map(|(id, value)| value.map(|value| (id, value)))
        .collect()
}

/// Each run's number of risers, given the stair's and those its runs
/// record: a single run has the stair's, and several runs have their own
/// when they add up to the stair's. Otherwise nothing is claimed.
pub fn flight_riser_counts(stair_risers: u32, runs: &[(u32, Option<u32>)]) -> BTreeMap<u32, u32> {
    if let [(run, _)] = runs {
        return BTreeMap::from([(*run, stair_risers)]);
    }
    let counts: Option<Vec<u32>> = runs.iter().map(|(_, count)| *count).collect();
    match counts {
        Some(counts) if counts.iter().sum::<u32>() == stair_risers => runs
            .iter()
            .zip(counts)
            .map(|((run, _), count)| (*run, count))
            .collect(),
        _ => BTreeMap::new(),
    }
}

/// How far a sketch line may stray from the run's base, from the run's
/// directions, and from its boundary lines, feet.
pub const SKETCH_TOLERANCE_FEET: f64 = 1e-6;

/// A straight run's plan sketch (RE-52), model feet.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSketch {
    /// Where the first boundary line starts: the first riser's end on that
    /// side, at the run's base elevation.
    pub origin: [f64; 3],
    /// Unit plan direction the run climbs in.
    pub climb: [f64; 2],
    /// Unit plan direction across the run, towards its second boundary line.
    pub across: [f64; 2],
    /// Distance between the boundary lines.
    pub width_feet: f64,
    /// Distance along `climb` from `origin` to each riser line, ascending,
    /// the first 0.
    pub risers: Vec<f64>,
}

fn plan(a: [f64; 3], b: [f64; 3]) -> [f64; 2] {
    [b[0] - a[0], b[1] - a[1]]
}

fn dot2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// The run sketch in a run's bounded lines, in the order its data holds
/// them: the first is a boundary line running from the first riser to the
/// last, and every line across the run that spans it from boundary to
/// boundary is a riser line. `None` unless the first line is level and
/// straight, a second boundary line and at least two riser lines lie at
/// its elevation, and the first boundary line ends on the last riser.
pub fn run_sketch(lines: &[BoundedLine]) -> Option<RunSketch> {
    let first = lines.first()?;
    let (origin, end) = (first.start(), first.end());
    let level = |line: &BoundedLine| {
        (line.start()[2] - origin[2]).abs() < SKETCH_TOLERANCE_FEET
            && (line.end()[2] - origin[2]).abs() < SKETCH_TOLERANCE_FEET
    };
    if !level(first) {
        return None;
    }
    let run = plan(origin, end);
    let length = run[0].hypot(run[1]);
    if !length.is_finite() || length <= SKETCH_TOLERANCE_FEET {
        return None;
    }
    let climb = [run[0] / length, run[1] / length];
    let normal = [-climb[1], climb[0]];
    let offset = |p: [f64; 3]| dot2(plan(origin, p), normal);
    let along = |p: [f64; 3]| dot2(plan(origin, p), climb);
    let direction = |line: &BoundedLine| [line.direction[0], line.direction[1]];

    let signed_width = lines[1..]
        .iter()
        .filter(|line| level(line))
        .find_map(|line| {
            let parallel = (1.0 - dot2(direction(line), climb).abs()) < SKETCH_TOLERANCE_FEET;
            let w = offset(line.start());
            (parallel && w.abs() > SKETCH_TOLERANCE_FEET).then_some(w)
        })?;
    let width_feet = signed_width.abs();
    let across = [
        normal[0] * signed_width.signum(),
        normal[1] * signed_width.signum(),
    ];

    let mut risers: Vec<f64> = lines[1..]
        .iter()
        .filter(|line| level(line) && dot2(direction(line), climb).abs() < SKETCH_TOLERANCE_FEET)
        .filter_map(|line| {
            let (a, b) = (line.start(), line.end());
            let (u, ua, ub) = (along(a), offset(a), offset(b));
            let spans = |x: f64, y: f64| {
                (x.abs() < SKETCH_TOLERANCE_FEET
                    && (y - signed_width).abs() < SKETCH_TOLERANCE_FEET)
                    || (y.abs() < SKETCH_TOLERANCE_FEET
                        && (x - signed_width).abs() < SKETCH_TOLERANCE_FEET)
            };
            ((u - along(b)).abs() < SKETCH_TOLERANCE_FEET && spans(ua, ub)).then_some(u)
        })
        .collect();
    risers.sort_by(f64::total_cmp);
    risers.dedup_by(|a, b| (*a - *b).abs() < SKETCH_TOLERANCE_FEET);
    let (&first_riser, &last_riser) = (risers.first()?, risers.last()?);
    if risers.len() < 2
        || first_riser.abs() > SKETCH_TOLERANCE_FEET
        || (last_riser - length).abs() > SKETCH_TOLERANCE_FEET
    {
        return None;
    }
    Some(RunSketch {
        origin,
        climb,
        across,
        width_feet,
        risers,
    })
}

/// The class tag of a stair run type's serialised object, a fixed distance
/// past its ElementId.
pub const RUN_TYPE_OBJECT_TAG: [u8; 2] = [0xdb, 0x0f];
/// Offsets past a run type's ElementId (the `u64` after `01 00 00 00`).
pub const RUN_TYPE_TAG_OFFSET: usize = 0x47;
/// See [`RUN_TYPE_TAG_OFFSET`]: `f64` feet.
pub const RUN_TYPE_STRUCTURAL_DEPTH_OFFSET: usize = 0x59;
/// See [`RUN_TYPE_TAG_OFFSET`]: `f64` feet.
pub const RUN_TYPE_TREAD_THICKNESS_OFFSET: usize = 0x69;
/// See [`RUN_TYPE_TAG_OFFSET`]: `f64` feet.
pub const RUN_TYPE_RISER_THICKNESS_OFFSET: usize = 0x71;
/// See [`RUN_TYPE_TAG_OFFSET`]: `f64` feet.
pub const RUN_TYPE_NOSING_LENGTH_OFFSET: usize = 0x79;
/// See [`RUN_TYPE_TAG_OFFSET`]: one byte, set only on the type whose treads
/// run on under the riser above.
pub const RUN_TYPE_TREAD_UNDER_RISER_OFFSET: usize = 0xb5;
/// See [`RUN_TYPE_TAG_OFFSET`]: four one-byte flags, monolithic, treads,
/// risers and slanted risers, then the type's name as `u32` length and
/// UTF-16.
pub const RUN_TYPE_FLAGS_OFFSET: usize = 0xbd;

/// A stair run type's construction (RE-52).
#[derive(Debug, Clone, PartialEq)]
pub struct RunType {
    /// Depth of a monolithic run's structure below its steps, feet.
    pub structural_depth_feet: f64,
    /// Feet.
    pub tread_thickness_feet: f64,
    /// Feet.
    pub riser_thickness_feet: f64,
    /// How far a tread overhangs the riser below it, feet.
    pub nosing_length_feet: f64,
    /// One cast body rather than separate treads and risers.
    pub monolithic: bool,
    /// The run has separate treads.
    pub treads: bool,
    /// The run has risers.
    pub risers: bool,
    /// Each riser leans back from the nosing above it to the tread below.
    pub slanted_risers: bool,
    /// Each tread runs on under the riser above instead of stopping at it.
    pub tread_under_riser: bool,
    /// The type's name.
    pub name: Option<String>,
}

fn flag(buf: &[u8], at: usize) -> Option<bool> {
    match buf.get(at)? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn utf16_at(buf: &[u8], at: usize) -> Option<String> {
    let length = usize::try_from(read_u32(buf, at)?).ok()?;
    if length == 0 || length > 256 {
        return None;
    }
    let raw = buf.get(at + 4..at + 4 + 2 * length)?;
    let units: Vec<u16> = raw
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).ok()
}

/// The run type whose ElementId starts at `id_at`, or `None` when the tag
/// is missing, a length is not one a run can have or a flag is not 0 or 1.
pub fn run_type_at(buf: &[u8], id_at: usize) -> Option<RunType> {
    let tag_at = id_at.checked_add(RUN_TYPE_TAG_OFFSET)?;
    if buf.get(tag_at..tag_at + RUN_TYPE_OBJECT_TAG.len())? != RUN_TYPE_OBJECT_TAG {
        return None;
    }
    let length = |offset: usize, limit_feet: f64| {
        let value = read_f64(buf, id_at + offset)?;
        (value.is_finite() && (0.0..limit_feet).contains(&value)).then_some(value)
    };
    let flags = id_at + RUN_TYPE_FLAGS_OFFSET;
    Some(RunType {
        structural_depth_feet: length(RUN_TYPE_STRUCTURAL_DEPTH_OFFSET, 10.0)?,
        tread_thickness_feet: length(RUN_TYPE_TREAD_THICKNESS_OFFSET, 2.0)?,
        riser_thickness_feet: length(RUN_TYPE_RISER_THICKNESS_OFFSET, 2.0)?,
        nosing_length_feet: length(RUN_TYPE_NOSING_LENGTH_OFFSET, 2.0)?,
        tread_under_riser: flag(buf, id_at + RUN_TYPE_TREAD_UNDER_RISER_OFFSET)?,
        monolithic: flag(buf, flags)?,
        treads: flag(buf, flags + 1)?,
        risers: flag(buf, flags + 2)?,
        slanted_risers: flag(buf, flags + 3)?,
        name: utf16_at(buf, flags + 4),
    })
}

/// Each run type in `ids` read from every partition. A type whose copies
/// disagree is dropped; the map is empty for a release this layout is not
/// measured on.
pub fn scan_run_types(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, RunType>> {
    let mut out: BTreeMap<u32, Option<RunType>> = BTreeMap::new();
    if !supports_revit_version(revit_version) || ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (id, id_at) in crate::partition_id_objects::find_id_objects(buf, ids) {
            if let Some(found) = run_type_at(buf, id_at) {
                merge(&mut out, id, found);
            }
        }
    }
    Ok(settled(out))
}

/// Stair types, landing types and support types are serialised as run
/// types are (RE-52): `01 00 00 00 · u64 id`, with [`RUN_TYPE_OBJECT_TAG`]
/// at [`RUN_TYPE_TAG_OFFSET`]. Each kind keeps its system family and name
/// at its own offsets from the id (RE-65, Revit 2024, Snowdon Towers).
///
/// A stair type's construction, `u32`: 0 on the types of all 23 stairs
/// Revit's export names Assembled Stair, 1 on all 3 it names Cast-In-Place
/// Stair. 2 is on the type named "Precast Stair", which no stair uses.
pub const STAIR_TYPE_CONSTRUCTION_OFFSET: usize = 0x129;
/// A stair type's component types: seven `u64` ElementId slots, 8 bytes
/// apart, unset as all ones. On Snowdon's stair types the first is the run
/// type, the second the landing type and the rest support types (two side
/// stringers, then carriages).
pub const STAIR_TYPE_COMPONENTS_OFFSET: usize = 0xc9;
/// How many slots [`STAIR_TYPE_COMPONENTS_OFFSET`] holds.
pub const STAIR_TYPE_COMPONENT_SLOTS: usize = 7;
/// A stair type's parameter entries: `u32` count, then per entry an `i64`
/// BuiltInParameter and a string (`u32` length, UTF-16), such as the
/// Uniformat description "Interiors". The type's name follows them.
pub const STAIR_TYPE_PARAMETERS_OFFSET: usize = 0x13f;
/// A landing type's one-byte flag: 0 on the one type Snowdon's landings use,
/// which Revit names Non-Monolithic Landing, 1 on two unused types.
pub const LANDING_TYPE_MONOLITHIC_OFFSET: usize = 0x95;
/// A landing type's name, `u32` length and UTF-16.
pub const LANDING_TYPE_NAME_OFFSET: usize = 0x98;
/// A support type's one-byte flag: 0 on the two types Revit names Stringer,
/// 1 on the two it names Carriage.
pub const SUPPORT_TYPE_CARRIAGE_OFFSET: usize = 0x89;
/// A support type's name, `u32` length and UTF-16.
pub const SUPPORT_TYPE_NAME_OFFSET: usize = 0x92;

/// The kinds of stair component type [`component_type_at`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// A stair's own type (`OST_Stairs`).
    Stair,
    /// A landing's type (`OST_StairsLandings`).
    Landing,
    /// A stringer's or carriage's type (`OST_StairsStringerCarriage`).
    Support,
}

/// A stair, landing or support type's system family and name (RE-65).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentType {
    /// Revit's system family for the type, where the type's flag has a
    /// value measured against Revit's own export; `None` otherwise.
    pub family: Option<&'static str>,
    /// The type's name.
    pub name: String,
    /// A stair type's run, landing and support types
    /// ([`STAIR_TYPE_COMPONENTS_OFFSET`]); empty for other kinds.
    pub components: BTreeSet<u32>,
}

fn type_name_text(name: String) -> Option<String> {
    (!name.trim().is_empty() && !name.chars().any(char::is_control)).then_some(name)
}

/// A stair type's name: the string after its parameter entries.
fn stair_type_name(buf: &[u8], at: usize) -> Option<String> {
    let count = read_u32(buf, at)?;
    if count > 16 {
        return None;
    }
    let mut at = at + 4;
    for _ in 0..count {
        let parameter = i64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?);
        if parameter >= 0 {
            return None;
        }
        let length = usize::try_from(read_u32(buf, at + 8)?).ok()?;
        if length > 256 {
            return None;
        }
        at += 12 + 2 * length;
    }
    utf16_at(buf, at)
}

/// The stair component type of `kind` whose ElementId starts at `id_at`, or
/// `None` when the tag is missing, a flag has a value not seen or the name
/// does not read.
pub fn component_type_at(buf: &[u8], id_at: usize, kind: ComponentKind) -> Option<ComponentType> {
    let tag_at = id_at.checked_add(RUN_TYPE_TAG_OFFSET)?;
    if buf.get(tag_at..tag_at + RUN_TYPE_OBJECT_TAG.len())? != RUN_TYPE_OBJECT_TAG {
        return None;
    }
    let mut components = BTreeSet::new();
    let (family, name) = match kind {
        ComponentKind::Stair => {
            // Every slot is an ElementId or unset. Type 54873 (on Snowdon and
            // on the MIT house, whose stair has no runs, landings or
            // supports) holds f64 values there instead, and is not read.
            for slot in 0..STAIR_TYPE_COMPONENT_SLOTS {
                let at = id_at + STAIR_TYPE_COMPONENTS_OFFSET + 8 * slot;
                let id = u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?);
                if id == u64::MAX {
                    continue;
                }
                components.insert(u32::try_from(id).ok()?);
            }
            let family = match read_u32(buf, id_at + STAIR_TYPE_CONSTRUCTION_OFFSET)? {
                0 => Some("Assembled Stair"),
                1 => Some("Cast-In-Place Stair"),
                2 => None,
                _ => return None,
            };
            (
                family,
                stair_type_name(buf, id_at + STAIR_TYPE_PARAMETERS_OFFSET)?,
            )
        }
        ComponentKind::Landing => {
            let family = if flag(buf, id_at + LANDING_TYPE_MONOLITHIC_OFFSET)? {
                None
            } else {
                Some("Non-Monolithic Landing")
            };
            (family, utf16_at(buf, id_at + LANDING_TYPE_NAME_OFFSET)?)
        }
        ComponentKind::Support => {
            let family = if flag(buf, id_at + SUPPORT_TYPE_CARRIAGE_OFFSET)? {
                "Carriage"
            } else {
                "Stringer"
            };
            (
                Some(family),
                utf16_at(buf, id_at + SUPPORT_TYPE_NAME_OFFSET)?,
            )
        }
    };
    Some(ComponentType {
        family,
        name: type_name_text(name)?,
        components,
    })
}

/// Each stair component type of `kind` in `ids`, read from every
/// partition. A type whose copies disagree is dropped; the map is empty for
/// a release this layout is not measured on.
pub fn scan_component_types(
    rf: &mut RevitFile,
    revit_version: u32,
    kind: ComponentKind,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, ComponentType>> {
    let mut out: BTreeMap<u32, Option<ComponentType>> = BTreeMap::new();
    if !supports_revit_version(revit_version) || ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (id, id_at) in crate::partition_id_objects::find_id_objects(buf, ids) {
            if let Some(found) = component_type_at(buf, id_at, kind) {
                merge(&mut out, id, found);
            }
        }
    }
    Ok(settled(out))
}

/// How far into a run's data its StairsRun record is looked for.
pub const RUN_ENDS_WINDOW: usize = 0x1000;
/// Bytes from the record's first `f64` to its flags (RE-92): seven `f64`
/// and a `u32`.
pub const RUN_ENDS_FLAGS_OFFSET: usize = 60;
/// A top riser index at or above this is not one a run has.
pub const RUN_TOP_RISER_INDEX_LIMIT: u32 = 300;

/// How a run starts and ends (RE-92): its "Begin with Riser" and "End with
/// Riser" settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunEnds {
    /// The run's first step is a riser.
    pub begin_with_riser: bool,
    /// The run's last step is a riser. When it is not, the run ends in a
    /// tread and has one riser line more than risers.
    pub end_with_riser: bool,
}

/// The ends of the run whose StairsRun record lies in `data` (RE-92). The
/// record is seven `f64` (bottom elevation, top elevation, extend below
/// base, extend below tread base, run width, left and right stringer
/// width), a `u32` top riser index and three one-byte flags: centre mark
/// visible, begin with riser, end with riser. It is taken where its bottom
/// elevation and width are the sketch's, within
/// [`SKETCH_TOLERANCE_FEET`], and the first such record in the first
/// [`RUN_ENDS_WINDOW`] bytes wins.
pub fn run_ends_at(data: &[u8], base_elevation_feet: f64, width_feet: f64) -> Option<RunEnds> {
    let end = data.len().min(RUN_ENDS_WINDOW);
    (0..end.saturating_sub(RUN_ENDS_FLAGS_OFFSET + 2)).find_map(|at| {
        let value = |index: usize| read_f64(data, at + 8 * index);
        let (bottom, top, width) = (value(0)?, value(1)?, value(4)?);
        if (bottom - base_elevation_feet).abs() >= SKETCH_TOLERANCE_FEET
            || (width - width_feet).abs() >= SKETCH_TOLERANCE_FEET
            || !(top.is_finite() && top > bottom)
            || read_u32(data, at + 56)? >= RUN_TOP_RISER_INDEX_LIMIT
        {
            return None;
        }
        let flags = at + RUN_ENDS_FLAGS_OFFSET;
        flag(data, flags)?;
        Some(RunEnds {
            begin_with_riser: flag(data, flags + 1)?,
            end_with_riser: flag(data, flags + 2)?,
        })
    })
}

/// The ends of each run in `sketches`, from the StairsRun record in its
/// data, matched against the run's sketch (RE-92). Empty for a release this
/// layout is not measured on.
pub fn scan_run_ends(
    rf: &mut RevitFile,
    revit_version: u32,
    sketches: &BTreeMap<u32, RunSketch>,
) -> Result<BTreeMap<u32, RunEnds>> {
    let runs: BTreeSet<u32> = sketches.keys().copied().collect();
    scan_run_data(rf, revit_version, &runs, |id, data| {
        let sketch = sketches.get(&id)?;
        run_ends_at(data, sketch.origin[2], sketch.width_feet)
    })
}

/// Every bounded line in the data of each run in `runs`, in data order,
/// where every copy of the data holds the same lines. Empty for a release
/// this layout is not measured on.
pub fn scan_run_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    runs: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, Vec<BoundedLine>>> {
    use crate::partition_beam_axes::{BOUNDED_LINE_TAG, bounded_line_at};
    scan_run_data(rf, revit_version, runs, |_, data| {
        let lines: Vec<BoundedLine> = memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG)
            .filter_map(|at| bounded_line_at(data, at))
            .collect();
        (!lines.is_empty()).then_some(lines)
    })
}

/// What `read` finds in each run of `runs`, from every partition's copy of
/// the run's data. A run whose copies disagree is dropped; the map is empty
/// for a release this layout is not measured on.
fn scan_run_data<T: PartialEq>(
    rf: &mut RevitFile,
    revit_version: u32,
    runs: &BTreeSet<u32>,
    mut read: impl FnMut(u32, &[u8]) -> Option<T>,
) -> Result<BTreeMap<u32, T>> {
    use crate::partition_beam_axes::BEAM_DATA_WINDOW;
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if supports_revit_version(revit_version) => header,
        _ => return Ok(BTreeMap::new()),
    };
    let mut out: BTreeMap<u32, Option<T>> = BTreeMap::new();
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
                .filter(|id| runs.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(BEAM_DATA_WINDOW))
                .min(buf.len());
            let Some(data) = buf.get(id_at + 8..end) else {
                continue;
            };
            if let Some(found) = read(id, data) {
                merge(&mut out, id, found);
            }
        }
    }
    Ok(settled(out))
}

/// The side view of a run (RE-52, RE-92): one closed outline in the run's
/// vertical plane, as `[along, up]` feet from the sketch origin,
/// counter-clockwise. `end_with_riser` is the run's setting where it is
/// read ([`run_ends_at`]).
///
/// A run with separate treads and risers that ends with a riser has as many
/// risers as riser lines and a tread between each two; the last riser meets
/// the landing or floor above. One that ends with a tread (RE-92) has one
/// riser fewer, and its last tread runs to the last riser line. A
/// monolithic run is drawn by [`monolithic_side_profile`].
///
/// Riser `k` stands at riser line `k`, behind the nosing. Two
/// constructions are measured, and only those are drawn:
/// - a slanted riser whose front runs from the nosing of the tread above
///   back to the tread below, with each tread running on under the riser
///   above it;
/// - an upright riser that runs down behind the tread below, which stops
///   against it.
///
/// `None` for a run without treads or risers, the other two constructions,
/// a slanted run ending with a tread, and dimensions a real run cannot
/// have. Where `end_with_riser` is not read, a run with separate treads
/// and risers is taken to end with a riser, and a monolithic run is not
/// drawn.
pub fn run_side_profile(
    sketch: &RunSketch,
    run_type: &RunType,
    riser_height_feet: f64,
    end_with_riser: Option<bool>,
) -> Option<Vec<[f64; 2]>> {
    if run_type.monolithic {
        return end_with_riser
            .and_then(|_| monolithic_side_profile(sketch, run_type, riser_height_feet));
    }
    let end_with_riser = end_with_riser.unwrap_or(true);
    let (h, t, rt, n) = (
        riser_height_feet,
        run_type.tread_thickness_feet,
        run_type.riser_thickness_feet,
        run_type.nosing_length_feet,
    );
    let construction = (run_type.slanted_risers, run_type.tread_under_riser);
    if run_type.monolithic
        || !run_type.treads
        || !run_type.risers
        || !matches!(construction, (true, true) | (false, false))
        || !(h.is_finite() && h > 0.0)
        || !(t > 0.0 && t < h)
        || rt <= 0.0
        || n < 0.0
    {
        return None;
    }
    let slanted = run_type.slanted_risers;
    if slanted && !end_with_riser {
        return None;
    }
    // Where a riser's front meets the underside of the tread above it,
    // past that tread's front edge, and the riser's horizontal thickness.
    // A slanted riser leans back by the nosing over one riser height, so
    // it is thicker across than through.
    let (top_front, rh) = if slanted {
        (n * t / h, rt * h.hypot(n) / h)
    } else {
        (n, rt)
    };
    let u = &sketch.risers;
    if u.windows(2).any(|pair| pair[1] - pair[0] <= n + rh) {
        return None;
    }
    let count = if end_with_riser { u.len() } else { u.len() - 1 };
    let rise = |k: usize| k as f64 * h;
    let mut ring: Vec<[f64; 2]> = Vec::with_capacity(6 * count + 3);
    // Up the steps' faces: each riser's front from the tread below (or the
    // floor), then the front and top of the tread it carries. A slanted
    // riser and its tread are one folded plate, so the tread's front is the
    // riser's face carried up to the nosing.
    for k in 1..=count {
        let a = u[k - 1];
        ring.push([a + n, rise(k - 1)]);
        if k < count || !end_with_riser {
            if !slanted {
                ring.push([a + top_front, rise(k) - t]);
                ring.push([a, rise(k) - t]);
            }
            ring.push([a, rise(k)]);
        } else {
            ring.push([a + top_front, rise(k) - t]);
            ring.push([a + top_front + rh, rise(k) - t]);
        }
    }
    // A run ending with a tread: the top tread runs to the last riser line,
    // and under it back to the last riser.
    if !end_with_riser {
        ring.push([u[count], rise(count)]);
        ring.push([u[count], rise(count) - t]);
        ring.push([u[count - 1] + n + rh, rise(count) - t]);
    }
    // Down their backs: each riser's back to the tread below, then under
    // that tread to the riser below. A slanted riser's back runs on through
    // the tread it stands on; an upright one runs down behind the tread.
    for k in (1..=count).rev() {
        let back = u[k - 1] + n + rh;
        if k == 1 {
            ring.push([back, 0.0]);
            break;
        }
        let through = if slanted { top_front } else { 0.0 };
        ring.push([back + through, rise(k - 1) - t]);
        ring.push([u[k - 2] + top_front + rh, rise(k - 1) - t]);
    }
    ring.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12);
    let area: f64 = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
        .sum();
    if area < 0.0 {
        ring.reverse();
    }
    Some(ring)
}

/// The side view of a monolithic run (RE-92), as [`run_side_profile`]
/// draws one: a riser less than riser lines, whichever way the run ends.
/// Each riser leans from the step's inner corner, the nosing length behind
/// the riser line, up to its nosing on the line, and each tread runs from
/// its nosing to the next inner corner; the last runs to the last riser
/// line. The underside is parallel to the line through the inner corners,
/// the type's structural depth below it measured square to the pitch, and
/// is cut off by the floor the run starts on. The last riser line closes
/// the run with a vertical face.
///
/// `None` unless the type is monolithic with slanted risers and no separate
/// treads or risers, the one construction measured, or for dimensions a
/// real run cannot have.
pub fn monolithic_side_profile(
    sketch: &RunSketch,
    run_type: &RunType,
    riser_height_feet: f64,
) -> Option<Vec<[f64; 2]>> {
    let (h, n, depth) = (
        riser_height_feet,
        run_type.nosing_length_feet,
        run_type.structural_depth_feet,
    );
    if !run_type.monolithic
        || !run_type.slanted_risers
        || run_type.treads
        || run_type.risers
        || !(h.is_finite() && h > 0.0)
        || !(depth.is_finite() && depth > 0.0)
        || n < 0.0
    {
        return None;
    }
    let u = &sketch.risers;
    if u.windows(2).any(|pair| pair[1] - pair[0] <= n) {
        return None;
    }
    let count = u.len() - 1;
    let rise = |k: usize| k as f64 * h;
    let pitch = rise(count) / (u[count] - u[0]);
    // How far below the inner corners the underside lies, vertically.
    let drop = depth * pitch.hypot(1.0);
    let mut ring: Vec<[f64; 2]> = Vec::with_capacity(2 * count + 3);
    for k in 1..=count {
        ring.push([u[k - 1] + n, rise(k - 1)]);
        ring.push([u[k - 1], rise(k)]);
    }
    ring.push([u[count], rise(count)]);
    let end = (u[count] - u[0] - n) * pitch - drop;
    if end > 0.0 {
        ring.push([u[count], end]);
        ring.push([u[0] + n + drop / pitch, 0.0]);
    } else {
        ring.push([u[count], 0.0]);
    }
    let area: f64 = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
        .sum();
    if area < 0.0 {
        ring.reverse();
    }
    Some(ring)
}
