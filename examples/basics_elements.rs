//! List the building elements of a Revit file as the exporter reads them,
//! counted by IFC type, with the ElementId of the first few.
//!
//! ```sh
//! cargo run --example basics_elements -- path/to/project.rvt
//! ```
//!
//! Without an argument it opens the synthetic project file the repository
//! ships, `corpus/tier1/architectural-2024/architectural-2024.rvt`, which
//! carries no element records, so it lists none. Give it a real project,
//! such as `Revit_IFC5_Einhoven.rvt` from
//! [magnetar-io/revit-test-datasets](https://github.com/magnetar-io/revit-test-datasets)
//! (MIT), to see them.

use std::collections::BTreeMap;
use std::path::PathBuf;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;

const FIXTURE: &str = "corpus/tier1/architectural-2024/architectural-2024.rvt";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from(FIXTURE), PathBuf::from);
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;

    let mut by_type: BTreeMap<&str, usize> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            ..
        } = entity
        {
            if by_type.values().sum::<usize>() < 5 {
                let id = type_guid.as_deref().unwrap_or("-");
                println!("{ifc_type} {name} (ElementId {id})");
            }
            *by_type.entry(ifc_type.as_str()).or_default() += 1;
        }
    }
    for (ifc_type, count) in &by_type {
        println!("{count:6} {ifc_type}");
    }
    println!("{} building elements", by_type.values().sum::<usize>());
    Ok(())
}
