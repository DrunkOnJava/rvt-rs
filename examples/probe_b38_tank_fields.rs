//! B38 (probe): why Revit's export of RE1 Plumbing leaves out water closet
//! tank 442378 while it writes the other rough-in fixtures.
//!
//! Prints the native record of the tank, its symbol 442362, and an exported
//! sibling fixture (sink 442581): each record's class and every field of its
//! root object, so the fields that differ stand out. Also lists every
//! FamilyInstance whose root names the tank in any ElementId field.
//!
//! Usage: probe_b38_tank_fields <RE1-Plumbing.rvt>

use rvt::{RevitFile, native_document};
use std::collections::BTreeSet;
use std::path::PathBuf;

const SHOWN: [u64; 3] = [442378, 442362, 442581];

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    if !path.to_string_lossy().contains("Plumbing") {
        println!("not RE1 Plumbing");
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    let mut naming_tank = Vec::new();
    native_document::extract(&mut rf, &native_document::Options::default(), |record| {
        let id = record.identity.element_id;
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        if SHOWN.contains(&id) {
            println!(
                "{id}: class {:?} status {} objects {}",
                record.class_name,
                record.status,
                graph.objects.len()
            );
            if let Some(fields) = root.fields.as_object() {
                for (name, value) in fields {
                    let text = value.to_string();
                    println!("  {name}: {}", text.chars().take(200).collect::<String>());
                }
            }
            for object in graph.objects.iter().skip(1).take(12) {
                println!(
                    "  object {}: {}",
                    object.class_name,
                    object
                        .fields
                        .to_string()
                        .chars()
                        .take(200)
                        .collect::<String>()
                );
            }
        }
        if id != 442378 && root.fields.to_string().contains("442378") {
            naming_tank.push((id, record.class_name.clone()));
        }
        Ok(())
    })?;
    let unique: BTreeSet<_> = naming_tank.into_iter().collect();
    println!("records whose root names 442378: {unique:?}");
    Ok(())
}
