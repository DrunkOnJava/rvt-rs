//! #1843: two saves of one project, compared element by element.
//!
//! FACT under test: `rvt::ifc::element_diff` keys each element the IFC export
//! writes by the GlobalId it writes. Two saves of one project then match
//! element for element, and only what differs between the saves is listed.
//! A file against itself lists nothing. Across the saves Autodesk ships of one
//! sample project, most elements keep their GlobalId (Revit's own, rebuilt
//! from the file). What the probe reports is the counts.
//!
//! The probe:
//! - with one file, compares its export with a second export of the same
//!   file;
//! - with two (Measure's `probe_pair_release`: a sample, then the same
//!   project saved by another release), compares their exports.
//!
//! Output (counts only, no names or ids):
//!   elements left=<n> right=<n> added=<n> removed=<n> changed=<n> unchanged=<n>
//!   added_by_type <IFCTYPE>=<n> ...
//!   removed_by_type <IFCTYPE>=<n> ...
//!   changed_by_field <field>=<n> ...      (properties counted as one field)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1843_element_diff -- A.rvt [B.rvt]

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::element_diff::{ElementState, diff_elements, element_states};

fn states(path: &str) -> anyhow::Result<Vec<ElementState>> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let run = rvt::ifc::run_export(&mut rf, &rvt::ifc::ExportOptions::default())?;
    Ok(element_states(&run.model))
}

/// A property's number, whether written bare or as its kind (`{"Real": 1.5}`).
fn number(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().or_else(|| {
        let object = value.as_object()?;
        (object.len() == 1).then(|| object.values().next()?.as_f64())?
    })
}

fn by_type(states: &[ElementState]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for state in states {
        *counts.entry(state.ifc_type.as_str()).or_default() += 1;
    }
    counts
        .iter()
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let (left, right) = match files.as_slice() {
        [one] => (states(one)?, states(one)?),
        [first, second, ..] => (states(first)?, states(second)?),
        [] => anyhow::bail!("usage: probe_1843_element_diff A.rvt [B.rvt]"),
    };
    let diff = diff_elements(&left, &right);
    println!(
        "elements left={} right={} added={} removed={} changed={} unchanged={}",
        diff.left_elements,
        diff.right_elements,
        diff.added.len(),
        diff.removed.len(),
        diff.changed.len(),
        diff.unchanged
    );
    println!("added_by_type {}", by_type(&diff.added));
    println!("removed_by_type {}", by_type(&diff.removed));
    let mut fields: BTreeMap<&str, usize> = BTreeMap::new();
    for change in &diff.changed {
        let kinds: BTreeSet<&str> = change
            .changes
            .iter()
            .map(|field| {
                if field.field.starts_with("property:") {
                    "properties"
                } else {
                    field.field.as_str()
                }
            })
            .collect();
        for kind in kinds {
            *fields.entry(kind).or_default() += 1;
        }
    }
    println!(
        "changed_by_field {}",
        fields
            .iter()
            .map(|(field, count)| format!("{field}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    // Which properties differ, by `<set>.<property>` (names only, no values):
    // a file against itself should list none (#1852's first run listed 11 on
    // Core Interior). Per key: how often, how often one side lacks it, and the
    // largest difference where both are numbers.
    let mut properties: BTreeMap<&str, (usize, usize, usize, f64)> = BTreeMap::new();
    for field in diff.changed.iter().flat_map(|change| &change.changes) {
        let Some(key) = field.field.strip_prefix("property:") else {
            continue;
        };
        let entry = properties.entry(key).or_insert((0, 0, 0, 0.0));
        entry.0 += 1;
        entry.1 += usize::from(field.left.is_null());
        entry.2 += usize::from(field.right.is_null());
        if let (Some(a), Some(b)) = (number(&field.left), number(&field.right)) {
            entry.3 = entry.3.max((a - b).abs());
        }
    }
    for (key, (n, left_absent, right_absent, delta)) in &properties {
        println!(
            "changed_property {key} n={n} left_absent={left_absent} right_absent={right_absent} max_numeric_delta={delta:e}"
        );
    }
    Ok(())
}
