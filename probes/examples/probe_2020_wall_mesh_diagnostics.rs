//! #2020 (probe): what failed to decode in the saved mesh of each wall whose mesh
//! is partial.
//!
//! FACT under test: `saved_meshes::attach` reads a wall's base quantities from
//! its saved mesh, and #2020 refuses a mesh `partial_refusal` calls partial, as
//! a beam's and a column's are refused. On `rac_advanced` 2023 to 2027 (8
//! parapet walls) and Snowdon Towers Architectural 2024 (9 split-face CMU
//! walls) that removes volumes equal to Revit's own drawn body (17 of 17). So
//! what makes those meshes partial is not a missing piece of the wall. This
//! prints, for every wall whose saved graphics hold a diagnostic, the kinds of
//! diagnostic, so the ones that leave the wall whole can be told from a failed
//! face or solid.
//!
//! Output:
//!
//!   file <path>
//!   wall <id> status <status> primitives <n> triangles <n> unbounded <n> partial <yes|no> kinds <kind> | <kind> ...
//!   summary walls <n> with_mesh <n> partial <n>
//!   kind <count> <kind>                one line per distinct kind over the partial walls
//!
//! A kind is the diagnostic without the object it names and with its digits as
//! `#`, as `SavedMeshRefusal` writes it. `partial` is what `partial_refusal` says:
//! every diagnostic but the nested detail-family filter and the view component
//! leaves a mesh partial.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2020_wall_mesh_diagnostics -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::Options;
use rvt::research::native_graphics_traversal::{
    NESTED_DETAIL_FILTER_DIAGNOSTIC, VIEW_COMPONENT_DIAGNOSTIC,
};
use rvt::research::native_saved_scene::extract_at_detail;

fn kind(message: &str) -> String {
    let text = message.split_once(": ").map_or(message, |(_, rest)| rest);
    text.chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

fn leaves_partial(message: &str) -> bool {
    !message.ends_with(NESTED_DETAIL_FILTER_DIAGNOSTIC)
        && !message.ends_with(VIEW_COMPONENT_DIAGNOSTIC)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    println!("file {path}");
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let walls: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCWALL") => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let options = Options {
        selected_ids: walls.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_at_detail(&mut rf, &options, 3)?;
    let mut with_mesh = 0usize;
    let mut partial = 0usize;
    let mut kinds_seen: BTreeMap<String, usize> = BTreeMap::new();
    for element in &scene.elements {
        let id = element.identity.element_id;
        if !walls.contains(&id) {
            continue;
        }
        let meshes = &element.meshes;
        if meshes.primitives.is_empty() {
            continue;
        }
        with_mesh += 1;
        if meshes.diagnostics.is_empty() {
            continue;
        }
        let is_partial = meshes.diagnostics.iter().any(|d| leaves_partial(d));
        let kinds: BTreeSet<String> = meshes.diagnostics.iter().map(|d| kind(d)).collect();
        if is_partial {
            partial += 1;
            for k in meshes
                .diagnostics
                .iter()
                .filter(|d| leaves_partial(d))
                .map(|d| kind(d))
                .collect::<BTreeSet<_>>()
            {
                *kinds_seen.entry(k).or_default() += 1;
            }
        }
        let triangles: usize = meshes.primitives.iter().map(|p| p.triangles.len()).sum();
        println!(
            "wall {id} status {} primitives {} triangles {triangles} unbounded {} partial {} kinds {}",
            element.status,
            meshes.primitives.len(),
            meshes.unbounded_faces,
            if is_partial { "yes" } else { "no" },
            kinds.into_iter().collect::<Vec<_>>().join(" | ")
        );
    }
    println!(
        "summary walls {} with_mesh {with_mesh} partial {partial}",
        walls.len()
    );
    for (k, n) in &kinds_seen {
        println!("kind {n} {k}");
    }
    Ok(())
}
