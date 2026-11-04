//! Revit 2024 Areas from their partition chain records (#763).
//!
//! An Area's chain record (RE-35) carries `OST_Areas` (-2003200) at
//! `+0x12` and no bounding-box marker at `+0x50`, so the element-record
//! scan, which finds records by that marker, never reads one. Its frame
//! holds RE-21's instance words: the container ElementId at `+0x32`, unset
//! (`ff` × 8) on every Area, and the placement kind at `+0x42`. After them
//! come two ascending lists of ElementIds, one of six (the Level, two ids of
//! the area scheme, two more and the Area's own) and one of two (the Level
//! and one more). Revit's export writes each placed Area as an `IfcSpace`
//! in its Level's storey: `Name` its number and `LongName` its name, which
//! RE-117's `ROOM_NUMBER` / `ROOM_NAME` entries hold as they do a room's.
//!
//! Measured on the BIM4LCA office (`research/bim4lca-measure.tsv`) against
//! Revit's IFC export, matched by Revit's GlobalId:
//!
//! - as built, 30 Areas, all placed, all 30 Revit's area spaces, with
//!   number, name and storey equal on 30 of 30;
//! - at building permit, 32 Areas. The 30 with the placed-instance kind are
//!   Revit's 30, with storey equal on 30 of 30. The other 2 carry
//!   `0xffff8020` and are not in Revit's export. Their copies in the latest
//!   partition (#800) give the number and name.
//!
//! The Level is the one recovered Level id the record names, as
//! [`crate::element_record_level_refs::unique_level_reference`] takes it from
//! an element record's list. Where an Area's record has copies in several
//! partitions, the latest partition's is read (RE-145). An Area in a
//! non-primary design option is left out, as an element record is (RE-40):
//! Revit's exporter collects spatial elements through the same design-option
//! filter as the rest (`ElementFilteringUtil.GetExportFilter` in revit-ifc,
//! CLEANROOM.md source 5), and skips an Area with no location.
//!
//! # The body (RE-317)
//!
//! Revit's export draws each area space as an `IfcExtrudedAreaSolid` of the
//! Area's boundary, from its Level up to the Level above, when both are
//! building stories. The boundary is not
//! in the Area. It is the region around the Area's point that the lines of
//! its area scheme on its Level enclose ([`crate::partition_area_boundaries`]),
//! and the point is two `f64`s at [`AREA_POINT_OFFSET`] in the payload of the
//! Area's data object (RE-153). On the BIM4LCA office all 30 Areas, as built
//! and at building permit, equal Revit's in area (within 0.1%) and in outline
//! (within 0.01 ft), and 29 of the 30 in the mesh's vertices
//! (`reports/element-framing/RE-317-area-boundaries.md`).
//!
//! # Honesty
//!
//! - An Area has a body only where its point lies in exactly one region of
//!   its scheme's lines on its Level, the lines it names bound that region,
//!   every line of the scheme on the Level is a straight line along the plan
//!   axes and in the primary design option, and every declared area boundary
//!   line of the file was read and placed on a Level. The lines its record
//!   names must be declared lines of that scheme on that Level whose copies
//!   agree. An Area whose region another placed Area of its scheme and Level
//!   shares has none, whatever stops the other from being drawn. Otherwise it
//!   has none, as before, and says why in [`AREA_BODY_REFUSAL_FIELD`].
//! - A line in a non-primary design option makes its group unreadable: no area
//!   boundary line of the office or of Autodesk's 2024 samples has a design
//!   option, so how Revit treats one is not measured.
//! - The height is the distance to the Level above, where that Level and the
//!   Area's own are building stories and no other Level lies between them or at
//!   either elevation. Revit's exporter takes the Level's "story above" first
//!   (`LEVEL_UP_TO_LEVEL`), which is not read, and ignores the Area's own
//!   upper Level; a Level whose story above is not the next Level up is not
//!   measured. An Area on the top storey has none.
//! - The base offset of the Area is read as zero, which is what the office
//!   measures.
//! - An Area bounded partly by walls is not read: its point lies in no region
//!   of the lines alone.
//! - Revit 2024 only, the one release measured; elsewhere nothing is read
//!   (fail closed).

