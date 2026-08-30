//! Schema / partition MVP recovers for production `iter_elements`.
//!
//! Extends the ArcWall-only partition merge with fail-closed recovers
//! for Level, Material, Room, Floor plan loops, and (on Revit 2024)
//! ArcWallRectOpening index rows plus `OST_Columns` element records
//! ([`columns_from_partition_category_records`]). Semantic `Door` / `Window` classes
//! are **not** invented from opening-index rows — those surface as
//! `ArcWallRectOpening` with related-id provenance only. Related ids
//! are cross-checked against `Global/ElemTable` when present; a hit
//! confirms the id is declared, not that it is a host Wall or a
//! Door/Window family instance.
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
                + self.columns.len(),
        );
        out.extend(self.levels);
        out.extend(self.materials);
        out.extend(self.rooms);
        out.extend(self.floors);
        out.extend(self.rect_openings);
        out.extend(self.columns);
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

    // --- 2024 OST_Columns partition element records (#204) ---
    out.columns = columns_from_partition_category_records(rf, revit_version)?;

    Ok(out)
}

/// Recover architectural column instances from partition element
/// records (M4-09 / #204).
///
/// Fail-closed pipeline, each step justified in
/// [`crate::partition_element_records`]:
///
/// 1. Every candidate record's leading `u64` must be an ElementId
///    declared in `Global/ElemTable`, and the record must carry the
///    fixed bbox marker — a random byte match cannot become a column.
/// 2. Records whose bbox is centred on the plan origin are family /
///    type definitions (the symbol's own envelope in family
///    coordinates), not placed instances, and are dropped.
/// 3. Instances sharing a project-coordinate footprint origin are a
///    superseded generation plus its replacement; Revit allocates
///    ElementIds monotonically, so the highest id in each footprint
///    group is the live element and the rest are dropped.
///
/// Nothing here invents an ElementId, a level binding, or a profile
/// shape: the emitted geometry is exactly the recorded bounding box.
pub fn columns_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
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
        crate::partition_element_records::OST_COLUMNS,
        &declared,
    )?;
    Ok(columns_from_records(records))
}

/// Instance selection over already-decoded category records — split
/// out so the rule is unit-testable without a corpus file.
pub fn columns_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> Vec<DecodedElement> {
    use crate::partition_element_records::PartitionElementRecord;
    use std::collections::BTreeMap;

    let mut by_footprint: BTreeMap<(i64, i64, i64), PartitionElementRecord> = BTreeMap::new();
    for record in records {
        if record.is_family_local() {
            continue;
        }
        let key = record.footprint_key();
        let replace = match by_footprint.get(&key) {
            Some(existing) => record.element_id > existing.element_id,
            None => true,
        };
        if replace {
            by_footprint.insert(key, record);
        }
    }
    let mut instances: Vec<PartitionElementRecord> = by_footprint.into_values().collect();
    instances.sort_by_key(|record| record.element_id);
    instances.iter().map(column_decoded).collect()
}

fn column_decoded(
    record: &crate::partition_element_records::PartitionElementRecord,
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
        class: "Column".into(),
        fields,
        byte_range: record.offset
            ..record
                .offset
                .saturating_add(crate::partition_element_records::RECORD_MIN_LEN),
        provenance: ElementProvenance::partition(
            &record.stream,
            record.offset,
            "partition_element_record",
            "partition_schema_mvp::column_category_record",
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
        crate::partition_element_records::PartitionElementRecord {
            stream: "Partitions/46".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0141,
            builtin_category: crate::partition_element_records::OST_COLUMNS,
            bbox_feet,
        }
    }

    #[test]
    fn column_selection_drops_family_local_type_envelopes() {
        let records = vec![
            column_record(5755, [-1.0, -1.0, 0.0, 1.0, 1.0, 9.0]),
            column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]),
        ];
        let out = columns_from_records(records);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, Some(20375));
        assert_eq!(out[0].class, "Column");
    }

    #[test]
    fn column_selection_keeps_newest_of_a_co_located_pair() {
        let records = vec![
            column_record(16347, [23.0, 109.0, 76.0, 25.0, 111.0, 91.0]),
            column_record(20375, [23.0, 109.0, 76.0, 25.0, 111.0, 90.33]),
            column_record(20376, [48.0, 109.0, 76.0, 50.0, 111.0, 90.33]),
        ];
        let out = columns_from_records(records);
        let ids: Vec<Option<u32>> = out.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![Some(20375), Some(20376)]);
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
