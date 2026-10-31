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
    use crate::element_record_plan_profiles as erpp;
    use std::collections::BTreeMap;
    let sketch_lines = current_sketch_records(rf, sketch_lines);
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
            profiles.insert(owner, profile);
        }
    }
    profiles
}

/// `records` with each sketch line whose records in different partitions
/// disagree on its box kept only in its current partition (RE-216,
/// [`crate::native_document`]), or left out where that cannot be told (#824).
/// Copies that agree are kept as they are.
pub(super) fn current_sketch_records(
    rf: &mut RevitFile,
    records: &[crate::partition_element_records::PartitionElementRecord],
) -> Vec<crate::partition_element_records::PartitionElementRecord> {
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
        return records.to_vec();
    }
    let current = crate::native_document::current_partitions(rf, &disagreeing);
    records
        .iter()
        .filter(|record| match disagreeing.get(&record.element_id) {
            None => true,
            Some(_) => current
                .get(&record.element_id)
                .is_some_and(|&p| partition(record) == Some(p)),
        })
        .cloned()
        .collect()
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

/// Mark each floor, ceiling and roof of `lists` whose native record was read
/// and holds no step that cuts its solid (RE-295): no `CutPairGStep`, the join
/// that cuts one element with another, and no `WallCutoutGStep`, the cut a
/// hosted symbol makes. Revit's export measures a cut floor's `NetVolume` less
/// the cut, which the floor's profile and layers do not give. An element whose
/// record is not read, on a release the native record path does not read, is
/// not marked.
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
    let options = crate::native_document::Options {
        selected_ids: ids,
        ..crate::native_document::Options::default()
    };
    let mut cut: BTreeMap<u64, bool> = BTreeMap::new();
    let read = crate::native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let steps = graph.objects.iter().any(|object| {
            matches!(
                object.class_name.as_str(),
                "CutPairGStep" | "WallCutoutGStep"
            )
        });
        *cut.entry(record.identity.element_id).or_default() |= steps;
        Ok(())
    });
    if read.is_err() {
        return;
    }
    for element in lists.iter_mut().flat_map(|list| list.iter_mut()) {
        if is_plate(element)
            && element
                .id
                .and_then(|id| cut.get(&u64::from(id)))
                .is_some_and(|seen| !seen)
        {
            element
                .fields
                .push((SLAB_UNCUT_FIELD.into(), InstanceField::Bool(true)));
        }
    }
}
