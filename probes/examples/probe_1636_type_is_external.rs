//! #1636 (RE-412): the `<property set>.IsExternal` parameters a type holds, which
//! `attach_is_external` scans for and does not give an element.
//!
//! FACT under test: revit-ifc reads a property set entry's parameter from the element
//! and then from its type (`PropertySetEntry("Pset_StairCommon.IsExternal",
//! "IsExternal")`, CLEANROOM.md source 5, RE-193). `attach_is_external` scans the
//! values of every Yes/No parameter named `IsExternal` or `<set>.IsExternal` on each
//! element and on its type, and gives the element the type's value only of the plain
//! name. On the BIM4LCA housing one `IfcStair` (the ladder `Talotikkaat:JSV-1.tikas`,
//! tag 1857214) has `Pset_StairCommon.IsExternal` False in rvt-rs's export and True in
//! Revit's. This lists, for every element whose type holds a set-specific value, what the
//! element and its type hold and what rvt-rs decoded.
//!
//! Output:
//!   def <name> <parameter id>
//!   element <id> class <class> type <id> own [<name>=<0|1>...] type [<name>=<0|1>...]
//!   decoded [<field>=<value>...]
//!   tally <release>: <class> type holds a set-specific value: <n>, of those the element
//!   holds the same parameter: <m>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1636_type_is_external -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_room_parameters::{
    find_parameter_definitions, scan_integer_parameters,
};
use rvt::research::partition_schema_mvp::{
    IS_EXTERNAL_FIELD, IS_EXTERNAL_PARAMETER_NAME, NAMED_PARAMETER_FIELD_PREFIX, TYPE_ID_FIELD,
    TYPE_NAMED_PARAMETER_FIELD_PREFIX, recover_partition_schema_mvp,
};
use rvt::walker::{InstanceField, WalkerLimits};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let definitions: BTreeMap<String, u32> = find_parameter_definitions(&mut rf, version)
        .into_iter()
        .filter(|(name, _)| {
            name == IS_EXTERNAL_PARAMETER_NAME
                || name.ends_with(&format!(".{IS_EXTERNAL_PARAMETER_NAME}"))
        })
        .collect();
    for (name, id) in &definitions {
        println!("def {name} {id}");
    }
    if definitions.is_empty() {
        println!("tally {version}: no IsExternal definitions");
        return Ok(());
    }
    let elements =
        recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?.into_elements();
    let type_of = |element: &rvt::walker::DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let owners: BTreeSet<u32> = elements
        .iter()
        .flat_map(|element| element.id.into_iter().chain(type_of(element)))
        .collect();
    let ids: Vec<i64> = definitions.values().map(|&id| i64::from(id)).collect();
    let found = scan_integer_parameters(&mut rf, version, &owners, &ids);
    let mut tally: BTreeMap<(String, bool), usize> = BTreeMap::new();
    for element in &elements {
        let (Some(id), Some(type_id)) = (element.id, type_of(element)) else {
            continue;
        };
        let held = |owner: u32| -> Vec<(String, u32)> {
            definitions
                .iter()
                .filter_map(|(name, definition)| {
                    found
                        .get(&i64::from(*definition))
                        .and_then(|values| values.get(&owner))
                        .map(|value| (name.clone(), *value))
                })
                .collect()
        };
        let (own, ty) = (held(id), held(type_id));
        let set_specific: Vec<&(String, u32)> = ty
            .iter()
            .filter(|(name, _)| name != IS_EXTERNAL_PARAMETER_NAME)
            .collect();
        if set_specific.is_empty() {
            continue;
        }
        let same = set_specific
            .iter()
            .any(|(name, _)| own.iter().any(|(other, _)| other == name));
        *tally.entry((element.class.clone(), same)).or_default() += 1;
        let decoded: Vec<String> = element
            .fields
            .iter()
            .filter_map(|(name, value)| {
                let named = (name.starts_with(NAMED_PARAMETER_FIELD_PREFIX)
                    || name.starts_with(TYPE_NAMED_PARAMETER_FIELD_PREFIX))
                    && name.contains(IS_EXTERNAL_PARAMETER_NAME);
                match value {
                    InstanceField::Bool(flag) if named || name == IS_EXTERNAL_FIELD => {
                        Some(format!("{name}={flag}"))
                    }
                    _ => None,
                }
            })
            .collect();
        println!(
            "element {id} class {} type {type_id} own {own:?} type {ty:?} decoded {decoded:?}",
            element.class
        );
    }
    for ((class, same), count) in &tally {
        println!(
            "tally {version}: {class} type holds a set-specific value, element holds the same parameter {same}: {count}"
        );
    }
    Ok(())
}
