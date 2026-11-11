//! RE-285 (probe): the integer built-in parameters of an element, for scoring
//! each against Revit's `LoadBearing` and `IsExternal` (#1158).
//!
//! FACT under test: a flag Revit's export writes from a built-in parameter
//! (a wall's `Structural Usage`, a floor's `Structural`, a type's `Function`)
//! is a saved parameter `native_element_parameters::read` reads on Revit 2015
//! to 2027 (RE-281), and its value says Revit's value.
//!
//! Prints one line per integer built-in parameter an element holds:
//!
//!   param <ElementId> <class> <level> <parameter id> <value>
//!   paramd <ElementId> <class> <level> <parameter id> <value>   (a double)
//!   named <ElementId> <class> <level> <name> <value>   (a parameter named
//!   for external or bearing, spaces as underscores)
//!   member <ElementId> <class> Instance <member name> <value>   (an integer
//!   or Yes/No member of a wall's root record, RE-429)
//!
//! `<class>` is the class of the element's own record, `<level>` `Instance`
//! or `Type`. `tools/re/native_flags_vs_ifc.py` scores them against the
//! export (counts only; the lines name ElementIds and stay on the runner).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re285_flag_candidates -- FILE.rvt

use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_element_parameters::{ParameterKind, ParameterLevel, read};
use std::collections::BTreeSet;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("usage: probe_re285_flag_candidates FILE.rvt");
        std::process::exit(2);
    };
    if let Err(error) = run(path) {
        println!("native readers: flag candidates failed {error:#}");
    }
}

fn run(path: &str) -> anyhow::Result<()> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    let declared: BTreeSet<u64> = elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .map(u64::from)
        .collect();
    let saved = read(&mut rf, &declared)?;
    let mut lines = 0usize;
    let mut classes: std::collections::BTreeMap<String, usize> = Default::default();
    for (id, parameters) in &saved.elements {
        let class = parameters
            .iter()
            .find(|p| p.level == ParameterLevel::Instance)
            .map(|p| p.source_class.as_str())
            .unwrap_or("?");
        *classes.entry(class.to_string()).or_insert(0usize) += 1;
        // `Wall` is the class of a 2024 file's walls (RE-429): the RE-285 list
        // named the 32-bit releases' `SWall` and `ArcWall` alone, so a wall's
        // Structural Usage was never scored against Revit's `LoadBearing`.
        if !matches!(
            class,
            "SWall" | "ArcWall" | "Wall" | "Floor" | "FamilyInstance"
        ) {
            continue;
        }
        for parameter in parameters {
            if let Some(name) = parameter.name.as_deref() {
                let lower = name.to_lowercase();
                if lower.contains("external") || lower.contains("bearing") {
                    let value = parameter.value.to_string().replace(' ', "_");
                    println!(
                        "named {id} {class} {:?} {} {value}",
                        parameter.level,
                        name.replace(' ', "_")
                    );
                }
            }
            if parameter.kind == ParameterKind::BuiltIn && parameter.storage_type == "Double" {
                if let Some(value) = parameter.value.as_f64() {
                    println!(
                        "paramd {id} {class} {:?} {} {value}",
                        parameter.level, parameter.parameter_id
                    );
                }
                continue;
            }
            if parameter.kind != ParameterKind::BuiltIn || parameter.storage_type != "Integer" {
                continue;
            }
            let value = match &parameter.value {
                serde_json::Value::Bool(flag) => i64::from(*flag),
                other => match other.as_i64() {
                    Some(value) => value,
                    None => continue,
                },
            };
            lines += 1;
            println!(
                "param {id} {class} {:?} {} {value}",
                parameter.level, parameter.parameter_id
            );
        }
    }
    println!(
        "native readers: flag candidates release {release}: elements with parameters {}, integer built-in values {lines}",
        saved.elements.len()
    );
    println!("native readers: flag candidates classes with saved parameters: {classes:?}");
    // A wall's Structural Usage is no integer among its saved parameters (the
    // 2024 walls hold none), so the integer and Yes/No members of its root
    // record are printed for the scorer to hold against `LoadBearing` (RE-429).
    let options = Options {
        selected_classes: BTreeSet::from(["SWall".to_string(), "ArcWall".to_string()]),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut walls = 0usize;
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(root)) = (
            record.class_name.as_deref(),
            record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
                .and_then(|object| object.fields.as_object()),
        ) else {
            return Ok(());
        };
        walls += 1;
        for (name, value) in root {
            let value = match value {
                serde_json::Value::Bool(flag) => i64::from(*flag),
                other => match other.as_i64() {
                    Some(value) => value,
                    None => continue,
                },
            };
            println!(
                "member {} {class} Instance {name} {value}",
                record.identity.element_id
            );
        }
        // revit-ifc's `LoadBearingCalculator` takes a wall's `LoadBearing` from
        // `StructuralUsage == Bearing`, which is the Structural box set and the
        // usage `Bearing` (1): printed as one member so the scorer holds the
        // rule itself, in every model, against Revit's value.
        let integer = |name: &str| match root.get(name) {
            Some(serde_json::Value::Bool(flag)) => Some(i64::from(*flag)),
            Some(other) => other.as_i64(),
            None => None,
        };
        if let (Some(significant), Some(usage)) = (
            integer("m_isStructuralSignificant"),
            integer("m_wallStructuralUsage"),
        ) {
            println!(
                "member {} {class} Instance wall_load_bearing_rule {}",
                record.identity.element_id,
                i64::from(significant == 1 && usage == 1)
            );
        }
        Ok(())
    })?;
    println!("native readers: flag candidates wall root records {walls}");
    Ok(())
}
