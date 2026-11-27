//! #1839: what stops a beam short of the end of its record box in Revit's export,
//! and does the near face of an element its list names say where?
//!
//! FACT under test: a structural framing record's box runs the beam's location
//! line to its ends, which sit inside the elements it frames into, and Revit's
//! export stops the beam at the near face of the element its reference list names
//! (RE-122 for columns). The rule ships for Revit 2023 (`element_record_beam_cuts::
//! beam_column_trims`). On Revit 2024 the naming holds, but most steel beams of
//! Snowdon Towers' structural sample stop a stored join cutback short of the
//! face, and concrete beams at it, so 2024 and later keep the line's ends. A
//! reporter's Revit 2027 concrete frame has 396 of 719 beams longer than Revit's
//! box (median 296 mm) and none shorter (#1839). The first run of this probe on the
//! BIM4LCA models (RE-455) found the record box ending where Revit's beam ends on
//! nearly every beam, and the beams Revit does shorten naming no column, so the
//! second question is what those beams name.
//!
//! The probe lists, for every placed structural framing record, its box along
//! the longer plan axis, the bounds the 2023 rule would give it, and for each kind
//! of element a beam's list can name (columns of both kinds, walls, other beams) how
//! many it names, how many hold each end and where the beam would stop at an end if
//! the one element of that kind that holds it cut it. It does not compare them with
//! Revit's export: `tools/re/beam_ends_vs_ifc.py` meshes Revit's beam in its own
//! frame and scores the box and each cut against its length. It prints ElementIds
//! and lengths in feet only.
//!
//! Output (one model per run):
//!   file revit=<release> beams=<n> columns=<n> walls=<n>
//!   beam tag=<ElementId> axis=<0|1|-> low=<ft> high=<ft> across=<ft> height=<ft> named=<n>
//!     held_low=<n> held_high=<n> trim_low=<ft|-> trim_high=<ft|->
//!     <kind>_named=<n> <kind>_held=<n>/<n> <kind>_low=<ft|-> <kind>_high=<ft|-> ...
//!     - `axis`: the plan axis the box is longer along (`-` for a square one), `low` and
//!       `high` its bounds on that axis;
//!     - `named`: the columns the beam's list names;
//!     - `held_low` and `held_high`: the named columns that hold each end in plan and
//!       overlap the beam in height (RE-122); one is a cut, two decline the beam;
//!     - `trim_low` and `trim_high`: the bounds on the axis that the 2023 rule gives;
//!     - `<kind>` is `column`, `wall` or `beam`: `_named` counts the elements of that kind
//!       the list names, `_held` the ones that hold each end (the same test as `held_*`),
//!       and `_low` and `_high` the bound on the axis at that end where exactly one holds
//!       it and its near face lies more than 1 mm inside the box, else `-`;
//!     - `section` is a fourth kind: the columns the list names, held and cut not by their
//!       record box, which lies beyond the section's face where a column is turned or its
//!       section is not the box, but by the section the export draws for them (a rectangle
//!       or a circle or a closed outline, turned by the element's rotation and placed at its
//!       location), along the beam's centre line: `section_named` counts the named columns
//!       whose section is read, `section_held` those that hold each end, and `section_low`
//!       and `section_high` the point where the centre line leaves the one that holds the
//!       end, when it lies more than 1 mm inside the box (#1839, a follow-up to RE-455).
//!     - `endcol_low` and `endcol_high` (#1839, the S5 probe): the column the beam names that lies
//!       nearest that end, as `<ElementId>:<t>:<extent>:<source>` (`-` where it names none in line
//!       with the beam, or none within its own extent of the end). `extent` is the column's span
//!       along the beam's centre line in feet, by its section where the export draws one (`s`),
//!       else by its record box (`b`); `t` is where the
//!       record box ends in that span, from the face toward the beam's middle (0) to the face away
//!       from it (1), below 0 where the box stops short of the column and above 1 where it runs
//!       past it. The scorer sets Revit's end beside it as the same fraction of the column Revit
//!       draws, which does not depend on the frame the two files are placed in.
//!     - `angle` (#1839, S5): the beam's plan angle from the bounded line in its element data, in
//!       degrees from the nearest plan axis (0 for a beam along X or Y, within (-45, 45]), or `-`
//!       where the element data holds no line. The record box is the axis-aligned box of the
//!       body, so on a beam that runs oblique to the plan axes its long axis is not the beam's.
//!     - `line_low` and `line_high` (#1839, the S6 probe): the same question as `endcol_*`, asked
//!       along the beam's own line and not along a plan axis, as `<ElementId>:<t>:<extent>:<source>`
//!       or `-`. The line is the bounded line of the beam's element data, `low` its end with the
//!       lower coordinate on `axis`. The column is the named one whose outline the line crosses
//!       nearest that end; `extent` is the chord of the line through that outline, in feet, and `t`
//!       is where the line ends in it, from the face toward the beam's middle (0) to the face away
//!       from it (1). The outline is the plan hull of the column's saved mesh where it decodes
//!       whole (`m`), which is the column's own shape whichever way it is turned; else the section
//!       the export draws, where that is not the column's record box (`s`); else the record box
//!       (`b`). S5 on a structure turned about 4 degrees from the plan axes read every extent
//!       longer than the chord: the export draws a column as its record box, which on a turned
//!       column is the axis-aligned box around it, and `endcol_*` cuts it along a plan axis at the
//!       middle of the beam's own axis-aligned box.
//!   file ... columns_exported=<n> sections=<n> unread=<n> section_centre_offset=<median>/<largest>:
//!   the `IfcColumn`s the export holds, those whose section is read, and those drawn another way
//!   (a mapped or solid body, another profile) or not placed, which the probe leaves out; and how
//!   far the centre of a read section lies from its record box's centre in plan, in feet.
//!   outlines mesh=<n> section=<n> box=<n>: where each column's outline for `line_*` is from.
//!   turncheck ...: the probe's own check on a model it turns (#1839, S6). It turns the whole model
//!   by 4 degrees about its origin (lines, mesh outlines and sections turn as they are; a record
//!   box becomes the axis-aligned box of the turned one, as a file of a turned structure holds it)
//!   and reads every end again:
//!     - `line ends=<n> same=<n> largest_t=<d> largest_extent=<d> mesh=<n>/<n>
//!       mesh_largest_t=<d> mesh_largest_extent=<d> section=<n>/<n> box=<n>/<n>`: the ends read
//!       both ways, those with the same column, `t` and `extent` within 1e-6, the largest
//!       differences over every end with the same column and over the mesh ends alone, and the
//!       same count over the total by the outline's source. A mesh outline must give the same
//!       answer however the model is turned; a record box cannot.
//!     - `s5 extent over the chord: ...`: what `endcol_*` would read as the extent on the turned
//!       model (the column's turned record box, cut along a plan axis), over the chord `line_*`
//!       reads from the mesh outline, in the scorer's bins.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1839_beam_column_ends -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, ProfileDef};
use rvt::research::element_record_beam_cuts::beam_column_trims;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;
use rvt::research::partition_beam_axes::BoundedLine;
use rvt::research::partition_element_records as per;

