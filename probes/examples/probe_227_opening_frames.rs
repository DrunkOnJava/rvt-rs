//! #227: the outline a door's or window's symbol cuts (RE-215), placed by
//! the instance's transform (RE-87), against the opening Revit's export
//! cuts.
//!
//! FACT under test: Revit's opening for a door or window is the cut loop of
//! the symbol that draws it (`InstanceInfo.m_symbolId`), carried into model
//! space by the instance's transform, and extruded across its host. RE-215
//! read the loop's extents, which are Revit's `OverallWidth` and
//! `OverallHeight`; this probe prints, per door and window the export
//! writes, the loop's corners in the symbol's frame and the instance's
//! transform, and `tools/re/opening_frames_vs_ifc.py` puts Revit's opening
//! into that frame to compare.
//!
//! Output, one line each:
//!   frame <ElementId> <loop min [x,y,z]> <loop max [x,y,z]> <axes [X,Y,Z]> <origin>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_227_opening_frames -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_symbol_cuts::instance_cut_loops;
use rvt::research::partition_instance_transforms::scan_instance_transforms;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let ids: BTreeSet<u32> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCDOOR")
                || ifc_type.eq_ignore_ascii_case("IFCWINDOW") =>
            {
                tag.parse().ok()
            }
            _ => None,
        })
        .collect();
    let loops = instance_cut_loops(&mut rf, &ids.iter().map(|&id| u64::from(id)).collect())?;
    let transforms = scan_instance_transforms(&mut rf, version, &ids)?;
    for id in &ids {
        let (Some(cut), Some(transform)) = (loops.get(&u64::from(*id)), transforms.get(id)) else {
            continue;
        };
        println!(
            "frame {id} {} {} {} {}",
            serde_json::to_string(&cut.min)?,
            serde_json::to_string(&cut.max)?,
            serde_json::to_string(&transform.axes)?,
            serde_json::to_string(&transform.origin)?
        );
    }
    eprintln!(
        "{} doors and windows, {} with a cut loop, {} with a transform",
        ids.len(),
        loops.len(),
        transforms.len()
    );
    Ok(())
}
