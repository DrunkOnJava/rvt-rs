//! #2096: is a column turned off the model's axes drawn turned where the export draws it from its
//! type's section, and would turning that section by the column's own transform draw it as Revit
//! does?
//!
//! FACT under test: the export draws a column whose body is not its saved mesh as its type's
//! section at its record box's centre, turned by the element's `rotation_radians`, which no reader
//! sets for a column (`attach_instance_axes`, RE-87, covers doors, windows and products only), so
//! the section stands square to the model's axes whatever the column's own turn. A column's saved
//! mesh is drawn turned as Revit draws it, so where both exist, the mesh's plan outline is the
//! answer the section should give, in the same file and the same frame.
//!
//! For every `IfcColumn` the section path draws (`RvtDocExporter`, no saved meshes attached), the
//! probe reads the column's instance transform and, where its saved graphics mesh whole, the hull
//! of the mesh in plan, and prints:
//!   column tag=<ElementId> angle=<deg|-> turn=<aligned|quarter|oblique|-> rotation=<deg|->
//!     section=<rectangle|circle|outline|box|other> mesh=<yes|no> drawn=<ft|-> turned=<ft|->
//!     section_size=<w>x<d> mesh_size=<w>x<d> centre_offset=<ft> mesh_corners=<n>
//!   - `angle`: the plan angle of the instance's X axis from the model's, degrees in (-180, 180],
//!     `-` where no transform is read or the instance is not upright;
//!   - `turn`: `aligned` at a multiple of 180 degrees, `quarter` at 90 or -90, else `oblique`
//!     (within 1e-6 degrees);
//!   - `rotation`: the `rotation_radians` the export sets, in degrees;
//!   - `section`: what the extrusion draws (`box` where its rectangle is the record box's plan
//!     rectangle within 0.001 ft, the export's fallback with no section joined);
//!   - `drawn`: the farthest a corner of the section as drawn lies from the nearest vertex of the
//!     mesh's hull, feet (`-` without a mesh or for a circle);
//!   - `turned`: the same with the section turned about its centre by `angle` less the export's
//!     rotation, the outline the fix would draw;
//!   - `section_size`, `mesh_size`, `centre_offset`, `mesh_corners`: the section's and the mesh
//!     outline's plan extents along the model's axes (feet), how far their centres lie apart,
//!     and how many corners the mesh's outline has (`-` without both), which say why the two
//!     differ where they do.
//!
//! and one `tally <key> <n>` line per turn and section kind. ElementIds, angles and distances in
//! feet only.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2096_turned_columns -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, ProfileDef};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;
use rvt::research::partition_element_records as per;
use rvt::research::partition_instance_transforms::scan_instance_transforms;

type P2 = [f64; 2];

/// Degrees within which an angle counts as a multiple of 90.
const SQUARE: f64 = 1e-6;
/// Feet within which a corner counts as on the mesh's.
const NEAR: f64 = 0.01;

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

