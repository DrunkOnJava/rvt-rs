//! RE-207: an element's saved parameters, named by the definitions the file
//! holds, with a family instance's type parameters after its own.
//!
//! FACT: on the BIM4LCA office (Revit 2024), every parameter named
//! `<set>.<property>` that rvt-rs reads from an element's native record holds
//! the value Revit's export writes for that property on the element: 7,606
//! of 7,606 as built, 7,592 of 7,592 at building permit. On the R2025 models
//! (#356) a window's `Pset_WindowCommon.FireRating` is its type's family
//! parameter `Fire Rating`, read through the window's `m_masterSymbolId`.
//! Verify with the Measure workflow, `-f nordic=true -f
//! probe=probe_re207_element_parameters -f
//! probe_scorer=element_parameters_vs_ifc`.
//!
//! The values come from `native_element_parameters::read`
//! (`src/research/native_element_parameters.rs`): each element's typed parameter sets
//! (`m_pParamValueSet*`), the family slots of its own `FamilyParams` that a
//! definition resolves, and the native fields validated as built-in
//! parameters; then, for a family instance, its type's values for the
//! parameters it does not hold, except the family's instance parameters,
//! whose value on the type is a default. A parameter is named by its
//! definition's `m_caption`; a built-in parameter keeps its negative id.
//!
//! The probe prints a first line counting, per element class, the values
//! read by kind (built-in, family, shared, project, global) and level
//! (instance, type), the family slots no definition resolves and the
//! instance parameters an instance holds no value for. Then one JSON line per
//! element of the production walker, `{id, class_name, parameters}`, as
//! `rvt-elements` lists them. `tools/re/element_parameters_vs_ifc.py` pairs
//! the lines with Revit's export.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re207_element_parameters -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_element_parameters::read;
use rvt::walker::{PRODUCTION_ELEMENT_MIN_SCORE, WalkerLimits, iter_elements_with_limits};
use serde_json::json;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re207_element_parameters MODEL.rvt"))?;
    let mut file = RevitFile::open(&path)?;
    let elements: Vec<_> = iter_elements_with_limits(
        &mut file,
        PRODUCTION_ELEMENT_MIN_SCORE,
        WalkerLimits::default(),
    )?
    .collect();
    let ids = elements
        .iter()
        .filter_map(|e| e.id)
        .map(u64::from)
        .collect();
    let saved = read(&mut file, &ids)?;
    let mut by_class: BTreeMap<&str, BTreeMap<String, usize>> = BTreeMap::new();
    for element in &elements {
        let Some(values) = element.id.and_then(|id| saved.elements.get(&u64::from(id))) else {
            continue;
        };
        let counts = by_class.entry(element.class.as_str()).or_default();
        *counts.entry("elements".into()).or_default() += 1;
        for value in values {
            let kind = serde_json::to_value(value.kind)?;
            let level = serde_json::to_value(value.level)?;
            let key = format!(
                "{}/{}",
                kind.as_str().unwrap_or("?"),
                level.as_str().unwrap_or("?")
            );
            *counts.entry(key).or_default() += 1;
            if value.name.is_some() {
                *counts.entry("named".into()).or_default() += 1;
            }
        }
    }
    println!(
        "{}",
        json!({
            "by_class": by_class,
            "unresolved_family_slots": saved.unresolved_family_slots,
            "instance_parameters_without_value": saved.instance_parameters_without_value,
        })
    );
    for element in &elements {
        let parameters = saved.json_entries(element.id);
        if parameters.is_empty() {
            continue;
        }
        println!(
            "{}",
            json!({"id": element.id, "class_name": element.class, "parameters": parameters})
        );
    }
    Ok(())
}
