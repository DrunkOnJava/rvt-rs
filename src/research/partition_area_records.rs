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
//! Area's boundary, from its Level up to the next storey. The boundary is not
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
//!   axes, and a building-story Level lies above its own. Otherwise it has
//!   none, as before.
//! - The base offset of the Area is read as zero and its height as the
//!   distance to the next storey, which is what the office measures; the
//!   Area's own "up to level" is not read. A Level is skipped as the next
//!   storey only where its Building Story setting is read as off, and all ten
//!   of the office's Levels are storeys, so that case is not measured.
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

/// The least distance to the next storey that gives an Area a body, feet.
pub const AREA_MIN_HEIGHT_FEET: f64 = 1e-3;

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

/// Every declared Area's chain record, its latest partition's copy; an Area
/// whose copies in that partition differ is left out. Beside them, the
/// ElementIds of the declared area schemes.
fn area_records(
    rf: &mut RevitFile,
    marker: &[u8; 8],
    declared: &BTreeSet<u32>,
) -> (BTreeMap<u32, AreaRecord>, BTreeSet<u32>) {
    let mut found = LatestCopy::default();
    let mut schemes = BTreeSet::new();
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
    (found.into_values(), schemes)
}

/// Every id of `wanted` that `bytes` holds as a `u64` past the frame's fixed
/// head.
fn named_ids(bytes: &[u8], wanted: &BTreeSet<u32>) -> BTreeSet<u32> {
    let mut named = BTreeSet::new();
    for at in CATEGORY_OFFSET..bytes.len().saturating_sub(7) {
        if let Some(id) = le_bytes::u64_at(bytes, at)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|id| wanted.contains(id))
        {
            named.insert(id);
        }
    }
    named
}

/// The one id of `wanted` that `bytes` holds as a `u64` past the frame's
/// fixed head, or `None` for none or several.
fn named_one(bytes: &[u8], wanted: &BTreeSet<u32>) -> Option<u32> {
    let mut ids = named_ids(bytes, wanted).into_iter();
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
    /// The height of every line that is in no group it can be named in, feet:
    /// it could lie in any group on its Level.
    stray_heights: Vec<f64>,
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
        if record
            .design_option
            .is_some_and(|option| options.is_non_primary(option))
        {
            continue;
        }
        let [x0, y0, z0, x1, y1, z1] = record.bbox_feet;
        let finite = record.bbox_feet.iter().all(|v| v.is_finite());
        let key = line_key(
            &record.references,
            surroundings.level_ids,
            surroundings.scheme_ids,
        );
        let usable = finite
            && (z1 - z0).abs() <= AREA_LEVEL_TOLERANCE_FEET
            && record.is_placed_instance()
            && !record.is_container_member();
        let (true, Some(key)) = (usable, key) else {
            if z0.is_finite() {
                out.stray_heights.push(z0);
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
            }
            Some(held) if *held == line => {}
            Some(_) => {
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

/// The elevation of the Level `level` and the distance up to the next
/// building-story Level, feet: Revit draws an Area from its Level to the next
/// storey (RE-317).
fn level_and_storey_height(levels: &[PartitionLevel], level: u32) -> Option<(f64, f64)> {
    let base = levels
        .iter()
        .find(|candidate| candidate.element_id == level)?
        .elevation_feet;
    if !base.is_finite() {
        return None;
    }
    let next = levels
        .iter()
        .filter(|candidate| {
            candidate.building_story != Some(false)
                && candidate.elevation_feet.is_finite()
                && candidate.elevation_feet > base + AREA_MIN_HEIGHT_FEET
        })
        .map(|candidate| candidate.elevation_feet)
        .fold(f64::INFINITY, f64::min);
    next.is_finite().then_some((base, next - base))
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
    let mut group = Vec::new();
    for (&id, line) in &lines.by_id {
        if line.key != key {
            continue;
        }
        if lines.conflicted.contains(&id)
            || (line.height - base_feet).abs() > AREA_LEVEL_TOLERANCE_FEET
        {
            return None;
        }
        group.push(BoundaryLine {
            id,
            from: line.from,
            to: line.to,
        });
    }
    Arrangement::new(&group)
}

/// The body of each of `placed` that has one: the region its point lies in
/// among the lines of its scheme on its Level, where the lines it names bound
/// that region, from its Level to the next storey (RE-317).
fn area_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    placed: &BTreeMap<u32, AreaRecord>,
    surroundings: &Surroundings<'_>,
    options: &DesignOptions,
) -> BTreeMap<u32, AreaBody> {
    let mut out = BTreeMap::new();
    if placed.is_empty() || surroundings.levels.is_empty() || surroundings.scheme_ids.is_empty() {
        return out;
    }
    let lines = boundary_lines(rf, revit_version, surroundings, options);
    if lines.by_id.is_empty() {
        return out;
    }
    let line_ids: BTreeSet<u32> = lines.by_id.keys().copied().collect();
    let points = area_points(rf, placed);
    let mut arrangements: BTreeMap<(u32, u32), Option<Arrangement>> = BTreeMap::new();
    for (&id, record) in placed {
        let Some(point) = points.get(&id).copied() else {
            continue;
        };
        let (Some(level), Some(scheme)) = (
            named_one(&record.bytes, surroundings.level_ids),
            named_one(&record.bytes, surroundings.scheme_ids),
        ) else {
            continue;
        };
        let named_lines = named_ids(&record.bytes, &line_ids);
        let key = (level, scheme);
        if named_lines.is_empty()
            || !named_lines
                .iter()
                .all(|line| lines.by_id.get(line).is_some_and(|held| held.key == key))
        {
            continue;
        }
        let Some((base_feet, height_feet)) = level_and_storey_height(surroundings.levels, level)
        else {
            continue;
        };
        let arrangement = arrangements
            .entry(key)
            .or_insert_with(|| group_arrangement(&lines, key, base_feet));
        let Some(face) = arrangement
            .as_ref()
            .and_then(|arrangement| arrangement.face_at(point))
        else {
            continue;
        };
        if !named_lines.is_subset(&face.line_ids) {
            continue;
        }
        out.insert(
            id,
            AreaBody {
                face,
                base_feet,
                height_feet,
            },
        );
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
    let (records, scheme_ids) = area_records(rf, &marker, &declared);
    let placed: BTreeMap<u32, AreaRecord> = records
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
        scheme_ids: &scheme_ids,
        levels,
    };
    let mut bodies = area_bodies(rf, revit_version, &placed, &surroundings, &options);
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
            if let Some(body) = bodies.remove(&id) {
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
