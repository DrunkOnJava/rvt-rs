//! Slab instances, sketched plan profiles and shaft voids.

use super::*;

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
/// Per-element IFC export-type overrides are attached later, with every
/// other element's, by `attach_ifc_export_overrides` (RE-45).
///
/// The plate's sketched plan profile
/// ([`crate::element_record_plan_profiles`]) is attached in the same
/// pass when it closes — the `OST_SketchLines` records that carry it
/// are read from the same stream sweep, so the profile costs one
/// extra needle search rather than a second inflate (#31, RE-25).
pub fn slabs_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    use crate::partition_element_records as per;

    if !per::supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(Vec::new()),
    };
    if declared.is_empty() {
        return Ok(Vec::new());
    }
    let categories = [
        per::OST_FLOORS,
        per::OST_BUILDING_PAD,
        per::OST_SKETCH_LINES,
    ];
    let scanned = per::scan_category_records_multi(rf, revit_version, &categories, &declared)?;
    let scanned = without_non_primary_options(rf, scanned);
    let sketch_lines: Vec<per::PartitionElementRecord> = scanned
        .iter()
        .filter(|record| record.builtin_category == per::OST_SKETCH_LINES)
        .cloned()
        .collect();
    // Each plate's box is the record its element is built from (RE-26: the
    // newest, tallest copy), not the first copy found: an outline is held
    // to it, and an edited slab keeps its older box in an older partition
    // (#824).
    let plates: BTreeMap<u32, [f64; 4]> = select_instance_records(
        scanned
            .iter()
            .filter(|record| {
                record.builtin_category == per::OST_FLOORS
                    || record.builtin_category == per::OST_BUILDING_PAD
            })
            .cloned()
            .collect(),
    )
    .into_iter()
    .map(|(id, record)| {
        let b = record.bbox_feet;
        (id, [b[0], b[1], b[3], b[4]])
    })
    .collect();
    let profiles =
        sketch_plan_profiles(rf, revit_version, &sketch_lines, &plates, &BTreeSet::new());

    let mut out = Vec::new();
    for (category, class) in [
        (per::OST_FLOORS, "Floor"),
        (per::OST_BUILDING_PAD, "BuildingPad"),
    ] {
        let records: Vec<per::PartitionElementRecord> = scanned
            .iter()
            .filter(|record| record.builtin_category == category)
            .cloned()
            .collect();
        for mut decoded in instances_from_records(records, class, level_ids) {
            if let Some(id) = decoded.id {
                if let Some(profile) = profiles.get(&id) {
                    decoded.fields.extend(profile.fields());
                }
            }
            out.push(decoded);
        }
    }
    Ok(out)
}

/// Sketched plan profiles of the elements in `owners` (their plan record
/// boxes, `[min x, min y, max x, max y]`), from their Sketch's curve list
/// where it has one (RE-97) or else the lines naming them, and their
/// `OST_SketchLines` records: the RE-25 solve over the records' boxes and,
/// where that does not close, the exact ends each sketch line's own data
/// carries (RE-50, [`crate::partition_beam_axes::scan_bounded_lines`]). A
/// line counts only when it is level and both its ends lie in its own
/// record's box; an element any of whose lines does not count keeps no
/// profile, except that a zero-length line on one of the others' ends is
/// left out (RE-95). A sketch line's curve may be an arc instead (RE-96,
/// [`crate::partition_beam_axes::scan_sketch_curves`]): it counts where it
/// lies in a horizontal plane and every point of it, drawn as chords, lies
/// in its record's box, and where its record does not also hold as a line.
/// An outline with an arc is kept only where it spans its element's record
/// box.
pub(super) fn sketch_plan_profiles(
    rf: &mut RevitFile,
    revit_version: u32,
    sketch_lines: &[crate::partition_element_records::PartitionElementRecord],
    owners: &BTreeMap<u32, [f64; 4]>,
    skip: &BTreeSet<u32>,
) -> BTreeMap<u32, crate::element_record_plan_profiles::PlanProfile> {
    let mut evidence = ProfileEvidence::default();
    let profiles =
        solve_sketch_plan_profiles(rf, revit_version, sketch_lines, owners, skip, &mut evidence);
    if rf.recording_value_sources() {
        note_profile_sources(rf, &profiles, &evidence);
    }
    profiles
}

/// The first frame of a sketch line, by frame order (#1368).
struct LineFrame {
    stream: String,
    offset: usize,
}

impl LineFrame {
    /// [`crate::partition_element_records::PartitionElementRecord::frame_order`].
    fn order(&self) -> (u32, &str, usize) {
        (
            crate::partition_room_parameters::partition_number(&self.stream),
            self.stream.as_str(),
            self.offset,
        )
    }
}

/// The frames of each sketched element's lines, by line (#1368).
type LineFrames = BTreeMap<u32, BTreeMap<u32, LineFrame>>;