type Records = BTreeMap<u32, per::PartitionElementRecord>;

/// A column's section in plan, model feet: the profile its extrusion draws, turned by the
/// element's rotation about Z and placed at its location.
enum Footprint {
    Polygon(Vec<[f64; 2]>),
    Circle { centre: [f64; 2], radius: f64 },
}

/// What the export draws for its columns.
struct Sections {
    /// The sections it draws as an extrusion of a rectangle, a circle or a closed outline, by
    /// ElementId.
    read: BTreeMap<u32, Footprint>,
    /// Every `IfcColumn` the export holds: the denominator of what is read, so that a probe that
    /// reads nothing says so beside the columns it should have read.
    exported: usize,
    /// The columns the export draws another way (a mapped or solid body, another profile) or
    /// does not place, which the probe leaves out.
    unread: usize,
}

/// The sections of the columns the export draws, and how many columns it draws another way.
fn footprints(rf: &mut RevitFile) -> anyhow::Result<Sections> {
    let model = RvtDocExporter.export_with_diagnostics(rf)?.model;
    let mut out = BTreeMap::new();
    let mut exported = 0usize;
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid,
            location_feet,
            rotation_radians,
            extrusion,
            solid_shape,
            representation_map_index,
            ..
        } = entity
        else {
            continue;
        };
        // The model writes the type in capitals, as STEP does: the first run of this compared
        // with `IfcColumn`, matched no column and printed `sections=0 unread=0` on ten models.
        if ifc_type != "IFCCOLUMN" {
            continue;
        }
        exported += 1;
        let (Some(tag), Some(origin)) = (type_guid, location_feet) else {
            continue;
        };
        let Ok(id) = tag.parse::<u32>() else {
            continue;
        };
        if out.contains_key(&id) {
            continue;
        }
        let (sin, cos) = rotation_radians.unwrap_or(0.0).sin_cos();
        let place = |p: (f64, f64)| -> [f64; 2] {
            [
                origin[0] + p.0 * cos - p.1 * sin,
                origin[1] + p.0 * sin + p.1 * cos,
            ]
        };
        let (Some(extrusion), None, None) = (extrusion, solid_shape, representation_map_index)
        else {
            continue;
        };
        let rectangle = match &extrusion.profile_override {
            None => Some((extrusion.width_feet, extrusion.depth_feet)),
            Some(ProfileDef::Rectangle {
                width_feet,
                depth_feet,
            }) => Some((*width_feet, *depth_feet)),
            Some(_) => None,
        };
        let footprint = if let Some((w, d)) = rectangle {
            Footprint::Polygon(
                [
                    (-w / 2.0, -d / 2.0),
                    (w / 2.0, -d / 2.0),
                    (w / 2.0, d / 2.0),
                    (-w / 2.0, d / 2.0),
                ]
                .map(place)
                .to_vec(),
            )
        } else {
            match &extrusion.profile_override {
                Some(ProfileDef::Circle { radius_feet }) => Footprint::Circle {
                    centre: place((0.0, 0.0)),
                    radius: *radius_feet,
                },
                Some(ProfileDef::ArbitraryClosed { points }) if points.len() >= 3 => {
                    Footprint::Polygon(points.iter().map(|p| place(*p)).collect())
                }
                _ => continue,
            }
        };
        out.insert(id, footprint);
    }
    Ok(Sections {
        unread: exported - out.len(),
        read: out,
        exported,
    })
}

