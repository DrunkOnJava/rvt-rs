//! #1584 (RE-364 section 6): whether a Revit 2015 to 2023 wall's native record
//! holds its control joins, and what it holds instead of them.
//!
//! FACT under test: a wall's record on 2015 to 2023 reads no `joins`,
//! `side_joins` or `exposed` (RE-364 section 6), so `Length`, the side areas,
//! the footprint and the volumes are left out there. The pieces the joins are
//! read from are `VWallDriver.m_controlJoinsSet` (per end, the walls joined
//! there and their ends), the `CDJoinInfo` objects (a wall's side paired with
//! the ends of the walls that meet it) and the `JoinEndGStep` objects (an end's
//! omitted target and its exposed faces). This prints, for each wall, which of
//! them the decoded graph holds, so that an empty read can be told from an
//! absent field and from a field whose identifiers do not decode, and a save can
//! be held to the same project's later save by ElementId.
//!
//! Output, one wall per line, then one summary line:
//!   wall <id> <class> <status> objects=<n> driver=<0|1> control=<absent|empty|<sets>> control_ids=<n> \
//!        cdjoin=<objects>/<entries> endsteps=<n> exposed=<n> sets=<[index,[[id,end],...]],...|->
//!   walls <n> records <n> with_driver <n> control_absent <n> control_empty <n> control_read <n>
//!
//! `control` is `absent` where the wall has no `VWallDriver` or its
//! `m_controlJoinsSet` is not a field of it, `empty` where the field is an empty
//! array, and the number of sets otherwise. `control_ids` counts the entries
//! whose element identifier decoded (as `{"m_id":{"m_id64":n}}` on 2024 and
//! later, `{"m_id":{"m_id":n}}` before, RE-282). `cdjoin` is the number of
//! `CDJoinInfo` objects and the number of entries in their `m_info`; `endsteps`
//! the `JoinEndGStep` objects and `exposed` the entries in their
//! `m_exposedFaceMap`. Counts and ElementIds only.
//!
//! Compare a 2015 to 2022 save with the same project's later save by ElementId
//! (`rac_advanced` 2015 to 2023 with its 2024 save, `rac_basic` 2015 to 2022
//! with its 2023 save, `rst_basic` 2016 to 2023 with its 2024 save, RE-364).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1584_wall_control_joins -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};

const WALLS: [&str; 2] = ["SWall", "ArcWall"];

/// An element identifier, `{"m_id":{"m_id64":n}}` or `{"m_id":{"m_id":n}}`.
fn id_of(value: &Fv) -> Option<i64> {
    value
        .pointer("/m_id/m_id64")
        .or_else(|| value.pointer("/m_id/m_id"))
        .and_then(Fv::as_i64)
}

#[derive(Default)]
struct Wall {
    class: String,
    status: String,
    objects: usize,
    driver: bool,
    control: Option<usize>,
    control_ids: usize,
    cdjoin_objects: usize,
    cdjoin_entries: usize,
    endsteps: usize,
    exposed: usize,
    sets: Vec<String>,
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
        let mut wall = Wall {
            class,
            status: record.status.clone(),
            ..Wall::default()
        };
        if let Some(graph) = record.graph.as_ref() {
            wall.objects = graph.objects.len();
            for object in &graph.objects {
                match object.class_name.as_str() {
                    "VWallDriver" => {
                        wall.driver = true;
                        let Some(field) = object.fields.get("m_controlJoinsSet") else {
                            continue;
                        };
                        let sets = field.as_array().unwrap_or(&[]);
                        wall.control = Some(sets.len());
                        for set in sets {
                            let index = set.get("m_index").and_then(Fv::as_i64);
                            let mut entry = String::new();
                            for item in set
                                .pointer("/m_JoinInfo/m_info")
                                .and_then(Fv::as_array)
                                .into_iter()
                                .flatten()
                            {
                                let other = item.get("m_elemId").and_then(id_of);
                                wall.control_ids += usize::from(other.is_some());
                                let end = item.get("m_joinedEnd").and_then(Fv::as_i64);
                                let show = |v: Option<i64>| {
                                    v.map_or("null".to_string(), |v| v.to_string())
                                };
                                if !entry.is_empty() {
                                    entry.push(',');
                                }
                                let _ = write!(entry, "[{},{}]", show(other), show(end));
                            }
                            wall.sets.push(format!(
                                "[{},[{entry}]]",
                                index.map_or("null".to_string(), |v| v.to_string())
                            ));
                        }
                    }
                    "CDJoinInfo" => {
                        wall.cdjoin_objects += 1;
                        wall.cdjoin_entries += object
                            .fields
                            .get("m_info")
                            .and_then(Fv::as_array)
                            .map_or(0, <[Fv]>::len);
                    }
                    "JoinEndGStep" => {
                        wall.endsteps += 1;
                        wall.exposed += object
                            .fields
                            .get("m_exposedFaceMap")
                            .and_then(Fv::as_array)
                            .map_or(0, <[Fv]>::len);
                    }
                    _ => {}
                }
            }
        }
        walls.insert(record.identity.element_id, wall);
        Ok(())
    })?;
    let ids: BTreeSet<u64> = walls.keys().copied().collect();
    let (mut with_driver, mut absent, mut empty, mut read) = (0, 0, 0, 0);
    for (id, wall) in &walls {
        with_driver += usize::from(wall.driver);
        let control = match wall.control {
            None => {
                absent += 1;
                "absent".to_string()
            }
            Some(0) => {
                empty += 1;
                "empty".to_string()
            }
            Some(sets) => {
                read += 1;
                sets.to_string()
            }
        };
        let sets = if wall.sets.is_empty() {
            "-".to_string()
        } else {
            wall.sets.join(",")
        };
        println!(
            "wall {id} {} {} objects={} driver={} control={control} control_ids={} cdjoin={}/{} endsteps={} exposed={} sets={sets}",
            wall.class,
            wall.status,
            wall.objects,
            u8::from(wall.driver),
            wall.control_ids,
            wall.cdjoin_objects,
            wall.cdjoin_entries,
            wall.endsteps,
            wall.exposed,
        );
    }
    println!(
        "walls {} records {records} with_driver {with_driver} control_absent {absent} control_empty {empty} control_read {read}",
        ids.len()
    );
    Ok(())
}