/// What [`sketch_plan_profiles`] read, kept for the value sources while the
/// handle records them (#1368).
#[derive(Default)]
struct ProfileEvidence {
    /// Each sketched element's lines, with the frame the solve took for each.
    frames: LineFrames,
    /// The lines whose frames disagreed between partitions, so the current
    /// partition's was kept.
    current: BTreeSet<u32>,
    /// The elements whose outline was read from their lines' own ends.
    exact: BTreeSet<u32>,
}

/// Note, under each sketched element with a profile, the records the profile
/// was solved from (#1368): the frames of its sketch lines (`sketch_lines`),
/// and for an outline read from the lines' own ends the data of those lines
/// (`sketch_line_data`). The Sketch's own curve list is noted by
/// [`crate::element_record_plan_profiles::scan_sketch_curve_lists`].
fn note_profile_sources(
    rf: &mut RevitFile,
    profiles: &BTreeMap<u32, crate::element_record_plan_profiles::PlanProfile>,
    evidence: &ProfileEvidence,
) {
    use crate::ifc::element_provenance::{CopyRule, SourcePart, ValueSource};
    let mut new_notes: Vec<(u32, ValueSource)> = Vec::new();
    for (owner, profile) in profiles {
        let Some(frames) = evidence.frames.get(owner) else {
            continue;
        };
        let mut parts = Vec::new();
        let mut current = false;
        for id in &profile.segment_ids {
            let Some(frame) = frames.get(id) else {
                continue;
            };
            current |= evidence.current.contains(id);
            parts.push(SourcePart {
                element_id: *id,
                stream: frame.stream.clone(),
                offset: frame.offset,
            });
        }
        if !parts.is_empty() {
            let rule = if current {
                CopyRule::Current
            } else {
                CopyRule::Agreeing
            };
            let copies = parts.len();
            new_notes.push((
                *owner,
                ValueSource::from_parts("sketch_lines", rule, copies, parts),
            ));
        }
        if !evidence.exact.contains(owner) {
            continue;
        }
        let Some(noted) = rf.value_sources() else {
            continue;
        };
        let mut data_parts = Vec::new();
        let mut data_current = false;
        for id in &profile.segment_ids {
            let Some(note) = noted.get(id).and_then(|sources| {
                sources.iter().find(|source| {
                    matches!(
                        source.reader.as_str(),
                        "sketch_line" | "sketch_curve" | "sketch_circle"
                    )
                })
            }) else {
                continue;
            };
            let (Some(stream), Some(offset)) = (&note.stream, note.offset) else {
                continue;
            };
            data_current |= note.rule == CopyRule::Current;
            data_parts.push(SourcePart {
                element_id: *id,
                stream: stream.clone(),
                offset,
            });
        }
        if !data_parts.is_empty() {
            let rule = if data_current {
                CopyRule::Current
            } else {
                CopyRule::Agreeing
            };
            let copies = data_parts.len();
            new_notes.push((
                *owner,
                ValueSource::from_parts("sketch_line_data", rule, copies, data_parts),
            ));
        }
    }
    for (owner, source) in new_notes {
        rf.note_value_source(owner, source);
    }
}

