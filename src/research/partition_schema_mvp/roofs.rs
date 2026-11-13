//! Roof profiles, extrusions and slopes.

use super::*;

/// Give each roof and ceiling the plan outline its sketch lines close
/// (RE-50, RE-98), as a floor has (RE-25). The body is the record box's
/// height; a shed roof's slope is attached later ([`attach_roof_slopes`],
/// RE-56).
pub(super) fn attach_roof_profiles(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_element_records as per;
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let roofs: BTreeMap<u32, [f64; 4]> = products
        .iter()
        .filter(|element| SKETCHED_PRODUCT_CLASSES.contains(&element.class.as_str()))
        .filter_map(|element| {
            let (x, y) = (
                float(element, "m_locationX")?,
                float(element, "m_locationY")?,
            );
            let (w, d) = (
                float(element, "m_bboxWidth")?,
                float(element, "m_bboxDepth")?,
            );
            Some((
                element.id?,
                [x - w / 2.0, y - d / 2.0, x + w / 2.0, y + d / 2.0],
            ))
        })
        .collect();
    if roofs.is_empty() || !per::supports_revit_version(revit_version) {
        return;
    }
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Ok(sketch_lines) =
        per::scan_category_records_multi(rf, revit_version, &[per::OST_SKETCH_LINES], &declared)
    else {
        return;
    };
    // RE-199: a slope arrow is one of its roof's sketch lines but no part of
    // the outline.
    let line_ids: BTreeSet<u32> = sketch_lines.iter().map(|line| line.element_id).collect();
    let arrows: BTreeSet<u32> =
        crate::partition_roof_slopes::scan_arrow_heights(rf, revit_version, &line_ids)
            .map(|found| found.into_keys().collect())
            .unwrap_or_default();
    // RE-341: on Revit 2026 and 2027 a roof is held to the record-box solve
    // alone; a ceiling takes the readers of the lines' exact ends and of a
    // Sketch's curve list.
    let roof_ids: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Roof")
        .filter_map(|element| element.id)
        .collect();
    let held_back = !roof_reads_exact_sketches(revit_version);
    let is_held = |id: &u32| held_back && roof_ids.contains(id);
    let box_only: BTreeMap<u32, [f64; 4]> = roofs
        .iter()
        .filter(|(id, _)| is_held(id))
        .map(|(id, plan_box)| (*id, *plan_box))
        .collect();
    let others: BTreeMap<u32, [f64; 4]> = roofs
        .iter()
        .filter(|(id, _)| !is_held(id))
        .map(|(id, plan_box)| (*id, *plan_box))
        .collect();
    let mut profiles = BTreeMap::new();
    for (owners, exact) in [(&others, true), (&box_only, false)] {
        if !owners.is_empty() {
            profiles.extend(sketch_plan_profiles(
                rf,
                revit_version,
                &sketch_lines,
                owners,
                &arrows,
                exact,
            ));
        }
    }
    for roof in products
        .iter_mut()
        .filter(|element| SKETCHED_PRODUCT_CLASSES.contains(&element.class.as_str()))
    {
        if let Some(profile) = roof.id.and_then(|id| profiles.get(&id)) {
            roof.fields.extend(profile.fields());
        }
    }
}

/// The readers that note a sketch line's geometry (#1368).
const ROOF_LINE_READERS: &[&str] = &[
    "sketch_line",
    "sketch_curve",
    "sketch_circle",
    "sketch_spline",
];

/// Note for `roof`, as one source named `reader`, the places the readers in
/// `readers` noted for the sketch lines `lines`: one part for each line, the
/// first reader that noted it (#1368). `current` where a line's copy was
/// chosen as the current partition's.
fn note_line_source(
    rf: &mut RevitFile,
    roof: u32,
    reader: &str,
    lines: &BTreeSet<u32>,
    readers: &[&str],
) {
    use crate::ifc::element_provenance::{CopyRule, SourcePart, ValueSource};
    let mut parts = Vec::new();
    let mut current = false;
    if let Some(noted) = rf.value_sources() {
        for line in lines {
            let note = noted.get(line).and_then(|sources| {
                sources
                    .iter()
                    .find(|s| readers.contains(&s.reader.as_str()))
            });
            let Some((note, stream, offset)) =
                note.and_then(|n| Some((n, n.stream.clone()?, n.offset?)))
            else {
                continue;
            };
            current |= note.rule == CopyRule::Current;
            parts.push(SourcePart {
                element_id: *line,
                stream,
                offset,
            });
        }
    }
    if parts.is_empty() {
        return;
    }
    let rule = if current {
        CopyRule::Current
    } else {
        CopyRule::Agreeing
    };
    let copies = parts.len();
    rf.note_value_source(roof, ValueSource::from_parts(reader, rule, copies, parts));
}

/// A roof by extrusion's section (RE-211): its top and its underside, each
/// `(u, z)` with `u` rising along the plane its sketch is in, and the model
/// axis it runs along.
#[derive(Debug, Clone, PartialEq)]
pub struct RoofExtrusion {
    pub axis: usize,
    pub top: Vec<(f64, f64)>,
    pub bottom: Vec<(f64, f64)>,
}

