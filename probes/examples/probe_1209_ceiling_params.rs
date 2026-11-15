//! #1209: whether a ceiling's saved parameters hold the area Revit's export
//! gives a cut ceiling.
//!
//! FACT to check: revit-ifc's `GrossAreaCalculator` (CLEANROOM.md source 5)
//! takes a covering's `GrossArea` from its extrusion's area where its body is
//! one extrusion, and otherwise from the element's computed area
//! (`HOST_AREA_COMPUTED`). Revit's value on a ceiling its record cuts
//! (`CutPairGStep`) is the area left after the cut, so if Revit saves that
//! computed area with the element, one of these parameters holds it.
//!
//! Output, one record per line:
//!   ceiling <id> cut <true|false>
//!   param <id> <parameter id> <value json> <source element id> <name>
//!
//! Every `Ceiling` record is listed with whether its graph holds a
//! `CutPairGStep`, then the saved parameters of each, with their names where
//! the file's definitions give one. `tools/re/ceiling_area_params_vs_ifc.py`
//! compares them with Revit's `GrossArea`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1209_ceiling_params -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_element_parameters;

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: BTreeSet::from(["Ceiling".to_string()]),
        ..Options::default()
    };
    let mut ceilings: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let cut = graph
            .objects
            .iter()
            .any(|object| object.class_name == "CutPairGStep");
        if ceilings.insert(id) {
            println!("ceiling {id} cut {cut}");
        }
        Ok(())
    })?;
    let saved = native_element_parameters::read(&mut rf, &ceilings)?;
    for (id, parameters) in &saved.elements {
        for parameter in parameters {
            println!(
                "param {id} {} {} {} {:?}",
                parameter.parameter_id,
                parameter.value,
                parameter.source_element_id,
                parameter.name.as_deref().unwrap_or("-"),
            );
        }
    }
    eprintln!("ceilings {}", ceilings.len());
    Ok(())
}