/// The solve behind [`sketch_plan_profiles`], with what it read put in
/// `evidence` while the handle records value sources.
fn solve_sketch_plan_profiles(
    rf: &mut RevitFile,
    revit_version: u32,
    sketch_lines: &[crate::partition_element_records::PartitionElementRecord],
    owners: &BTreeMap<u32, [f64; 4]>,
    skip: &BTreeSet<u32>,
    evidence: &mut ProfileEvidence,
) -> BTreeMap<u32, crate::element_record_plan_profiles::PlanProfile> {
    use crate::element_record_plan_profiles as erpp;
    use std::collections::BTreeMap;
    let recording = rf.recording_value_sources();
    let (sketch_lines, current_lines) = current_sketch_records(rf, sketch_lines);
    evidence.current = current_lines;
    let sketch_lines = sketch_lines.as_slice();
    // RE-97: where an element's Sketch lists its curves, that list is the
    // sketch's membership; a line's own owner reference names another
    // element where the sketch was edited.
    let line_ids: BTreeSet<u32> = sketch_lines.iter().map(|r| r.element_id).collect();
    let lists: BTreeMap<u32, Vec<u32>> =
        erpp::scan_sketch_curve_lists(rf, revit_version, &line_ids)
            .into_iter()
            .filter(|(owner, _)| owners.contains_key(owner))
            .map(|(owner, mut curves)| {
                curves.retain(|id| !skip.contains(id));
                (owner, curves)
            })
            .collect();
    let listed: BTreeMap<u32, u32> = lists
        .iter()
        .flat_map(|(owner, curves)| curves.iter().map(move |id| (*id, *owner)))
        .collect();
    let remapped: Vec<crate::partition_element_records::PartitionElementRecord> = sketch_lines
        .iter()
        .filter(|record| !skip.contains(&record.element_id))
        .filter_map(|record| {
            let owner = match listed.get(&record.element_id) {
                Some(owner) => Some(*owner),
                None => record
                    .owner_reference
                    .filter(|owner| !lists.contains_key(owner)),
            }?;
            let mut record = record.clone();
            record.owner_reference = Some(owner);
            Some(record)
        })
        .collect();
    // An element whose list differs from the lines naming it keeps an outline
    // only where it spans its record box, as an arc outline does (RE-96).
    let mut named: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for record in sketch_lines
        .iter()
        .filter(|record| !skip.contains(&record.element_id))
    {
        if let Some(owner) = record.owner_reference {
            named.entry(owner).or_default().insert(record.element_id);
        }
    }
    let relisted: BTreeSet<u32> = lists
        .iter()
        .filter(|(owner, curves)| {
            named.get(owner) != Some(&curves.iter().copied().collect::<BTreeSet<u32>>())
        })
        .map(|(owner, _)| *owner)
        .collect();
    let spans_box = |owner: u32, profile: &erpp::PlanProfile| {
        owners.get(&owner).is_some_and(|plan_box| {
            profile.plan_extent_feet().is_some_and(|extent| {
                extent
                    .iter()
                    .zip(plan_box)
                    .all(|(a, b)| (a - b).abs() <= SKETCH_BOX_TOLERANCE_FEET)
            })
        })
    };
    let sketch_lines = remapped.as_slice();
    if recording {
        for record in sketch_lines {
            let Some(owner) = record.owner_reference else {
                continue;
            };
            let frames = evidence.frames.entry(owner).or_default();
            let frame = LineFrame {
                stream: record.stream.clone(),
                offset: record.offset,
            };
            match frames.get(&record.element_id) {
                Some(held) if held.order() <= frame.order() => {}
                _ => {
                    frames.insert(record.element_id, frame);
                }
            }
        }
    }
    let mut profiles = erpp::plan_profiles_from_sketch_line_records(sketch_lines);
    profiles.retain(|owner, profile| !relisted.contains(owner) || spans_box(*owner, profile));
    profiles.retain(|owner, _| owners.contains_key(owner));
    // RE-96: a quarter arc's ends are two corners of its record box, so the
    // box solve closes its sketch with the arc's chord. An owner with a line
    // whose box is not flat in plan and whose curve reads as an arc, not as a
    // line, is read from its lines' curves below instead (#824).
    let eps = erpp::VERTEX_EPS_FEET;
    let diagonal: BTreeMap<u32, (u32, [f64; 6])> = sketch_lines
        .iter()
        .filter(|record| {
            let b = record.bbox_feet;
            b[3] - b[0] > eps && b[4] - b[1] > eps
        })
        .filter_map(|record| {
            let owner = record
                .owner_reference
                .filter(|owner| profiles.contains_key(owner))?;
            Some((record.element_id, (owner, record.bbox_feet)))
        })
        .collect();
    if !diagonal.is_empty() {
        let ids: BTreeSet<u32> = diagonal.keys().copied().collect();
        let curves = crate::partition_beam_axes::scan_sketch_curves(rf, revit_version, &ids)
            .unwrap_or_default();
        for (id, (owner, bbox)) in &diagonal {
            let Some(curve) = curves.get(id) else {
                continue;
            };
            let inside = |p: [f64; 3]| {
                (0..3).all(|axis| p[axis] >= bbox[axis] - eps && p[axis] <= bbox[axis + 3] + eps)
            };
            let line = curve
                .line
                .as_ref()
                .is_some_and(|line| inside(line.start()) && inside(line.end()));
            let arc = curve.arc.as_ref().is_some_and(|arc| {
                erpp::arc_points(arc).is_some_and(|points| points.iter().all(|&p| inside(p)))
            });
            if arc && !line {
                profiles.remove(owner);
            }
        }
    }
    let mut unsolved: BTreeMap<u32, BTreeMap<u32, [f64; 6]>> = BTreeMap::new();
    for record in sketch_lines {
        let Some(owner) = record.owner_reference else {
            continue;
        };
        if owners.contains_key(&owner) && !profiles.contains_key(&owner) {
            unsolved
                .entry(owner)
                .or_default()
                .entry(record.element_id)
                .or_insert(record.bbox_feet);
        }
    }
    if unsolved.is_empty() {
        return profiles;
    }
    let ids: BTreeSet<u32> = unsolved
        .values()
        .flat_map(|lines| lines.keys().copied())
        .collect();
    let (Ok(lines), Ok(curves)) = (
        crate::partition_beam_axes::scan_sketch_bounded_lines(rf, revit_version, &ids),
        crate::partition_beam_axes::scan_sketch_curves(rf, revit_version, &ids),
    ) else {
        return profiles;
    };
    let arcs: BTreeMap<u32, crate::partition_beam_axes::BoundedArc> = curves
        .iter()
        .filter_map(|(&id, curve)| Some((id, curve.arc?)))
        .collect();
    // RE-95: a sketch line whose box is a single point is a zero-length
    // segment with no line of its own. It adds no edge, and is dropped where
    // its point ends another of the sketch's lines.
    let is_point = |bbox: &[f64; 6]| (0..3).all(|axis| (bbox[axis + 3] - bbox[axis]).abs() <= eps);
    for (owner, segments) in unsolved {
        let ends: Vec<[f64; 3]> = segments
            .keys()
            .filter_map(|id| lines.get(id))
            .flat_map(|line| [line.start(), line.end()])
            .chain(
                segments
                    .keys()
                    .filter_map(|id| arcs.get(id))
                    .flat_map(|arc| [arc.point(arc.start_angle), arc.point(arc.end_angle)]),
            )
            .collect();
        let exact: Option<ExactSketch> = segments
            .iter()
            .filter(|(id, bbox)| {
                let point = [bbox[0], bbox[1], bbox[2]];
                !(is_point(bbox)
                    && !lines.contains_key(id)
                    && !arcs.contains_key(id)
                    && ends
                        .iter()
                        .any(|end| (0..3).all(|axis| (end[axis] - point[axis]).abs() <= eps)))
            })
            .map(|(id, bbox)| {
                let inside = |p: [f64; 3]| {
                    (0..3)
                        .all(|axis| p[axis] >= bbox[axis] - eps && p[axis] <= bbox[axis + 3] + eps)
                };
                let line = lines.get(id).and_then(|line| {
                    let (a, b) = (line.start(), line.end());
                    (inside(a) && inside(b) && (a[2] - b[2]).abs() <= eps)
                        .then(|| vec![[a[0], a[1], b[0], b[1]]])
                });
                let arc = arcs.get(id).and_then(|arc| {
                    let points = erpp::arc_points(arc)?;
                    let flat = arc.x_axis[2].abs() <= eps && arc.y_axis[2].abs() <= eps;
                    (flat && points.iter().all(|&p| inside(p))).then(|| {
                        points
                            .windows(2)
                            .map(|pair| [pair[0][0], pair[0][1], pair[1][0], pair[1][1]])
                            .collect::<Vec<_>>()
                    })
                });
                match (line, arc) {
                    (Some(chords), None) => Some((*id, chords, false)),
                    (None, Some(chords)) => Some((*id, chords, true)),
                    _ => None,
                }
            })
            .collect::<Option<Vec<(u32, Vec<[f64; 4]>, bool)>>>()
            .map(|curves| {
                let has_arc = curves.iter().any(|(_, _, arc)| *arc);
                // RE-151: each line's first chord is an edge of its loop.
                let points = curves
                    .iter()
                    .filter_map(|(id, chords, _)| {
                        let c = chords.first()?;
                        Some((*id, ((c[0] + c[2]) / 2.0, (c[1] + c[3]) / 2.0)))
                    })
                    .collect();
                let chords: Vec<[f64; 4]> = curves
                    .into_iter()
                    .flat_map(|(_, chords, _)| chords)
                    .collect();
                (chords, has_arc, points)
            });
        let Some((mut profile, has_arc, points)) = exact.and_then(|(chords, has_arc, points)| {
            Some((erpp::plan_profile_from_lines(&chords)?, has_arc, points))
        }) else {
            continue;
        };
        // RE-96: an outline with an arc spans the element's own record box,
        // or the sketch holds curves that are not the element's edge.
        if !(has_arc || relisted.contains(&owner)) || spans_box(owner, &profile) {
            erpp::tag_voids(&mut profile, &points);
            profile.segment_ids = segments.keys().copied().collect();
            if recording {
                evidence.exact.insert(owner);
            }
            profiles.insert(owner, profile);
        }
    }
    profiles
}

