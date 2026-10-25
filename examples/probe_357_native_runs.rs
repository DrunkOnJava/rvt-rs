//! #357: which Revit 2025 stair runs `native_stairs::native_runs` reads, and
//! for every run it leaves out, which of its checks the run fails.
//!
//! FACT under test: a straight run's native records give its sketch (width,
//! base, one tread depth, one riser height, one climb direction, treads on
//! the centre line) and a unique `StairsRunType` it references by element
//! id, and `partition_stairs::run_side_profile` draws that sketch and type.
//!
//! Output, per `StairsRun` record:
//!   run <id> treads <n> refs <member=id ...> types <id ...>
//!   check <id> <name> <value>
//!   native <id> read|absent [profile <points>|no profile]
//!   typeref <run> <object index> <class> <path> <type id>
//!   stairs <id> <class> refs <member=id ...>
//!   record <id> refs <ids> own-type <id|none>
//!
//! A `typeref` line is every place in the run's whole graph that holds a
//! `StairsRunType`'s id; a `stairs` line is the record the run's
//! `m_stairsId` names, and each `StairsType`, with their top-level ids.
//! A `record` line is the run's element record's reference list (RE-28) and
//! the one `OST_StairsRuns` type definition it names, the run's own type,
//! which `native_runs` is given; `tools/re/run_types_vs_ifc.py` follows the
//! `run` and `stairs` lines to the stairs' default run type, so a run whose
//! own type is not that default shows as two different ids.
//!
//! Usage:
//!   cargo run --profile ci --example probe_357_native_runs -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_stairs::native_runs;
use rvt::research::partition_element_records::{
    OST_STAIRS_RUNS, REFERENCE_LIST_OFFSET, decode_reference_list,
};
use rvt::research::partition_stairs::run_side_profile;
use rvt::research::partition_type_records::{
    scan_type_records, type_definition_ids, unique_type_reference,
};
use rvt::walker::{InstanceField, iter_elements};
use serde_json::Value;

fn ids(fields: &Value) -> Vec<(String, i64)> {
    fields
        .as_object()
        .into_iter()
        .flat_map(|object| object.iter())
        .filter_map(|(name, v)| Some((name.clone(), v.get("m_id")?.get("m_id64")?.as_i64()?)))
        .collect()
}

