//! #357: what a stair's native records hold on Revit 2025, read by member
//! name through the file's own schema.
//!
//! FACT under test: RE-47 and RE-92 read a run's riser height, tread depth,
//! riser count and ends from byte anchors measured on Revit 2024, and none of
//! them is found on Revit 2025, so every flight of the R2025 stair models is
//! drawn as its record box. The native reader decodes the `StairsRun`,
//! `StairsRunType`, `Stairs`, `StairsType` and `StairsLanding` records by
//! their schema's member names (`m_endWithRiser` among them, #357). This
//! prints every object of each such record's graph, for the run's geometry
//! to be found by name.
//!
//! Output:
//!   record <id> <class> <objects>
//!   object <id> <index> <class> <fields as JSON, at most 6000 characters>
//!
//! Usage:
//!   cargo run --profile ci --example probe_357_stairs_native -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

const CLASSES: &[&str] = &[
    "StairsRun",
    "StairsRunType",
    "Stairs",
    "StairsType",
    "StairsLanding",
    "StairsLandingType",
];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: CLASSES
            .iter()
            .map(|c| c.to_string())
            .collect::<BTreeSet<_>>(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        println!("record {id} {class} {}", graph.objects.len());
        for (index, object) in graph.objects.iter().enumerate() {
            let mut fields = serde_json::to_string(&object.fields)?;
            if fields.len() > 6000 {
                let mut cut = 6000;
                while !fields.is_char_boundary(cut) {
                    cut -= 1;
                }
                fields.truncate(cut);
                fields.push_str("...");
            }
            println!("object {id} {index} {} {fields}", object.class_name);
        }
        Ok(())
    })?;
    Ok(())
}
