//! RE-300 (probe): the scalar fields of walls', floors' and their types'
//! native records, for scoring each against Revit's `LoadBearing`,
//! `IsExternal` and `ThermalTransmittance` (#1158).
//!
//! FACT under test: a flag Revit's export writes for a wall or a floor is a
//! scalar field of the element's native record or its type's, and its value
//! says Revit's value.
//!
//! Prints, for each `SWall`, `ArcWall`, `Floor`, `BasicWallType` and
//! `FloorAttributes` record, one line per scalar (integer, boolean or double)
//! field of its first objects:
//!
//!   field <ElementId> <record class> <object class>#<n> <field> <value>
//!   link <ElementId> <type ElementId>
//!
//! `<n>` counts the objects of that class in the graph. The lines name
//! ElementIds and stay on the runner; `tools/re/native_fields_vs_ifc.py`
//! scores them against the export and prints counts.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re300_native_fields -- FILE.rvt

use rvt::RevitFile;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const CLASSES: [&str; 5] = [
    "SWall",
    "ArcWall",
    "Floor",
    "BasicWallType",
    "FloorAttributes",
];
const TYPE_LINKS: [&str; 2] = ["m_WallAttributesId", "m_floorAttributesId"];
const MAX_OBJECTS: usize = 12;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: probe_re300_native_fields FILE.rvt");
        std::process::exit(2);
    };
    if let Err(error) = run(path) {
        println!("native readers: native fields failed {error:#}");
    }
}

fn identifier(value: &Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    inner
        .get("m_id64")
        .or_else(|| inner.get("m_id"))
        .and_then(Value::as_i64)
}

fn run(path: &str) -> anyhow::Result<()> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    let options = Options {
        selected_classes: CLASSES.iter().map(|c| c.to_string()).collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut records = 0usize;
    let mut lines = 0usize;
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        if !CLASSES.contains(&class) {
            return Ok(());
        }
        records += 1;
        let id = record.identity.element_id;
        let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
        let mut printed: BTreeSet<String> = BTreeSet::new();
        for object in graph.objects.iter().take(MAX_OBJECTS) {
            if object.class_name.ends_with("Step") {
                continue;
            }
            let n = seen.entry(object.class_name.as_str()).or_default();
            let object_class = format!("{}#{}", object.class_name, n);
            *n += 1;
            let Some(fields) = object.fields.as_object() else {
                continue;
            };
            for (name, value) in fields {
                if TYPE_LINKS.contains(&name.as_str()) {
                    if let Some(type_id) = identifier(value).filter(|t| *t > 0) {
                        if printed.insert(format!("link {type_id}")) {
                            println!("link {id} {type_id}");
                            lines += 1;
                        }
                    }
                    continue;
                }
                let scalar = match value {
                    Value::Bool(flag) => Some(i64::from(*flag).to_string()),
                    Value::Number(number) if number.is_i64() => Some(number.to_string()),
                    Value::Number(number) => number.as_f64().map(|f| format!("{f:.9}")),
                    _ => None,
                };
                if let Some(scalar) = scalar {
                    println!("field {id} {class} {object_class} {name} {scalar}");
                    lines += 1;
                }
            }
        }
        Ok(())
    })?;
    println!(
        "native readers: native fields release {release}: records {records}, scalar lines {lines}"
    );
    Ok(())
}