fn find_ids(value: &Value, wanted: &BTreeSet<u64>, path: String, out: &mut Vec<(String, u64)>) {
    match value {
        Value::Object(members) => {
            if let Some(id) = members
                .get("m_id64")
                .and_then(Value::as_u64)
                .filter(|id| wanted.contains(id))
            {
                out.push((path.clone(), id));
            }
            for (name, member) in members {
                find_ids(member, wanted, format!("{path}.{name}"), out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                find_ids(item, wanted, format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: ["StairsRun".to_string(), "StairsRunType".to_string()].into(),
        ..Options::default()
    };
    let mut types: BTreeSet<u64> = BTreeSet::new();
    let mut runs: BTreeMap<u64, (Value, Vec<Value>)> = BTreeMap::new();
    let mut graphs: BTreeMap<u64, Vec<(String, Value)>> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match class {
            "StairsRunType" => {
                types.insert(id);
                for name in [
                    "m_treadThickness",
                    "m_riserThickness",
                    "m_nosingLength",
                    "m_structuralDepth",
                    "m_hasMonolithicSupport",
                    "m_hasTread",
                    "m_hasRiser",
                    "m_riserIsSlanted",
                    "m_riserTreadConnection",
                ] {
                    println!(
                        "type {id} {name} {}",
                        root.fields.get(name).unwrap_or(&Value::Null)
                    );
                }
            }
            "StairsRun" => {
                let treads = graph
                    .objects
                    .iter()
                    .filter(|o| o.class_name == "StairsTreadInfo")
                    .map(|o| o.fields.clone())
                    .collect();
                runs.insert(id, (root.fields.clone(), treads));
                graphs.insert(
                    id,
                    graph
                        .objects
                        .iter()
                        .map(|o| (o.class_name.clone(), o.fields.clone()))
                        .collect(),
                );
            }
            _ => {}
        }
        Ok(())
    })?;
    for (id, (run, treads)) in &runs {
        let refs = ids(run);
        let typed: Vec<_> = refs
            .iter()
            .filter(|(_, t)| u64::try_from(*t).is_ok_and(|t| types.contains(&t)))
            .collect();
        println!(
            "run {id} treads {} refs {} types {:?}",
            treads.len(),
            refs.iter()
                .map(|(n, t)| format!("{n}={t}"))
                .collect::<Vec<_>>()
                .join(" "),
            typed
        );
        for name in [
            "m_beginWithRiser",
            "m_endWithRiser",
            "m_actualRunWidth",
            "m_bottomElevation",
            "m_extendBelowBase",
            "m_extendBelowTreadBase",
            "m_baseRiserIndex",
        ] {
            println!(
                "check {id} {name} {}",
                run.get(name).unwrap_or(&Value::Null)
            );
        }
        for (k, tread) in treads.iter().enumerate() {
            let get = |name: &str| tread.get(name).cloned().unwrap_or(Value::Null);
            println!(
                "check {id} tread {k} idx {} runType {} depth {} origin {} dir {} riserDir {}",
                tread
                    .get("m_startRiser")
                    .and_then(|r| r.get("m_specifiedIdx"))
                    .unwrap_or(&Value::Null),
                get("m_runType"),
                get("m_actDepth"),
                get("m_origin"),
                get("m_runDirProj"),
                get("m_riserDir"),
            );
        }
    }
    for (id, objects) in &graphs {
        for (index, (class, fields)) in objects.iter().enumerate() {
            let mut found = Vec::new();
            find_ids(fields, &types, String::new(), &mut found);
            for (at, type_id) in found {
                println!("typeref {id} {index} {class} {at} {type_id}");
            }
        }
    }
    let stairs: BTreeSet<u64> = runs
        .values()
        .filter_map(|(run, _)| run.get("m_stairsId")?.get("m_id")?.get("m_id64")?.as_u64())
        .collect();
    let options = Options {
        selected_ids: stairs,
        selected_classes: ["StairsType".to_string()].into(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        println!(
            "stairs {} {class} refs {}",
            record.identity.element_id,
            ids(&root.fields)
                .iter()
                .filter(|(_, t)| *t >= 0)
                .map(|(n, t)| format!("{n}={t}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        Ok(())
    })?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let run_types = type_definition_ids(&scan_type_records(
        &mut rf,
        version,
        OST_STAIRS_RUNS,
        &declared,
    )?);
    println!("run-type-definitions {run_types:?}");
    let mut own_types: BTreeMap<u64, u64> = BTreeMap::new();
    let elements: Vec<_> = iter_elements(&mut rf)?
        .filter(|element| element.class == "StairsRun")
        .collect();
    for element in elements {
        let Some(id) = element.id else { continue };
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
        let references = match (stream, offset) {
            (Some(stream), Some(offset)) => rf
                .inflated_partition(&stream)
                .ok()
                .and_then(|inflated| {
                    decode_reference_list(inflated.bytes(), offset + REFERENCE_LIST_OFFSET)
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let own = unique_type_reference(&references, &run_types);
        println!("record {id} refs {references:?} own-type {own:?}");
        if let Some(own) = own {
            own_types.insert(u64::from(id), u64::from(own));
        }
    }
    let native = native_runs(&mut rf, &own_types)?;
    for id in runs.keys() {
        match native.get(id) {
            Some(run) => {
                let profile = run_side_profile(
                    &run.sketch,
                    &run.run_type,
                    run.riser_height_feet,
                    run.end_with_riser,
                );
                match profile {
                    Some(points) => println!("native {id} read profile {}", points.len()),
                    None => println!("native {id} read no profile {:?}", run.run_type),
                }
            }
            None => println!("native {id} absent"),
        }
    }
    Ok(())
}