/// How far the centre of each read section lies from the centre of its column's record box in
/// plan: the median and the largest, in feet. A section placed in another frame than the records
/// would show here, and so would a column drawn off its box's centre.
fn centre_offsets(sections: &BTreeMap<u32, Footprint>, columns: &Records) -> Option<(f64, f64)> {
    let mut away: Vec<f64> = sections
        .iter()
        .filter_map(|(id, footprint)| {
            let b = columns.get(id)?.bbox_feet;
            let centre = match footprint {
                Footprint::Circle { centre, .. } => *centre,
                Footprint::Polygon(points) => {
                    let n = points.len() as f64;
                    [
                        points.iter().map(|p| p[0]).sum::<f64>() / n,
                        points.iter().map(|p| p[1]).sum::<f64>() / n,
                    ]
                }
            };
            Some((centre[0] - (b[0] + b[3]) / 2.0).hypot(centre[1] - (b[1] + b[4]) / 2.0))
        })
        .collect();
    away.sort_by(f64::total_cmp);
    let median = *away.get(away.len() / 2)?;
    Some((median, *away.last()?))
}

/// Where the line `across = c` of a plan axis enters and leaves a section: the first and the
/// last point of the section on it along `axis`, or `None` where it misses the section.
fn span(footprint: &Footprint, axis: usize, c: f64) -> Option<[f64; 2]> {
    let across = 1 - axis;
    match footprint {
        Footprint::Circle { centre, radius } => {
            let offset = c - centre[across];
            (offset.abs() < *radius).then(|| {
                let half = (radius * radius - offset * offset).sqrt();
                [centre[axis] - half, centre[axis] + half]
            })
        }
        Footprint::Polygon(points) => {
            let mut hits = Vec::new();
            for (i, p) in points.iter().enumerate() {
                let q = points[(i + 1) % points.len()];
                let (a, b) = (p[across] - c, q[across] - c);
                // Half open, so that a corner on the line is counted once.
                if (a <= 0.0 && b > 0.0) || (b <= 0.0 && a > 0.0) {
                    let t = a / (a - b);
                    hits.push(p[axis] + t * (q[axis] - p[axis]));
                }
            }
            hits.sort_by(f64::total_cmp);
            Some([*hits.first()?, *hits.last()?])
        }
    }
}

/// The columns `beam` names whose section is read, that hold `end` on `axis` along the beam's
/// centre line and overlap the beam in height, with the span of each section on that line.
fn section_holders<'a>(
    beam: &per::PartitionElementRecord,
    columns: &'a Records,
    sections: &BTreeMap<u32, Footprint>,
    axis: usize,
    end: f64,
) -> Vec<(&'a per::PartitionElementRecord, [f64; 2])> {
    let b = beam.bbox_feet;
    let across = 1 - axis;
    let middle = (b[across] + b[across + 3]) / 2.0;
    columns
        .values()
        .filter_map(|member| {
            let c = member.bbox_feet;
            if member.element_id == beam.element_id
                || !beam.references.contains(&u64::from(member.element_id))
                || !(c[2] < b[5] && b[2] < c[5])
            {
                return None;
            }
            let along = span(sections.get(&member.element_id)?, axis, middle)?;
            (along[0] <= end && end <= along[1]).then_some((member, along))
        })
        .collect()
}

/// Where `beam` would stop at its low or high end on `axis` if the one column that holds that
/// end by its section cut it: the point where the beam's centre line leaves that section,
/// when it lies more than [`MIN_CUT_FEET`] inside the beam's box.
fn section_face(
    beam: &per::PartitionElementRecord,
    columns: &Records,
    sections: &BTreeMap<u32, Footprint>,
    axis: usize,
    low_end: bool,
) -> Option<f64> {
    let b = beam.bbox_feet;
    let end = if low_end { b[axis] } else { b[axis + 3] };
    let held = section_holders(beam, columns, sections, axis, end);
    let [(_, along)] = held.as_slice() else {
        return None;
    };
    let face = if low_end { along[1] } else { along[0] };
    (b[axis] + MIN_CUT_FEET < face && face < b[axis + 3] - MIN_CUT_FEET).then_some(face)
}

/// The kinds of element a beam's list can name, with the categories of each. The columns
/// come first and the walls second: `main` reads them by position.
static KINDS: [(&str, &[i64]); 3] = [
    ("column", &[per::OST_COLUMNS, per::OST_STRUCTURAL_COLUMNS]),
    ("wall", &[per::OST_WALLS]),
    ("beam", &[per::OST_STRUCTURAL_FRAMING]),
];

/// A cut shorter than this does not count as one, feet (1 mm).
const MIN_CUT_FEET: f64 = 0.0033;

