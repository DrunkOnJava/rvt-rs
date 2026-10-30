//! RE-295 (#1016, W7): what a slab's native record holds that says Revit cuts
//! its solid.
//!
//! FACT under test: Revit's `Qto_SlabBaseQuantities.NetVolume` is the slab's
//! `GrossArea` times its type's layers' total thickness unless something cuts
//! its solid; the one slab of the BIM4LCA office that Revit cuts (a roof the
//! hatches of RE-225 cut) has its cut openings in rvt-rs's export. This probe
//! lists, for every `IfcSlab` rvt-rs writes, the join and cut steps its native
//! record holds, for finding whether a cut that is not an opening shows there.
//!
//! Output, one line per slab: `slab <ElementId> <predefined type> <name>
//! steps <Class>[<m_otherId>|<m_baseOrTop>|<m_ids>] ...`, and `slab <ElementId>
//! -` for a slab whose record is not read.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re295_slab_records -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let slabs: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid: Some(tag),
                predefined_type,
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCSLAB") => Some((
                tag.parse().ok()?,
                format!(
                    "{} {}",
                    predefined_type.as_deref().unwrap_or("-"),
                    name.replace(' ', "_")
                ),
            )),
            _ => None,
        })
        .collect();
    let mut lines: BTreeMap<u64, String> = BTreeMap::new();
    extract(
        &mut rf,
        &Options {
            selected_ids: slabs.keys().copied().collect(),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let mut steps = Vec::new();
            for object in &graph.objects {
                if !object.class_name.ends_with("GStep") {
                    continue;
                }
                let field = |name: &str| {
                    object
                        .fields
                        .get(name)
                        .map_or("-".to_string(), |value| value.to_string())
                };
                steps.push(format!(
                    "{}[{}|{}|{}]",
                    object.class_name,
                    field("m_otherId"),
                    field("m_baseOrTop"),
                    field("m_ids").replace(' ', "")
                ));
            }
            lines.insert(record.identity.element_id, steps.join(" "));
            Ok(())
        },
    )?;
    for (id, label) in &slabs {
        match lines.get(id) {
            Some(steps) => println!("slab {id} {label} steps {steps}"),
            None => println!("slab {id} {label} -"),
        }
    }
    Ok(())
}
