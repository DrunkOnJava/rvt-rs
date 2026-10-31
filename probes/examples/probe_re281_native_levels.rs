//! RE-281 (probe): the Level and the phases an element's own native record
//! names, on Revit 2015 to 2027 (#1158, #879).
//!
//! FACT under test: the Element base of every native record
//! (`m_assocLevelId`, `m_createdPhaseId`, `m_demolishedPhaseId`) reads on
//! Revit 2015 to 2022 as it does on 2024 (`native_element::decode_base`
//! takes a 4-byte `Identifier`, RE-281), and the Level it names is the
//! storey Revit's own export puts the element in: on BIMcollab's Revit 2015
//! example, the 16 footings that sit on no storey (#879) name a Level, and the
//! three air terminals Revit's export leaves out (RE-198) are not in the
//! export phase. `tools/re/native_levels_vs_ifc.py` scores the lines against
//! Revit's export.
//!
//! Output:
//!   native levels: ... count lines: elements with an Element base, with an
//!     associated Level, with a created phase, and the phases listed;
//!   element <ElementId> <class> <assoc level> <created phase> <demolished phase>
//!     one line per declared element whose record reads (ElementIds, so a
//!     model without a licence keeps only the count lines);
//!   param <ElementId> <parameter id> <value>
//!     each ElementId-valued saved parameter an element holds itself
//!     (`native_element_parameters::read`), unless `--records` is given: a
//!     Level an element names that its Element base does not.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re281_native_levels -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document::{self, NATIVE_RELEASES, Options};
use rvt::research::native_element_parameters::{ParameterLevel, read};
use std::collections::BTreeSet;

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re281_native_levels MODEL.rvt");
        std::process::exit(2);
    }
    let records_only = std::env::args().any(|arg| arg == "--records");
    for path in paths.iter().filter(|p| !p.starts_with("--")) {
        if let Err(error) = run(path, records_only) {
            let text: String = format!("{error:#}")
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == ' ' {
                        c
                    } else {
                        '_'
                    }
                })
                .take(120)
                .collect();
            println!("native levels: failed {text}");
        }
    }
}

fn run(path: &str, records_only: bool) -> anyhow::Result<()> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    let options = Options {
        graph_classes: Some(BTreeSet::new()),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut elements = 0usize;
    let mut with_level = 0usize;
    let mut with_phase = 0usize;
    let mut lines = Vec::new();
    let mut ids = BTreeSet::new();
    let summary = native_document::extract_graphs(&mut rf, &options, |record| {
        let Some(base) = &record.base else {
            return Ok(());
        };
        elements += 1;
        with_level += usize::from(base.associated_level_id != -1);
        with_phase += usize::from(base.created_phase_id != -1);
        let class = record.class_name.clone().unwrap_or_default();
        ids.insert(record.identity.element_id);
        lines.push(format!(
            "element {} {} {} {} {}",
            record.identity.element_id,
            class,
            base.associated_level_id,
            base.created_phase_id,
            base.demolished_phase_id
        ));
        Ok(())
    })?;
    let phases = rf.phase_filter();
    println!(
        "native levels: release {release}: records with an Element base {elements} of {} emitted, with an associated Level {with_level}, with a created phase {with_phase}, phases listed {}, elements left out of the last phase {}",
        summary.emitted_records,
        phases.order.len(),
        phases.excluded.len()
    );
    println!(
        "native levels: Element base refused {}",
        summary.emitted_records - elements
    );
    for line in lines {
        println!("{line}");
    }
    if !records_only {
        let saved = read(&mut rf, &ids)?;
        let (mut elements_with, mut values) = (0usize, 0usize);
        for (id, parameters) in &saved.elements {
            let mut any = false;
            for parameter in parameters {
                if parameter.level != ParameterLevel::Instance
                    || parameter.storage_type != "ElementId"
                {
                    continue;
                }
                if let Some(value) = parameter.value.as_i64() {
                    println!("param {id} {} {value}", parameter.parameter_id);
                    values += 1;
                    any = true;
                }
            }
            elements_with += usize::from(any);
        }
        println!(
            "native levels: saved parameters read for {} elements, {elements_with} holding an ElementId parameter of their own, {values} such values",
            saved.elements.len()
        );
    }
    Ok(())
}
