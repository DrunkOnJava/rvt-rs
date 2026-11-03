//! RE-312: which family instances named `IfcSpace` Revit's export leaves out
//! (#1262).
//!
//! FACT under test: on the BIM4LCA terrain office (Revit 2024) 94 `Parking
//! Space` family instances carry "Export to IFC As" `IfcSpace` with `PARKING`,
//! and Revit's own IFC4 export writes 2 of them (type `AP`, named
//! `Parking Space:AP:<ElementId>`) and none of the 92 of type `PP`. rvt-rs
//! follows the value and writes every one (#1262). Revit's exporter reads an
//! element's own value and then its type's (`ExporterUtil.
//! GetIFCExportElementParameterInfo`, revit-ifc, CLEANROOM.md source 5), and
//! before that decides whether to export the element at all
//! (`ElementFilteringUtil.ShouldElementBeExported`: "Export to IFC" on the
//! element or its type, the category, an excluded entity; the design option
//! and phase filters; the export view's visibility). A family instance with
//! no 3D body is dropped (`FamilyInstanceExporter.ExportFamilyInstanceElement`).
//!
//! This prints, for every element whose own "Export to IFC As" is `IfcSpace`,
//! the facts any of those could read, so a scorer
//! (`tools/re/parking_spaces_vs_ifc.py`) can set them against Revit's export:
//!
//!   release <n>
//!   export <id> as=<v> predefined=<v> type_as=<v> type_predefined=<v>
//!   record <id> category=<n> container=<hex> placement=<n> design_option=<id|-> flags=<hex>
//!          dims=<x>x<y>x<z> refs=<n> class_tag=<n> stream=<name>
//!   instance <id> class=<name> <path>=<value> ...
//!                      every scalar of the element's native record (RE-153 graph)
//!   symbol <id> class=<name> <path>=<value> ...
//!                      the same for each type symbol an instance's integers name
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re312_parking_spaces -- FILE.rvt

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_element_records::scan_category_records_multi;
use rvt::research::partition_ifc_export_overrides::scan_export_parameters;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const MAX_ARRAY: usize = 8;
const MAX_TEXT: usize = 60;

fn text(value: &str) -> String {
    value
        .chars()
        .take(MAX_TEXT)
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

fn flatten(prefix: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) if map.contains_key("pointer_token") => {
            out.push((prefix.to_string(), "->".into()));
        }
        Value::Object(map) => {
            for (key, inner) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(&path, inner, out);
            }
        }
        Value::Array(items) => {
            let scalars = items
                .iter()
                .all(|item| !item.is_object() && !item.is_array());
            if scalars && items.len() <= MAX_ARRAY {
                let joined: Vec<String> = items
                    .iter()
                    .map(|item| match item {
                        Value::String(s) => text(s),
                        other => other.to_string(),
                    })
                    .collect();
                out.push((prefix.to_string(), format!("[{}]", joined.join(","))));
            } else {
                out.push((format!("{prefix}.len"), items.len().to_string()));
            }
        }
        Value::String(s) => out.push((prefix.to_string(), text(s))),
        other => out.push((prefix.to_string(), other.to_string())),
    }
}

fn integers(value: &Value, out: &mut BTreeSet<u64>) {
    match value {
        Value::Number(n) => {
            if let Some(v) = n.as_u64() {
                out.insert(v);
            }
        }
        Value::Object(map) => map.values().for_each(|inner| integers(inner, out)),
        Value::Array(items) => items.iter().for_each(|inner| integers(inner, out)),
        _ => {}
    }
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("release {version}");
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let parameters = scan_export_parameters(&mut rf, version, &declared)?;
    let mut ids: BTreeSet<u64> = BTreeSet::new();
    for (id, p) in &parameters {
        let show = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
        let space = p
            .export_as
            .as_deref()
            .is_some_and(|v| v.eq_ignore_ascii_case("IfcSpace"));
        if space {
            ids.insert(u64::from(*id));
            println!(
                "export {id} as={} predefined={} type_as={} type_predefined={}",
                show(&p.export_as),
                show(&p.predefined_type),
                show(&p.type_export_as),
                show(&p.type_predefined_type),
            );
        }
    }

    let categories = [
        rvt::research::partition_element_records::OST_PARKING,
        -2_000_160,
        -2_003_200,
        -2_003_600,
    ];
    for record in scan_category_records_multi(&mut rf, version, &categories, &declared)? {
        if !ids.contains(&u64::from(record.element_id)) {
            continue;
        }
        let b = record.bbox_feet;
        println!(
            "record {} category={} container={:#x} placement={} design_option={} flags={:#x} dims={:.4}x{:.4}x{:.4} refs={} class_tag={} stream={}",
            record.element_id,
            record.builtin_category,
            record.container,
            record.placement_kind,
            record
                .design_option
                .map_or_else(|| "-".to_string(), |o| o.to_string()),
            record.flags,
            b[3] - b[0],
            b[4] - b[1],
            b[5] - b[2],
            record.references.len(),
            record.class_tag,
            record.stream,
        );
    }

    let mut named: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    let options = Options {
        selected_ids: ids.clone(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(object) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
            return Ok(());
        };
        let mut leaves = Vec::new();
        flatten("", &object.fields, &mut leaves);
        let line: Vec<String> = leaves.iter().map(|(k, v)| format!("{k}={v}")).collect();
        println!(
            "instance {} class={} {}",
            record.identity.element_id,
            record.class_name.clone().unwrap_or_default(),
            line.join(" ")
        );
        let mut found = BTreeSet::new();
        integers(&object.fields, &mut found);
        named.insert(record.identity.element_id, found);
        Ok(())
    })?;

    let wanted: BTreeSet<u64> = named.values().flatten().copied().collect();
    let options = Options {
        selected_classes: ["FamilySymbol".to_string()].into_iter().collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        if !wanted.contains(&record.identity.element_id) {
            return Ok(());
        }
        let Some(object) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
            return Ok(());
        };
        let mut leaves = Vec::new();
        flatten("", &object.fields, &mut leaves);
        let line: Vec<String> = leaves.iter().map(|(k, v)| format!("{k}={v}")).collect();
        println!(
            "symbol {} class={} {}",
            record.identity.element_id,
            record.class_name.clone().unwrap_or_default(),
            line.join(" ")
        );
        Ok(())
    })?;
    Ok(())
}
