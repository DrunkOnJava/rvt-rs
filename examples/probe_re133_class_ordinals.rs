//! RE-133 (probe): the serialization tags of jakobhirn-bit's class table
//! (Discussion #112).
//!
//! jakobhirn-bit gave the class numbers, definition ordinals, he reads ten
//! classes by in Revit 2024 and 2026: `ElementHeader`, `Family`,
//! `FamilyInstance`, `FamilySymbol`, `ContentMarker`, `ElementParents`,
//! `FamilyInstancePatternHelper`, `InstanceInfo`, `RbsPipeCurve` and
//! `RbsCurveConnectorManager`. This prints the tag `Formats/Latest` gives
//! each of them in the file it is run on (a class's tag is its definition
//! ordinal) with its base class, one JSON line per class and a header line
//! per file, so the table can be checked release by release on the reference
//! models and the Autodesk family corpus.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re133_class_ordinals -- FILE.rvt > tags.jsonl

use rvt::RevitFile;

const CLASSES: &[&str] = &[
    "ElementHeader",
    "Family",
    "FamilyInstance",
    "FamilySymbol",
    "ContentMarker",
    "ElementParents",
    "FamilyInstancePatternHelper",
    "InstanceInfo",
    "RbsPipeCurve",
    "RbsCurveConnectorManager",
];

/// A JSON value for an optional string.
fn text(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_string(), |v| format!("{v:?}"))
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let file = std::path::Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let schema = rf.schema_classes()?;
    println!(
        "{{\"file\":{file:?},\"revit\":{version},\"classes\":{},\"first_tag\":{},\"parsed_bytes\":{},\"stopped\":{}}}",
        schema.classes.len(),
        schema.classes.first().map_or(0, |class| class.tag),
        schema.parsed_bytes,
        text(schema.stopped.as_deref())
    );
    for name in CLASSES {
        let mut found = false;
        for class in schema.classes.iter().filter(|class| class.name == *name) {
            found = true;
            let base = class
                .base
                .and_then(|tag| schema.by_tag(tag))
                .map(|base| base.name.as_str());
            println!(
                "{{\"file\":{file:?},\"revit\":{version},\"class\":{name:?},\"tag\":{},\"base\":{},\"version\":{},\"fields\":{}}}",
                class.tag,
                text(base),
                class.version,
                class.field_count
            );
        }
        if !found {
            println!("{{\"file\":{file:?},\"revit\":{version},\"class\":{name:?},\"tag\":null}}");
        }
    }
    Ok(())
}
