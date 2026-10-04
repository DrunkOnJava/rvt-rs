//! B59, B43 (probe): what tells a duct type's shape (round, rectangular,
//! oval), which names its system family (`Round Duct`, `Rectangular Duct`,
//! `Oval Duct`) in Revit's export.
//!
//! Prints every duct type's native record (class `AbsDuctType` and its kin):
//! its id, its name, and every root field whose value is small (a number, a
//! flag, a short string), and the classes of its other objects; and which
//! duct types the model's ducts use.
//!
//! Usage: probe_b59_duct_types <model.rvt>

use rvt::{RevitFile, native_document};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let name = path.to_string_lossy().to_string();
    if !(name.contains("Mechanical") || name.contains("rme")) {
        println!("not a model with ducts");
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    let mut used: BTreeMap<u64, usize> = BTreeMap::new();
    let mut types = Vec::new();
    native_document::extract(&mut rf, &native_document::Options::default(), |record| {
        let class = record.class_name.clone().unwrap_or_default();
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        if class.contains("DuctType") {
            let small: Vec<String> = root
                .fields
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, value)| value.to_string().len() <= 40)
                .map(|(field, value)| format!("{field}={value}"))
                .collect();
            let others: Vec<String> = graph
                .objects
                .iter()
                .skip(1)
                .map(|object| {
                    format!(
                        "{} {}",
                        object.class_name,
                        object
                            .fields
                            .to_string()
                            .chars()
                            .take(160)
                            .collect::<String>()
                    )
                })
                .collect();
            types.push(format!(
                "{} {class}: {}\n    objects: {:?}",
                record.identity.element_id,
                small.join(" "),
                others
            ));
        }
        if class.contains("DuctCurve") || class == "RbsDuctCurve" {
            for key in ["m_symbolId", "m_typeId", "m_masterSymbolId", "m_ductTypeId"] {
                if let Some(id) = root
                    .fields
                    .get(key)
                    .and_then(|v| v.get("m_id"))
                    .and_then(|v| v.get("m_id64"))
                    .and_then(|v| v.as_u64())
                {
                    *used.entry(id).or_default() += 1;
                }
            }
        }
        Ok(())
    })?;
    for line in types.iter().take(40) {
        println!("{line}");
    }
    println!("{} duct types; ducts by type id {used:?}", types.len());
    Ok(())
}