/// `records` with each sketch line whose records in different partitions
/// disagree on its box kept only in its current partition (RE-216,
/// [`crate::native_document`]), or left out where that cannot be told (#824).
/// Copies that agree are kept as they are. The second value names the lines
/// that disagreed, and is filled only while the handle records value sources
/// (#1374).
pub(super) fn current_sketch_records(
    rf: &mut RevitFile,
    records: &[crate::partition_element_records::PartitionElementRecord],
) -> (
    Vec<crate::partition_element_records::PartitionElementRecord>,
    BTreeSet<u32>,
) {
    let partition = |record: &crate::partition_element_records::PartitionElementRecord| {
        record
            .stream
            .strip_prefix("Partitions/")
            .and_then(|n| n.parse::<u32>().ok())
    };
    let mut boxes: BTreeMap<u32, Vec<[f64; 6]>> = BTreeMap::new();
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for record in records {
        boxes
            .entry(record.element_id)
            .or_default()
            .push(record.bbox_feet);
        if let Some(p) = partition(record) {
            holders.entry(record.element_id).or_default().insert(p);
        }
    }
    let disagreeing: BTreeMap<u32, BTreeSet<u32>> = boxes
        .iter()
        .filter(|(_, b)| b.iter().any(|bbox| *bbox != b[0]))
        .filter_map(|(id, _)| Some((*id, holders.get(id)?.clone())))
        .collect();
    if disagreeing.is_empty() {
        return (records.to_vec(), BTreeSet::new());
    }
    let recording = rf.recording_value_sources();
    let current = crate::native_document::current_partitions(rf, &disagreeing);
    let kept = records
        .iter()
        .filter(|record| match disagreeing.get(&record.element_id) {
            None => true,
            Some(_) => current
                .get(&record.element_id)
                .is_some_and(|&p| partition(record) == Some(p)),
        })
        .cloned()
        .collect();
    let current_lines = if recording {
        disagreeing.into_keys().collect()
    } else {
        BTreeSet::new()
    };
    (kept, current_lines)
}

