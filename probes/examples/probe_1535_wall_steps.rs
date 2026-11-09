//! #1535 (RE-364): whether a Revit 2015 to 2023 wall's native record holds
//! the steps the same wall's record holds in the project's 2024 save.
//!
//! FACT under test: `native_walls_and_top_joins` builds `uncut` and `whole`
//! from the `*Step` classes of each wall's record (RE-266, RE-270). RE-364
//! turned both off on 2015 to 2023 because a record's control joins
//! (`joins`, `side_joins`, `exposed`) read empty there, so the wall
//! quantities those flags allow are not written. The steps are a different
//! part of the graph: if every wall holds the same step classes in a 2015 to
//! 2023 save as in the same project's 2024 save, the two flags read there as
//! they do on 2024.
//!
//! Output, one wall per line, then one summary line:
//!   wall <id> <class> <status> objects=<n> consumed=<bytes>/<body bytes> uncut=<0|1> whole=<0|1> steps=<Class:count,...|->
//!   walls <n> records <n>
//!
//! `uncut` and `whole` are what `native_walls_and_top_joins` gives the same
//! walls. Compare a save with the same project's 2024 save by ElementId
//! (`rac_basic` with its 2023 save, as RE-364 does).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1535_wall_steps -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_walls::native_walls_and_top_joins;

const WALLS: [&str; 2] = ["SWall", "ArcWall"];

struct Wall {
    class: String,
    status: String,
    objects: usize,
    consumed: usize,
    body: usize,
    steps: BTreeMap<String, usize>,
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: WALLS.iter().map(|name| (*name).to_string()).collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut walls: BTreeMap<u64, Wall> = BTreeMap::new();
    let mut records = 0usize;
    extract_graphs(&mut rf, &options, |record| {
        records += 1;
        let Some(class) = record.class_name.clone() else {
            return Ok(());
        };
        if !WALLS.contains(&class.as_str()) {
            return Ok(());
        }
        let mut steps: BTreeMap<String, usize> = BTreeMap::new();
        let (objects, consumed) = match record.graph.as_ref() {
            Some(graph) => {
                for object in &graph.objects {
                    if object.class_name.ends_with("Step") {
                        *steps.entry(object.class_name.clone()).or_insert(0) += 1;
                    }
                }
                (graph.objects.len(), graph.consumed_bytes)
            }
            None => (0, 0),
        };
        walls.insert(
            record.identity.element_id,
            Wall {
                class,
                status: record.status.clone(),
                objects,
                consumed,
                body: record.source.body_bytes,
                steps,
            },
        );
        Ok(())
    })?;
    let ids: BTreeSet<u64> = walls.keys().copied().collect();
    let flags = native_walls_and_top_joins(&mut rf, &ids)?;
    for (id, wall) in &walls {
        let steps = if wall.steps.is_empty() {
            "-".to_string()
        } else {
            wall.steps
                .iter()
                .map(|(class, count)| format!("{class}:{count}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "wall {id} {} {} objects={} consumed={}/{} uncut={} whole={} steps={steps}",
            wall.class,
            wall.status,
            wall.objects,
            wall.consumed,
            wall.body,
            u8::from(flags.uncut.contains(id)),
            u8::from(flags.whole.contains(id)),
        );
    }
    println!("walls {} records {records}", walls.len());
    Ok(())
}