/// The [`RoofExtrusion`] the extrusion-roof pass recorded in an element's
/// fields (RE-211).
pub fn roof_extrusion_from_fields(fields: &[(String, InstanceField)]) -> Option<RoofExtrusion> {
    let field = |wanted: &str| {
        fields
            .iter()
            .find_map(|(name, value)| (name == wanted).then_some(value))
    };
    let axis = match field(ROOF_EXTRUSION_AXIS_FIELD)? {
        InstanceField::Integer { value, .. } => usize::try_from(*value).ok().filter(|a| *a < 2)?,
        _ => return None,
    };
    Some(RoofExtrusion {
        axis,
        top: crate::element_record_plan_profiles::points_from_field(field(
            ROOF_EXTRUSION_TOP_FIELD,
        )?)?,
        bottom: crate::element_record_plan_profiles::points_from_field(field(
            ROOF_EXTRUSION_BOTTOM_FIELD,
        )?)?,
    })
}

/// A sketch spline's points for a roof by extrusion's top (RE-211): each
/// knot span inside the curve's domain cut into its share of
/// [`ROOF_EXTRUSION_SPLINE_SEGMENTS`] chords, at least
/// [`ROOF_EXTRUSION_MIN_SPAN_SEGMENTS`], with the knots among the points, and
/// a point [`ROOF_EXTRUSION_END_SHARE`] of its span in from each end, so the
/// first and last chords lie along the curve's ends, which the plumb cuts
/// follow. Each chord is then halved until the curve at its quarter, half
/// and three-quarter points lies within [`ROOF_EXTRUSION_CHORD_FEET`] of it,
/// so a span short in parameter but long on the curve is still followed.
/// `None` where that takes more than [`ROOF_EXTRUSION_MAX_SPLINE_POINTS`]
/// points or halving no longer moves the parameter.
pub(super) fn roof_extrusion_spline_points(
    spline: &crate::partition_beam_axes::BoundedSpline,
) -> Option<Vec<[f64; 3]>> {
    let (start, end) = (spline.start_parameter, spline.end_parameter);
    let span = end - start;
    if !(span.is_finite() && span > 0.0) {
        return None;
    }
    let mut breaks = vec![start];
    breaks.extend(
        spline
            .knots
            .iter()
            .copied()
            .filter(|&k| k > start && k < end),
    );
    breaks.push(end);
    breaks.dedup();
    let mut parameters = vec![
        start + span * ROOF_EXTRUSION_END_SHARE,
        end - span * ROOF_EXTRUSION_END_SHARE,
    ];
    for pair in breaks.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let share = (ROOF_EXTRUSION_SPLINE_SEGMENTS as f64 * (b - a) / span).ceil() as usize;
        let segments = share.max(ROOF_EXTRUSION_MIN_SPAN_SEGMENTS);
        parameters.extend((0..segments).map(|i| a + (b - a) * i as f64 / segments as f64));
    }
    parameters.push(end);
    parameters.sort_by(f64::total_cmp);
    parameters.dedup();
    let mut points = vec![spline.point(start)?];
    let mut done = (start, points[0]);
    for &target in &parameters[1..] {
        let mut pending = vec![(target, spline.point(target)?)];
        while let Some(&(b, to)) = pending.last() {
            let (a, from) = done;
            let mut straight = true;
            for share in [0.25, 0.5, 0.75] {
                let on = spline.point(a + (b - a) * share)?;
                if chord_distance(on, from, to) > ROOF_EXTRUSION_CHORD_FEET {
                    straight = false;
                    break;
                }
            }
            if straight {
                points.push(to);
                if points.len() > ROOF_EXTRUSION_MAX_SPLINE_POINTS {
                    return None;
                }
                done = (b, to);
                pending.pop();
            } else {
                let middle = a + (b - a) / 2.0;
                if !(middle > a && middle < b) {
                    return None;
                }
                pending.push((middle, spline.point(middle)?));
            }
        }
    }
    Some(points)
}

