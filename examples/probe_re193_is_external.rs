//! RE-193: the inputs of Revit's `IsExternal`, element by element.
//!
//! FACT: Revit's IFC export writes each element's `IsExternal` by the rule
//! below. On the BIM4LCA office, as built, the rule gives Revit's value on
//! every wall (607), curtain wall (220), door, window, column, beam, slab,
//! ramp, stair, railing, covering, space and proxy it reaches, and the same
//! on the building-permit file, RE1 and the R2025 models. Curtain mullions and
//! panels are the exception: their curtain wall's value is Revit's on 1,423
//! of 10,003 members. Verify with `tools/re/is_external_vs_ifc.py <probe
//! output> <Revit's export> --list`.
//!
//! Revit's IFC exporter gives every element it writes an `IsExternal` by one
//! rule (`CategoryUtil.IsElementExternal` in revit-ifc, CLEANROOM.md source
//! 5), the first step that gives a value winning:
//!
//! 1. a Yes/No parameter named `IsExternal` on the element, then its type;
//! 2. the type's Function (`FUNCTION_PARAM`), external unless Interior (0);
//! 3. a roof is external;
//! 4. a family instance takes its host's value;
//! 5. otherwise the element is not external.
//!
//! The parameter of step 1 is looked up by the set being written:
//! revit-ifc's property set entry for `Pset_WallCommon.IsExternal` reads a
//! parameter of that name first and one named `IsExternal` after it
//! (`ExporterInitializer_PsetDef` and `PropertySetEntryMap` in revit-ifc).
//! A host's value is its own calculation, which reads only the plain name.
//!
//! The probe prints one JSON line per element of the partition decode: its
//! class, its type and Revit GlobalId, the Function its type's data objects
//! hold (the latest copy rvt-rs reads, RE-158 and #800, and every
//! partition's copy), a wall's or curtain wall's type Function from its
//! native record (`WallType.m_function`, RE-185), the host wall of a door or
//! window (#439) and the curtain wall of a mullion or panel (RE-46) with
//! their Functions, and the value of every project, shared or family
//! parameter named `IsExternal` or `<set>.IsExternal` on the element or its
//! type. Two first lines name those parameters and every parameter whose name
//! starts with `Pset_`. `tools/re/is_external_vs_ifc.py` joins the lines with
//! Revit's export and scores each step of the rule.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re193_is_external -- MODEL.rvt

use rvt::research::partition_room_parameters::{
    FUNCTION_PARAMETER, enclosing_data_object, find_parameter_definitions, scan_integer_parameter,
};
use rvt::research::partition_schema_mvp::{
    AGGREGATE_WHOLE_FIELD, OPENING_HOST_FIELD, TYPE_ID_FIELD, recover_partition_schema_mvp,
};
use rvt::research::partition_type_parameters::family_parameters_named;
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use rvt::{RevitFile, research::native_document};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const NAME: &str = "IsExternal";

fn id_field(element: &DecodedElement, wanted: &str) -> Option<u32> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. } if name == wanted => Some(*id),
        _ => None,
    })
}

/// A field of a native object, as JSON.
fn field<'a>(fields: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    fields.as_object()?.get(name)
}

/// `{"m_id":{"m_id64":n}}` as `n`.
fn element_id(value: &serde_json::Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")
        .and_then(serde_json::Value::as_i64)
        .and_then(|n| u64::try_from(n).ok())
}

/// Per wall its native type (`m_WallAttributesId`), and per wall type its
/// Function (`m_function`, RE-185).
#[derive(Default)]
struct WallTypes {
    wall_type: BTreeMap<u64, u64>,
    function: BTreeMap<u64, i64>,
}

fn wall_types(rf: &mut RevitFile, walls: &BTreeSet<u64>) -> anyhow::Result<WallTypes> {
    let mut out = WallTypes::default();
    let pass = |rf: &mut RevitFile, ids: BTreeSet<u64>, out: &mut WallTypes| {
        let options = native_document::Options {
            selected_ids: ids,
            ..native_document::Options::default()
        };
        native_document::extract_graphs(rf, &options, |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            for object in &graph.objects {
                let fields = serde_json::to_value(&object.fields)?;
                if let Some(t) = field(&fields, "m_WallAttributesId").and_then(element_id) {
                    out.wall_type.entry(id).or_insert(t);
                }
                if let Some(f) = field(&fields, "m_function").and_then(serde_json::Value::as_i64) {
                    out.function.entry(id).or_insert(f);
                }
            }
            Ok(())
        })
    };
    pass(rf, walls.clone(), &mut out)?;
    let types: BTreeSet<u64> = out.wall_type.values().copied().collect();
    pass(rf, types, &mut out)?;
    Ok(out)
}