/// A sketch read from its lines' exact ends: the chords, whether any is an
/// arc's, and each line's ElementId with a point on its edge (RE-151).
pub(super) type ExactSketch = (Vec<[f64; 4]>, bool, Vec<(u32, (f64, f64))>);

/// One curve of a shaft's sketch as chords, and whether it is a straight
/// line (RE-100).
pub(super) type ShaftCurve = (Vec<[(f64, f64); 2]>, bool);

/// A shaft's plan outline and the bottom and top of its height, feet
/// (RE-99).
pub(super) type ShaftOutline = (Vec<(f64, f64)>, (f64, f64));

/// Cut each shaft opening's outline from the floors, roofs and ceilings its
/// height passes through (RE-99). A shaft (`OST_ShaftOpening`) is sketched
/// as a floor is, and its record box spans its height. An element whose
/// outline is one piece, lies within the shaft's height, and holds the
/// shaft's outline strictly inside its outer loop and clear of its voids
/// takes the outline as a void. An element the shaft only partly overlaps
/// is left as it is.
pub(super) fn attach_shaft_voids(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 2],
) {
    use crate::element_record_plan_profiles as erpp;
    use crate::partition_element_records as per;
    if !per::supports_revit_version(revit_version) {
        return;
    }
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Ok(scanned) = per::scan_category_records_multi(
        rf,
        revit_version,
        &[per::OST_SHAFT_OPENING, per::OST_SKETCH_LINES],
        &declared,
    ) else {
        return;
    };
    let mut boxes: BTreeMap<u32, [f64; 4]> = Default::default();
    let mut heights: BTreeMap<u32, (f64, f64)> = Default::default();
    for record in scanned
        .iter()
        .filter(|record| record.builtin_category == per::OST_SHAFT_OPENING)
    {
        let b = record.bbox_feet;
        boxes
            .entry(record.element_id)
            .or_insert([b[0], b[1], b[3], b[4]]);
        heights.entry(record.element_id).or_insert((b[2], b[5]));
    }
    if boxes.is_empty() {
        return;
    }
    let sketch_lines: Vec<per::PartitionElementRecord> = scanned
        .into_iter()
        .filter(|record| record.builtin_category == per::OST_SKETCH_LINES)
        .collect();
    let shafts: Vec<ShaftOutline> = shaft_outlines(rf, revit_version, &sketch_lines, &boxes)
        .into_iter()
        .filter_map(|(id, outline)| Some((outline, *heights.get(&id)?)))
        .collect();
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        let Some(profile) = erpp::plan_profile_from_fields(&element.fields) else {
            continue;
        };
        let (Some(base), Some(height)) = (
            float(element, "m_locationZ"),
            float(element, "m_bboxHeight"),
        ) else {
            continue;
        };
        if !profile.pieces.is_empty() {
            continue;
        }
        let eps = SHAFT_HEIGHT_TOLERANCE_FEET;
        let voids: Vec<Vec<(f64, f64)>> = shafts
            .iter()
            .filter(|(outline, (bottom, top))| {
                base >= bottom - eps
                    && base + height <= top + eps
                    && erpp::loop_within(&profile.outer_xy, outline)
                    && profile
                        .inner_xy
                        .iter()
                        .all(|void| erpp::loops_disjoint(void, outline))
            })
            .map(|(outline, _)| outline.clone())
            .collect();
        if !voids.is_empty() {
            erpp::add_voids_to_fields(&mut element.fields, &voids);
        }
    }
}