/// How far `point` lies from the segment `from`..`to`.
pub(super) fn chord_distance(point: [f64; 3], from: [f64; 3], to: [f64; 3]) -> f64 {
    let along = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let off = [point[0] - from[0], point[1] - from[1], point[2] - from[2]];
    let length = along.iter().map(|v| v * v).sum::<f64>();
    let t = if length > 0.0 {
        (off.iter().zip(&along).map(|(o, a)| o * a).sum::<f64>() / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    off.iter()
        .zip(&along)
        .map(|(o, a)| (o - t * a).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// The underside of a roof whose top is `top` (`(u, z)`, `u` rising) and
/// whose thickness is `thickness`, measured square to the top: the top
/// offset down by it, its ends carried along their tangents to the plumb
/// lines through the top's ends and cut there (RE-211). `None` where the
/// underside does not rise along `u` throughout, as where the top curves
/// tighter than the thickness.
pub fn roof_extrusion_bottom(top: &[(f64, f64)], thickness: f64) -> Option<Vec<(f64, f64)>> {
    if top.len() < 2 || !(thickness.is_finite() && thickness > 0.0) {
        return None;
    }
    // Each segment's normal down from it, and each point's offset: an end
    // along its segment's normal, a joint where the two segments' offsets
    // meet, `t (n1 + n2) / (1 + n1 · n2)` from it.
    let normals: Vec<(f64, f64)> = top
        .windows(2)
        .map(|pair| {
            let (du, dv) = (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
            let length = du.hypot(dv);
            (length > 0.0).then(|| (dv / length, -du / length))
        })
        .collect::<Option<_>>()?;
    let mut offset = Vec::with_capacity(top.len());
    for (i, &(u, v)) in top.iter().enumerate() {
        let (a, b) = (
            normals[i.saturating_sub(1)],
            normals[i.min(normals.len() - 1)],
        );
        let scale = 1.0 + a.0 * b.0 + a.1 * b.1;
        if scale < 1e-6 {
            return None;
        }
        offset.push((
            u + thickness * (a.0 + b.0) / scale,
            v + thickness * (a.1 + b.1) / scale,
        ));
    }
    // An end carried along its tangent past the plumb line through the top's
    // end, however short its last chord: twice the profile's run and
    // thickness, divided by how far the tangent leans along `u`.
    let reach = 2.0 * ((top[top.len() - 1].0 - top[0].0).abs() + thickness);
    let carried = |a: (f64, f64), b: (f64, f64)| {
        let (du, dv) = (b.0 - a.0, b.1 - a.1);
        let length = du.hypot(dv);
        if length > 0.0 {
            let distance = reach / (du.abs() / length).max(1e-3);
            (b.0 + distance * du / length, b.1 + distance * dv / length)
        } else {
            b
        }
    };
    let mut chain = vec![carried(offset[1], offset[0])];
    chain.extend(offset.iter().copied());
    chain.push(carried(offset[offset.len() - 2], offset[offset.len() - 1]));
    let (low, high) = (top[0].0, top[top.len() - 1].0);
    let mut bottom = Vec::new();
    for pair in chain.windows(2) {
        let ((u0, v0), (u1, v1)) = (pair[0], pair[1]);
        for edge in [low, high] {
            if (u0 - edge) * (u1 - edge) < 0.0 {
                bottom.push((edge, v0 + (v1 - v0) * (edge - u0) / (u1 - u0)));
            }
        }
        if (low..=high).contains(&u1) {
            bottom.push((u1, v1));
        }
    }
    let rises = bottom.windows(2).all(|pair| pair[1].0 > pair[0].0);
    let ends = bottom.first().is_some_and(|p| (p.0 - low).abs() < 1e-9)
        && bottom.last().is_some_and(|p| (p.0 - high).abs() < 1e-9);
    (rises && ends).then_some(bottom)
}

/// Give each roof by extrusion its section (RE-211). Such a roof is
/// sketched in a vertical plane: every sketch line of it is flat along one
/// horizontal model axis, at one place along it. Its profile is those lines
/// joined end to end, rising along the plane's other horizontal axis, each a
/// straight line or a B-spline ([`crate::partition_beam_axes::BoundedSpline`])
/// within its record box. The profile is the roof's top; its type's layers
/// give the thickness square to it, and both ends are cut plumb. The section
/// is kept only where it reproduces the roof's record box across the plane
/// to [`ROOF_EXTRUSION_BOX_TOLERANCE_FEET`]; the roof then runs along the
/// axis over its box. Measured on `11_RoofByExtrusion` (R2025): every one of
/// the 1,187 vertices of Revit's body within 0.001 ft of the section.
pub(super) fn attach_extrusion_roofs(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_beam_axes as pba;
    use crate::partition_element_records as per;
    if !pba::SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let float = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let boxes: BTreeMap<u32, [f64; 6]> = products
        .iter()
        .filter(|element| element.class == "Roof" && type_of(element).is_some())
        .filter(|element| {
            crate::element_record_plan_profiles::plan_profile_from_fields(&element.fields).is_none()
        })
        .filter_map(|element| {
            let (x, y, z) = (
                float(element, "m_locationX")?,
                float(element, "m_locationY")?,
                float(element, "m_locationZ")?,
            );
            let (w, d, h) = (
                float(element, "m_bboxWidth")?,
                float(element, "m_bboxDepth")?,
                float(element, "m_bboxHeight")?,
            );
            Some((
                element.id?,
                [x - w / 2.0, y - d / 2.0, z, x + w / 2.0, y + d / 2.0, z + h],
            ))
        })
        .collect();
    if boxes.is_empty() {
        return;
    }
    let recording = rf.recording_value_sources();
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Ok(sketch_lines) =
        per::scan_category_records_multi(rf, revit_version, &[per::OST_SKETCH_LINES], &declared)
    else {
        return;
    };
    // A roof's lines are those its Sketch lists (RE-97), which an edited
    // sketch's lines' owner references need not name, or else those naming
    // it.
    let all_lines: BTreeSet<u32> = sketch_lines.iter().map(|line| line.element_id).collect();
    let lists =
        crate::element_record_plan_profiles::scan_sketch_curve_lists(rf, revit_version, &all_lines);
    let listed: BTreeMap<u32, u32> = lists
        .iter()
        .filter(|(roof, _)| boxes.contains_key(roof))
        .flat_map(|(&roof, curves)| curves.iter().map(move |&line| (line, roof)))
        .collect();
    let roof_of = |line: &per::PartitionElementRecord| {
        listed.get(&line.element_id).copied().or_else(|| {
            line.owner_reference
                .filter(|owner| boxes.contains_key(owner) && !lists.contains_key(owner))
        })
    };
    // A line framed in more than one partition is one line: its first copy.
    let mut copies: BTreeMap<u32, BTreeMap<u32, &per::PartitionElementRecord>> = BTreeMap::new();
    for line in &sketch_lines {
        if let Some(owner) = roof_of(line) {
            let kept = copies.entry(owner).or_default();
            let first = kept
                .get(&line.element_id)
                .is_none_or(|held| line.frame_order() < held.frame_order());
            if first {
                kept.insert(line.element_id, line);
            }
        }
    }
    let lines_of: BTreeMap<u32, Vec<&per::PartitionElementRecord>> = copies
        .into_iter()
        .map(|(owner, lines)| (owner, lines.into_values().collect()))
        .collect();
    // The axis each roof's sketch is flat along, where all its lines are flat
    // along the same one at the same place.
    let planes: BTreeMap<u32, (usize, f64)> = lines_of
        .iter()
        .filter_map(|(&roof, lines)| {
            let flat = |line: &per::PartitionElementRecord, k: usize| {
                (line.bbox_feet[k + 3] - line.bbox_feet[k]).abs() < 1e-6
            };
            let axis = (0..2).find(|&k| lines.iter().all(|line| flat(line, k)))?;
            let at = lines.first()?.bbox_feet[axis];
            lines
                .iter()
                .all(|line| (line.bbox_feet[axis] - at).abs() < 1e-6)
                .then_some((roof, (axis, at)))
        })
        .collect();
    if planes.is_empty() {
        return;
    }
    let line_ids: BTreeSet<u32> = planes
        .keys()
        .flat_map(|roof| lines_of[roof].iter().map(|line| line.element_id))
        .collect();
    let splines = pba::scan_sketch_splines(rf, revit_version, &line_ids).unwrap_or_default();
    let straight = pba::scan_sketch_bounded_lines(rf, revit_version, &line_ids).unwrap_or_default();
    let types: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.id.is_some_and(|id| planes.contains_key(&id)))
        .filter_map(type_of)
        .collect();
    let materials: BTreeSet<u32> = crate::partition_type_records::scan_type_records(
        rf,
        revit_version,
        crate::partition_type_records::OST_MATERIALS,
        &declared,
    )
    .unwrap_or_default()
    .iter()
    .map(|record| record.element_id)
    .collect();
    let Ok(layers) = crate::partition_compound_structure::scan_type_layers(
        rf,
        revit_version,
        &types,
        &materials,
        &declared,
    ) else {
        return;
    };
    let inside = |point: [f64; 3], record: &[f64; 6]| {
        (0..3).all(|k| {
            point[k] >= record[k] - pba::END_TOLERANCE_FEET
                && point[k] <= record[k + 3] + pba::END_TOLERANCE_FEET
        })
    };
    for roof in products.iter_mut() {
        let Some(id) = roof.id else {
            continue;
        };
        let (Some(&(axis, _)), Some(record)) = (planes.get(&id), boxes.get(&id)) else {
            continue;
        };
        let Some(thickness) = type_of(roof)
            .and_then(|t| layers.get(&t))
            .map(|layers| layers.iter().map(|layer| layer.width_feet).sum::<f64>())
        else {
            continue;
        };
        let u_axis = 1 - axis;
        // Each line's points across the plane, from its own data, within its
        // record box.
        let mut curves: Vec<Vec<(f64, f64)>> = Vec::new();
        for line in &lines_of[&id] {
            let points = if let Some(spline) = splines.get(&line.element_id) {
                spline
                    .control_points
                    .iter()
                    .all(|&[_, x, y, z]| inside([x, y, z], &line.bbox_feet))
                    .then(|| roof_extrusion_spline_points(spline))
                    .flatten()
            } else {
                straight.get(&line.element_id).and_then(|segment| {
                    let ends = [segment.start(), segment.end()];
                    ends.iter()
                        .all(|end| inside(*end, &line.bbox_feet))
                        .then(|| ends.to_vec())
                })
            };
            let Some(points) = points else {
                curves.clear();
                break;
            };
            let mut across: Vec<(f64, f64)> = points.iter().map(|p| (p[u_axis], p[2])).collect();
            if across.last().map(|p| p.0) < across.first().map(|p| p.0) {
                across.reverse();
            }
            curves.push(across);
        }
        if curves.is_empty() {
            continue;
        }
        curves.sort_by(|a, b| a[0].0.total_cmp(&b[0].0));
        let mut top: Vec<(f64, f64)> = Vec::new();
        let mut joined = true;
        for curve in &curves {
            match top.last() {
                None => top.extend(curve.iter().copied()),
                Some(&(u, v)) => {
                    let (cu, cv) = curve[0];
                    if (cu - u).hypot(cv - v) > ROOF_EXTRUSION_JOIN_FEET {
                        joined = false;
                        break;
                    }
                    top.extend(curve.iter().skip(1).copied());
                }
            }
        }
        if !joined || !top.windows(2).all(|pair| pair[1].0 > pair[0].0) {
            continue;
        }
        let Some(bottom) = roof_extrusion_bottom(&top, thickness) else {
            continue;
        };
        let us = || top.iter().chain(&bottom).map(|p| p.0);
        let vs = || top.iter().chain(&bottom).map(|p| p.1);
        let section = [
            us().fold(f64::INFINITY, f64::min),
            vs().fold(f64::INFINITY, f64::min),
            us().fold(f64::NEG_INFINITY, f64::max),
            vs().fold(f64::NEG_INFINITY, f64::max),
        ];
        let wanted = [record[u_axis], record[2], record[u_axis + 3], record[5]];
        let reproduces = section
            .iter()
            .zip(wanted)
            .all(|(got, want)| (got - want).abs() <= ROOF_EXTRUSION_BOX_TOLERANCE_FEET);
        // The extrusion runs over the record box along `axis`, wherever its
        // sketch plane is: Revit's Start and End may put it to one side.
        if !reproduces {
            continue;
        }
        if recording {
            use crate::ifc::element_provenance::{CopyRule, SourcePart, ValueSource};
            // The first frame of each of the roof's sketch lines, the lines'
            // own data, and the type whose layers gave the thickness.
            let frames: Vec<SourcePart> = lines_of[&id]
                .iter()
                .map(|line| SourcePart {
                    element_id: line.element_id,
                    stream: line.stream.clone(),
                    offset: line.offset,
                })
                .collect();
            let copies = frames.len();
            rf.note_value_source(
                id,
                ValueSource::from_parts("sketch_lines", CopyRule::Frame, copies, frames),
            );
            let used: BTreeSet<u32> = lines_of[&id].iter().map(|line| line.element_id).collect();
            note_line_source(rf, id, "roof_line_data", &used, ROOF_LINE_READERS);
            if let Some(type_id) = type_of(roof) {
                rf.attribute_value_source(type_id, "type_layers", id, "roof_type_layers");
            }
        }
        roof.fields.push((
            ROOF_EXTRUSION_AXIS_FIELD.into(),
            InstanceField::Integer {
                value: axis as i64,
                signed: false,
                size: 8,
            },
        ));
        roof.fields.push((
            ROOF_EXTRUSION_TOP_FIELD.into(),
            crate::element_record_plan_profiles::loop_field(&top),
        ));
        roof.fields.push((
            ROOF_EXTRUSION_BOTTOM_FIELD.into(),
            crate::element_record_plan_profiles::loop_field(&bottom),
        ));
    }
}

/// [`ROOF_SLOPE_PLATE_OFFSET_FIELD`] for the `index`th defining edge.
pub fn roof_slope_plate_offset_field(index: usize) -> String {
    match index {
        0 => ROOF_SLOPE_PLATE_OFFSET_FIELD.to_string(),
        _ => format!("{ROOF_SLOPE_PLATE_OFFSET_FIELD}_{index}"),
    }
}

/// A roof edge's slope: the edge that defines it, its angle, and the roof's
/// thickness measured square to the slope (RE-56).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoofSlope {
    /// Plan start of the defining edge, model feet.
    pub edge_start: [f64; 2],
    /// Plan end of the defining edge, model feet.
    pub edge_end: [f64; 2],
    /// Slope from horizontal, radians.
    pub angle_radians: f64,
    /// The roof type's layers summed, feet.
    pub thickness_feet: f64,
    /// How far the edge's plane is raised at the edge, feet (RE-199).
    pub plate_offset_feet: f64,
}

/// The names of the start, end and angle fields of a roof's `index`th
/// defining edge: [`ROOF_SLOPE_EDGE_FIELDS`] and [`ROOF_SLOPE_ANGLE_FIELD`]
/// for the first, the same names with `_<index>` after `edge` or `angle`
/// for each further one (RE-199).
pub fn roof_slope_edge_fields(index: usize) -> [String; 5] {
    let names = [
        ROOF_SLOPE_EDGE_FIELDS[0],
        ROOF_SLOPE_EDGE_FIELDS[1],
        ROOF_SLOPE_EDGE_FIELDS[2],
        ROOF_SLOPE_EDGE_FIELDS[3],
        ROOF_SLOPE_ANGLE_FIELD,
    ];
    names.map(|name| match index {
        0 => name.to_string(),
        _ => match name.find("_edge_") {
            Some(at) => format!("{}_edge_{index}_{}", &name[..at], &name[at + 6..]),
            None => format!("{name}_{index}"),
        },
    })
}

/// The slope [`ROOF_SLOPE_EDGE_FIELDS`], [`ROOF_SLOPE_ANGLE_FIELD`] and
/// [`ROOF_TYPE_THICKNESS_FIELD`] record: the first defining edge.
pub fn roof_slope_from_fields(fields: &[(String, InstanceField)]) -> Option<RoofSlope> {
    roof_slopes_from_fields(fields).into_iter().next()
}

/// Every defining edge a roof's fields record, in order
/// ([`roof_slope_edge_fields`]); empty when none is.
pub fn roof_slopes_from_fields(fields: &[(String, InstanceField)]) -> Vec<RoofSlope> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let Some(thickness_feet) = float(ROOF_TYPE_THICKNESS_FIELD) else {
        return Vec::new();
    };
    (0..)
        .map_while(|index| {
            let [sx, sy, ex, ey, angle] = roof_slope_edge_fields(index).map(|name| float(&name));
            Some(RoofSlope {
                edge_start: [sx?, sy?],
                edge_end: [ex?, ey?],
                angle_radians: angle?,
                thickness_feet,
                plate_offset_feet: float(&roof_slope_plate_offset_field(index)).unwrap_or(0.0),
            })
        })
        .collect()
}

/// Give each roof whose sketch lines all carry a slope block, exactly one
/// of them defining the roof's slope, that edge, its angle and its type's
/// thickness (RE-56, [`crate::partition_roof_slopes`]). A roof with no
/// defining edge is flat and gets nothing; one with two or more (a hip or
/// a gable) gets every defining edge, and is drawn as its edges'
/// straight-skeleton roof where that reproduces its record box's height
/// (RE-199).
///
/// A roof with no defining edge and several slope arrows (RE-307) gets, for
/// each arrow, the boundary line its tail lies on and its slope, with
/// [`ROOF_ARROW_PLANES_FIELD`] set.
pub(super) fn attach_roof_slopes(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_element_records as per;
    use crate::partition_roof_slopes as prs;
    if !prs::ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let roofs: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Roof" && type_of(element).is_some())
        .filter(|element| {
            crate::element_record_plan_profiles::plan_profile_from_fields(&element.fields).is_some()
        })
        .filter_map(|element| element.id)
        .collect();
    if roofs.is_empty() {
        return;
    }
    let recording = rf.recording_value_sources();
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Ok(sketch_lines) =
        per::scan_category_records_multi(rf, revit_version, &[per::OST_SKETCH_LINES], &declared)
    else {
        return;
    };
    // RE-97: where a roof's Sketch lists its curves, that list is its
    // membership; once the sketch is edited a line's owner reference names
    // another element (RE-199, on 2025).
    let all_lines: BTreeSet<u32> = sketch_lines.iter().map(|line| line.element_id).collect();
    let lists =
        crate::element_record_plan_profiles::scan_sketch_curve_lists(rf, revit_version, &all_lines);
    let mut edges: BTreeMap<u32, BTreeSet<u32>> = Default::default();
    for line in &sketch_lines {
        if let Some(owner) = line
            .owner_reference
            .filter(|owner| roofs.contains(owner) && !lists.contains_key(owner))
        {
            edges.entry(owner).or_default().insert(line.element_id);
        }
    }
    for (owner, curves) in lists.iter().filter(|(owner, _)| roofs.contains(owner)) {
        edges
            .entry(*owner)
            .or_default()
            .extend(curves.iter().copied());
    }
    let line_ids: BTreeSet<u32> = edges.values().flatten().copied().collect();
    let Ok(slopes) = prs::scan_edge_slopes(rf, revit_version, &line_ids) else {
        return;
    };
    let unblocked: BTreeSet<u32> = line_ids
        .iter()
        .filter(|line| !slopes.contains_key(line))
        .copied()
        .collect();
    let Ok(arrows) = prs::scan_arrow_heights(rf, revit_version, &unblocked) else {
        return;
    };
    // Every defining edge of a roof whose edges all carry a block: one is a
    // shed (RE-56), more a gable or hip (RE-199). A roof whose edges define
    // none, with one slope arrow, rises along the arrow (RE-199).
    let mut defining: BTreeMap<u32, Vec<(u32, f64, f64)>> = Default::default();
    let mut sloped_by_arrow: BTreeMap<u32, (u32, prs::ArrowHeights)> = Default::default();
    let mut sloped_by_arrows: BTreeMap<u32, ArrowRoofLines> = Default::default();
    for (&roof, lines) in &edges {
        let (blocked, rest): (Vec<u32>, Vec<u32>) =
            lines.iter().partition(|line| slopes.contains_key(line));
        if let [arrow] = rest.as_slice() {
            let flat = blocked.iter().all(|line| !slopes[line].defines_slope);
            if let (true, Some(heights)) = (flat, arrows.get(arrow)) {
                sloped_by_arrow.insert(roof, (*arrow, *heights));
            }
            continue;
        }
        // RE-307: several slope arrows and no defining edge, measured on
        // Revit 2025 only.
        if rest.len() >= 2 && revit_version == 2025 {
            let flat = blocked.iter().all(|line| !slopes[line].defines_slope);
            let found: Option<Vec<(u32, prs::ArrowHeights)>> = rest
                .iter()
                .map(|line| arrows.get(line).map(|heights| (*line, *heights)))
                .collect();
            if let (true, false, Some(found)) = (flat, blocked.is_empty(), found) {
                sloped_by_arrows.insert(roof, (blocked, found));
            }
            continue;
        }
        if !rest.is_empty() {
            continue;
        }
        let sloped: Vec<(u32, f64, f64)> = lines
            .iter()
            .filter_map(|line| slopes.get(line).map(|slope| (*line, slope)))
            .filter(|(_, slope)| slope.defines_slope)
            .map(|(line, slope)| (line, slope.angle_radians, slope.plate_offset_feet))
            .collect();
        if !sloped.is_empty() {
            defining.insert(roof, sloped);
        }
    }
    if defining.is_empty() && sloped_by_arrow.is_empty() && sloped_by_arrows.is_empty() {
        return;
    }
    let defining_lines: BTreeSet<u32> = defining
        .values()
        .flatten()
        .map(|(line, _, _)| *line)
        .chain(sloped_by_arrow.values().map(|(line, _)| *line))
        .chain(sloped_by_arrows.values().flat_map(|(boundary, found)| {
            boundary
                .iter()
                .copied()
                .chain(found.iter().map(|(line, _)| *line))
        }))
        .collect();
    let types: BTreeSet<u32> = products
        .iter()
        .filter(|element| {
            element.id.is_some_and(|id| {
                defining.contains_key(&id)
                    || sloped_by_arrow.contains_key(&id)
                    || sloped_by_arrows.contains_key(&id)
            })
        })
        .filter_map(type_of)
        .collect();
    let materials: BTreeSet<u32> = crate::partition_type_records::scan_type_records(
        rf,
        revit_version,
        crate::partition_type_records::OST_MATERIALS,
        &declared,
    )
    .unwrap_or_default()
    .iter()
    .map(|record| record.element_id)
    .collect();
    let sloped_roofs: BTreeSet<u32> = defining
        .keys()
        .chain(sloped_by_arrow.keys())
        .chain(sloped_by_arrows.keys())
        .copied()
        .collect();
    let fascia = prs::scan_fascia_depths(rf, revit_version, &sloped_roofs).unwrap_or_default();
    let cuts = prs::scan_rafter_cuts(rf, revit_version, &sloped_roofs);
    let (Ok(lines), Ok(layers)) = (
        crate::partition_beam_axes::scan_sketch_bounded_lines(rf, revit_version, &defining_lines),
        crate::partition_compound_structure::scan_type_layers(
            rf,
            revit_version,
            &types,
            &materials,
            &declared,
        ),
    ) else {
        return;
    };
    for roof in products
        .iter_mut()
        .filter(|element| element.class == "Roof")
    {
        let (Some(id), Some(type_id)) = (roof.id, type_of(roof)) else {
            continue;
        };
        let Some(type_layers) = layers.get(&type_id) else {
            continue;
        };
        // An arrow is a shed whose edge runs through the arrow's tail square
        // to it, rising from the tail's height to the head's along it.
        let edges: Option<Vec<RoofEdgeSlope>> = if let Some((line_id, heights)) =
            sloped_by_arrow.get(&id)
        {
            lines.get(line_id).and_then(|line| {
                let (tail, head) = (line.start(), line.end());
                let run = (head[0] - tail[0]).hypot(head[1] - tail[1]);
                let slope = heights.rise_over_run(run);
                (run > 1e-9 && slope > 0.0 && slope.is_finite()).then(|| {
                    let (ux, uy) = ((head[0] - tail[0]) / run, (head[1] - tail[1]) / run);
                    let across = [tail[0] - uy, tail[1] + ux, tail[2]];
                    vec![(tail, across, slope.atan(), 0.0)]
                })
            })
        } else if let Some((boundary, found)) = sloped_by_arrows.get(&id) {
            crate::element_record_plan_profiles::plan_profile_from_fields(&roof.fields)
                .filter(|profile| profile.pieces.is_empty())
                .and_then(|profile| arrow_roof_edges(boundary, found, &lines, &profile.outer_xy))
        } else {
            defining.get(&id).and_then(|sloped| {
                sloped
                    .iter()
                    .map(|(line_id, angle, offset)| {
                        lines
                            .get(line_id)
                            .map(|line| (line.start(), line.end(), *angle, *offset))
                    })
                    .collect()
            })
        };
        let Some(edges) = edges else {
            continue;
        };
        let thickness: f64 = type_layers.iter().map(|layer| layer.width_feet).sum();
        if !(thickness.is_finite() && thickness > 0.0) {
            continue;
        }
        if recording {
            // The sketch lines whose slope blocks and geometry the edges were
            // made from, and the type whose layers gave the thickness.
            let used: BTreeSet<u32> = if let Some((line_id, _)) = sloped_by_arrow.get(&id) {
                BTreeSet::from([*line_id])
            } else if let Some((boundary, found)) = sloped_by_arrows.get(&id) {
                boundary
                    .iter()
                    .copied()
                    .chain(found.iter().map(|(line, _)| *line))
                    .collect()
            } else {
                defining
                    .get(&id)
                    .map(|sloped| {
                        sloped
                            .iter()
                            .map(|(line, _, _)| *line)
                            .collect::<BTreeSet<u32>>()
                    })
                    .unwrap_or_default()
            };
            note_line_source(
                rf,
                id,
                "roof_slope_edges",
                &used,
                &["edge_slope", "arrow_heights"],
            );
            // The geometry the edges were made from is what the slope scan's
            // line reader read, not an earlier plan-profile note of the line.
            note_line_source(rf, id, "roof_line_data", &used, &["sketch_line"]);
            rf.attribute_value_source(type_id, "type_layers", id, "roof_type_layers");
        }
        if sloped_by_arrows.contains_key(&id) {
            roof.fields.push((
                ROOF_ARROW_PLANES_FIELD.into(),
                InstanceField::Integer {
                    value: 1,
                    signed: false,
                    size: 8,
                },
            ));
        }
        for (index, (start, end, angle, offset)) in edges.into_iter().enumerate() {
            for (name, value) in roof_slope_edge_fields(index)
                .into_iter()
                .zip([start[0], start[1], end[0], end[1], angle])
            {
                roof.fields
                    .push((name, InstanceField::Float { value, size: 8 }));
            }
            if offset != 0.0 {
                roof.fields.push((
                    roof_slope_plate_offset_field(index),
                    InstanceField::Float {
                        value: offset,
                        size: 8,
                    },
                ));
            }
        }
        roof.fields.push((
            ROOF_TYPE_THICKNESS_FIELD.into(),
            InstanceField::Float {
                value: thickness,
                size: 8,
            },
        ));
        if let Some(&depth) = fascia.get(&id) {
            roof.fields.push((
                ROOF_FASCIA_DEPTH_FIELD.into(),
                InstanceField::Float {
                    value: depth,
                    size: 8,
                },
            ));
        }
        if let Some(cut) = cuts.get(&id) {
            roof.fields.push((
                ROOF_RAFTER_CUT_FIELD.into(),
                InstanceField::String(cut.name().into()),
            ));
        }
    }
}

/// The [`ROOF_RAFTER_CUT_FIELD`] a roof's fields record.
pub fn roof_rafter_cut_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::partition_roof_slopes::RafterCut> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(cut) if name == ROOF_RAFTER_CUT_FIELD => {
            crate::partition_roof_slopes::RafterCut::from_name(cut)
        }
        _ => None,
    })
}

