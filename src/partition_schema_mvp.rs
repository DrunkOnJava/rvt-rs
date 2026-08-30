//! Schema / partition MVP recovers for production `iter_elements`.
//!
//! Extends the ArcWall-only partition merge with fail-closed recovers
//! for Level, Material, Room, Floor plan loops, and (on Revit 2024)
//! ArcWallRectOpening index rows plus `OST_Columns` / `OST_Walls` /
//! `OST_Doors` / `OST_Windows` element records
//! ([`instances_from_partition_category_records`]) and
//! `OST_Floors` / `OST_BuildingPad` slab instances
//! ([`slabs_from_partition_category_records`], #212 / RE-22), which
//! supersede the plan-loop floors on files where they decode.
//!
//! Semantic `Door` / `Window` classes are still **not** invented from
//! opening-index rows — those keep surfacing as `ArcWallRectOpening`
//! with related-id provenance only, and RE-19's negative (no
//! discriminator in the opening-index bytes, no schema-field Wall)
//! stands untouched. Typed `Wall` / `Door` / `Window` come from a
//! different carrier: the element record's own `BuiltInCategory`
//! field, which names the category outright (#211). Opening-index
//! related ids are cross-checked against `Global/ElemTable` when
//! present; a hit confirms the id is declared, not that it is a host
//! Wall or a Door/Window family instance.
//!
//! # Version guard
//!
//! Partition byte scans reuse [`crate::partition_scanner`] /
//! [`crate::partition_arc_walls`] / [`crate::rect_opening_index`]
//! version gates. Unsupported releases yield empty lists.
//!
//! # Honesty
//!
//! - Never invent ElementIds, host walls, elevations, or geometry.
//! - Tier1 synthetics have no partition building elements — callers
//!   must observe zero Level/Material/Floor/opening hits there.
//! - Floor boundaries require closed plan polylines that survive
//!   ArcWall-centerline exclusion and area thresholds (RE-15-07).
//! - The plan-loop floors and the record-backed slabs are two views
//!   of the same plates, so they are never emitted together: the
//!   loops stand down when records decode, and remain the only floor
//!   path where they do not.

use crate::compression;
use crate::partition_arc_walls::{self, PartitionArcWall};
use crate::partition_name_candidates::{
    NameBucket, building_storey_name_candidates, classify_name, collect_name_candidates,
};
use crate::rect_opening_index::ArcWallRectOpeningIndex;
use crate::walker::{DecodedElement, ElementProvenance, InstanceField, WalkerLimits};
use crate::{Result, RevitFile};
use std::collections::BTreeSet;

/// Minimum plan span (feet) for a floor-loop candidate.
const FLOOR_MIN_SPAN_FEET: f64 = 5.0;
/// Minimum shoelace area (ft²) for a floor-loop candidate.
const FLOOR_MIN_AREA_SQFT: f64 = 25.0;
/// ArcWall endpoint exclusion epsilon (feet).
const ARCWALL_EXCLUDE_EPS: f64 = 0.05;

/// Bundle of partition-derived MVP `DecodedElement`s.
#[derive(Debug, Clone, Default)]
pub struct PartitionSchemaMvp {
    pub levels: Vec<DecodedElement>,
    pub materials: Vec<DecodedElement>,
    pub rooms: Vec<DecodedElement>,
    pub floors: Vec<DecodedElement>,
    /// 2024 ArcWallRectOpening index rows — not typed Door/Window.
    pub rect_openings: Vec<DecodedElement>,
    /// 2024 `OST_Columns` partition element records (M4-09 / #204).
    pub columns: Vec<DecodedElement>,
    /// 2024 `OST_Walls` partition element records (#211).
    pub walls: Vec<DecodedElement>,
    /// 2024 `OST_Doors` partition element records (#211).
    pub doors: Vec<DecodedElement>,
    /// 2024 `OST_Windows` partition element records (#211).
    pub windows: Vec<DecodedElement>,
    /// 2024 `OST_Floors` / `OST_BuildingPad` partition element
    /// records (#212, RE-22). When this is non-empty it supersedes
    /// [`Self::floors`] — see [`recover_partition_schema_mvp`].
    pub slabs: Vec<DecodedElement>,
}

impl PartitionSchemaMvp {
    /// Flatten in a stable order for merging into `iter_elements`.
    pub fn into_elements(self) -> Vec<DecodedElement> {
        let mut out = Vec::with_capacity(
            self.levels.len()
                + self.materials.len()
                + self.rooms.len()
                + self.floors.len()
                + self.rect_openings.len()
                + self.columns.len()
                + self.walls.len()
                + self.doors.len()
                + self.windows.len()
                + self.slabs.len(),
        );
        out.extend(self.levels);
        out.extend(self.materials);
        out.extend(self.rooms);
        out.extend(self.floors);
        out.extend(self.rect_openings);
        out.extend(self.columns);
        out.extend(self.walls);
        out.extend(self.doors);
        out.extend(self.windows);
        out.extend(self.slabs);
        out
    }
}

