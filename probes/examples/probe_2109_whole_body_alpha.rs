//! #2109: which elements does the glTF exporter draw translucent in a category's colour,
//! and does any category colour appear as more than one material?
//!
//! FACT under test: #2109 gives an element that keeps its category's colour the alpha of its
//! most transparent read material. Run on `main` and on #2109, it says how many elements on
//! each file take a category colour with an alpha below 1, by IFC class, alpha and body shape;
//! for those, how many materials the element is associated with and how many of them were read
//! as transparent (a door with a vision panel: one of several); and how many category materials
//! share one name and one alpha (two materials for one category's colour at one opacity).
//!
//! The model is exported as `rvt-gltf` exports it (`run_export_with_limits_and_control`,
//! default saved meshes) and put to `gltf::build_gltf`. It prints:
//!
//! ```text
//! translucent class=<IFC class> alpha=<a> shape=<cube|mesh> nodes=<n>
//! parts class=<IFC class> materials=<n> transparent=<n> nodes=<n>
//! categories materials=<n> names=<n> duplicated=<n> [<name> <alpha>, ...]
//! materials total=<n> blend=<n> nodes=<n>
//! ```
//!
//! Counts only.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_2109_whole_body_alpha -- MODEL.rvt
//! ```

use std::collections::BTreeMap;

use anyhow::Result;
use rvt::RevitFile;
use rvt::ifc::gltf::build_gltf;
use rvt::ifc::{ExportOptions, SavedMeshChoice, run_export_with_limits_and_control};

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2109_whole_body_alpha MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let run = run_export_with_limits_and_control(
        &mut rf,
        &ExportOptions::default().with_saved_meshes(SavedMeshChoice::Default),
        rvt::walker::WalkerLimits::default(),
        &rvt::control::WalkerControl::new(),
    )?;
    let model = &run.model;
    let (doc, _) = build_gltf(model);
    // Each element's associated materials, as #2109 reads them: its constituent sets, its
    // single-profile set and its layer set.
    let mut associated: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for set in &model.material_constituent_sets {
        for &element in &set.elements {
            associated
                .entry(element)
                .or_default()
                .extend(set.material_indices.iter().copied());
        }
    }
    for (index, entity) in model.entities.iter().enumerate() {
        if let rvt::ifc::entities::IfcEntity::BuildingElement {
            material_profile_set_index,
            material_layer_set_index,
            ..
        } = entity
        {
            if let Some(set) =
                material_profile_set_index.and_then(|i| model.material_profile_sets.get(i))
            {
                associated
                    .entry(index)
                    .or_default()
                    .extend(set.profiles.iter().map(|p| p.material_index));
            }
            if let Some(set) =
                material_layer_set_index.and_then(|i| model.material_layer_sets.get(i))
            {
                associated
                    .entry(index)
                    .or_default()
                    .extend(set.layers.iter().filter_map(|l| l.material_index));
            }
        }
    }
    let mut parts: BTreeMap<(String, usize, usize), usize> = BTreeMap::new();
    let mut translucent: BTreeMap<(String, String, &str), usize> = BTreeMap::new();
    let mut nodes = 0usize;
    for node in &doc.nodes {
        let (Some(mesh), Some(extras)) = (node.mesh, node.extras.as_ref()) else {
            continue;
        };
        nodes += 1;
        let Some(primitive) = doc.meshes.get(mesh).and_then(|m| m.primitives.first()) else {
            continue;
        };
        let Some(material) = primitive.material.and_then(|i| doc.materials.get(i)) else {
            continue;
        };
        let is_category = material
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("Category "));
        let alpha = material.pbr_metallic_roughness.base_color_factor[3];
        if is_category && alpha < 1.0 {
            // The shared unit cube's positions are accessor 0.
            let shape = if primitive.attributes.get("POSITION") == Some(&0) {
                "cube"
            } else {
                "mesh"
            };
            *translucent
                .entry((extras.ifc_type.clone(), format!("{alpha:.3}"), shape))
                .or_default() += 1;
            let mut materials = associated
                .get(&extras.entity_index)
                .cloned()
                .unwrap_or_default();
            materials.sort_unstable();
            materials.dedup();
            let transparent = materials
                .iter()
                .filter_map(|i| model.materials.get(*i))
                .filter(|m| m.transparency.is_some_and(|t| t > 0.0))
                .count();
            *parts
                .entry((extras.ifc_type.clone(), materials.len(), transparent))
                .or_default() += 1;
        }
    }
    for ((class, alpha, shape), n) in &translucent {
        println!("translucent class={class} alpha={alpha} shape={shape} nodes={n}");
    }
    for ((class, materials, transparent), n) in &parts {
        println!("parts class={class} materials={materials} transparent={transparent} nodes={n}");
    }
    let mut names: BTreeMap<(&str, String), usize> = BTreeMap::new();
    let mut categories = 0usize;
    for material in &doc.materials {
        if let Some(name) = material
            .name
            .as_deref()
            .filter(|n| n.starts_with("Category "))
        {
            categories += 1;
            let alpha = format!(
                "{:.3}",
                material.pbr_metallic_roughness.base_color_factor[3]
            );
            *names.entry((name, alpha)).or_default() += 1;
        }
    }
    let duplicated: Vec<String> = names
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|((name, alpha), _)| format!("{name} {alpha}"))
        .collect();
    println!(
        "categories materials={categories} names={} duplicated={} {duplicated:?}",
        names.len(),
        duplicated.len()
    );
    let blend = doc
        .materials
        .iter()
        .filter(|m| m.alpha_mode.as_deref() == Some("BLEND"))
        .count();
    println!(
        "materials total={} blend={blend} nodes={nodes}",
        doc.materials.len()
    );
    Ok(())
}
