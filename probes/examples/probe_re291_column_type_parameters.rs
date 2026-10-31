//! RE-291 (#1046, W7): what each column type's family parameters hold, for
//! telling a round or otherwise non-rectangular section from the record box
//! a column is drawn as.
//!
//! FACT under test: a column type stores its section's dimensions as
//! `f64 · ff×8 · i64` length entries of its value block (RE-93), under
//! family parameters whose captions name them (RE-221: `b`, `Ht`, `t` and
//! `kr` for a hollow section). A rectangular column type holds two
//! dimensions equal to its columns' record boxes' sides; a round one holds a
//! diameter or a radius. A box-drawn column no clean pair joins needs such a
//! check before it gets the box's quantities (issue 1203, item 2).
//!
//! Output:
//!   type <type id> <family>:<type name> columns <n> box <width> <depth>
//!   elem <id>                              up to 3 columns per type
//!   param <type id> <parameter> <values, feet>
//!   native <parameter> <object index> <record class>/<object class> <scalar members>
//!                                          the element of every positive parameter id
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re291_column_type_parameters -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_schema_mvp::{
    FAMILY_NAME_FIELD, TYPE_ID_FIELD, TYPE_NAME_FIELD, recover_partition_schema_mvp,
};
use rvt::research::partition_type_parameters::partition_length_entries;
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};

/// A column type's name, its columns' ElementIds and the sides of the first one's record box.
type TypeColumns = (String, Vec<u32>, [Option<f64>; 2]);

fn text(element: &DecodedElement, wanted: &str) -> Option<String> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(text) if name == wanted => Some(text.clone()),
        _ => None,
    })
}

fn float(element: &DecodedElement, wanted: &str) -> Option<f64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

fn show(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut by_type: BTreeMap<u32, TypeColumns> = BTreeMap::new();
    for element in mvp.columns.iter().chain(mvp.products.iter()) {
        if !matches!(element.class.as_str(), "Column" | "StructuralColumn") {
            continue;
        }
        let family = text(element, FAMILY_NAME_FIELD).unwrap_or_default();
        let name = text(element, TYPE_NAME_FIELD).unwrap_or_default();
        let type_id = element
            .fields
            .iter()
            .find_map(|(field, value)| match value {
                InstanceField::ElementId { id, .. } if field == TYPE_ID_FIELD => Some(*id),
                _ => None,
            });
        let (Some(id), Some(type_id)) = (element.id, type_id) else {
            continue;
        };
        let held = by_type.entry(type_id).or_insert_with(|| {
            (
                format!("{family}:{name}"),
                Vec::new(),
                [float(element, "m_bboxWidth"), float(element, "m_bboxDepth")],
            )
        });
        held.1.push(id);
    }
    for (type_id, (name, columns, size)) in &by_type {
        println!(
            "type {type_id} {} columns {} box {} {}",
            name.replace(' ', "_"),
            columns.len(),
            show(size[0]),
            show(size[1]),
        );
        for id in columns.iter().take(3) {
            println!("elem {id}");
        }
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let types: BTreeSet<u32> = by_type.keys().copied().collect();
    let mut found: BTreeMap<(u32, i64), BTreeSet<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, entries) in partition_length_entries(inflated.bytes(), &declared) {
            if !types.contains(&owner) {
                continue;
            }
            for (parameter, value) in entries {
                found
                    .entry((owner, parameter))
                    .or_default()
                    .insert(format!("{value:.6}"));
            }
        }
    }
    let mut elements: BTreeSet<u64> = BTreeSet::new();
    for ((owner, parameter), values) in found {
        let values: Vec<String> = values.into_iter().collect();
        println!("param {owner} {parameter} {}", values.join(" "));
        if let Ok(id) = u64::try_from(parameter) {
            if id > 1000 && id < u64::from(u32::MAX) {
                elements.insert(id);
            }
        }
    }
    let options = Options {
        selected_ids: elements,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate().take(6) {
            let members: Vec<String> = object
                .fields
                .as_object()
                .into_iter()
                .flat_map(|fields| fields.iter())
                .filter(|(_, v)| v.is_number() || v.is_boolean() || v.is_string())
                .map(|(name, v)| format!("{name}={v}"))
                .collect();
            println!(
                "native {id} {index} {class}/{} {}",
                object.class_name,
                members.join(" ")
            );
        }
        Ok(())
    })?;
    Ok(())
}