/// Each shaft's outline (RE-99), `shafts` its plan record boxes. A shaft's
/// sketch holds its boundary and the symbolic lines drawn across it, the
/// diagonals of the X Revit shows in plan; lines that cross another of the
/// sketch's lines are those, and are left out. The rest must be straight
/// lines read exactly (RE-50), or a full circle (RE-100), that close into one
/// loop spanning the shaft's record box. Its lines are those its Sketch
/// lists (RE-97), or else those naming it.
pub(super) fn shaft_outlines(
    rf: &mut RevitFile,
    revit_version: u32,
    sketch_lines: &[crate::partition_element_records::PartitionElementRecord],
    shafts: &BTreeMap<u32, [f64; 4]>,
) -> BTreeMap<u32, Vec<(f64, f64)>> {
    use crate::element_record_plan_profiles as erpp;
    use std::collections::BTreeMap;
    let line_ids: BTreeSet<u32> = sketch_lines.iter().map(|r| r.element_id).collect();
    let lists = erpp::scan_sketch_curve_lists(rf, revit_version, &line_ids);
    let mut members: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for &shaft in shafts.keys() {
        let listed = lists
            .get(&shaft)
            .map(|curves| curves.iter().copied().collect());
        let named = || {
            sketch_lines
                .iter()
                .filter(|r| r.owner_reference == Some(shaft))
                .map(|r| r.element_id)
                .collect()
        };
        members.insert(shaft, listed.unwrap_or_else(named));
    }
    let ids: BTreeSet<u32> = members.values().flatten().copied().collect();
    let (Ok(lines), Ok(circles)) = (
        crate::partition_beam_axes::scan_bounded_lines(rf, revit_version, &ids),
        crate::partition_beam_axes::scan_sketch_circles(rf, revit_version, &ids),
    ) else {
        return BTreeMap::new();
    };
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    // Strictly across: each end of one line clearly off the other, so lines
    // meeting at a corner, whose ends agree only to round-off, do not count.
    let apart = |o: (f64, f64), p: (f64, f64), q: (f64, f64), r: (f64, f64)| {
        let (c1, c2) = (cross(o, p, q), cross(o, p, r));
        c1 * c2 < 0.0 && c1.abs() > SHAFT_CROSSING_EPS && c2.abs() > SHAFT_CROSSING_EPS
    };
    let crossing =
        |[a, b]: [(f64, f64); 2], [c, d]: [(f64, f64); 2]| apart(a, b, c, d) && apart(c, d, a, b);
    let mut out = BTreeMap::new();
    for (shaft, ids) in members {
        // Each line, or a full circle drawn as chords (RE-100); only lines
        // can be the crossing diagonals left out.
        let curves: Option<Vec<ShaftCurve>> = ids
            .iter()
            .map(|id| {
                if let Some(line) = lines.get(id) {
                    let (a, b) = (line.start(), line.end());
                    return ((a[2] - b[2]).abs() <= erpp::VERTEX_EPS_FEET)
                        .then(|| (vec![[(a[0], a[1]), (b[0], b[1])]], true));
                }
                let circle = circles.get(id)?;
                let flat = circle.x_axis[2].abs() <= erpp::VERTEX_EPS_FEET
                    && circle.y_axis[2].abs() <= erpp::VERTEX_EPS_FEET;
                let points = erpp::arc_points(circle).filter(|_| flat)?;
                let chords = points
                    .windows(2)
                    .map(|pair| [(pair[0][0], pair[0][1]), (pair[1][0], pair[1][1])])
                    .collect();
                Some((chords, false))
            })
            .collect();
        let Some(curves) = curves else {
            continue;
        };
        let straight: Vec<[(f64, f64); 2]> = curves
            .iter()
            .filter(|(_, line)| *line)
            .flat_map(|(chords, _)| chords.iter().copied())
            .collect();
        let boundary: Vec<[f64; 4]> = curves
            .iter()
            .filter(|(chords, line)| !*line || !straight.iter().any(|t| crossing(chords[0], *t)))
            .flat_map(|(chords, _)| chords.iter().map(|[a, b]| [a.0, a.1, b.0, b.1]))
            .collect();
        let Some(profile) = erpp::plan_profile_from_lines(&boundary) else {
            continue;
        };
        let spans = profile.plan_extent_feet().is_some_and(|extent| {
            extent
                .iter()
                .zip(&shafts[&shaft])
                .all(|(a, b)| (a - b).abs() <= SKETCH_BOX_TOLERANCE_FEET)
        });
        if spans && profile.inner_xy.is_empty() && profile.pieces.is_empty() {
            out.insert(shaft, profile.outer_xy);
        }
    }
    out
}

/// Whether `element` is a floor, a ceiling or a roof.
fn is_plate(element: &DecodedElement) -> bool {
    matches!(element.class.as_str(), "Floor" | "Ceiling" | "Roof")
}

/// How far four or more corner heights of a shape-edited floor may stray from
/// the plane through the first three, feet (#1384).
const SLAB_PLANE_TOLERANCE_FEET: f64 = 1e-5;