/// Recover partition MVP elements for a file (version-gated, fail-closed).
pub fn recover_partition_schema_mvp(
    rf: &mut RevitFile,
    revit_version: u32,
    limits: WalkerLimits,
) -> Result<PartitionSchemaMvp> {
    let mut out = PartitionSchemaMvp::default();

    // --- Levels + Materials + Rooms from partition strings / ArcWall elev ---
    let strings = crate::object_graph::string_records_from_partitions(rf).unwrap_or_default();
    let string_values: Vec<&str> = strings.iter().map(|r| r.value.as_str()).collect();

    let level_names = building_storey_name_candidates(string_values.iter().copied());
    let name_set = collect_name_candidates(string_values.iter().copied());

    let walls = match partition_arc_walls::scan_partition_arc_walls_with_limits(
        rf,
        revit_version,
        limits,
    ) {
        Ok(scan) => scan.walls,
        Err(_) => Vec::new(),
    };

    out.levels = levels_from_storeys_and_names(&walls, &level_names);
    out.materials = materials_from_names(&name_set);
    out.rooms = rooms_from_names(&name_set);

    // --- Floor plan loops (ArcWall-excluded) ---
    out.floors = floors_from_partition_plan_loops(rf, &walls, limits)?;

    // --- 2024 opening index (not Door/Window) ---
    if ArcWallRectOpeningIndex::supports_revit_version(revit_version) {
        out.rect_openings = rect_openings_from_partitions(rf, revit_version, limits)?;
    }

    // --- 2024 partition element records (#204 columns, #211 the rest) ---
    out.columns = columns_from_partition_category_records(rf, revit_version)?;
    out.walls = instances_from_partition_category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_WALLS,
        "Wall",
    )?;
    out.doors = instances_from_partition_category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_DOORS,
        "Door",
    )?;
    out.windows = instances_from_partition_category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_WINDOWS,
        "Window",
    )?;

    // --- 2024 slab instances from element records (#212, RE-22) ---
    //
    // Record-backed slabs carry an ElementId, a model bounding box,
    // a measured thickness and a storey, none of which the plan-loop
    // scan can supply; emitting both would double-count the same
    // plate. So the loops stand down whenever records were recovered,
    // and stay the only floor path on releases (2023 and earlier) and
    // files where no element record decodes.
    out.slabs = slabs_from_partition_category_records(rf, revit_version)?;
    if !out.slabs.is_empty() {
        out.floors.clear();
    }

    Ok(out)
}

/// Recover architectural column instances from partition element
/// records (M4-09 / #204, instance rule replaced in #211).
///
/// Thin wrapper over [`instances_from_partition_category_records`]
/// for `OST_Columns`.
pub fn columns_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<Vec<DecodedElement>> {
    instances_from_partition_category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_COLUMNS,
        "Column",
    )
}

/// Recover placed element instances of one `BuiltInCategory` from
/// partition element records (#211).
///
/// Fail-closed pipeline, each step justified in
/// [`crate::partition_element_records`]:
///
/// 1. Every candidate record's leading `u64` must be an ElementId
///    declared in `Global/ElemTable`, and the record must carry the
///    fixed bbox marker — a random byte match cannot become an
///    element.
/// 2. The record must be a standalone placed instance:
///    [`crate::partition_element_records::PartitionElementRecord::is_exported_instance`] — no container
///    reference at `+0x32`, placement kind at `+0x42` equal to
///    [`crate::partition_element_records::PLACEMENT_KIND_INSTANCE`].
///
/// Nothing here invents an ElementId, a level binding, or a profile
/// shape: the emitted geometry is exactly the recorded bounding box.
pub fn instances_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
    class: &str,
) -> Result<Vec<DecodedElement>> {
    if !crate::partition_element_records::supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let records = crate::partition_element_records::scan_category_records(
        rf,
        revit_version,
        builtin_category,
        &declared,
    )?;
    Ok(instances_from_records(records, class))
}