use crate::RevitFile;
use crate::element_record_plan_profiles::{PLAN_PROFILE_SOURCE_FIELD, PlanProfile};
use crate::le_bytes;
use crate::partition_area_boundaries::{AreaFace, Arrangement, BoundaryLine};
use crate::partition_design_options::DesignOptions;
use crate::partition_element_records::{
    CATEGORY_OFFSET, CLASS_TAG_OFFSET, CONTAINER_NONE, CONTAINER_OFFSET, PLACEMENT_KIND_INSTANCE,
    PLACEMENT_KIND_OFFSET, file_bbox_marker, frame_design_option, partition_record_chain,
    scan_category_records_multi,
};
use crate::partition_level_records::PartitionLevel;
use crate::partition_room_parameters::{
    DATA_OBJECT_HEADER, LatestCopy, data_objects, partition_number,
};
use crate::walker::{DecodedElement, ElementProvenance, InstanceField};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where the Area record is measured.
pub const AREA_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Autodesk `BuiltInCategory.OST_Areas`.
pub const OST_AREAS: i64 = -2_003_200;

/// The `m_source` of an Area read here.
pub const AREA_RECORD_SOURCE: &str = "partition_area_record";

/// Autodesk `BuiltInCategory.OST_AreaSchemes`: an area scheme.
pub const OST_AREA_SCHEMES: i64 = -2_003_201;

/// Autodesk `BuiltInCategory.OST_AreaSchemeLines`: an area boundary line.
pub const OST_AREA_SCHEME_LINES: i64 = -2_000_079;

/// Where the point of an Area is in the payload of its data object, two
/// `f64`s of plan feet (RE-317).
pub const AREA_POINT_OFFSET: usize = 107;

/// The source of an Area's plan profile and of its body (RE-317).
pub const AREA_BOUNDARY_SOURCE: &str = "partition_area_boundary_lines";

/// The field of an Area that holds the elevation of its body's base, feet.
pub const AREA_BODY_BASE_FIELD: &str = "m_area_body_base";

/// The field of an Area that holds its body's height, feet.
pub const AREA_BODY_HEIGHT_FIELD: &str = "m_area_body_height";

/// How far a boundary line's height may lie from its Level's, feet.
pub const AREA_LEVEL_TOLERANCE_FEET: f64 = 1e-3;

/// The least distance to the Level above that gives an Area a body, feet.
pub const AREA_MIN_HEIGHT_FEET: f64 = 1e-3;

/// The lowest ElementId read as an area boundary line in an Area's record.
pub const AREA_LINE_ID_FLOOR: u32 = 1000;

/// The field of an Area that holds why it has no body, where it has none.
pub const AREA_BODY_REFUSAL_FIELD: &str = "m_area_body_refusal";

/// An Area has no body because the Levels were not recovered.
pub const REFUSED_LEVELS: &str = "levels_not_recovered";
/// A declared area boundary line was not read, or cannot be placed on a Level:
/// no Area of the file is drawn, since the line may be in any group.
pub const REFUSED_UNACCOUNTED: &str = "boundary_line_not_read";
/// The file holds no readable area boundary line.
pub const REFUSED_NO_LINES: &str = "no_boundary_lines";
/// The Area's data object holds no single point.
pub const REFUSED_NO_POINT: &str = "point_not_read";
/// The Area's record does not name one Level and one area scheme.
pub const REFUSED_UNNAMED: &str = "level_or_scheme_not_named";
/// A line the Area's record names is not a declared line of its scheme on its
/// Level that was read: it has no Level or scheme, its copies disagree, or it is
/// in a non-primary design option.
pub const REFUSED_LINES: &str = "named_lines_not_in_scheme_and_level";
/// The Area's Level and the Level above it are not both building stories, or
/// another Level lies at the Area's.
pub const REFUSED_HEIGHT: &str = "next_storey_not_proven";
/// The lines of the Area's scheme on its Level cannot be read as an arrangement
/// of straight lines along the axes, one of them may be missing, or one is in a
/// non-primary design option.
pub const REFUSED_GROUP: &str = "boundary_lines_not_readable";
/// The Area's point lies in no single region of the lines.
pub const REFUSED_REGION: &str = "point_in_no_single_region";
/// The lines the Area's record names do not bound its region.
pub const REFUSED_OFF_REGION: &str = "named_lines_not_on_region";
/// Another placed Area of the same scheme and Level has the same region,
/// whether or not that Area is drawn.
pub const REFUSED_REDUNDANT: &str = "two_areas_one_region";