/// What a floor's native `SlabShapeEditGStep` says of its top (#1384, RE-336).
#[derive(Debug, Clone, PartialEq)]
enum SlabShape {
    /// No vertex is moved.
    Unedited,
    /// Its corners are moved to heights that lie in the plane
    /// `z = a·x + b·y + c`, model feet, with the plan place of each.
    Plane {
        plane: [f64; 3],
        corners: Vec<[f64; 2]>,
    },
    /// It is edited in a way that is not read: the reason.
    Unread(&'static str),
}

/// The shape of the floor whose native record is `graph`: [`SlabShape::Unedited`]
/// where it holds no `SlabShapeEditGStep` or every one says no vertex is moved
/// and nothing is added. A step whose fields are not all read makes it
/// [`SlabShape::Unread`]; so do steps that disagree, and an unedited step beside
/// an edited one, since the order of the steps is not known (#1384).
fn slab_shape(graph: &crate::native_parameters::ObjectGraph) -> SlabShape {
    let steps: Vec<SlabShape> = graph
        .objects
        .iter()
        .filter(|object| object.class_name == "SlabShapeEditGStep")
        .map(|object| slab_step_shape(&object.fields))
        .collect();
    if steps.iter().all(|step| *step == SlabShape::Unedited) {
        return SlabShape::Unedited;
    }
    match steps.as_slice() {
        [only] => only.clone(),
        [first, rest @ ..]
            if !matches!(first, SlabShape::Unread(_)) && rest.iter().all(|step| step == first) =>
        {
            first.clone()
        }
        _ => SlabShape::Unread("shape_edit_steps_disagree"),
    }
}

/// What one `SlabShapeEditGStep` says: its corner vertices, each given at its
/// plan place with the offset of its height from the floor's original face
/// height (`m_originalFaceHeight`). The interior vertices, edge vertices and
/// creases a step can also hold are not read, and neither are corners that do
/// not lie in one plane; a field that is absent or not of the shape read makes
/// the step unread.
fn slab_step_shape(fields: &serde_json::Value) -> SlabShape {
    read_slab_step(fields).unwrap_or(SlabShape::Unread("shape_edit_fields_not_read"))
}

fn read_slab_step(fields: &serde_json::Value) -> Option<SlabShape> {
    let held = |name: &str| fields.get(name).and_then(serde_json::Value::as_array);
    let corners = held("m_cornerVertices")?;
    let others = ["m_interiorVertices", "m_edgeVertices", "m_explicitCreases"]
        .iter()
        .map(|name| held(name).map(|items| !items.is_empty()))
        .collect::<Option<Vec<bool>>>()?
        .into_iter()
        .any(|added| added);
    let original = fields
        .get("m_originalFaceHeight")
        .and_then(serde_json::Value::as_f64)?;
    let mut points: Vec<[f64; 3]> = Vec::with_capacity(corners.len());
    for corner in corners {
        let vertex = corner.get("m_vertex")?;
        let at = vertex.get("m_xyLocation")?.as_array()?;
        let (x, y) = (at.first()?.as_f64()?, at.get(1)?.as_f64()?);
        let offset = vertex.get("m_offset")?.as_f64()?;
        if !(x.is_finite() && y.is_finite() && offset.is_finite()) {
            return Some(SlabShape::Unread("corner_vertex_not_finite"));
        }
        points.push([x, y, original + offset]);
    }
    let moved = points
        .iter()
        .any(|p| (p[2] - original).abs() > SLAB_PLANE_TOLERANCE_FEET);
    if !moved && !others {
        return Some(SlabShape::Unedited);
    }
    if fields.get("m_dormant").and_then(serde_json::Value::as_bool) != Some(false) {
        return Some(SlabShape::Unread("shape_edit_dormant_or_unmarked"));
    }
    if others {
        return Some(SlabShape::Unread("interior_edge_vertices_or_creases"));
    }
    if points.len() < 3 || !original.is_finite() {
        return Some(SlabShape::Unread("fewer_than_three_corner_vertices"));
    }
    let [x0, y0, z0] = points[0];
    let [x1, y1, z1] = points[1];
    let [x2, y2, z2] = points[2];
    let det = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0);
    if det.abs() < 1e-9 {
        return Some(SlabShape::Unread("first_three_corners_collinear"));
    }
    let a = ((z1 - z0) * (y2 - y0) - (z2 - z0) * (y1 - y0)) / det;
    let b = ((x1 - x0) * (z2 - z0) - (x2 - x0) * (z1 - z0)) / det;
    let c = z0 - a * x0 - b * y0;
    let flat = points
        .iter()
        .all(|p| (a * p[0] + b * p[1] + c - p[2]).abs() <= SLAB_PLANE_TOLERANCE_FEET);
    if !flat || !(a.is_finite() && b.is_finite() && c.is_finite()) {
        return Some(SlabShape::Unread("corners_not_in_one_plane"));
    }
    Some(SlabShape::Plane {
        plane: [a, b, c],
        corners: points.iter().map(|p| [p[0], p[1]]).collect(),
    })
}

/// The plane [`SLAB_TOP_PLANE_FIELD`] records, `z = a·x + b·y + c` in model
/// feet (#1384).
pub fn slab_top_plane_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 3]> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(items) if name == SLAB_TOP_PLANE_FIELD => {
            let [a, b, c] = items.as_slice() else {
                return None;
            };
            let float = |item: &InstanceField| match item {
                InstanceField::Float { value, .. } => Some(*value),
                _ => None,
            };
            Some([float(a)?, float(b)?, float(c)?])
        }
        _ => None,
    })
}

/// The plan places [`SLAB_SHAPE_CORNERS_FIELD`] records, one `[x, y]` per
/// corner vertex of a floor whose shape-edited corners lie in one plane,
/// model feet (#1384). Empty where it holds none.
pub fn slab_shape_corners_from_fields(fields: &[(String, InstanceField)]) -> Vec<[f64; 2]> {
    let flat: Vec<f64> = fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == SLAB_SHAPE_CORNERS_FIELD => items
                .iter()
                .map(|item| match item {
                    InstanceField::Float { value, .. } => Some(*value),
                    _ => None,
                })
                .collect::<Option<Vec<f64>>>(),
            _ => None,
        })
        .unwrap_or_default();
    flat.chunks_exact(2)
        .map(|pair| [pair[0], pair[1]])
        .collect()
}

