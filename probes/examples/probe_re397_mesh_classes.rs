//! RE-397 (#1635): which classes of element the default export draws from a
//! saved mesh, and which it leaves as a record box.
//!
//! FACT under test: the rule of RE-397 (draw the instances of a railing's saved
//! graphics that are hidden in the normal draw and forbidden the targets 53246,
//! and leave a flip control out of the walk) moves 263 bodies from a record box
//! to a saved mesh on the 26 models and samples Measure holds that it moves, 15
//! of them on the office, whose export checks them. The walk is the same for
//! every class, so this counts, per IFC entity, the bodies of the default
//! export by how they are drawn, to be run on `main` and on the change and
//! subtracted: which classes it moved.
//!
//! Output, one line per entity:
//!   class <IFC entity> saved=<n> box=<n>
//! where `saved` is the bodies drawn from the element's saved mesh and `box`
//! those that are an element record's box (a beam's along its line, an
//! instance's turned to its angle included), and a last line
//!   total saved=<n> box=<n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_mesh_classes -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::{ExportOptions, run_export};

const BOXES: [&str; 3] = [
    "partition_element_record_bbox",
    "partition_beam_box_along_axis",
    "partition_family_instance_turned_box",
];
const SAVED: &str = "saved_graphics_mesh";

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    let mut by_class: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        let source = set.properties.iter().find_map(|p| match &p.value {
            PropertyValue::Text(t) if p.name == "BodySource" => Some(t.as_str()),
            _ => None,
        });
        let counts = by_class.entry(ifc_type.clone()).or_default();
        match source {
            Some(SAVED) => counts.0 += 1,
            Some(source) if BOXES.contains(&source) => counts.1 += 1,
            _ => {}
        }
    }
    let (mut saved, mut boxed) = (0usize, 0usize);
    for (class, (s, b)) in &by_class {
        if *s + *b > 0 {
            println!("class {class} saved={s} box={b}");
        }
        saved += s;
        boxed += b;
    }
    println!("total saved={saved} box={boxed}");
    Ok(())
}
