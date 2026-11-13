//! #1856: every stair run the export draws from its sketch that also has a saved
//! mesh, and how far the two bodies' volumes are apart.
//!
//! FACT under test: a straight stair run drawn from its treads and risers
//! (`BodySource` `partition_stair_run_sketch`, RE-52) never takes the mesh Revit
//! saved for it, because `saved_meshes::attach` replaces record boxes only. #1846
//! made the first run of each of `rac_advanced` 2024's five stairs read its own
//! record, so these five are now drawn from their sketch, and the kept exports of
//! Measure run 43325269883 give both bodies: the same box within 0.0001 ft, the
//! sketch 2.55% larger in volume (8.2973 against 8.0906 ft³ on 354390, 358394
//! and 360244; 11.8273 against 11.5327 on 355864 and 357396). The issue asks for
//! the count of such runs over the corpus and the gap of each before a body is
//! chosen.
//!
//! The probe exports the file as the reader does, takes the runs whose body is
//! the sketch, reads their saved scene, and prints the run's sketch volume (the
//! `NetVolume` of its `Qto_StairFlightBaseQuantities`, the side views' areas times
//! the width, which is the volume of the solids the sketch draws) beside the
//! volume the saved mesh encloses (the sum of the signed volumes of its
//! triangles, whose sign is the winding's).
//!
//! Registered before the run: on `rac_advanced` 2024 the probe reproduces the five
//! runs above, to four digits on both volumes. If it does not, the volume it
//! computes for one of the two bodies is not the volume of the scorer's shape, and
//! no other row of the output is read until it is.
//!
//! Output:
//!
//!   file <path> release <release>
//!   run <id> status <status> triangles <n> unbounded_faces <n> diagnostics <n> mesh_volume <ft3> sketch_volume <ft3> ratio <sketch / mesh>
//!   summary sketch_runs <n> with_saved_mesh <n> complete <n> ratio_min <r> ratio_median <r> ratio_max <r>
//!
//! `complete` counts the saved meshes whose status is `decoded_saved_graphics`;
//! the ratios are over those only. A file whose saved scene cannot be read prints
//! `scene_unavailable <reason>` and the count of sketch runs.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1856_sketch_runs_mesh -- MODEL.rvt

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertySet, PropertyValue};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;
use std::collections::{BTreeMap, BTreeSet};

/// `BodySource` of a stair run drawn as its treads and risers (RE-52).
const SKETCH_RUN: &str = "partition_stair_run_sketch";

/// What a complete saved scene's status reads.
const COMPLETE: &str = "decoded_saved_graphics";

fn property_text<'a>(set: &'a PropertySet, name: &str) -> Option<&'a str> {
    set.properties
        .iter()
        .find_map(|property| match &property.value {
            PropertyValue::Text(text) if property.name == name => Some(text.as_str()),
            _ => None,
        })
}

/// The volume a closed triangle mesh encloses, signed by its winding: the sum of
/// the tetrahedra each triangle makes with the origin.
fn signed_volume(triangles: &[[u32; 3]], vertices: &[[f64; 3]]) -> f64 {
    triangles
        .iter()
        .filter_map(|&[a, b, c]| {
            let (a, b, c) = (
                vertices.get(a as usize)?,
                vertices.get(b as usize)?,
                vertices.get(c as usize)?,
            );
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            Some((a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0)
        })
        .sum()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("file {path} release {version}");
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    // The runs drawn from their sketch: ElementId by entity index.
    let mut runs: BTreeMap<usize, u64> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        if property_text(set, "BodySource") != Some(SKETCH_RUN) {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            runs.insert(index, id);
        }
    }
    // Each run's sketch volume, from its quantity set.
    let mut sketch: BTreeMap<u64, f64> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::ElementPropertySet { element, set } = entity else {
            continue;
        };
        if set.name != "Qto_StairFlightBaseQuantities" {
            continue;
        }
        let Some(&id) = runs.get(element) else {
            continue;
        };
        for property in &set.properties {
            if let (true, PropertyValue::VolumeCubicFeet(volume)) =
                (property.name == "NetVolume", &property.value)
            {
                sketch.insert(id, *volume);
            }
        }
    }
    let selected: BTreeSet<u64> = runs.values().copied().collect();
    if selected.is_empty() {
        println!(
            "summary sketch_runs 0 with_saved_mesh 0 complete 0 ratio_min - ratio_median - ratio_max -"
        );
        return Ok(());
    }
    let options = Options {
        selected_ids: selected.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = match extract_at_detail(&mut rf, &options, 3) {
        Ok(scene) => scene,
        Err(error) => {
            println!("scene_unavailable {error}");
            println!(
                "summary sketch_runs {} with_saved_mesh 0 complete 0 ratio_min - ratio_median - ratio_max -",
                selected.len()
            );
            return Ok(());
        }
    };
    let (mut with_mesh, mut complete) = (0usize, 0usize);
    let mut ratios: Vec<f64> = Vec::new();
    for element in &scene.elements {
        let id = element.identity.element_id;
        if !selected.contains(&id) {
            continue;
        }
        let meshes = &element.meshes;
        if meshes.primitives.is_empty() {
            continue;
        }
        with_mesh += 1;
        let triangles: usize = meshes
            .primitives
            .iter()
            .map(|primitive| primitive.triangles.len())
            .sum();
        let enclosed: f64 = meshes
            .primitives
            .iter()
            .map(|primitive| signed_volume(&primitive.triangles, &primitive.vertices))
            .sum::<f64>()
            .abs();
        let sketch_volume = sketch.get(&id).copied();
        let ratio = sketch_volume
            .filter(|_| enclosed > 0.0)
            .map(|v| v / enclosed);
        if element.status == COMPLETE {
            complete += 1;
            ratios.extend(ratio);
        }
        println!(
            "run {id} status {} triangles {triangles} unbounded_faces {} diagnostics {} mesh_volume {enclosed:.4} sketch_volume {} ratio {}",
            element.status,
            meshes.unbounded_faces,
            meshes.diagnostics.len(),
            sketch_volume.map_or("-".to_string(), |v| format!("{v:.4}")),
            ratio.map_or("-".to_string(), |r| format!("{r:.4}")),
        );
    }
    ratios.sort_by(|a, b| a.total_cmp(b));
    let pick = |at: Option<&f64>| at.map_or("-".to_string(), |r| format!("{r:.4}"));
    println!(
        "summary sketch_runs {} with_saved_mesh {with_mesh} complete {complete} ratio_min {} ratio_median {} ratio_max {}",
        selected.len(),
        pick(ratios.first()),
        pick(ratios.get(ratios.len() / 2)),
        pick(ratios.last()),
    );
    Ok(())
}
