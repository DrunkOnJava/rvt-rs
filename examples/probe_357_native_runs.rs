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
//!
//! Usage:
//!   cargo run --profile ci --example probe_357_native_runs -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_stairs::native_runs;
use rvt::research::partition_stairs::run_side_profile;
use serde_json::Value;

fn ids(fields: &Value) -> Vec<(String, i64)> {
    fields
        .as_object()
        .into_iter()
        .flat_map(|object| object.iter())
        .filter_map(|(name, v)| Some((name.clone(), v.get("m_id")?.get("m_id64")?.as_i64()?)))
        .collect()
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
    let native = native_runs(&mut rf, &runs.keys().copied().collect())?;
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