/// One Area record as its latest partition holds it.
struct AreaRecord {
    stream: String,
    offset: usize,
    bytes: Vec<u8>,
}

/// Two copies of a record agree when their bytes do, wherever they sit.
impl PartialEq for AreaRecord {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

/// What the chain records of a file hold that an Area's body depends on.
struct ChainCensus {
    /// Every declared Area's chain record, its latest partition's copy; an
    /// Area whose copies in that partition differ is left out.
    areas: BTreeMap<u32, AreaRecord>,
    /// The ElementIds of the declared area schemes.
    scheme_ids: BTreeSet<u32>,
    /// The ElementIds of every declared area boundary line.
    line_ids: BTreeSet<u32>,
}

/// The chain records of the Areas, the area schemes and the area boundary
/// lines of a file.
fn area_records(rf: &mut RevitFile, marker: &[u8; 8], declared: &BTreeSet<u32>) -> ChainCensus {
    let mut found = LatestCopy::default();
    let mut schemes = BTreeSet::new();
    let mut line_ids = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let partition = partition_number(&stream);
        let buf = inflated.bytes();
        for span in partition_record_chain(buf, marker) {
            let Ok(id) = u32::try_from(span.element_id) else {
                continue;
            };
            if !declared.contains(&id) {
                continue;
            }
            let category = le_bytes::u64_at(buf, span.start + CATEGORY_OFFSET).map(|c| c as i64);
            if category == Some(OST_AREA_SCHEMES) {
                schemes.insert(id);
                continue;
            }
            if category == Some(OST_AREA_SCHEME_LINES) {
                line_ids.insert(id);
                continue;
            }
            if category != Some(OST_AREAS) {
                continue;
            }
            found.offer(
                id,
                partition,
                AreaRecord {
                    stream: stream.clone(),
                    offset: span.start,
                    bytes: buf[span.start..span.end].to_vec(),
                },
            );
        }
    }
    ChainCensus {
        areas: found.into_values(),
        scheme_ids: schemes,
        line_ids,
    }
}

/// Every id of `wanted` from `floor` up that `bytes` holds as a `u64` past the
/// frame's fixed head.
fn named_ids(bytes: &[u8], wanted: &BTreeSet<u32>, floor: u32) -> BTreeSet<u32> {
    let mut named = BTreeSet::new();
    for at in CATEGORY_OFFSET..bytes.len().saturating_sub(7) {
        if let Some(id) = le_bytes::u64_at(bytes, at)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|id| *id >= floor && wanted.contains(id))
        {
            named.insert(id);
        }
    }
    named
}

/// The one id of `wanted` that `bytes` holds as a `u64` past the frame's
/// fixed head, or `None` for none or several.
fn named_one(bytes: &[u8], wanted: &BTreeSet<u32>) -> Option<u32> {
    let mut ids = named_ids(bytes, wanted, 0).into_iter();
    match (ids.next(), ids.next()) {
        (Some(id), None) => Some(id),
        _ => None,
    }
}

/// One area boundary line as its record holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LineRecord {
    from: (f64, f64),
    to: (f64, f64),
    height: f64,
    key: (u32, u32),
}