/// Instance selection over already-decoded category records — split
/// out so the rule is unit-testable without a corpus file.
///
/// The selector is the direct test
/// [`crate::partition_element_records::PartitionElementRecord::is_exported_instance`]. It supersedes
/// the family-local bbox proxy plus highest-id-per-footprint collapse
/// that #204 shipped for columns: both reproduced the 256 exported
/// columns, only the direct test also reproduces the exact exported
/// id sets for walls, doors and windows (#211).
pub fn instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    class: &str,
) -> Vec<DecodedElement> {
    use crate::partition_element_records::PartitionElementRecord;
    use std::collections::BTreeMap;

    // One record per ElementId: a single element can be framed more
    // than once across partitions, and the frames can disagree on the
    // vertical extent. Keep the greatest z-extent — that is the
    // element's full body, and it is measured: against the reference
    // export's `IfcExtrudedAreaSolid.Depth`, first-by-(stream, offset)
    // agrees on 67 of 80 slabs and greatest-extent on 79 of 80 (the
    // 80th is exported as two stacked solids whose depths sum to the
    // recorded extent). The choice changes no wall, door, window or
    // column bounding box on `2024_Core_Interior.rvt` — 268 walls,
    // 132 doors and 88 columns carry more than one record and every
    // one of them agrees on the box (#212, RE-22). Ties fall back to
    // the first by (stream, offset), so the choice stays
    // deterministic and byte-anchored.
    let mut by_id: BTreeMap<u32, PartitionElementRecord> = BTreeMap::new();
    for record in records {
        if !record.is_exported_instance() {
            continue;
        }
        let better = match by_id.get(&record.element_id) {
            None => true,
            Some(existing) => {
                let candidate = z_extent_key(&record);
                let held = z_extent_key(existing);
                candidate > held
                    || (candidate == held
                        && (record.stream.as_str(), record.offset)
                            < (existing.stream.as_str(), existing.offset))
            }
        };
        if better {
            by_id.insert(record.element_id, record);
        }
    }
    by_id
        .values()
        .map(|record| element_record_decoded(record, class))
        .collect()
}

/// Vertical extent of a record's bounding box, quantised to 1e-4 ft
/// so two frames of the same element compare exactly.
fn z_extent_key(record: &crate::partition_element_records::PartitionElementRecord) -> i64 {
    let (_, _, dz) = record.extents_feet();
    if !dz.is_finite() {
        return i64::MIN;
    }
    (dz * 10_000.0).round() as i64
}

/// Recover slab instances from partition element records (#212, RE-22).
///
/// Two `BuiltInCategory` ids feed one class, because Revit's own
/// exporter maps both to `IfcSlab`:
/// [`crate::partition_element_records::OST_FLOORS`] (class `Floor`)
/// and [`crate::partition_element_records::OST_BUILDING_PAD`]
/// (class `BuildingPad`). The pad is kept as its own class rather
/// than relabelled a floor — the mapping to `IFCSLAB` happens in
/// [`crate::ifc::category_map`], where it is visible.
///
/// Per-element IFC export-type overrides
/// ([`crate::partition_ifc_export_overrides`]) are attached as the
/// `m_ifc_export_as` field. The decoder does not act on the value;
/// the IFC writer decides which values it is willing to honour.
pub fn slabs_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<Vec<DecodedElement>> {
    use crate::partition_element_records as per;

    if !per::supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let overrides = crate::partition_ifc_export_overrides::scan_ifc_export_overrides(
        rf,
        revit_version,
        &declared,
    )
    .unwrap_or_default();

    let mut out = Vec::new();
    for (category, class) in [
        (per::OST_FLOORS, "Floor"),
        (per::OST_BUILDING_PAD, "BuildingPad"),
    ] {
        let records = per::scan_category_records(rf, revit_version, category, &declared)?;
        for mut decoded in instances_from_records(records, class) {
            if let Some(id) = decoded.id {
                if let Some(value) = overrides.get(&id) {
                    decoded.fields.push((
                        "m_ifc_export_as".into(),
                        InstanceField::String(value.clone()),
                    ));
                }
            }
            out.push(decoded);
        }
    }
    Ok(out)
}

/// Back-compat alias for the #204 entry point.
pub fn columns_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> Vec<DecodedElement> {
    instances_from_records(records, "Column")
}

