//! #810: what sets apart the BIM4LCA office's doors whose `OverallHeight`
//! in Revit's export is a horizontal extent of their record box.
//!
//! FACT under test: Revit sizes a door or window whose symbol cuts no loop
//! by its record box along its own X axis and its height (RE-218), except
//! on four hatch doors (type 1107673) and four doors of type 3267242, whose
//! height is the box's extent along the instance's Y axis; and two doors of
//! type 1106810/1106812 are 0.1137 ft narrower in Revit's export than their
//! box. This prints, for those doors and for two that match (1382481,
//! 1432311): the instance's transform; every element id its element record
//! names, with that element's class; and the scalar, string and element-id
//! members of the instance's and its symbol's native records.
//!
//! Output:
//!   door <id> class <class> axes <x> <y> <z> origin <o>
//!   ref <id> <element id> <class>
//!   native <id> <object index> <class> <member=value ...>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_810_odd_doors -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_element_records::{REFERENCE_LIST_OFFSET, decode_reference_list};
use rvt::research::partition_instance_transforms::scan_instance_transforms;
use rvt::walker::{InstanceField, iter_elements};
use serde_json::Value;

const DOORS: &[u32] = &[
    2053330, 2896990, 3269270, 3270587, 1756089, 1765267, 1382481, 1432311,
];

fn members(fields: &Value) -> String {
    fields
        .as_object()
        .into_iter()
        .flat_map(|members| members.iter())
        .filter_map(|(name, value)| match value {
            Value::Number(_) | Value::Bool(_) => Some(format!("{name}={value}")),
            Value::String(text) => Some(format!("{name}={}", text.replace(' ', "_"))),
            _ => value
                .get("m_id")?
                .get("m_id64")
                .map(|id| format!("{name}=@{id}")),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let wanted: BTreeSet<u32> = DOORS.iter().copied().collect();
    let elements: Vec<_> = iter_elements(&mut rf)?.collect();
    let classes: BTreeMap<u64, String> = elements
        .iter()
        .filter_map(|e| Some((u64::from(e.id?), e.class.clone())))
        .collect();
    let transforms = scan_instance_transforms(&mut rf, version, &wanted)?;
    let mut symbols = BTreeSet::new();
    for element in elements
        .iter()
        .filter(|e| e.id.is_some_and(|id| wanted.contains(&id)))
    {
        let id = element.id.unwrap_or_default();
        let t = transforms.get(&id);
        println!(
            "door {id} class {} axes {:?} origin {:?}",
            element.class,
            t.map(|t| t.axes),
            t.map(|t| t.origin)
        );
        let mut stream = None;
        let mut offset = None;
        for (name, value) in &element.fields {
            match (name.as_str(), value) {
                ("m_source_stream", InstanceField::String(v)) => stream = Some(v.clone()),
                ("m_source_offset", InstanceField::Integer { value, .. }) => {
                    offset = usize::try_from(*value).ok();
                }
                _ => {}
            }
        }
        if let (Some(stream), Some(offset)) = (stream, offset) {
            let references = rf
                .inflated_partition(&stream)
                .ok()
                .and_then(|inflated| {
                    decode_reference_list(inflated.bytes(), offset + REFERENCE_LIST_OFFSET)
                })
                .unwrap_or_default();
            for r in references {
                println!(
                    "ref {id} {r} {}",
                    classes.get(&r).map(String::as_str).unwrap_or("-")
                );
            }
        }
    }
    let options = Options {
        selected_ids: wanted.iter().map(|&id| u64::from(id)).collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate() {
            if index > 0 && !matches!(object.class_name.as_str(), "InstanceInfo" | "SymbolInfo") {
                continue;
            }
            if object.class_name == "InstanceInfo" {
                if let Some(symbol) = object
                    .fields
                    .get("m_symbolId")
                    .and_then(|v| v.get("m_id")?.get("m_id64")?.as_u64())
                {
                    symbols.insert(symbol);
                }
            }
            println!(
                "native {id} {index} {class}/{} {}",
                object.class_name,
                members(&object.fields)
            );
        }
        Ok(())
    })?;
    let options = Options {
        selected_ids: symbols,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        if let Some(root) = graph.objects.first() {
            let cut = root.fields.get("m_pCutLoops").map(|v| v.to_string());
            println!(
                "symbol {} {class} cut-loops {} {}",
                record.identity.element_id,
                cut.unwrap_or_else(|| "absent".into()),
                members(&root.fields)
            );
        }
        Ok(())
    })?;
    Ok(())
}