/// Whether the plate was read ([`SLAB_SHAPE_READ_FIELD`], #1384).
pub fn slab_shape_read_from_fields(fields: &[(String, InstanceField)]) -> bool {
    fields.iter().any(|(name, value)| {
        name == SLAB_SHAPE_READ_FIELD && matches!(value, InstanceField::Bool(true))
    })
}

/// The reason [`SLAB_SHAPE_UNREAD_FIELD`] records: why a shape-edited floor's
/// top is not read (#1384).
pub fn slab_shape_unread_from_fields(fields: &[(String, InstanceField)]) -> Option<&str> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(reason) if name == SLAB_SHAPE_UNREAD_FIELD => Some(reason.as_str()),
        _ => None,
    })
}

/// Mark each floor, ceiling and roof of `lists` whose native record was read
/// and holds no step that cuts its solid (RE-295): no `CutPairGStep`, the join
/// that cuts one element with another, and no `WallCutoutGStep`, the cut a
/// hosted symbol makes. Revit's export measures a cut floor's `NetVolume` less
/// the cut, which the floor's profile and layers do not give. An element whose
/// record is not read, on a release the native record path does not read, or
/// when the pass fails, is not marked.
///
/// The same pass marks each plate it read ([`SLAB_SHAPE_READ_FIELD`]) and gives
/// a floor whose `SlabShapeEditGStep` moves its corner vertices the plane of
/// its top ([`SLAB_TOP_PLANE_FIELD`]), and one whose edits are not read the
/// reason ([`SLAB_SHAPE_UNREAD_FIELD`], #1384). A plate without the read mark
/// has no known top, and nothing may take it for flat.
pub(super) fn attach_slab_uncut(rf: &mut RevitFile, lists: &mut [&mut Vec<DecodedElement>]) {
    let ids: BTreeSet<u64> = lists
        .iter()
        .flat_map(|list| list.iter())
        .filter(|element| is_plate(element))
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if ids.is_empty() {
        return;
    }
    // #1368: the current native record of each plate is noted as the source
    // of its shape-edited top; it is listed only for a floor drawn along it.
    let options = crate::native_document::Options {
        selected_ids: ids,
        value_source: Some(SLAB_SHAPE_READER),
        ..crate::native_document::Options::default()
    };
    let mut cut: BTreeMap<u64, bool> = BTreeMap::new();
    let mut shapes: BTreeMap<u64, SlabShape> = BTreeMap::new();
    let mut blind: BTreeSet<u64> = BTreeSet::new();
    let read = crate::native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            blind.insert(record.identity.element_id);
            return Ok(());
        };
        let steps = graph.objects.iter().any(|object| {
            matches!(
                object.class_name.as_str(),
                "CutPairGStep" | "WallCutoutGStep"
            )
        });
        *cut.entry(record.identity.element_id).or_default() |= steps;
        let shape = slab_shape(graph);
        let held = shapes
            .entry(record.identity.element_id)
            .or_insert_with(|| shape.clone());
        if *held != shape {
            *held = SlabShape::Unread("record_copies_disagree");
        }
        Ok(())
    });
    if read.is_err() {
        return;
    }
    // A copy of a record that was not read beside one that was: the top is not
    // known.
    for id in &blind {
        if let Some(shape) = shapes.get_mut(id) {
            *shape = SlabShape::Unread("record_copy_not_read");
        }
    }
    for element in lists.iter_mut().flat_map(|list| list.iter_mut()) {
        if !is_plate(element) {
            continue;
        }
        let id = element.id.map(u64::from);
        // A plate with no record read keeps neither mark: its top is unknown.
        if id.is_some_and(|id| shapes.contains_key(&id)) {
            element
                .fields
                .push((SLAB_SHAPE_READ_FIELD.into(), InstanceField::Bool(true)));
        }
        if id.and_then(|id| cut.get(&id)).is_some_and(|seen| !seen) {
            element
                .fields
                .push((SLAB_UNCUT_FIELD.into(), InstanceField::Bool(true)));
        }
        match id.and_then(|id| shapes.get(&id)) {
            Some(SlabShape::Plane { plane, corners }) => {
                let float = |value: f64| InstanceField::Float { value, size: 8 };
                element.fields.push((
                    SLAB_TOP_PLANE_FIELD.into(),
                    InstanceField::Vector(plane.iter().map(|&value| float(value)).collect()),
                ));
                element.fields.push((
                    SLAB_SHAPE_CORNERS_FIELD.into(),
                    InstanceField::Vector(
                        corners
                            .iter()
                            .flat_map(|corner| corner.iter().map(|&value| float(value)))
                            .collect(),
                    ),
                ));
            }
            Some(SlabShape::Unread(reason)) => element.fields.push((
                SLAB_SHAPE_UNREAD_FIELD.into(),
                InstanceField::String((*reason).into()),
            )),
            Some(SlabShape::Unedited) | None => {}
        }
    }
}