/// The [`ROOF_FASCIA_DEPTH_FIELD`] a roof's fields record.
pub fn roof_fascia_depth_from_fields(fields: &[(String, InstanceField)]) -> Option<f64> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == ROOF_FASCIA_DEPTH_FIELD => Some(*value),
        _ => None,
    })
}

/// A roof edge's start and end, its slope angle, and how far its plane is
/// raised at the edge (RE-199).
pub(super) type RoofEdgeSlope = ([f64; 3], [f64; 3], f64, f64);

/// A footprint roof's boundary lines and its slope arrows with their
/// heights, by ElementId (RE-307).
type ArrowRoofLines = (
    Vec<u32>,
    Vec<(u32, crate::partition_roof_slopes::ArrowHeights)>,
);

/// How far a slope arrow's tail may lie from the boundary line it is on,
/// feet (RE-307).
const ARROW_TAIL_FEET: f64 = 1e-4;

/// How square to its boundary line a slope arrow must run, as the cosine of
/// the angle between them (RE-307).
const ARROW_SQUARE_TOLERANCE: f64 = 1e-6;

/// Whether `point` lies inside `ring` (even-odd rule).
fn outline_contains((px, py): (f64, f64), ring: &[(f64, f64)]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let ((ax, ay), (bx, by)) = (ring[i], ring[(i + 1) % ring.len()]);
        if (ay > py) != (by > py) && px < ax + (py - ay) * (bx - ax) / (by - ay) {
            inside = !inside;
        }
    }
    inside
}