/// The area boundary lines of a file, and what could not be read of them.
#[derive(Default)]
struct BoundaryLines {
    by_id: BTreeMap<u32, LineRecord>,
    /// Lines whose copies in several partitions disagree.
    conflicted: BTreeSet<u32>,
    /// The height of every line that is in no group it can be named in, and
    /// of both copies of a line whose copies disagree, feet: each could lie in
    /// any group on its Level.
    stray_heights: Vec<f64>,
    /// The ids of each scheme's lines on each Level, in the order met.
    groups: BTreeMap<(u32, u32), Vec<u32>>,
    /// Every line id the scan returned a record for, whatever became of it.
    seen: BTreeSet<u32>,
    /// A line whose height is not a number: it can be placed on no Level.
    unplaceable: bool,
    /// The groups that hold a line in a non-primary design option. Revit's
    /// handling of such a line is not measured (no area boundary line of the
    /// office or of Autodesk's 2024 samples has a design option), so the group
    /// is not read.
    option_groups: BTreeSet<(u32, u32)>,
}

/// What an Area's body is: its region, the base and the height.
struct AreaBody {
    face: AreaFace,
    base_feet: f64,
    height_feet: f64,
}

/// What is known of the file around its Areas.
struct Surroundings<'a> {
    declared: &'a BTreeSet<u32>,
    level_ids: &'a BTreeSet<u32>,
    scheme_ids: &'a BTreeSet<u32>,
    levels: &'a [PartitionLevel],
}

/// The one Level and the one area scheme a line's reference list names.
fn line_key(
    references: &[u64],
    level_ids: &BTreeSet<u32>,
    scheme_ids: &BTreeSet<u32>,
) -> Option<(u32, u32)> {
    let named: BTreeSet<u32> = references
        .iter()
        .filter_map(|reference| u32::try_from(*reference).ok())
        .collect();
    let mut levels = named.intersection(level_ids);
    let mut schemes = named.intersection(scheme_ids);
    match (levels.next(), levels.next(), schemes.next(), schemes.next()) {
        (Some(&level), None, Some(&scheme), None) => Some((level, scheme)),
        _ => None,
    }
}

/// Every area boundary line of the file, from its element record. A line in
/// a non-primary design option is left out, as an element is (RE-40).
fn boundary_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    surroundings: &Surroundings<'_>,
    options: &DesignOptions,
) -> BoundaryLines {
    let mut out = BoundaryLines::default();
    let Ok(records) = scan_category_records_multi(
        rf,
        revit_version,
        &[OST_AREA_SCHEME_LINES],
        surroundings.declared,
    ) else {
        return out;
    };
    for record in records {
        out.seen.insert(record.element_id);
        let [x0, y0, z0, x1, y1, z1] = record.bbox_feet;
        let finite = record.bbox_feet.iter().all(|v| v.is_finite());
        let key = line_key(
            &record.references,
            surroundings.level_ids,
            surroundings.scheme_ids,
        );
        if record
            .design_option
            .is_some_and(|option| options.is_non_primary(option))
        {
            if let Some(key) = key {
                out.option_groups.insert(key);
            } else if z0.is_finite() {
                out.stray_heights.push(z0);
            } else {
                out.unplaceable = true;
            }
            continue;
        }
        let usable = finite
            && (z1 - z0).abs() <= AREA_LEVEL_TOLERANCE_FEET
            && record.is_placed_instance()
            && !record.is_container_member();
        let (true, Some(key)) = (usable, key) else {
            if z0.is_finite() {
                out.stray_heights.push(z0);
            } else {
                out.unplaceable = true;
            }
            continue;
        };
        let line = LineRecord {
            from: (x0, y0),
            to: (x1, y1),
            height: z0,
            key,
        };
        match out.by_id.get(&record.element_id) {
            None => {
                out.by_id.insert(record.element_id, line);
                out.groups.entry(key).or_default().push(record.element_id);
            }
            Some(held) if *held == line => {}
            Some(held) => {
                out.stray_heights.push(held.height);
                out.stray_heights.push(line.height);
                out.conflicted.insert(record.element_id);
            }
        }
    }
    out
}

