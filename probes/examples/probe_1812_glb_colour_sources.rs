//! #1812: where the colour and alpha of each material in the GLB come from.
//!
//! FACT under test: `gltf::build_gltf` marks every material's
//! `extras.colourSource`:
//! - `"file"` where its base colour and alpha are the Revit material's own;
//! - `"rvt-rs"` where rvt-rs chose them: the category colour of an element with
//!   no material of its own, or the grey of a material whose colour was not
//!   read.
//!
//! The probe makes the GLB `rvt-gltf` makes with no flags. It counts the
//! materials, and the element primitives drawn in them, by source and by
//! whether they are translucent (alpha below 1). A material with no mark, or a
//! category material marked other than `"rvt-rs"`, is a failure.
//!
//! Output:
//!   materials source=<s> translucent=<n> opaque=<n>
//!   primitives source=<s> translucent=<n> opaque=<n>
//!   category <name> primitives=<n> alpha=<a> source=<s>
//!   failures unmarked=<n> category_not_rvt_rs=<n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1812_glb_colour_sources -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::gltf::{COLOUR_FROM_RVT_RS, Material, build_gltf};

fn source(material: &Material) -> &str {
    material
        .extras
        .as_ref()
        .map_or("-", |extras| extras.colour_source.as_str())
}

fn translucent(material: &Material) -> bool {
    material.pbr_metallic_roughness.base_color_factor[3] < 1.0
}

fn main() -> anyhow::Result<()> {
    // Measure passes the samples `--records` too.
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_1812_glb_colour_sources MODEL.rvt");
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let run = rvt::ifc::run_export_with_limits(
        &mut rf,
        &rvt::ifc::ExportOptions::default(),
        rvt::walker::WalkerLimits::default(),
    )?;
    let (doc, _) = build_gltf(&run.model);

    let mut materials: BTreeMap<&str, [u64; 2]> = BTreeMap::new();
    for material in &doc.materials {
        materials.entry(source(material)).or_default()[usize::from(!translucent(material))] += 1;
    }
    let mut primitives: BTreeMap<&str, [u64; 2]> = BTreeMap::new();
    let mut categories: BTreeMap<usize, u64> = BTreeMap::new();
    for node in doc.nodes.iter().filter(|node| node.extras.is_some()) {
        let Some(mesh) = node.mesh.and_then(|index| doc.meshes.get(index)) else {
            continue;
        };
        for index in mesh.primitives.iter().filter_map(|p| p.material) {
            let Some(material) = doc.materials.get(index) else {
                continue;
            };
            primitives.entry(source(material)).or_default()[usize::from(!translucent(material))] +=
                1;
            if material
                .name
                .as_deref()
                .is_some_and(|name| name.starts_with("Category "))
            {
                *categories.entry(index).or_default() += 1;
            }
        }
    }

    for (label, counts) in [("materials", &materials), ("primitives", &primitives)] {
        for (from, [clear, solid]) in counts {
            println!("{label} source={from} translucent={clear} opaque={solid}");
        }
    }
    let mut category_not_rvt_rs = 0;
    for (index, count) in &categories {
        let material = &doc.materials[*index];
        if source(material) != COLOUR_FROM_RVT_RS {
            category_not_rvt_rs += 1;
        }
        println!(
            "category {} primitives={count} alpha={:.2} source={}",
            material.name.as_deref().unwrap_or(""),
            material.pbr_metallic_roughness.base_color_factor[3],
            source(material)
        );
    }
    let unmarked = doc.materials.iter().filter(|m| m.extras.is_none()).count();
    println!("failures unmarked={unmarked} category_not_rvt_rs={category_not_rvt_rs}");
    Ok(())
}
