//! #824: the curve loops a Revit 2024 stair landing's native record holds,
//! against Revit's exported outline of the landing.
//!
//! FACT under test: the BIM4LCA office's non-monolithic landing 3058084,
//! whose outline in Revit's export has an arc where it meets the spiral run,
//! holds that outline as a `CurveLoop` of `GLine`s and a `GArc` in its
//! `StairsLanding` record, the arc on the spiral's inner circle (radius
//! 0.259402 ft).
//!
//! Output, per landing in `RVT_PROBE_OWNERS` (default 3058084):
//!   owner <id>
//!   loop <id> <object index> open <bool> curves <n>
//!   curve <id> <loop index> GLine <start x y z> <end x y z>
//!   curve <id> <loop index> <class> <fields as JSON>
//!   type <id> <object index> <class> <member=value ...>
//!     the scalar members of each record in `RVT_PROBE_TYPES` (default
//!     141731, 3058084's landing type, and 3051881, its stair's spiral run
//!     type), for where Revit's 0.0984 ft landing thickness comes from
//!
//! `tools/re/slab_outlines_list.py` prints Revit's outline of each owner.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_824_landing_loops -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::pointer;
use serde_json::Value;

fn point(value: Option<&Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    let [x, y, z] = items.as_slice() else {
        return None;
    };
    Some([x.as_f64()?, y.as_f64()?, z.as_f64()?])
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let wanted: BTreeSet<u64> = std::env::var("RVT_PROBE_OWNERS")
        .unwrap_or_else(|_| "3058084".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: wanted.clone(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if !wanted.contains(&id) {
            return Ok(());
        }
        println!("owner {id}");
        for (index, object) in graph.objects.iter().enumerate() {
            if object.class_name != "CurveLoop" {
                continue;
            }
            let curves = object
                .fields
                .get("m_curves")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            println!(
                "loop {id} {index} open {} curves {}",
                object.fields.get("m_open").unwrap_or(&Value::Null),
                curves.len()
            );
            for curve in &curves {
                let Some(target) = pointer(graph, index, curve).ok().flatten() else {
                    println!("curve {id} {index} unresolved {curve}");
                    continue;
                };
                let Some(found) = graph.objects.get(target) else {
                    continue;
                };
                let line = (found.class_name == "GLine")
                    .then(|| {
                        let origin = point(found.fields.get("m_origin"))?;
                        let direction = point(found.fields.get("m_dirVec"))?;
                        let params = found.fields.get("m_endParams")?.as_array()?;
                        let [a, b] = params.as_slice() else {
                            return None;
                        };
                        let at = |t: f64| [0, 1, 2].map(|i| origin[i] + direction[i] * t);
                        Some((at(a.as_f64()?), at(b.as_f64()?)))
                    })
                    .flatten();
                match line {
                    Some((a, b)) => println!(
                        "curve {id} {index} GLine {:.6} {:.6} {:.6} {:.6} {:.6} {:.6}",
                        a[0], a[1], a[2], b[0], b[1], b[2]
                    ),
                    None => println!(
                        "curve {id} {index} {} {}",
                        found.class_name,
                        serde_json::to_string(&found.fields).unwrap_or_default()
                    ),
                }
            }
        }
        Ok(())
    })?;
    let types: BTreeSet<u64> = std::env::var("RVT_PROBE_TYPES")
        .unwrap_or_else(|_| "141731,3051881".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let options = Options {
        selected_ids: types,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate() {
            let members: Vec<String> = object
                .fields
                .as_object()
                .into_iter()
                .flat_map(|fields| fields.iter())
                .filter(|(_, value)| value.is_number() || value.is_boolean() || value.is_string())
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            println!(
                "type {id} {index} {class}/{} {}",
                object.class_name,
                members.join(" ")
            );
        }
        Ok(())
    })?;
    Ok(())
}
