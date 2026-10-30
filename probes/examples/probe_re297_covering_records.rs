//! RE-297 (#1016, W7): what a covering's native record holds that says Revit
//! cuts its solid.
//!
//! FACT under test: Revit's `Qto_CoveringBaseQuantities.GrossArea` is the area
//! of the profile a compound ceiling extrudes, except on the ceilings whose
//! solid something cuts (RE-250: 2 of the BIM4LCA office's ceilings as built
//! and 9 at building permit, whose sketch is 0.7 to 1.1 percent larger).
//! This probe lists, for every `IfcCovering` rvt-rs writes, the join and cut
//! steps its native record holds, for finding whether those ceilings show them.
//!
//! Output, one line per covering: `covering <ElementId> <predefined type>
//! <name> steps <Class>[<m_otherId>|<m_baseOrTop>|<m_ids>] ...`, and
//! `covering <ElementId> <predefined type> <name> -` for a covering whose
//! record is not read.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re297_covering_records -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let coverings: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid: Some(tag),
                predefined_type,
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCCOVERING") => Some((
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
            selected_ids: coverings.keys().copied().collect(),
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
    for (id, label) in &coverings {
        match lines.get(id) {
            Some(steps) => println!("covering {id} {label} steps {steps}"),
            None => println!("covering {id} {label} -"),
        }
    }
    Ok(())
}