/// The point of each Area, from the payload of its data object: where the
/// object's class is the record's and every copy in the record's partition
/// gives the same point.
fn area_points(
    rf: &mut RevitFile,
    placed: &BTreeMap<u32, AreaRecord>,
) -> BTreeMap<u32, (f64, f64)> {
    let mut by_stream: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    for (&id, record) in placed {
        by_stream
            .entry(record.stream.as_str())
            .or_default()
            .insert(id);
    }
    let mut found: BTreeMap<u32, Option<(f64, f64)>> = BTreeMap::new();
    for (stream, ids) in by_stream {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (at, object) in data_objects(buf) {
            let Some(record) = placed.get(&object.element_id) else {
                continue;
            };
            if !ids.contains(&object.element_id)
                || le_bytes::u16_at(&record.bytes, CLASS_TAG_OFFSET).map(u32::from)
                    != Some(object.class)
            {
                continue;
            }
            let start = at + DATA_OBJECT_HEADER + AREA_POINT_OFFSET;
            if start + 16 > object.end {
                continue;
            }
            let point = match (
                le_bytes::f64_at(buf, start),
                le_bytes::f64_at(buf, start + 8),
            ) {
                (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Some((x, y)),
                _ => None,
            };
            match found.get(&object.element_id) {
                Some(held) if *held != point => {
                    found.insert(object.element_id, None);
                }
                Some(_) => {}
                None => {
                    found.insert(object.element_id, point);
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, point)| Some((id, point?)))
        .collect()
}

/// The elevation of the Level `level` and the distance up to the Level above
/// it, feet, where both are building stories and no other Level lies at either
/// elevation or between them (RE-317). Revit's exporter takes an Area's height
/// from its Level's "story above" where that is a building story, and else
/// from the next building story (`LevelUtil.CalculateDistanceToNextLevel` in
/// revit-ifc); the "story above" is not read, so a Level above that is not
/// known to be the next building story proves nothing.
fn level_and_storey_height(levels: &[PartitionLevel], level: u32) -> Option<(f64, f64)> {
    let base = levels
        .iter()
        .find(|candidate| candidate.element_id == level)?;
    if base.building_story != Some(true) || !base.elevation_feet.is_finite() {
        return None;
    }
    let mut next: Option<&PartitionLevel> = None;
    for other in levels {
        if other.element_id == level {
            continue;
        }
        if !other.elevation_feet.is_finite() {
            return None;
        }
        let gap = other.elevation_feet - base.elevation_feet;
        if gap.abs() <= AREA_MIN_HEIGHT_FEET {
            return None;
        }
        if gap < 0.0 {
            continue;
        }
        match next {
            Some(held)
                if (other.elevation_feet - held.elevation_feet).abs() <= AREA_MIN_HEIGHT_FEET =>
            {
                return None;
            }
            Some(held) if held.elevation_feet < other.elevation_feet => {}
            _ => next = Some(other),
        }
    }
    let next = next?;
    (next.building_story == Some(true)).then_some((
        base.elevation_feet,
        next.elevation_feet - base.elevation_feet,
    ))
}

/// The arrangement of the lines of one scheme on one Level, or `None` where a
/// line cannot be read or may be missing.
fn group_arrangement(
    lines: &BoundaryLines,
    key: (u32, u32),
    base_feet: f64,
) -> Option<Arrangement> {
    if lines
        .stray_heights
        .iter()
        .any(|height| (height - base_feet).abs() <= AREA_LEVEL_TOLERANCE_FEET)
    {
        return None;
    }
    if lines.option_groups.contains(&key) {
        return None;
    }
    let ids = lines.groups.get(&key)?;
    let mut group = Vec::with_capacity(ids.len());
    for id in ids {
        let line = lines.by_id.get(id)?;
        if lines.conflicted.contains(id)
            || (line.height - base_feet).abs() > AREA_LEVEL_TOLERANCE_FEET
        {
            return None;
        }
        group.push(BoundaryLine {
            id: *id,
            from: line.from,
            to: line.to,
        });
    }
    Arrangement::new(&group)
}

/// The Areas of a file that have a body, and why the others have none.
#[derive(Default)]
struct AreaBodies {
    drawn: BTreeMap<u32, AreaBody>,
    refused: BTreeMap<u32, &'static str>,
}

/// An area scheme on a Level.
type GroupKey = (u32, u32);

/// A scheme on a Level and the corners of a region in it, by their bits.
type RegionKey = (GroupKey, Vec<(u64, u64)>);

/// Where an Area's point lies among the lines of its scheme on its Level.
struct Located {
    key: GroupKey,
    /// The region, or why there is none: the lines cannot be read, or the point
    /// is in no single region of them.
    face: Result<AreaFace, &'static str>,
}

/// The elevation of the Level `level`, feet.
fn level_elevation(levels: &[PartitionLevel], level: u32) -> Option<f64> {
    let elevation = levels
        .iter()
        .find(|candidate| candidate.element_id == level)?
        .elevation_feet;
    elevation.is_finite().then_some(elevation)
}

/// The region of one Area, whatever else stops it from being drawn: its point
/// among the lines of its scheme on its Level. `Err` where it has no point, or
/// its Level or scheme is not named, or its Level's elevation is not known.
fn locate(
    record: &AreaRecord,
    point: Option<(f64, f64)>,
    lines: &BoundaryLines,
    surroundings: &Surroundings<'_>,
    arrangements: &mut BTreeMap<GroupKey, Option<Arrangement>>,
) -> Result<Located, &'static str> {
    let point = point.ok_or(REFUSED_NO_POINT)?;
    let (Some(level), Some(scheme)) = (
        named_one(&record.bytes, surroundings.level_ids),
        named_one(&record.bytes, surroundings.scheme_ids),
    ) else {
        return Err(REFUSED_UNNAMED);
    };
    let key = (level, scheme);
    let base_feet = level_elevation(surroundings.levels, level).ok_or(REFUSED_HEIGHT)?;
    let face = match arrangements
        .entry(key)
        .or_insert_with(|| group_arrangement(lines, key, base_feet))
    {
        Some(arrangement) => arrangement.face_at(point).ok_or(REFUSED_REGION),
        None => Err(REFUSED_GROUP),
    };
    Ok(Located { key, face })
}

/// The body of one Area: its region, where the lines its record names are all
/// declared lines of its scheme on its Level, are not conflicted and bound that
/// region, from its Level up to the Level above (RE-317); or why it has none.
fn draw_area(
    record: &AreaRecord,
    located: &Located,
    lines: &BoundaryLines,
    known_lines: &BTreeSet<u32>,
    surroundings: &Surroundings<'_>,
) -> Result<AreaBody, &'static str> {
    let named_lines = named_ids(&record.bytes, known_lines, AREA_LINE_ID_FLOOR);
    let named_in_group = named_lines.iter().all(|line| {
        !lines.conflicted.contains(line)
            && lines
                .by_id
                .get(line)
                .is_some_and(|held| held.key == located.key)
    });
    if named_lines.is_empty() || !named_in_group {
        return Err(REFUSED_LINES);
    }
    let (base_feet, height_feet) =
        level_and_storey_height(surroundings.levels, located.key.0).ok_or(REFUSED_HEIGHT)?;
    let face = located.face.as_ref().map_err(|reason| *reason)?;
    if !named_lines.is_subset(&face.line_ids) {
        return Err(REFUSED_OFF_REGION);
    }
    Ok(AreaBody {
        face: face.clone(),
        base_feet,
        height_feet,
    })
}

/// The body of each of `placed` that has one, and the reason for each that has
/// none (RE-317). An Area of a file with a declared area boundary line that
/// was not read, or cannot be placed on a Level, has none, since that line may
/// belong to any group. An Area whose region another placed Area of its scheme
/// and Level shares has none, whatever stops the other from being drawn:
/// Revit's second, redundant Area has no area of its own.
fn area_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    placed: &BTreeMap<u32, AreaRecord>,
    declared_lines: &BTreeSet<u32>,
    surroundings: &Surroundings<'_>,
    options: &DesignOptions,
) -> AreaBodies {
    let mut out = AreaBodies::default();
    if placed.is_empty() {
        return out;
    }
    let refuse_all = |out: &mut AreaBodies, reason: &'static str| {
        for &id in placed.keys() {
            out.refused.insert(id, reason);
        }
    };
    if surroundings.levels.is_empty() {
        refuse_all(&mut out, REFUSED_LEVELS);
        return out;
    }
    let lines = boundary_lines(rf, revit_version, surroundings, options);
    if lines.unplaceable || declared_lines.iter().any(|id| !lines.seen.contains(id)) {
        refuse_all(&mut out, REFUSED_UNACCOUNTED);
        return out;
    }
    if lines.by_id.is_empty() {
        refuse_all(&mut out, REFUSED_NO_LINES);
        return out;
    }
    let points = area_points(rf, placed);
    let mut arrangements: BTreeMap<GroupKey, Option<Arrangement>> = BTreeMap::new();
    let mut located: BTreeMap<u32, Result<Located, &'static str>> = BTreeMap::new();
    for (&id, record) in placed {
        let point = points.get(&id).copied();
        located.insert(
            id,
            locate(record, point, &lines, surroundings, &mut arrangements),
        );
    }
    // Each Area's region is located; the groups' arrangements are not needed
    // again, so they go before the regions are compared.
    drop(arrangements);
    // The lines an Area's record may name: those the chain declares, and those
    // the scan returned without a chain record of their own.
    let known_lines: BTreeSet<u32> = declared_lines
        .iter()
        .copied()
        .chain(lines.by_id.keys().copied())
        .collect();
    let mut sharing: BTreeMap<RegionKey, Vec<u32>> = BTreeMap::new();
    for (&id, place) in &located {
        let Ok(place) = place else {
            continue;
        };
        let Ok(face) = &place.face else {
            continue;
        };
        let outline = face
            .outer
            .iter()
            .map(|p| (p.0.to_bits(), p.1.to_bits()))
            .collect();
        sharing.entry((place.key, outline)).or_default().push(id);
    }
    let redundant: BTreeSet<u32> = sharing
        .values()
        .filter(|ids| ids.len() > 1)
        .flatten()
        .copied()
        .collect();
    for (&id, record) in placed {
        let result = match located.get(&id) {
            Some(Ok(place)) => draw_area(record, place, &lines, &known_lines, surroundings),
            Some(Err(reason)) => Err(*reason),
            None => Err(REFUSED_NO_POINT),
        };
        match result {
            Ok(_) if redundant.contains(&id) => {
                out.refused.insert(id, REFUSED_REDUNDANT);
            }
            Ok(body) => {
                out.drawn.insert(id, body);
            }
            Err(reason) => {
                out.refused.insert(id, reason);
            }
        }
    }
    out
}

