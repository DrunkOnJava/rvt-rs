//! RE-53: walls drawn as their layers, each in its material's colour.
//!
//! FACT: a wall, floor, roof or ceiling type's data carries its compound
//! structure (`rvt::partition_compound_structure`), a material's object
//! carries its shading colour (`rvt::partition_materials`), and a wall's
//! data carries a flip flag that puts its exterior to the right or the left
//! of its location line. On Autodesk's Snowdon Towers 2024 architectural
//! sample the layers equal Revit's own on every type measured, and the flag
//! gives the side Revit's own IFC4 export draws the exterior layer on for
//! every wall where that side can be measured.
//!
//! The probe exports the model as the viewer does and, for each wall with
//! layers, cuts its body into them as the glTF export does. It prints how
//! many walls are drawn in layers and why the others are not. With
//! `--json PATH` it writes, per wall, its exterior direction and each
//! layer's span along it in model feet, for comparison with the per-layer
//! solids of Revit's IFC4 export.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re53_wall_layers -- \
//!     MODEL.rvt [--json OUT.json] [--verbose]

use rvt::RevitFile;
use rvt::ifc::body_geometry::{Body, Placement, element_body, layered_extrusion_meshes};
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{Exporter, RvtDocExporter};
use std::collections::BTreeMap;
use std::fmt::Write as _;

fn main() -> rvt::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: MODEL.rvt [--json OUT.json]");
    let rest: Vec<String> = args.collect();
    let json_path = rest
        .iter()
        .position(|a| a == "--json")
        .and_then(|i| rest.get(i + 1).cloned());
    let verbose = rest.iter().any(|a| a == "--verbose");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export(&mut rf)?;
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    let mut json = String::from("[");
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            type_guid,
            location_feet,
            rotation_radians,
            ..
        } = entity
        else {
            continue;
        };
        let Some(layers) = type_guid
            .as_deref()
            .and_then(|tag| tag.parse::<u32>().ok())
            .and_then(|id| model.element_layers.get(&id))
        else {
            continue;
        };
        let Some(Body::Extrusion(extrusion)) = element_body(entity, &model.representation_maps)
        else {
            *census.entry("layers, body not an extrusion").or_default() += 1;
            continue;
        };
        let placement = Placement::new(*location_feet, *rotation_radians);
        let [nx, ny] = layers.exterior_normal;
        let local = [
            nx * placement.cos + ny * placement.sin,
            -nx * placement.sin + ny * placement.cos,
        ];
        let widths: Vec<f64> = layers.layers.iter().map(|l| l.width_feet).collect();
        let Some(meshes) = layered_extrusion_meshes(extrusion, local, &widths) else {
            *census
                .entry("layers do not add up to the body's thickness")
                .or_default() += 1;
            if verbose {
                let thickness =
                    rvt::ifc::body_geometry::extrusion_rings(extrusion).map(|(ring, _)| {
                        let (lo, hi) = ring
                            .iter()
                            .map(|p| p.0 * local[0] + p.1 * local[1])
                            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), s| {
                                (a.min(s), b.max(s))
                            });
                        hi - lo
                    });
                println!(
                    "  not drawn: {} body {:.4} ft across, layers {:.4} ft",
                    type_guid.as_deref().unwrap_or("?"),
                    thickness.unwrap_or(f64::NAN),
                    widths.iter().sum::<f64>()
                );
            }
            continue;
        };
        *census.entry("drawn in layers").or_default() += 1;
        if !json.ends_with('[') {
            json.push(',');
        }
        let spans: Vec<String> = meshes
            .iter()
            .zip(&layers.layers)
            .map(|(mesh, band)| {
                let (lo, hi) = mesh
                    .vertices
                    .iter()
                    .map(|&v| {
                        let w = placement.apply(v);
                        w[0] * nx + w[1] * ny
                    })
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), s| {
                        (a.min(s), b.max(s))
                    });
                format!(
                    "[{lo},{hi},{},{}]",
                    band.width_feet,
                    band.color_packed.map_or(-1, i64::from)
                )
            })
            .collect();
        let _ = write!(
            json,
            "{{\"tag\":{},\"normal\":[{nx},{ny}],\"bands\":[{}]}}",
            type_guid.as_deref().unwrap_or("0"),
            spans.join(",")
        );
    }
    json.push(']');
    println!(
        "{} layered elements in the export",
        model.element_layers.len()
    );
    for (what, count) in &census {
        println!("  {count:>5}  {what}");
    }
    if let Some(out) = json_path {
        std::fs::write(&out, json).expect("write json");
        println!("wrote {out}");
    }
    Ok(())
}