/// The number of partition stream `stream` (`Partitions/398` is 398).
fn partition_number(stream: &str) -> u32 {
    stream
        .rsplit('/')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let elements =
        recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?.into_elements();
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let global_ids = rvt::research::revit_global_ids::revit_global_ids(&mut rf).unwrap_or_default();

    let types: BTreeMap<u32, u32> = elements
        .iter()
        .filter_map(|element| Some((element.id?, id_field(element, TYPE_ID_FIELD)?)))
        .collect();
    let classes: BTreeMap<u32, &str> = elements
        .iter()
        .filter_map(|element| Some((element.id?, element.class.as_str())))
        .collect();
    let mut owners: BTreeSet<u32> = elements.iter().filter_map(|element| element.id).collect();
    owners.extend(types.values().copied());

    // Every partition's copy of each owner's Function, and the family
    // parameters named IsExternal.
    let tag = FUNCTION_PARAMETER.to_le_bytes();
    let mut copies: BTreeMap<u32, Vec<(u32, u32)>> = BTreeMap::new();
    let mut family_named: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let partition = partition_number(&stream);
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &tag) {
            let Some(value) = buf
                .get(hit + 8..hit + 12)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
            else {
                continue;
            };
            let Some(owner) = enclosing_data_object(buf, hit).filter(|id| owners.contains(id))
            else {
                continue;
            };
            copies.entry(owner).or_default().push((partition, value));
        }
        family_named.extend(family_parameters_named(buf, &declared, NAME));
    }

    let definitions = find_parameter_definitions(&mut rf, version);
    let pset_names: Vec<&String> = definitions
        .keys()
        .filter(|name| name.starts_with("Pset_"))
        .collect();
    println!("{}", json!({ "pset_parameters": pset_names }));
    let mut parameters: BTreeMap<u32, String> = definitions
        .iter()
        .filter(|(name, _)| *name == NAME || name.ends_with(&format!(".{NAME}")))
        .map(|(name, &id)| (id, name.clone()))
        .collect();
    for id in family_named {
        parameters
            .entry(id)
            .or_insert_with(|| format!("{NAME} (family parameter)"));
    }
    println!("{}", json!({ "parameters": parameters }));

    let mut named: BTreeMap<u32, BTreeMap<u32, u32>> = BTreeMap::new();
    for &parameter in parameters.keys() {
        for (owner, value) in
            scan_integer_parameter(&mut rf, version, &owners, i64::from(parameter))
        {
            named.entry(owner).or_default().insert(parameter, value);
        }
    }
    let function = scan_integer_parameter(&mut rf, version, &owners, FUNCTION_PARAMETER);
    let walls: BTreeSet<u64> = elements
        .iter()
        .filter(|element| {
            matches!(
                element.class.as_str(),
                "Wall" | "CurtainWall" | "CurtainWallPanel"
            )
        })
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    let native = wall_types(&mut rf, &walls).unwrap_or_else(|error| {
        println!("{}", json!({ "native_error": error.to_string() }));
        WallTypes::default()
    });
    let wall_function = |id: u32| {
        native
            .wall_type
            .get(&u64::from(id))
            .and_then(|t| native.function.get(t))
            .copied()
    };

    let related = |owner: Option<u32>| {
        owner.map(|id| {
            let type_id = types.get(&id).copied();
            json!({
                "id": id,
                "class": classes.get(&id),
                "type": type_id,
                "function": type_id.and_then(|t| function.get(&t)),
                "wall_function": wall_function(id),
                "named": named.get(&id),
                "type_named": type_id.and_then(|t| named.get(&t)),
            })
        })
    };
    for element in &elements {
        let Some(id) = element.id else {
            continue;
        };
        let type_id = types.get(&id).copied();
        println!(
            "{}",
            json!({
                "id": id,
                "global_id": global_ids.get(&id),
                "class": element.class,
                "type": type_id,
                "function": type_id.and_then(|t| function.get(&t)),
                "function_copies": type_id.and_then(|t| copies.get(&t)),
                "own_function": function.get(&id),
                "wall_function": wall_function(id),
                "named": named.get(&id),
                "type_named": type_id.and_then(|t| named.get(&t)),
                "host": related(id_field(element, OPENING_HOST_FIELD)),
                "whole": related(id_field(element, AGGREGATE_WHOLE_FIELD)),
            })
        );
    }
    Ok(())
}
