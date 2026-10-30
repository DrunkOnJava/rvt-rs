//! RE-227 (#35, W7): which names a project's own parameters carry for the
//! standard properties Revit's export fills from parameters.
//!
//! FACT under test: revit-ifc's `PropertySetEntryMap` (CLEANROOM.md source
//! 5) fills a standard property from a parameter named, in turn, for its
//! localized name, `<set>.<property>`, and the bare property name, on the
//! element and then on its type. RE-194 reads only `<set>.<property>` on
//! the element, and the BIM4LCA office's export writes values rvt-rs does
//! not, such as `Pset_DoorCommon.FireExit` on all 266 doors. This lists
//! every project or shared parameter definition, marking each whose name is
//! a standard property's, qualified or bare, ignoring case and spaces.
//!
//! Output, one line per definition, sorted by name:
//!   def <id> <qualified|bare|spaced|-> <sets holding that property, comma-separated|-> <name>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re227_parameter_names -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::standard_property_sets::STANDARD_PROPERTY_SETS;
use rvt::research::partition_room_parameters::find_parameter_definitions;

fn squash(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mut by_property: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    let mut qualified: BTreeMap<String, &str> = BTreeMap::new();
    for set in STANDARD_PROPERTY_SETS {
        for (property, _) in set.properties {
            by_property
                .entry(squash(property))
                .or_default()
                .push(set.name);
            qualified.insert(format!("{}.{property}", set.name), set.name);
        }
    }
    let definitions = find_parameter_definitions(&mut rf, version);
    let mut marked = 0;
    for (name, id) in &definitions {
        let (kind, sets) = if let Some(set) = qualified.get(name) {
            ("qualified", set.to_string())
        } else if let Some(sets) = by_property.get(&squash(name)) {
            let kind = if name.contains(' ') { "spaced" } else { "bare" };
            (kind, sets.join(","))
        } else {
            ("-", "-".to_string())
        };
        if kind != "-" {
            marked += 1;
        }
        println!("def {id} {kind} {sets} {name}");
    }
    eprintln!(
        "Revit {version}: {} parameter definitions, {marked} named for a standard property",
        definitions.len()
    );
    Ok(())
}
