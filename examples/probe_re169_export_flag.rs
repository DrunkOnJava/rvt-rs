//! RE-169 (probe, B38): does anything in RE1 Plumbing tell Revit's exporter to leave
//! out water closet tank 442378?
//!
//! Searches every native record's decoded objects for the text
//! `DontExport`, `IfcExportAs` or `IFCExport`, printing each record's id, class
//! and the matching text; prints the tank's family 491228 in full; and lists
//! the records whose objects name the tank's family or symbol.
//!
//! Usage: probe_re169_export_flag <RE1-Plumbing.rvt>

use rvt::{RevitFile, native_document};
use std::path::PathBuf;

const FAMILY: u64 = 491228;

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    if !path.to_string_lossy().contains("Plumbing") {
        println!("not RE1 Plumbing");
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    native_document::extract(&mut rf, &native_document::Options::default(), |record| {
        let id = record.identity.element_id;
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let text: String = graph
            .objects
            .iter()
            .map(|object| format!("{} {}\n", object.class_name, object.fields))
            .collect();
        for needle in ["DontExport", "IfcExportAs", "IFCExport"] {
            if let Some(at) = text.find(needle) {
                let start = at.saturating_sub(160);
                let snippet: String = text[start..].chars().take(320).collect();
                println!("{id} {:?} holds {needle}: …{snippet}…", record.class_name);
                break;
            }
        }
        if id == FAMILY {
            println!("family {id} {:?}:", record.class_name);
            for object in &graph.objects {
                println!(
                    "  {}: {}",
                    object.class_name,
                    object
                        .fields
                        .to_string()
                        .chars()
                        .take(600)
                        .collect::<String>()
                );
            }
        }
        if id != FAMILY
            && (text.contains("\"m_id64\":491228") || text.contains("\"m_id64\":442362"))
        {
            println!(
                "{id} {:?} names the tank's family or symbol",
                record.class_name
            );
        }
        Ok(())
    })?;
    Ok(())
}