/// The first placed-instance record of each ElementId among `records` in `categories`.
fn instances(records: &[per::PartitionElementRecord], categories: &[i64]) -> Records {
    let mut out = BTreeMap::new();
    for record in records {
        if categories.contains(&record.builtin_category) && record.is_exported_instance() {
            out.entry(record.element_id)
                .or_insert_with(|| record.clone());
        }
    }
    out
}

/// The elements of `kind` that `beam`'s list names and that hold `end` on `axis`: they
/// contain it along the axis and the beam's middle across it, and overlap the beam in
/// height (RE-122).
fn holders<'a>(
    beam: &per::PartitionElementRecord,
    kind: &'a Records,
    axis: usize,
    end: f64,
) -> Vec<&'a per::PartitionElementRecord> {
    let b = beam.bbox_feet;
    let across = 1 - axis;
    let middle = (b[across] + b[across + 3]) / 2.0;
    kind.values()
        .filter(|member| {
            let c = member.bbox_feet;
            member.element_id != beam.element_id
                && beam.references.contains(&u64::from(member.element_id))
                && c[axis] <= end
                && end <= c[axis + 3]
                && c[across] <= middle
                && middle <= c[across + 3]
                && c[2] < b[5]
                && b[2] < c[5]
        })
        .collect()
}

/// Where `beam` would stop at its low or high end on `axis` if the one element of `kind`
/// that holds that end cut it: that element's near face, when it lies more than
/// [`MIN_CUT_FEET`] inside the beam's box.
fn near_face(
    beam: &per::PartitionElementRecord,
    kind: &Records,
    axis: usize,
    low_end: bool,
) -> Option<f64> {
    let b = beam.bbox_feet;
    let end = if low_end { b[axis] } else { b[axis + 3] };
    let held = holders(beam, kind, axis, end);
    let [member] = held.as_slice() else {
        return None;
    };
    let c = member.bbox_feet;
    let face = if low_end { c[axis + 3] } else { c[axis] };
    (b[axis] + MIN_CUT_FEET < face && face < b[axis + 3] - MIN_CUT_FEET).then_some(face)
}

/// The beam's plan angle from its bounded line, in degrees from the nearest plan axis, within
/// (-45, 45]: 0 for a beam along X or Y. `None` for a line with no length in plan.
fn plan_angle(line: &BoundedLine) -> Option<f64> {
    let (start, end) = (line.start(), line.end());
    let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
    let length = dx.hypot(dy);
    if !length.is_finite() || length <= 1e-6 {
        return None;
    }
    let mut angle = dy.atan2(dx).to_degrees() % 90.0;
    if angle > 45.0 {
        angle -= 90.0;
    } else if angle <= -45.0 {
        angle += 90.0;
    }
    Some(angle)
}

fn feet(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| format!("{v:.4}"))
}

/// The column `beam` names that lies nearest its low or high end on `axis`, as
/// `<ElementId>:<t>:<extent>:<source>`, or `-` where it names none in line with the beam (#1839).
///
/// `extent` is the column's span along the beam's centre line, in feet: by its section where
/// the export draws one (source `s`), else by its record box (`b`). `t` is where the beam's
/// record box ends in that span, from the face toward the beam's middle (0) to the face away
/// from it (1): below 0 the box stops short of the column, above 1 it runs past it. The
/// scorer sets Revit's end beside it as the same fraction of the column Revit draws, which
/// does not depend on the frame the two files are placed in.
fn nearest_column(
    beam: &per::PartitionElementRecord,
    columns: &Records,
    sections: &BTreeMap<u32, Footprint>,
    axis: usize,
    low_end: bool,
) -> String {
    let b = beam.bbox_feet;
    let across = 1 - axis;
    let middle = (b[across] + b[across + 3]) / 2.0;
    let end = if low_end { b[axis] } else { b[axis + 3] };
    let mut best: Option<(f64, u32, [f64; 2], char)> = None;
    for member in columns.values() {
        let c = member.bbox_feet;
        if member.element_id == beam.element_id
            || !beam.references.contains(&u64::from(member.element_id))
            || !(c[2] < b[5] && b[2] < c[5])
            || !(c[across] <= middle && middle <= c[across + 3])
        {
            continue;
        }
        let (along, source) = match sections
            .get(&member.element_id)
            .and_then(|footprint| span(footprint, axis, middle))
        {
            Some(along) => (along, 's'),
            None => ([c[axis], c[axis + 3]], 'b'),
        };
        let distance = (along[0] - end).max(end - along[1]).max(0.0);
        if best.is_none_or(|(nearest, ..)| distance < nearest) {
            best = Some((distance, member.element_id, along, source));
        }
    }
    let Some((distance, id, along, source)) = best else {
        return "-".to_string();
    };
    // A column more than its own extent away from the end holds none: a beam whose list names
    // one column has it at the other end, and the first run took it for both (#1839, S5).
    let extent = along[1] - along[0];
    if extent <= 0.0 || distance > extent {
        return "-".to_string();
    }
    let (near, far) = if low_end {
        (along[1], along[0])
    } else {
        (along[0], along[1])
    };
    format!(
        "{id}:{:.4}:{extent:.4}:{source}",
        (end - near) / (far - near)
    )
}

