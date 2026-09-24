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
        for &id in ids {
            let mut pattern = [0u8; 12];
            pattern[0] = 1;
            pattern[4..].copy_from_slice(&u64::from(id).to_le_bytes());
            for at in memchr::memmem::find_iter(buf, &pattern) {
                if let Some(found) = run_type_at(buf, at + 4) {
                    merge(&mut out, id, found);
                }
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
        for &id in ids {
            let mut pattern = [0u8; 12];
            pattern[0] = 1;
            pattern[4..].copy_from_slice(&u64::from(id).to_le_bytes());
            for at in memchr::memmem::find_iter(buf, &pattern) {
                if let Some(found) = component_type_at(buf, at + 4, kind) {
                    merge(&mut out, id, found);
                }
            }
        }
    }
    Ok(settled(out))
}

/// Every bounded line in the data of each run in `runs`, in data order,
/// where every copy of the data holds the same lines. Empty for a release
/// this layout is not measured on.
pub fn scan_run_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    runs: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, Vec<BoundedLine>>> {
    use crate::partition_beam_axes::{BEAM_DATA_WINDOW, BOUNDED_LINE_TAG, bounded_line_at};
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if supports_revit_version(revit_version) => header,
        _ => return Ok(BTreeMap::new()),
    };
    let mut out: BTreeMap<u32, Option<Vec<BoundedLine>>> = BTreeMap::new();
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
            let lines: Vec<BoundedLine> = memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG)
                .filter_map(|at| bounded_line_at(data, at))
                .collect();
            if !lines.is_empty() {
                merge(&mut out, id, lines);
            }
        }
    }
    Ok(settled(out))
}