/// The farthest a point of `outline` lies from the nearest vertex of `hull`.
fn farthest(outline: &[P2], hull: &[P2]) -> f64 {
    outline
        .iter()
        .map(|p| {
            hull.iter()
                .map(|q| (p[0] - q[0]).hypot(p[1] - q[1]))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

/// `points` turned about `centre` by `radians`.
fn turned(points: &[P2], centre: P2, radians: f64) -> Vec<P2> {
    let (sin, cos) = radians.sin_cos();
    points
        .iter()
        .map(|p| {
            let (x, y) = (p[0] - centre[0], p[1] - centre[1]);
            [centre[0] + x * cos - y * sin, centre[1] + x * sin + y * cos]
        })
        .collect()
}

/// A column the section path draws.
struct Drawn {
    rotation: Option<f64>,
    section: &'static str,
    /// The section's plan corners as drawn, and its centre; none for a circle or another body.
    corners: Option<(Vec<P2>, P2)>,
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2096_turned_columns MODEL.rvt"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.basic_file_info()?.version;
    // Each column's record box, to tell a section from the export's record-box fallback.
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = [per::OST_COLUMNS, per::OST_STRUCTURAL_COLUMNS];
    let boxes: BTreeMap<u32, [f64; 6]> =
        per::scan_category_records_multi(&mut rf, release, &categories, &declared)?
            .into_iter()
            .filter(|record| {
                categories.contains(&record.builtin_category) && record.is_exported_instance()
            })
            .map(|record| (record.element_id, record.bbox_feet))
            .collect();
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut drawn: BTreeMap<u32, Drawn> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
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
        // The model writes the type in capitals, as STEP does.
        if ifc_type != "IFCCOLUMN" {
            continue;
        }
        let Ok(id) = tag.parse::<u32>() else {
            continue;
        };
        let rotation = *rotation_radians;
        let (Some(extrusion), None, None, Some(origin)) = (
            extrusion,
            solid_shape,
            representation_map_index,
            location_feet,
        ) else {
            drawn.entry(id).or_insert(Drawn {
                rotation,
                section: "other",
                corners: None,
            });
            continue;
        };
        let (sin, cos) = rotation.unwrap_or(0.0).sin_cos();
        let place = |p: (f64, f64)| -> P2 {
            [
                origin[0] + p.0 * cos - p.1 * sin,
                origin[1] + p.0 * sin + p.1 * cos,
            ]
        };
        let rectangle = match &extrusion.profile_override {
            None => Some((extrusion.width_feet, extrusion.depth_feet)),
            Some(ProfileDef::Rectangle {
                width_feet,
                depth_feet,
            }) => Some((*width_feet, *depth_feet)),
            Some(_) => None,
        };
        let centre = place((0.0, 0.0));
        let (section, corners) = match (rectangle, &extrusion.profile_override) {
            (Some((w, d)), _) => {
                let corners: Vec<P2> = [
                    (-w / 2.0, -d / 2.0),
                    (w / 2.0, -d / 2.0),
                    (w / 2.0, d / 2.0),
                    (-w / 2.0, d / 2.0),
                ]
                .map(place)
                .to_vec();
                let is_box = boxes.get(&id).is_some_and(|b| {
                    corners.iter().all(|p| {
                        ((p[0] - b[0]).abs() < 1e-3 || (p[0] - b[3]).abs() < 1e-3)
                            && ((p[1] - b[1]).abs() < 1e-3 || (p[1] - b[4]).abs() < 1e-3)
                    })
                });
                (
                    if is_box { "box" } else { "rectangle" },
                    Some((corners, centre)),
                )
            }
            (None, Some(ProfileDef::Circle { .. })) => ("circle", None),
            (None, Some(ProfileDef::ArbitraryClosed { points })) if points.len() >= 3 => (
                "outline",
                Some((points.iter().map(|p| place(*p)).collect(), centre)),
            ),
            _ => ("other", None),
        };
        drawn.entry(id).or_insert(Drawn {
            rotation,
            section,
            corners,
        });
    }

    let ids: BTreeSet<u32> = drawn.keys().copied().collect();
    let transforms = scan_instance_transforms(&mut rf, release, &ids)?;
    let mut meshes: BTreeMap<u32, Vec<P2>> = BTreeMap::new();
    if !ids.is_empty() {
        let options = Options {
            selected_ids: ids.iter().map(|id| u64::from(*id)).collect(),
            max_graph_values: 1_000_000,
            max_graph_objects: 1_000_000,
            ..Options::default()
        };
        match extract_at_detail(&mut rf, &options, 3) {
            Ok(scene) => {
                for element in &scene.elements {
                    if !element.meshes.diagnostics.is_empty() || element.meshes.unbounded_faces != 0
                    {
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
                        meshes.insert(id, outline);
                    }
                }
            }
            Err(error) => println!("saved graphics: {error:#}"),
        }
    }

    println!(
        "file revit={release} columns={} transforms={} meshes={}",
        drawn.len(),
        transforms.len(),
        meshes.len()
    );
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for (id, column) in &drawn {
        let angle = transforms
            .get(id)
            .filter(|t| t.is_upright())
            .map(|t| t.axes[0][1].atan2(t.axes[0][0]).to_degrees());
        let turn = match angle {
            None => "-",
            Some(a) => {
                let folded = a.rem_euclid(180.0);
                if folded.min(180.0 - folded) < SQUARE {
                    "aligned"
                } else if (folded - 90.0).abs() < SQUARE {
                    "quarter"
                } else {
                    "oblique"
                }
            }
        };
        let mesh = meshes.get(id);
        let distance = |outline: &[P2]| -> Option<f64> { mesh.map(|hull| farthest(outline, hull)) };
        let as_drawn = column
            .corners
            .as_ref()
            .and_then(|(corners, _)| distance(corners));
        let as_turned = column
            .corners
            .as_ref()
            .zip(angle)
            .and_then(|((corners, centre), a)| {
                let by = a.to_radians() - column.rotation.unwrap_or(0.0);
                distance(&turned(corners, *centre, by))
            });
        let feet = |d: Option<f64>| d.map_or_else(|| "-".to_string(), |d| format!("{d:.6}"));
        // Why a section and its mesh differ: each one's plan extents along the model's
        // axes, the offset of their centres, and how many corners the mesh's outline has.
        let extents = |points: &[P2]| -> (f64, f64, P2) {
            let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            for p in points {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            (
                hi[0] - lo[0],
                hi[1] - lo[1],
                [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.],
            )
        };
        let shape = match (column.corners.as_ref(), mesh) {
            (Some((corners, _)), Some(hull)) => {
                let (sw, sd, sc) = extents(corners);
                let (mw, md, mc) = extents(hull);
                format!(
                    "section_size={sw:.4}x{sd:.4} mesh_size={mw:.4}x{md:.4} centre_offset={:.4} mesh_corners={}",
                    (sc[0] - mc[0]).hypot(sc[1] - mc[1]),
                    hull.len()
                )
            }
            _ => "section_size=- mesh_size=- centre_offset=- mesh_corners=-".to_string(),
        };
        println!(
            "column tag={id} angle={} turn={turn} rotation={} section={} mesh={} drawn={} turned={} {shape}",
            angle.map_or_else(|| "-".to_string(), |a| format!("{a:.6}")),
            column
                .rotation
                .map_or_else(|| "-".to_string(), |r| format!("{:.6}", r.to_degrees())),
            column.section,
            if mesh.is_some() { "yes" } else { "no" },
            feet(as_drawn),
            feet(as_turned),
        );
        *tally
            .entry(format!("{turn} section={}", column.section))
            .or_default() += 1;
        if let (Some(d), Some(t)) = (as_drawn, as_turned) {
            let near = |x: f64| if x <= NEAR { "near" } else { "far" };
            *tally
                .entry(format!(
                    "{turn} section={} with mesh: drawn {} turned {}",
                    column.section,
                    near(d),
                    near(t)
                ))
                .or_default() += 1;
        }
    }
    for (key, n) in &tally {
        println!("tally {key} {n}");
    }
    Ok(())
}