/// The edge each of a footprint roof's slope arrows defines (RE-307): the
/// boundary line its tail lies on, square to it, with the arrow's slope over
/// its run as the angle, so each arrow's plane rises from that line as a
/// defining edge's does. `None` unless every arrow starts at height 0 on
/// exactly one boundary line, square to it, runs into the outline with a
/// rising slope set by its two heights, no boundary line carries two, and
/// the outline is a single loop.
fn arrow_roof_edges(
    boundary: &[u32],
    arrows: &[(u32, crate::partition_roof_slopes::ArrowHeights)],
    lines: &BTreeMap<u32, crate::partition_beam_axes::BoundedLine>,
    outline: &[(f64, f64)],
) -> Option<Vec<RoofEdgeSlope>> {
    let mut taken: BTreeSet<u32> = BTreeSet::new();
    let mut edges = Vec::with_capacity(arrows.len());
    for (arrow, heights) in arrows {
        let line = lines.get(arrow)?;
        let (tail, head) = (line.start(), line.end());
        let run = (head[0] - tail[0]).hypot(head[1] - tail[1]);
        let slope = heights.rise_over_run(run);
        if !(run > 1e-9
            && !heights.by_angle
            && slope > 0.0
            && slope.is_finite()
            && heights.tail_feet.abs() < 1e-9
            && (head[2] - tail[2]).abs() < 1e-6
            && outline_contains((head[0], head[1]), outline))
        {
            return None;
        }
        let (ux, uy) = ((head[0] - tail[0]) / run, (head[1] - tail[1]) / run);
        let mut on_line = boundary.iter().filter_map(|id| {
            let edge = lines.get(id)?;
            let (a, b) = (edge.start(), edge.end());
            let length = (b[0] - a[0]).hypot(b[1] - a[1]);
            if length < 1e-9 {
                return None;
            }
            let (ex, ey) = ((b[0] - a[0]) / length, (b[1] - a[1]) / length);
            let along = (tail[0] - a[0]) * ex + (tail[1] - a[1]) * ey;
            let off = (tail[0] - a[0]) * -ey + (tail[1] - a[1]) * ex;
            (off.abs() <= ARROW_TAIL_FEET
                && along >= -ARROW_TAIL_FEET
                && along <= length + ARROW_TAIL_FEET
                && (ux * ex + uy * ey).abs() <= ARROW_SQUARE_TOLERANCE)
                .then_some((*id, a, b))
        });
        let (id, a, b) = on_line.next()?;
        if on_line.next().is_some() || !taken.insert(id) {
            return None;
        }
        edges.push((a, b, slope.atan(), 0.0));
    }
    Some(edges)
}

/// Whether a roof's fields record that its planes are its slope arrows'
/// (RE-307): [`ROOF_ARROW_PLANES_FIELD`].
pub fn roof_arrow_planes_from_fields(fields: &[(String, InstanceField)]) -> bool {
    fields.iter().any(|(name, value)| {
        name == ROOF_ARROW_PLANES_FIELD && matches!(value, InstanceField::Integer { value: 1, .. })
    })
}