/// The side view of a run with separate treads and risers (RE-52): one
/// closed outline in the run's vertical plane, as `[along, up]` feet from
/// the sketch origin, counter-clockwise. The run has as many risers as
/// riser lines and a tread between each two; the last riser meets the
/// landing or floor above.
///
/// Riser `k` stands at riser line `k`, behind the nosing. Two
/// constructions are measured, and only those are drawn:
/// - a slanted riser whose front runs from the nosing of the tread above
///   back to the tread below, with each tread running on under the riser
///   above it;
/// - an upright riser that runs down behind the tread below, which stops
///   against it.
///
/// `None` for a monolithic run, a run without treads or risers, the other
/// two constructions, and dimensions a real run cannot have.
pub fn run_side_profile(
    sketch: &RunSketch,
    run_type: &RunType,
    riser_height_feet: f64,
) -> Option<Vec<[f64; 2]>> {
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
    let count = u.len();
    let rise = |k: usize| k as f64 * h;
    let mut ring: Vec<[f64; 2]> = Vec::with_capacity(6 * count);
    // Up the steps' faces: each riser's front from the tread below (or the
    // floor), then the front and top of the tread it carries. A slanted
    // riser and its tread are one folded plate, so the tread's front is the
    // riser's face carried up to the nosing.
    for k in 1..=count {
        let a = u[k - 1];
        ring.push([a + n, rise(k - 1)]);
        if k < count {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partition_names::ELEMENT_DATA_HEADER;

    /// Snowdon stair 620883's data up to its riser count: 19 risers of
    /// 11/19 ft, treads of 11 in, an 11 ft stair.
    fn stair_620883(shift: usize) -> Vec<u8> {
        let mut out = ELEMENT_DATA_HEADER.to_vec();
        out.extend_from_slice(&620_883u64.to_le_bytes());
        out.extend(vec![0xff; 0x45 + shift]);
        out.extend_from_slice(&STAIR_DIMENSIONS_ANCHOR);
        out.extend_from_slice(&(11.0f64 / 19.0).to_le_bytes());
        out.extend_from_slice(&(11.0f64 / 12.0).to_le_bytes());
        out.extend(vec![0u8; 24]);
        out.extend_from_slice(&11.0f64.to_le_bytes());
        out.extend(vec![0u8; 24]);
        out.extend_from_slice(&19u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out
    }

    #[test]
    fn stair_dimensions_follow_their_anchor() {
        for shift in [0, 8] {
            let buf = stair_620883(shift);
            let found = stair_dimensions_at(&buf, 0).expect("dimensions");
            assert_eq!(found.riser_count, 19);
            assert!((found.riser_height_feet * 0.3048 - 0.176_463_157_894_736_85).abs() < 1e-12);
            assert!((found.tread_depth_feet * 0.3048 - 0.2794).abs() < 1e-12);
        }
        // No anchor, no dimensions.
        let mut buf = stair_620883(0);
        let at = memchr::memmem::find(&buf, &STAIR_DIMENSIONS_ANCHOR).expect("anchor");
        buf[at + 4] = 0;
        assert_eq!(stair_dimensions_at(&buf, 0), None);
        // An anchor past the window is not the stair's.
        assert_eq!(stair_dimensions_at(&stair_620883(ANCHOR_WINDOW), 0), None);
    }

    #[test]
    fn a_run_records_its_risers() {
        // Snowdon run 621141: 10 risers.
        let mut buf = ELEMENT_DATA_HEADER.to_vec();
        buf.extend_from_slice(&621_141u64.to_le_bytes());
        buf.extend(vec![0xff; 0x2a]);
        buf.extend_from_slice(&STAIR_RUN_RISERS_ANCHOR);
        buf.extend_from_slice(&10u32.to_le_bytes());
        assert_eq!(run_riser_count_at(&buf, 0), Some(10));
        let offsets = data_offsets(&buf, &ELEMENT_DATA_HEADER, &[621_141]);
        assert_eq!(offsets.get(&621_141), Some(&0));
    }

    #[test]
    fn flight_counts_add_up_to_the_stair_or_are_not_claimed() {
        assert_eq!(
            flight_riser_counts(19, &[(621_141, Some(10)), (621_208, Some(9))]),
            BTreeMap::from([(621_141, 10), (621_208, 9)])
        );
        // A single run is the whole stair, whatever it records.
        assert_eq!(
            flight_riser_counts(2, &[(1_563_804, Some(3))]),
            BTreeMap::from([(1_563_804, 2)])
        );
        assert!(flight_riser_counts(19, &[(1, Some(10)), (2, Some(10))]).is_empty());
        assert!(flight_riser_counts(19, &[(1, Some(10)), (2, None)]).is_empty());
    }

    /// A level bounded line from `a` to `b`.
    fn line(a: [f64; 3], b: [f64; 3]) -> BoundedLine {
        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        BoundedLine {
            start_parameter: 0.0,
            end_parameter: length,
            origin: a,
            direction: [d[0] / length, d[1] / length, d[2] / length],
        }
    }

    /// Snowdon run 621141's sketch: 10 risers 11 in apart on a run 44 in
    /// wide, climbing +Y, then the riser lines again as the data repeats
    /// them.
    fn run_621141_lines() -> Vec<BoundedLine> {
        let (x0, x1, y0, z) = (-108.979_167, -105.3125, 25.6875, -16.916_667);
        let tread = 11.0 / 12.0;
        let y1 = y0 + 9.0 * tread;
        let mut lines = vec![
            line([x0, y0, z], [x0, y1, z]),
            line([x1, y0, z], [x1, y1, z]),
        ];
        for _ in 0..2 {
            for k in 0..10 {
                let y = y0 + k as f64 * tread;
                lines.push(line([x0, y, z], [x1, y, z]));
            }
        }
        lines
    }

    #[test]
    fn a_run_sketch_reads_its_boundary_and_riser_lines() {
        let sketch = run_sketch(&run_621141_lines()).expect("sketch");
        assert_eq!(sketch.risers.len(), 10);
        assert!(sketch.risers[0].abs() < 1e-12);
        assert!((sketch.risers[9] - 8.25).abs() < 1e-9);
        assert!((sketch.width_feet - 3.666_667).abs() < 1e-6);
        assert!((sketch.climb[1] - 1.0).abs() < 1e-12);
        assert!((sketch.across[0] - 1.0).abs() < 1e-12);

        // A first line that climbs is not a boundary line.
        let mut lines = run_621141_lines();
        lines[0] = line([0.0, 0.0, 0.0], [0.0, 8.25, 1.0]);
        assert_eq!(run_sketch(&lines), None);
        // A first boundary line that stops short of the last riser.
        let mut lines = run_621141_lines();
        lines.retain(|l| (l.start()[1] - (25.6875 + 8.25)).abs() > 1e-6 || l.direction[0] == 0.0);
        assert_eq!(run_sketch(&lines), None);
        // Without its second boundary line the run has no width.
        let mut lines = run_621141_lines();
        lines.remove(1);
        assert_eq!(run_sketch(&lines), None);
    }

    /// Snowdon run type 613941's object: 2.5 in structural depth, 1/4 in
    /// tread and riser, 1 in nosing, treads and slanted risers, each tread
    /// running on under the riser above, and its name.
    fn run_type_613941() -> Vec<u8> {
        let mut buf = vec![1u8, 0, 0, 0];
        buf.extend_from_slice(&613_941u64.to_le_bytes());
        buf.resize(4 + 0x160, 0xff);
        let id_at = 4;
        buf[id_at + RUN_TYPE_TAG_OFFSET..id_at + RUN_TYPE_TAG_OFFSET + 2]
            .copy_from_slice(&RUN_TYPE_OBJECT_TAG);
        for (offset, value) in [
            (RUN_TYPE_STRUCTURAL_DEPTH_OFFSET, 2.5 / 12.0),
            (RUN_TYPE_STRUCTURAL_DEPTH_OFFSET + 8, 0.0),
            (RUN_TYPE_TREAD_THICKNESS_OFFSET, 0.25 / 12.0),
            (RUN_TYPE_RISER_THICKNESS_OFFSET, 0.25 / 12.0),
            (RUN_TYPE_NOSING_LENGTH_OFFSET, 1.0 / 12.0),
        ] {
            buf[id_at + offset..id_at + offset + 8].copy_from_slice(&f64::to_le_bytes(value));
        }
        buf[id_at + RUN_TYPE_TREAD_UNDER_RISER_OFFSET] = 1;
        let flags = id_at + RUN_TYPE_FLAGS_OFFSET;
        buf[flags..flags + 4].copy_from_slice(&[0, 1, 1, 1]);
        let name = "1/4\" Tread 1\" Nosing 1/4\" Riser";
        buf[flags + 4..flags + 8].copy_from_slice(&(name.len() as u32).to_le_bytes());
        let units: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        buf[flags + 8..flags + 8 + units.len()].copy_from_slice(&units);
        buf
    }

    #[test]
    fn a_run_type_reads_its_construction() {
        let buf = run_type_613941();
        let found = run_type_at(&buf, 4).expect("run type");
        assert!((found.tread_thickness_feet * 12.0 - 0.25).abs() < 1e-12);
        assert!((found.riser_thickness_feet * 12.0 - 0.25).abs() < 1e-12);
        assert!((found.nosing_length_feet * 12.0 - 1.0).abs() < 1e-12);
        assert!((found.structural_depth_feet * 12.0 - 2.5).abs() < 1e-12);
        assert!(!found.monolithic && found.treads && found.risers);
        assert!(found.slanted_risers && found.tread_under_riser);
        assert_eq!(
            found.name.as_deref(),
            Some("1/4\" Tread 1\" Nosing 1/4\" Riser")
        );
        // Without its tag the object is not a run type.
        let mut untagged = buf.clone();
        untagged[4 + RUN_TYPE_TAG_OFFSET] = 0;
        assert_eq!(run_type_at(&untagged, 4), None);
        // A flag byte that is not 0 or 1 is not one.
        let mut odd = buf;
        odd[4 + RUN_TYPE_FLAGS_OFFSET + 2] = 3;
        assert_eq!(run_type_at(&odd, 4), None);
    }

    fn upright_type() -> RunType {
        RunType {
            structural_depth_feet: 2.5 / 12.0,
            tread_thickness_feet: 2.0 / 12.0,
            riser_thickness_feet: 0.25 / 12.0,
            nosing_length_feet: 1.0 / 12.0,
            monolithic: false,
            treads: true,
            risers: true,
            slanted_risers: false,
            tread_under_riser: false,
            name: None,
        }
    }

    fn area(ring: &[[f64; 2]]) -> f64 {
        ring.iter()
            .zip(ring.iter().cycle().skip(1))
            .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn an_upright_riser_run_is_its_treads_and_risers() {
        let sketch = run_sketch(&run_621141_lines()).expect("sketch");
        let run_type = upright_type();
        let h = 0.570_833;
        let ring = run_side_profile(&sketch, &run_type, h).expect("profile");
        // Nine treads, each over its going and the nosing, and ten risers:
        // the first from the floor to the first tread, the others from the
        // underside of the tread below to the underside of the next.
        let (t, rt, n) = (2.0 / 12.0, 0.25 / 12.0, 1.0 / 12.0);
        let expected = 9.0 * (11.0 / 12.0 + n) * t + rt * ((h - t) + 9.0 * h);
        assert!((area(&ring) - expected).abs() < 1e-9, "{}", area(&ring));
        assert!(ring.contains(&[0.0, h]));
        assert!(ring.contains(&[n, 0.0]));
    }

    #[test]
    fn a_slanted_riser_run_is_one_folded_plate() {
        let sketch = run_sketch(&run_621141_lines()).expect("sketch");
        let buf = run_type_613941();
        let run_type = run_type_at(&buf, 4).expect("run type");
        let h = 11.0 / 19.0;
        let ring = run_side_profile(&sketch, &run_type, h).expect("profile");
        let (t, n) = (0.25 / 12.0, 1.0 / 12.0);
        let rh = (0.25 / 12.0) * h.hypot(n) / h;
        let near = |p: [f64; 2]| {
            ring.iter()
                .any(|q| (q[0] - p[0]).abs() < 1e-9 && (q[1] - p[1]).abs() < 1e-9)
        };
        // The first riser leans from behind the nosing at the floor to the
        // first tread's nosing; that tread's front is the riser's face.
        assert!(near([n, 0.0]) && near([0.0, h]));
        assert!(!near([0.0, h - t]));
        // The last riser ends under the landing.
        let top = 10.0 * h - t;
        assert!(near([8.25 + n * t / h, top]) && near([8.25 + n * t / h + rh, top]));
        // Every point lies within the run's steps.
        assert!(
            ring.iter()
                .all(|p| p[0] >= -1e-12 && p[1] >= -1e-12 && p[1] <= 10.0 * h)
        );
    }

    #[test]
    fn runs_rvt_rs_has_not_measured_are_not_drawn() {
        let sketch = run_sketch(&run_621141_lines()).expect("sketch");
        let h = 0.57;
        let mut monolithic = upright_type();
        monolithic.monolithic = true;
        assert_eq!(run_side_profile(&sketch, &monolithic, h), None);
        let mut no_risers = upright_type();
        no_risers.risers = false;
        assert_eq!(run_side_profile(&sketch, &no_risers, h), None);
        // A slanted riser behind the tread below is not a construction any
        // measured type has.
        let mut unmeasured = upright_type();
        unmeasured.slanted_risers = true;
        assert_eq!(run_side_profile(&sketch, &unmeasured, h), None);
        // Treads thicker than a riser is high cannot be.
        assert_eq!(run_side_profile(&sketch, &upright_type(), 0.1), None);
    }

    #[test]
    fn only_2024_is_read() {
        assert!(supports_revit_version(2024));
        assert!(!supports_revit_version(2025));
    }
}