fn element_record_decoded(
    record: &crate::partition_element_records::PartitionElementRecord,
    class: &str,
) -> DecodedElement {
    let (cx, cy) = record.plan_centre_feet();
    let (dx, dy, dz) = record.extents_feet();
    let fields = vec![
        (
            "m_locationX".into(),
            InstanceField::Float { value: cx, size: 8 },
        ),
        (
            "m_locationY".into(),
            InstanceField::Float { value: cy, size: 8 },
        ),
        (
            "m_locationZ".into(),
            InstanceField::Float {
                value: record.bbox_feet[2],
                size: 8,
            },
        ),
        (
            "m_bboxWidth".into(),
            InstanceField::Float { value: dx, size: 8 },
        ),
        (
            "m_bboxDepth".into(),
            InstanceField::Float { value: dy, size: 8 },
        ),
        (
            "m_bboxHeight".into(),
            InstanceField::Float { value: dz, size: 8 },
        ),
        (
            "m_builtinCategory".into(),
            InstanceField::Integer {
                value: record.builtin_category,
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
            InstanceField::String("partition_element_record".into()),
        ),
        // Base/top Level ElementIds stay unrecovered (#86); the
        // extrusion height below is the recorded bbox extent, not a
        // level-to-level span.
        ("m_level_bound".into(), InstanceField::Bool(false)),
    ];
    DecodedElement {
        id: Some(record.element_id),
        class: class.into(),
        fields,
        byte_range: record.offset
            ..record
                .offset
                .saturating_add(crate::partition_element_records::RECORD_MIN_LEN),
        provenance: ElementProvenance::partition(
            &record.stream,
            record.offset,
            "partition_element_record",
            "partition_schema_mvp::element_category_record",
            0.8,
            Some("level_binding_unresolved"),
        ),
    }
}

fn levels_from_storeys_and_names(
    walls: &[PartitionArcWall],
    level_names: &[String],
) -> Vec<DecodedElement> {
    let recovery = partition_arc_walls::recover_storeys_from_arc_walls(walls, level_names);
    if recovery.storeys.is_empty() {
        // Elevation-less files: still surface named building storeys as
        // Level rows with name only (elevation Absent for geometry).
        return level_names
            .iter()
            .enumerate()
            .map(|(i, name)| level_decoded(name, None, i))
            .collect();
    }
    recovery
        .storeys
        .into_iter()
        .enumerate()
        .map(|(i, s)| level_decoded(&s.name, Some(s.elevation_feet), i))
        .collect()
}

fn level_decoded(name: &str, elevation: Option<f64>, index: usize) -> DecodedElement {
    let mut fields = vec![
        ("m_name".into(), InstanceField::String(name.to_string())),
        ("m_isBuildingStory".into(), InstanceField::Bool(true)),
        (
            "m_source".into(),
            InstanceField::String("partition_schema_mvp".into()),
        ),
    ];
    if let Some(elev) = elevation {
        fields.push((
            "m_elevation".into(),
            InstanceField::Float {
                value: elev,
                size: 8,
            },
        ));
    }
    let confidence = if elevation.is_some() { 0.75 } else { 0.55 };
    DecodedElement {
        id: None,
        class: "Level".into(),
        fields,
        byte_range: index..index,
        provenance: ElementProvenance::partition(
            "partition",
            index,
            "partition_schema_mvp",
            "partition_schema_mvp::level",
            confidence,
            if elevation.is_none() {
                Some("elevation_unknown")
            } else {
                None
            },
        ),
    }
}

fn materials_from_names(name_set: &BTreeSet<(NameBucket, String)>) -> Vec<DecodedElement> {
    name_set
        .iter()
        .filter(|(b, n)| *b == NameBucket::MaterialLike && is_strict_material_name(n))
        .enumerate()
        .map(|(i, (_, name))| {
            DecodedElement {
                id: None,
                class: "Material".into(),
                fields: vec![
                    ("m_name".into(), InstanceField::String(name.clone())),
                    (
                        "m_source".into(),
                        InstanceField::String("partition_schema_mvp".into()),
                    ),
                ],
                byte_range: i..i,
                provenance: Default::default(),
            }
            .with_provenance(ElementProvenance::partition(
                "partition",
                i,
                "partition_schema_mvp",
                "partition_schema_mvp::material_name",
                0.6,
                None::<String>,
            ))
        })
        .collect()
}

fn rooms_from_names(name_set: &BTreeSet<(NameBucket, String)>) -> Vec<DecodedElement> {
    name_set
        .iter()
        .filter(|(b, n)| *b == NameBucket::SpaceLike && is_strict_room_name(n))
        .enumerate()
        .map(|(i, (_, name))| {
            DecodedElement {
                id: None,
                class: "Room".into(),
                fields: vec![
                    ("m_name".into(), InstanceField::String(name.clone())),
                    (
                        "m_source".into(),
                        InstanceField::String("partition_schema_mvp".into()),
                    ),
                ],
                byte_range: i..i,
                provenance: Default::default(),
            }
            .with_provenance(ElementProvenance::partition(
                "partition",
                i,
                "partition_schema_mvp",
                "partition_schema_mvp::room_name",
                0.6,
                None::<String>,
            ))
        })
        .collect()
}

/// Reject schedule / schema / compound material path noise.
pub fn is_strict_material_name(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 || t.len() > 48 {
        return false;
    }
    if t.contains(':') || t.contains('/') || t.contains('\\') || t.contains('.') {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.contains("schema") || lower.ends_with(" material") || lower.contains("default") {
        return false;
    }
    // Keep classify_name's material bucket; just tighten.
    classify_name(t) == Some(NameBucket::MaterialLike)
}

/// Reject occupancy schedules and long program labels.
pub fn is_strict_room_name(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 || t.len() > 32 {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower.contains("occupancy")
        || lower.contains("lighting")
        || lower.contains(" am ")
        || lower.contains(" pm")
        || lower.contains("facility")
        || lower.contains('/')
    {
        return false;
    }
    classify_name(t) == Some(NameBucket::SpaceLike)
}

fn partition_streams_largest_first(rf: &mut RevitFile) -> Vec<String> {
    let mut streams: Vec<(usize, String)> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .filter_map(|s| {
            let raw = rf.read_stream(&s).ok()?;
            Some((raw.len(), s))
        })
        .collect();
    streams.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    streams.into_iter().map(|(_, s)| s).collect()
}

fn floors_from_partition_plan_loops(
    rf: &mut RevitFile,
    walls: &[PartitionArcWall],
    limits: WalkerLimits,
) -> Result<Vec<DecodedElement>> {
    let mut arc_endpoints: Vec<(f64, f64)> = Vec::new();
    for wall in walls {
        let (sx, sy, _) = wall.record.start_point();
        let (ex, ey, _) = wall.record.end_point();
        arc_endpoints.push((sx, sy));
        arc_endpoints.push((ex, ey));
    }

    let mut seen = BTreeSet::new();
    let mut floors = Vec::new();
    let mut scanned: u64 = 0;
    // Cap unique floors — plan-loop scan is heuristic; a handful is
    // enough for geometry P0 progress without flooding iter_elements.
    let floor_cap = limits.max_candidates.min(64);

    for stream in partition_streams_largest_first(rf) {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(&stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        scanned = scanned.saturating_add(concat.len() as u64);
        if scanned > limits.max_scan_bytes as u64 && floors.is_empty() {
            // Still allow the first (largest) stream even if over budget.
        } else if scanned > limits.max_scan_bytes as u64 {
            break;
        }

        for candidate in scan_closed_plan_loops(&concat) {
            if floors.len() >= floor_cap {
                break;
            }
            if loop_matches_arcwall(&candidate.vertices_xy, &arc_endpoints) {
                continue;
            }
            let key = loop_dedup_key(&candidate.vertices_xy);
            if !seen.insert(key) {
                continue;
            }
            let area = polygon_area_xy(&candidate.vertices_xy);
            if area < FLOOR_MIN_AREA_SQFT {
                continue;
            }
            let span_x = candidate.span_x;
            let span_y = candidate.span_y;
            if span_x < FLOOR_MIN_SPAN_FEET || span_y < FLOOR_MIN_SPAN_FEET {
                continue;
            }
            floors.push(floor_decoded(
                &stream,
                candidate.offset,
                &candidate.vertices_xy,
                area,
            ));
        }
        if floors.len() >= floor_cap {
            break;
        }
        // After the largest stream, stop if we already have floors —
        // additional partitions rarely add unique building plates and
        // dominate runtime on large 2024 files.
        if !floors.is_empty() {
            break;
        }
    }

    Ok(floors)
}

fn floor_decoded(
    stream: &str,
    offset: usize,
    vertices_xy: &[(f64, f64)],
    area: f64,
) -> DecodedElement {
    let mut point_fields = Vec::with_capacity(vertices_xy.len());
    for &(x, y) in vertices_xy {
        point_fields.push(InstanceField::Vector(vec![
            InstanceField::Float { value: x, size: 8 },
            InstanceField::Float { value: y, size: 8 },
        ]));
    }
    DecodedElement {
        id: None,
        class: "Floor".into(),
        fields: vec![
            ("m_boundary".into(), InstanceField::Vector(point_fields)),
            (
                "m_area".into(),
                InstanceField::Float {
                    value: area,
                    size: 8,
                },
            ),
            (
                "m_source_stream".into(),
                InstanceField::String(stream.to_string()),
            ),
            (
                "m_source_offset".into(),
                InstanceField::Integer {
                    value: offset as i64,
                    signed: false,
                    size: 8,
                },
            ),
            (
                "m_source".into(),
                InstanceField::String("partition_plan_loop".into()),
            ),
            (
                // Plan-loop recoveries are not yet joined to Floor
                // ElementIds (AnalyticalModelSlab / ElemTable bind
                // remains open — nearby u32 hits are ambiguous).
                "m_elem_table_bound".into(),
                InstanceField::Bool(false),
            ),
        ],
        byte_range: offset..offset.saturating_add(vertices_xy.len().saturating_mul(16)),
        provenance: ElementProvenance::partition(
            stream,
            offset,
            "partition_schema_mvp",
            "partition_schema_mvp::floor_plan_loop",
            0.65,
            Some("elem_table_unbound"),
        ),
    }
}

struct PlanLoop {
    offset: usize,
    vertices_xy: Vec<(f64, f64)>,
    span_x: f64,
    span_y: f64,
}

fn scan_closed_plan_loops(buf: &[u8]) -> Vec<PlanLoop> {
    let mut found = Vec::new();
    let nmin = 4usize;
    let nmax = 8usize;
    let step = if buf.len() > 5_000_000 { 64 } else { 16 };
    let limit = buf.len().saturating_sub(nmax * 16 + 16);
    let mut i = 0usize;
    while i < limit {
        for n in nmin..=nmax {
            let need = n * 16;
            if i + need + 16 > buf.len() {
                break;
            }
            let mut pts = Vec::with_capacity(n);
            let mut ok = true;
            for k in 0..n {
                let Some(x) = read_f64(buf, i + k * 16) else {
                    ok = false;
                    break;
                };
                let Some(y) = read_f64(buf, i + k * 16 + 8) else {
                    ok = false;
                    break;
                };
                if !plan_coord(x) || !plan_coord(y) || (x.abs() < 1e-9 && y.abs() < 1e-9) {
                    ok = false;
                    break;
                }
                pts.push((x, y));
            }
            if !ok || pts.len() < nmin {
                continue;
            }
            let unique = unique_plan_vertices(&pts);
            if unique.len() < 3 {
                continue;
            }
            let (x0, y0) = pts[0];
            let (xn, yn) = pts[pts.len() - 1];
            let mut closed_err = (xn - x0).hypot(yn - y0);
            if let (Some(nx), Some(ny)) = (read_f64(buf, i + n * 16), read_f64(buf, i + n * 16 + 8))
            {
                closed_err = closed_err.min((nx - x0).hypot(ny - y0));
            }
            if closed_err > 0.05 {
                continue;
            }
            let min_x = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let max_x = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let min_y = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let max_y = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            let span_x = max_x - min_x;
            let span_y = max_y - min_y;
            if span_x < FLOOR_MIN_SPAN_FEET || span_y < FLOOR_MIN_SPAN_FEET {
                continue;
            }
            found.push(PlanLoop {
                offset: i,
                vertices_xy: unique,
                span_x,
                span_y,
            });
            break;
        }
        i += step;
    }
    found
}

fn loop_matches_arcwall(verts: &[(f64, f64)], endpoints: &[(f64, f64)]) -> bool {
    if endpoints.is_empty() {
        return false;
    }
    let mut hits = 0usize;
    for &(x, y) in verts {
        if endpoints
            .iter()
            .any(|&(ex, ey)| (ex - x).hypot(ey - y) < ARCWALL_EXCLUDE_EPS)
        {
            hits += 1;
        }
    }
    // Contaminated when ≥2 vertices sit on ArcWall endpoints.
    hits >= 2
}

fn loop_dedup_key(verts: &[(f64, f64)]) -> String {
    let mut parts: Vec<String> = verts
        .iter()
        .map(|(x, y)| format!("{:.2},{:.2}", x, y))
        .collect();
    parts.sort();
    parts.join("|")
}

fn unique_plan_vertices(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for &p in pts {
        if out
            .iter()
            .any(|&(x, y)| (x - p.0).abs() < 1e-6 && (y - p.1).abs() < 1e-6)
        {
            continue;
        }
        out.push(p);
    }
    out
}

fn polygon_area_xy(pts: &[(f64, f64)]) -> f64 {
    if pts.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..pts.len() {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % pts.len()];
        sum += x0 * y1 - x1 * y0;
    }
    (sum * 0.5).abs()
}

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    if off + 8 > buf.len() {
        return None;
    }
    let v = f64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
    v.is_finite().then_some(v)
}

fn plan_coord(v: f64) -> bool {
    v.is_finite() && v.abs() < 500.0
}

fn rect_openings_from_partitions(
    rf: &mut RevitFile,
    revit_version: u32,
    limits: WalkerLimits,
) -> Result<Vec<DecodedElement>> {
    let mut out = Vec::new();
    let opening_cap = limits.max_candidates.min(5_000);

    // Confirm related ids against ElemTable when available — never invent
    // Door/Window classes from the index alone.
    let elem_ids: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => BTreeSet::new(),
    };

    // Largest partition first — 2024 Core Interior openings live in
    // the ~98 MiB Partitions/46 stream.
    for stream in partition_streams_largest_first(rf) {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(&stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        let offsets = ArcWallRectOpeningIndex::find_all_for_revit_version(revit_version, &concat);
        for off in offsets {
            if out.len() >= opening_cap {
                break;
            }
            let Ok(rec) = ArcWallRectOpeningIndex::decode(&concat, off) else {
                continue;
            };
            let a_in = elem_ids.contains(&rec.related_id_a);
            let b_in = elem_ids.contains(&rec.related_id_b);
            // related_id_a is the historical host *candidate* (RE-15);
            // ElemTable confirmation only proves the id is declared —
            // not that it is a Wall host or a Door/Window instance.
            let host_confirmed = a_in;
            out.push(DecodedElement {
                id: None,
                class: "ArcWallRectOpening".into(),
                fields: vec![
                    (
                        "m_index".into(),
                        InstanceField::Integer {
                            value: i64::from(rec.index),
                            signed: false,
                            size: 4,
                        },
                    ),
                    (
                        "m_related_id_a".into(),
                        InstanceField::ElementId {
                            tag: 0,
                            id: rec.related_id_a,
                        },
                    ),
                    (
                        "m_related_id_b".into(),
                        InstanceField::ElementId {
                            tag: 0,
                            id: rec.related_id_b,
                        },
                    ),
                    (
                        "m_related_id_a_in_elem_table".into(),
                        InstanceField::Bool(a_in),
                    ),
                    (
                        "m_related_id_b_in_elem_table".into(),
                        InstanceField::Bool(b_in),
                    ),
                    (
                        "m_host_id".into(),
                        InstanceField::ElementId {
                            tag: 0,
                            id: rec.related_id_a,
                        },
                    ),
                    (
                        "m_host_elem_table_confirmed".into(),
                        InstanceField::Bool(host_confirmed),
                    ),
                    (
                        "m_host_provenance".into(),
                        InstanceField::String(if host_confirmed {
                            "related_id_a_in_elem_table".into()
                        } else {
                            "related_id_a_unvalidated".into()
                        }),
                    ),
                    (
                        "m_source_stream".into(),
                        InstanceField::String(stream.clone()),
                    ),
                    (
                        "m_source_offset".into(),
                        InstanceField::Integer {
                            value: off as i64,
                            signed: false,
                            size: 8,
                        },
                    ),
                    (
                        "m_source".into(),
                        InstanceField::String("partition_rect_opening_index".into()),
                    ),
                ],
                byte_range: off..off
                    .saturating_add(crate::rect_opening_index::OPENING_INDEX_STRIDE),
                provenance: ElementProvenance::partition(
                    &stream,
                    off,
                    "partition_rect_opening_index",
                    "partition_schema_mvp::arcwall_rect_opening",
                    if host_confirmed { 0.7 } else { 0.55 },
                    if host_confirmed {
                        None
                    } else {
                        Some("related_id_a_unvalidated")
                    },
                ),
            });
        }
        // Openings concentrate in one large partition; stop once we
        // found any validated index rows.
        if !out.is_empty() {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_material_keeps_concrete_rejects_schema() {
        assert!(is_strict_material_name("Concrete"));
        assert!(is_strict_material_name("Masonry - Brick"));
        assert!(!is_strict_material_name("HardwoodSchema"));
        assert!(!is_strict_material_name("Glass/Glazing:Default:Glass"));
    }

    #[test]
    fn strict_room_keeps_lobby_rejects_schedule() {
        assert!(is_strict_room_name("Lobby"));
        assert!(is_strict_room_name("Office"));
        assert!(!is_strict_room_name(
            "Common Office Occupancy - 8 AM to 5 PM"
        ));
        assert!(!is_strict_room_name("Corridor/Transition"));
    }

    #[test]
    fn level_decoded_projects_elevation() {
        let el = level_decoded("Level 1", Some(10.0), 0);
        assert_eq!(el.class, "Level");
        let level = crate::elements::level::Level::from_decoded(&el);
        assert_eq!(level.name.as_deref(), Some("Level 1"));
        assert_eq!(level.elevation_feet, Some(10.0));
        assert_eq!(level.is_building_story, Some(true));
    }

    fn column_record(
        element_id: u32,
        bbox_feet: [f64; 6],
    ) -> crate::partition_element_records::PartitionElementRecord {
        element_record(
            element_id,
            crate::partition_element_records::OST_COLUMNS,
            bbox_feet,
        )
    }

    fn element_record(
        element_id: u32,
        builtin_category: i64,
        bbox_feet: [f64; 6],
    ) -> crate::partition_element_records::PartitionElementRecord {
        crate::partition_element_records::PartitionElementRecord {
            stream: "Partitions/46".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0141,
            builtin_category,
            container: crate::partition_element_records::CONTAINER_NONE,
            placement_kind: crate::partition_element_records::PLACEMENT_KIND_INSTANCE,
            bbox_feet,
        }
    }

    #[test]
    fn selection_keeps_the_greatest_vertical_extent_for_one_id() {
        // A `Floor:Floor 1` plate is framed twice on Core Interior:
        // Partitions/46 sees the 2 in topping, Partitions/55 the full
        // 1 ft slab. The export's extrusion depth is 1 ft (#212).
        let mut thin = element_record(
            70433,
            crate::partition_element_records::OST_FLOORS,
            [20.0, 25.0, 30.667, 167.0, 114.0, 30.833],
        );
        thin.stream = "Partitions/46".into();
        let mut full = element_record(
            70433,
            crate::partition_element_records::OST_FLOORS,
            [20.0, 25.0, 29.833, 167.0, 114.0, 30.833],
        );
        full.stream = "Partitions/55".into();
        for records in [
            vec![thin.clone(), full.clone()],
            vec![full.clone(), thin.clone()],
        ] {
            let out = instances_from_records(records, "Floor");
            assert_eq!(out.len(), 1);
            let height =
                out[0]
                    .fields
                    .iter()
                    .find_map(|(name, value)| match (name.as_str(), value) {
                        ("m_bboxHeight", InstanceField::Float { value, .. }) => Some(*value),
                        _ => None,
                    });
            assert!(
                height.is_some_and(|h| (h - 1.0).abs() < 1e-3),
                "expected the 1 ft frame, got {height:?}"
            );
        }
    }

    #[test]
    fn selection_tie_breaks_on_the_first_stream_and_offset() {
        let mut a = element_record(
            20311,
            crate::partition_element_records::OST_FLOORS,
            [9.0, 16.0, 75.833, 177.0, 123.0, 76.0],
        );
        a.stream = "Partitions/51".into();
        let mut b = a.clone();
        b.stream = "Partitions/46".into();
        let out = instances_from_records(vec![a, b], "Floor");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, Some(20311));
    }

    #[test]
    fn selection_drops_type_symbol_envelopes() {
        let mut symbol = column_record(5755, [-1.0, -1.0, 0.0, 1.0, 1.0, 9.0]);
        symbol.placement_kind = crate::partition_element_records::PLACEMENT_KIND_SYMBOL;
        let records = vec![
            symbol,
            column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]),
        ];
        let out = columns_from_records(records);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, Some(20375));
        assert_eq!(out[0].class, "Column");
    }

    #[test]
    fn selection_drops_container_members_not_the_higher_id() {
        // The #204 rule kept the highest ElementId per footprint origin.
        // The #211 rule keeps whichever record is standalone — here the
        // *lower* id, which is what the container reference dictates.
        let mut member = column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]);
        member.container = 16_229;
        let records = vec![
            column_record(16347, [23.0, 109.0, 76.0, 25.0, 111.0, 91.0]),
            member,
            column_record(20376, [48.0, 109.0, 76.0, 50.0, 111.0, 90.33]),
        ];
        let out = columns_from_records(records);
        let ids: Vec<Option<u32>> = out.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![Some(16347), Some(20376)]);
    }

    #[test]
    fn selection_labels_each_category_with_its_own_class() {
        for (category, class) in [
            (crate::partition_element_records::OST_WALLS, "Wall"),
            (crate::partition_element_records::OST_DOORS, "Door"),
            (crate::partition_element_records::OST_WINDOWS, "Window"),
        ] {
            let out = instances_from_records(
                vec![element_record(
                    4242,
                    category,
                    [0.0, 0.0, 0.0, 4.0, 1.0, 8.0],
                )],
                class,
            );
            assert_eq!(out.len(), 1);
            assert_eq!(out[0].class, class);
            assert_eq!(out[0].id, Some(4242));
            assert_eq!(
                out[0].provenance.decoder.as_deref(),
                Some("partition_schema_mvp::element_category_record")
            );
        }
    }

    #[test]
    fn selection_keeps_one_record_per_element_id() {
        let mut second = column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]);
        second.offset = 999_999;
        let out = columns_from_records(vec![
            column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]),
            second,
        ]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn column_decoded_carries_plan_centre_and_extents() {
        let out = columns_from_records(vec![column_record(
            20375,
            [23.0, 109.0, 76.0, 25.0, 111.0, 90.33],
        )]);
        let column = &out[0];
        let field = |name: &str| {
            column
                .fields
                .iter()
                .find(|(n, _)| n == name)
                .and_then(|(_, v)| match v {
                    InstanceField::Float { value, .. } => Some(*value),
                    _ => None,
                })
        };
        assert_eq!(field("m_locationX"), Some(24.0));
        assert_eq!(field("m_locationY"), Some(110.0));
        assert_eq!(field("m_locationZ"), Some(76.0));
        assert_eq!(field("m_bboxWidth"), Some(2.0));
        assert_eq!(field("m_bboxDepth"), Some(2.0));
        assert!((field("m_bboxHeight").unwrap() - 14.33).abs() < 1e-9);
        assert!(column.provenance.confidence >= 0.55);
    }

    #[test]
    fn floor_boundary_recovers_from_partition_decoded() {
        let floor = floor_decoded(
            "Partitions/5",
            100,
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)],
            80.0,
        );
        let outcome = crate::geometry::recover_floor_boundary(&floor);
        assert!(outcome.is_recovered());
        let loop_ = outcome.as_recovered().unwrap();
        assert!(loop_.vertices_xy.len() >= 3);
        assert!(loop_.area_sqft() > 0.0);
    }
}