/// Every placed Area of a Revit 2024 file, as a `DecodedElement` of class
/// `Area` with its number, name and Level where they are read, and its
/// boundary, base and height where they are proven (RE-317). Empty on every
/// other release.
pub fn areas_from_partition_chain_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
    levels: &[PartitionLevel],
) -> Vec<DecodedElement> {
    if !AREA_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Vec::new();
    }
    let Some(marker) = file_bbox_marker(rf, revit_version) else {
        return Vec::new();
    };
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Vec::new(),
    };
    let options = rf.memoised_design_options();
    let census = area_records(rf, &marker, &declared);
    let placed: BTreeMap<u32, AreaRecord> = census
        .areas
        .into_iter()
        .filter(|(_, record)| {
            le_bytes::u64_at(&record.bytes, CONTAINER_OFFSET) == Some(CONTAINER_NONE)
                && le_bytes::u32_at(&record.bytes, PLACEMENT_KIND_OFFSET)
                    == Some(PLACEMENT_KIND_INSTANCE)
                && !frame_design_option(&record.bytes, 0)
                    .is_some_and(|option| options.is_non_primary(option))
        })
        .collect();
    let ids: BTreeSet<u32> = placed.keys().copied().collect();
    let entries =
        crate::partition_room_parameters::scan_room_parameter_entries(rf, revit_version, &ids);
    let surroundings = Surroundings {
        declared: &declared,
        level_ids,
        scheme_ids: &census.scheme_ids,
        levels,
    };
    let mut bodies = area_bodies(
        rf,
        revit_version,
        &placed,
        &census.line_ids,
        &surroundings,
        &options,
    );
    placed
        .into_iter()
        .map(|(id, record)| {
            let level = named_one(&record.bytes, level_ids);
            let mut fields = vec![
                (
                    "m_builtinCategory".into(),
                    InstanceField::Integer {
                        value: OST_AREAS,
                        signed: true,
                        size: 8,
                    },
                ),
                (
                    "m_source_stream".into(),
                    InstanceField::String(record.stream.clone()),
                ),
                (
                    "m_source_offset".into(),
                    InstanceField::Integer {
                        value: record.offset as i64,
                        signed: false,
                        size: 8,
                    },
                ),
                (
                    "m_source".into(),
                    InstanceField::String(AREA_RECORD_SOURCE.into()),
                ),
                ("m_level_bound".into(), InstanceField::Bool(level.is_some())),
            ];
            if let Some(level) = level {
                fields.push((
                    crate::element_record_level_refs::LEVEL_REFERENCE_FIELD.into(),
                    InstanceField::ElementId { tag: 0, id: level },
                ));
            }
            if let Some((number, name)) = entries.get(&id) {
                fields.push(("m_name".into(), InstanceField::String(name.clone())));
                fields.push((
                    crate::partition_schema_mvp::ROOM_NUMBER_FIELD.into(),
                    InstanceField::String(number.clone()),
                ));
            }
            if let Some(reason) = bodies.refused.get(&id) {
                fields.push((
                    AREA_BODY_REFUSAL_FIELD.into(),
                    InstanceField::String((*reason).into()),
                ));
            }
            if let Some(body) = bodies.drawn.remove(&id) {
                let profile = PlanProfile {
                    outer_xy: body.face.outer,
                    inner_xy: body.face.holes,
                    segment_ids: body.face.line_ids.into_iter().collect(),
                    pieces: Vec::new(),
                    void_openings: Vec::new(),
                };
                for (name, mut value) in profile.fields() {
                    if name == PLAN_PROFILE_SOURCE_FIELD {
                        value = InstanceField::String(AREA_BOUNDARY_SOURCE.into());
                    }
                    fields.push((name, value));
                }
                fields.push((
                    AREA_BODY_BASE_FIELD.into(),
                    InstanceField::Float {
                        value: body.base_feet,
                        size: 8,
                    },
                ));
                fields.push((
                    AREA_BODY_HEIGHT_FIELD.into(),
                    InstanceField::Float {
                        value: body.height_feet,
                        size: 8,
                    },
                ));
            }
            DecodedElement {
                id: Some(id),
                class: "Area".into(),
                fields,
                byte_range: record.offset..record.offset + record.bytes.len(),
                provenance: ElementProvenance::partition(
                    &record.stream,
                    record.offset,
                    AREA_RECORD_SOURCE,
                    "partition_area_records::areas_from_partition_chain_records",
                    0.8,
                    level.is_none().then_some("level_binding_unresolved"),
                ),
            }
        })
        .collect()
}
