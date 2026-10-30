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
//!
//! `<class>` is the class of the element's own record, `<level>` `Instance`
//! or `Type`. `tools/re/native_flags_vs_ifc.py` scores them against the
//! export (counts only; the lines name ElementIds and stay on the runner).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re285_flag_candidates -- FILE.rvt

use rvt::RevitFile;
use rvt::elem_table;
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
    for (id, parameters) in &saved.elements {
        let class = parameters
            .iter()
            .find(|p| p.level == ParameterLevel::Instance)
            .map(|p| p.source_class.as_str())
            .unwrap_or("?");
        if !matches!(class, "SWall" | "ArcWall" | "Floor" | "FamilyInstance") {
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
    Ok(())
}