/// A plan point, model feet.
type P2 = [f64; 2];

/// The turn the probe checks itself with, degrees (#1839, S6): the reporter's structure is turned
/// about this much from the plan axes of the model's own frame.
const TURN_CHECK_DEGREES: f64 = 4.0;

/// The sine and cosine of no turn.
const NO_TURN: (f64, f64) = (0.0, 1.0);

/// `p` turned about the model's origin by the angle whose sine and cosine are `turn`.
fn turned(p: P2, turn: (f64, f64)) -> P2 {
    let (sin, cos) = turn;
    [p[0] * cos - p[1] * sin, p[0] * sin + p[1] * cos]
}

/// The axis-aligned box of `b` turned about the model's origin: what the record of a turned
/// element holds as its box.
fn turned_box(b: [f64; 6], turn: (f64, f64)) -> [f64; 6] {
    let corners = [[b[0], b[1]], [b[3], b[1]], [b[3], b[4]], [b[0], b[4]]].map(|p| turned(p, turn));
    let xs = corners.map(|p| p[0]);
    let ys = corners.map(|p| p[1]);
    let low = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
    let high = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
    [low(xs), low(ys), b[2], high(xs), high(ys), b[5]]
}

/// The convex hull of plan points, counter-clockwise (the monotone chain).
fn hull(mut points: Vec<P2>) -> Vec<P2> {
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let cross = |o: P2, a: P2, b: P2| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut lower: Vec<P2> = Vec::new();
    for p in &points {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<P2> = Vec::new();
    for p in points.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The plan outline of each column whose saved mesh decodes whole: the hull of its vertices, in
/// the document's frame. A column turned from the plan axes has its own outline here, where its
/// record box is the axis-aligned box around it.
fn mesh_outlines(rf: &mut RevitFile, columns: &Records) -> BTreeMap<u32, Vec<P2>> {
    let mut out = BTreeMap::new();
    if columns.is_empty() {
        return out;
    }
    let options = Options {
        selected_ids: columns.keys().map(|id| u64::from(*id)).collect(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = match extract_at_detail(rf, &options, 3) {
        Ok(scene) => scene,
        Err(error) => {
            println!("saved graphics: {error:#}");
            return out;
        }
    };
    for element in &scene.elements {
        if !element.meshes.diagnostics.is_empty() || element.meshes.unbounded_faces != 0 {
            continue;
        }
        let points: Vec<P2> = element
            .meshes
            .primitives
            .iter()
            .filter(|primitive| !primitive.hidden && !primitive.filtered)
            .flat_map(|primitive| primitive.vertices.iter().map(|v| [v[0], v[1]]))
            .collect();
        let outline = hull(points);
        if let (Ok(id), true) = (u32::try_from(element.id), outline.len() >= 3) {
            out.insert(id, outline);
        }
    }
    out
}

/// Whether a section's outline is the plan rectangle of the record box `c`, within a thousandth
/// of a foot: the export draws a column with no section of its own as that rectangle.
fn is_box(points: &[P2], c: &[f64; 6]) -> bool {
    points.len() == 4
        && points.iter().all(|p| {
            ((p[0] - c[0]).abs() < 1e-3 || (p[0] - c[3]).abs() < 1e-3)
                && ((p[1] - c[1]).abs() < 1e-3 || (p[1] - c[4]).abs() < 1e-3)
        })
}

/// A column's plan outline with the whole model turned by `turn`, and where it is from: the hull
/// of its saved mesh (`m`), else the section the export draws where that is not its record box
/// (`s`), else its record box (`b`). The mesh and the section turn as they are; a record box
/// becomes the axis-aligned box of the turned one.
fn column_outline(
    column: &per::PartitionElementRecord,
    meshes: &BTreeMap<u32, Vec<P2>>,
    sections: &BTreeMap<u32, Footprint>,
    turn: (f64, f64),
) -> (Footprint, char) {
    if let Some(outline) = meshes.get(&column.element_id) {
        let points = outline.iter().map(|p| turned(*p, turn)).collect();
        return (Footprint::Polygon(points), 'm');
    }
    let c = column.bbox_feet;
    match sections.get(&column.element_id) {
        Some(Footprint::Circle { centre, radius }) => {
            let circle = Footprint::Circle {
                centre: turned(*centre, turn),
                radius: *radius,
            };
            return (circle, 's');
        }
        Some(Footprint::Polygon(points)) if !is_box(points, &c) => {
            let points = points.iter().map(|p| turned(*p, turn)).collect();
            return (Footprint::Polygon(points), 's');
        }
        _ => {}
    }
    let b = turned_box(c, turn);
    let rectangle = vec![[b[0], b[1]], [b[3], b[1]], [b[3], b[4]], [b[0], b[4]]];
    (Footprint::Polygon(rectangle), 'b')
}

/// Where the line through `origin` along the unit `direction` enters and leaves an outline, as
/// distances along it from `origin`, or `None` where it misses.
fn chord(outline: &Footprint, origin: P2, direction: P2) -> Option<[f64; 2]> {
    let along = |p: P2| (p[0] - origin[0]) * direction[0] + (p[1] - origin[1]) * direction[1];
    let side = |p: P2| (p[1] - origin[1]) * direction[0] - (p[0] - origin[0]) * direction[1];
    match outline {
        Footprint::Circle { centre, radius } => {
            let offset = side(*centre);
            (offset.abs() < *radius).then(|| {
                let half = (radius * radius - offset * offset).sqrt();
                [along(*centre) - half, along(*centre) + half]
            })
        }
        Footprint::Polygon(points) => {
            let mut hits = Vec::new();
            for (i, p) in points.iter().enumerate() {
                let q = points[(i + 1) % points.len()];
                let (a, b) = (side(*p), side(q));
                // Half open, so that a corner on the line is counted once.
                if (a <= 0.0 && b > 0.0) || (b <= 0.0 && a > 0.0) {
                    let t = a / (a - b);
                    hits.push(along(*p) + t * (along(q) - along(*p)));
                }
            }
            hits.sort_by(f64::total_cmp);
            Some([*hits.first()?, *hits.last()?])
        }
    }
}

/// One end of a beam by the beam's own line (#1839, S6).
#[derive(Clone, Copy)]
struct LineEnd {
    column: u32,
    t: f64,
    extent: f64,
    source: char,
}

/// The column `beam` names whose outline its own line crosses nearest its first (`at_start`) or
/// second end, with the whole model turned by `turn`: the chord of the line through that outline
/// and where the line ends in it, as [`nearest_column`] gives them along a plan axis.
fn line_end(
    beam: &per::PartitionElementRecord,
    line: &BoundedLine,
    columns: &Records,
    meshes: &BTreeMap<u32, Vec<P2>>,
    sections: &BTreeMap<u32, Footprint>,
    at_start: bool,
    turn: (f64, f64),
) -> Option<LineEnd> {
    let (start, end) = (line.start(), line.end());
    let start = turned([start[0], start[1]], turn);
    let end = turned([end[0], end[1]], turn);
    let length = (end[0] - start[0]).hypot(end[1] - start[1]);
    if !length.is_finite() || length <= 1e-6 {
        return None;
    }
    let direction = [(end[0] - start[0]) / length, (end[1] - start[1]) / length];
    let at = if at_start { 0.0 } else { length };
    let b = beam.bbox_feet;
    let mut best: Option<(f64, u32, [f64; 2], char)> = None;
    for member in columns.values() {
        let c = member.bbox_feet;
        if member.element_id == beam.element_id
            || !beam.references.contains(&u64::from(member.element_id))
            || !(c[2] < b[5] && b[2] < c[5])
        {
            continue;
        }
        let (outline, source) = column_outline(member, meshes, sections, turn);
        let Some(along) = chord(&outline, start, direction) else {
            continue;
        };
        let distance = (along[0] - at).max(at - along[1]).max(0.0);
        if best.is_none_or(|(nearest, ..)| distance < nearest) {
            best = Some((distance, member.element_id, along, source));
        }
    }
    let (distance, column, along, source) = best?;
    let extent = along[1] - along[0];
    if extent <= 0.0 || distance > extent {
        return None;
    }
    let (near, far) = if at_start {
        (along[1], along[0])
    } else {
        (along[0], along[1])
    };
    Some(LineEnd {
        column,
        t: (at - near) / (far - near),
        extent,
        source,
    })
}

fn line_field(end: Option<LineEnd>) -> String {
    end.map_or_else(
        || "-".to_string(),
        |e| format!("{}:{:.4}:{:.4}:{}", e.column, e.t, e.extent, e.source),
    )
}

/// What `endcol_*` reads as a column's extent at the low or high end of a beam whose record box
/// is `b`, where every named column is drawn as its record box (`named`): the box nearest that
/// end along the box's longer plan axis, at the middle of the beam's box across it.
fn boxed_extent(b: [f64; 6], named: &[[f64; 6]], low_end: bool) -> Option<f64> {
    let (along_x, along_y) = (b[3] - b[0], b[4] - b[1]);
    if along_x == along_y {
        return None;
    }
    let axis = usize::from(along_y > along_x);
    let across = 1 - axis;
    let middle = (b[across] + b[across + 3]) / 2.0;
    let end = if low_end { b[axis] } else { b[axis + 3] };
    let mut best: Option<(f64, f64)> = None;
    for c in named {
        if !(c[across] <= middle && middle <= c[across + 3]) {
            continue;
        }
        let distance = (c[axis] - end).max(end - c[axis + 3]).max(0.0);
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, c[axis + 3] - c[axis]));
        }
    }
    let (distance, extent) = best?;
    (extent > 0.0 && distance <= extent).then_some(extent)
}

/// The probe's check of `line_*` on a model it turns (#1839, S6).
#[derive(Default)]
struct TurnCheck {
    /// Ends read with and without the turn, and those with the same column, `t` and extent.
    ends: usize,
    same: usize,
    largest_t: f64,
    largest_extent: f64,
    /// The same, over the ends whose outline is a saved mesh alone: a box has to change.
    mesh_largest_t: f64,
    mesh_largest_extent: f64,
    /// By the outline's source without the turn (`m`, `s`, `b`): same, total.
    by_source: BTreeMap<char, (usize, usize)>,
    /// `endcol_*`'s extent on the turned model over the chord of the mesh outline, in the scorer's
    /// bins: under 0.99, within 1 per cent, 1.01 to 1.1, 1.1 to 1.5, over 1.5; and no extent.
    s5_bins: [usize; 5],
    s5_none: usize,
}

impl TurnCheck {
    #[allow(clippy::too_many_arguments)]
    fn read(
        &mut self,
        beam: &per::PartitionElementRecord,
        line: &BoundedLine,
        columns: &Records,
        meshes: &BTreeMap<u32, Vec<P2>>,
        sections: &BTreeMap<u32, Footprint>,
        turn: (f64, f64),
    ) {
        let b = turned_box(beam.bbox_feet, turn);
        let named: Vec<[f64; 6]> = columns
            .values()
            .filter(|member| {
                let c = member.bbox_feet;
                member.element_id != beam.element_id
                    && beam.references.contains(&u64::from(member.element_id))
                    && c[2] < b[5]
                    && b[2] < c[5]
            })
            .map(|member| turned_box(member.bbox_feet, turn))
            .collect();
        let axis = usize::from(b[4] - b[1] > b[3] - b[0]);
        let (start, end) = (line.start(), line.end());
        let start = turned([start[0], start[1]], turn);
        let end = turned([end[0], end[1]], turn);
        let low_is_start = start[axis] <= end[axis];
        for at_start in [true, false] {
            let plain = line_end(beam, line, columns, meshes, sections, at_start, NO_TURN);
            let again = line_end(beam, line, columns, meshes, sections, at_start, turn);
            if let (Some(plain), Some(again)) = (plain, again) {
                let (dt, de) = (
                    (plain.t - again.t).abs(),
                    (plain.extent - again.extent).abs(),
                );
                let same = plain.column == again.column && dt <= 1e-6 && de <= 1e-6;
                self.ends += 1;
                self.same += usize::from(same);
                let entry = self.by_source.entry(plain.source).or_default();
                entry.0 += usize::from(same);
                entry.1 += 1;
                if plain.column == again.column {
                    self.largest_t = self.largest_t.max(dt);
                    self.largest_extent = self.largest_extent.max(de);
                    if plain.source == 'm' {
                        self.mesh_largest_t = self.mesh_largest_t.max(dt);
                        self.mesh_largest_extent = self.mesh_largest_extent.max(de);
                    }
                }
            }
            let Some(plain) = plain.filter(|end| end.source == 'm') else {
                continue;
            };
            match boxed_extent(b, &named, at_start == low_is_start) {
                Some(extent) => {
                    let ratio = extent / plain.extent;
                    let bin = if ratio < 0.99 {
                        0
                    } else if ratio <= 1.01 {
                        1
                    } else if ratio <= 1.1 {
                        2
                    } else if ratio <= 1.5 {
                        3
                    } else {
                        4
                    };
                    self.s5_bins[bin] += 1;
                }
                None => self.s5_none += 1,
            }
        }
    }

    fn print(&self) {
        let of = |source: char| {
            let (same, total) = self.by_source.get(&source).copied().unwrap_or_default();
            format!("{same}/{total}")
        };
        println!(
            "turncheck degrees={TURN_CHECK_DEGREES} line ends={} same={} largest_t={:.9} largest_extent={:.9} mesh={} mesh_largest_t={:.9} mesh_largest_extent={:.9} section={} box={}",
            self.ends,
            self.same,
            self.largest_t,
            self.largest_extent,
            of('m'),
            self.mesh_largest_t,
            self.mesh_largest_extent,
            of('s'),
            of('b'),
        );
        let [under, within, to_110, to_150, over] = self.s5_bins;
        println!(
            "turncheck degrees={TURN_CHECK_DEGREES} s5 extent over the chord: under 0.99 {under}; within 1 per cent {within}; 1.01 to 1.1 {to_110}; 1.1 to 1.5 {to_150}; over 1.5 {over}; no s5 extent {}",
            self.s5_none
        );
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1839_beam_column_ends MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories: Vec<i64> = KINDS
        .iter()
        .flat_map(|(_, kind)| kind.iter().copied())
        .collect();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let kinds: Vec<(&str, Records)> = KINDS
        .iter()
        .map(|(name, kind)| (*name, instances(&records, kind)))
        .collect();
    let beams = instances(&records, &[per::OST_STRUCTURAL_FRAMING]);
    let columns = &kinds[0].1;
    // Each beam's own line, for its plan angle (RE-428): none where the element data holds none.
    let beam_ids: BTreeSet<u32> = beams.keys().copied().collect();
    let lines =
        rvt::research::partition_beam_axes::scan_first_bounded_lines(&mut rf, version, &beam_ids)
            .unwrap_or_default();
    let beam_list: Vec<per::PartitionElementRecord> = beams.values().cloned().collect();
    let column_list: Vec<per::PartitionElementRecord> = columns.values().cloned().collect();
    let trims = beam_column_trims(&beam_list, &column_list);
    let drawn = footprints(&mut rf)?;
    let sections = drawn.read;
    let meshes = mesh_outlines(&mut rf, columns);
    let mut sources: BTreeMap<char, usize> = BTreeMap::new();
    for column in columns.values() {
        *sources
            .entry(column_outline(column, &meshes, &sections, NO_TURN).1)
            .or_default() += 1;
    }
    let turn = TURN_CHECK_DEGREES.to_radians().sin_cos();
    let mut check = TurnCheck::default();
    let offset = centre_offsets(&sections, columns).map_or_else(
        || "-".to_string(),
        |(median, largest)| format!("{median:.4}/{largest:.4}"),
    );
    println!(
        "file revit={version} beams={} columns={} walls={} columns_exported={} sections={} unread={} section_centre_offset={offset}",
        beams.len(),
        columns.len(),
        kinds[1].1.len(),
        drawn.exported,
        sections.len(),
        drawn.unread
    );
    println!(
        "outlines mesh={} section={} box={}",
        sources.get(&'m').copied().unwrap_or(0),
        sources.get(&'s').copied().unwrap_or(0),
        sources.get(&'b').copied().unwrap_or(0),
    );
    for (id, beam) in &beams {
        let b = beam.bbox_feet;
        let (along_x, along_y) = (b[3] - b[0], b[4] - b[1]);
        let named = columns
            .keys()
            .filter(|column| beam.references.contains(&u64::from(**column)))
            .count();
        let angle = lines
            .get(id)
            .and_then(plan_angle)
            .map_or_else(|| "-".to_string(), |degrees| format!("{degrees:.2}"));
        if along_x == along_y {
            println!(
                "beam tag={id} axis=- low=0 high=0 across=0 height={:.4} named={named} held_low=0 held_high=0 trim_low=- trim_high=- angle={angle}",
                b[5] - b[2]
            );
            continue;
        }
        let axis = usize::from(along_y > along_x);
        let across = 1 - axis;
        let (trim_low, trim_high) = match trims.get(id) {
            Some(trim) => (
                format!("{:.4}", trim.low_feet),
                format!("{:.4}", trim.high_feet),
            ),
            None => ("-".to_string(), "-".to_string()),
        };
        let mut line = format!(
            "beam tag={id} axis={axis} low={:.4} high={:.4} across={:.4} height={:.4} named={named} held_low={} held_high={} trim_low={trim_low} trim_high={trim_high}",
            b[axis],
            b[axis + 3],
            b[across + 3] - b[across],
            b[5] - b[2],
            holders(beam, columns, axis, b[axis]).len(),
            holders(beam, columns, axis, b[axis + 3]).len(),
        );
        for (name, kind) in &kinds {
            let named = kind
                .values()
                .filter(|member| {
                    member.element_id != *id
                        && beam.references.contains(&u64::from(member.element_id))
                })
                .count();
            write!(
                line,
                " {name}_named={named} {name}_held={}/{} {name}_low={} {name}_high={}",
                holders(beam, kind, axis, b[axis]).len(),
                holders(beam, kind, axis, b[axis + 3]).len(),
                feet(near_face(beam, kind, axis, true)),
                feet(near_face(beam, kind, axis, false)),
            )?;
        }
        // The columns by their section: the named ones whose section is read, the ones that
        // hold each end along the centre line, and where it leaves the one that holds an end.
        let read = columns
            .keys()
            .filter(|column| {
                beam.references.contains(&u64::from(**column)) && sections.contains_key(*column)
            })
            .count();
        write!(
            line,
            " section_named={read} section_held={}/{} section_low={} section_high={}",
            section_holders(beam, columns, &sections, axis, b[axis]).len(),
            section_holders(beam, columns, &sections, axis, b[axis + 3]).len(),
            feet(section_face(beam, columns, &sections, axis, true)),
            feet(section_face(beam, columns, &sections, axis, false)),
        )?;
        // The named column nearest each end, and where the record box ends in it (#1839).
        write!(
            line,
            " endcol_low={} endcol_high={} angle={angle}",
            nearest_column(beam, columns, &sections, axis, true),
            nearest_column(beam, columns, &sections, axis, false),
        )?;
        // The same along the beam's own line, by the column's own outline (#1839, S6).
        let (line_low, line_high) = match lines.get(id) {
            Some(own) => {
                let low_is_start = own.start()[axis] <= own.end()[axis];
                let at = |at_start: bool| {
                    line_end(beam, own, columns, &meshes, &sections, at_start, NO_TURN)
                };
                check.read(beam, own, columns, &meshes, &sections, turn);
                (at(low_is_start), at(!low_is_start))
            }
            None => (None, None),
        };
        write!(
            line,
            " line_low={} line_high={}",
            line_field(line_low),
            line_field(line_high)
        )?;
        println!("{line}");
    }
    check.print();
    Ok(())
}
