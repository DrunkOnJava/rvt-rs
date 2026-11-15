//! #1589: where a Level keeps the height a room's area is taken at.
//!
//! FACT to find: Revit takes a room's area from a horizontal section of its
//! solid at a height above its Level. RE-374 measured one of Snowdon Towers'
//! stepped rooms at 4.0 ft above its floor, and another matches only at about
//! 6 ft. Revit's Level has a Computation Height for this. The hypothesis is
//! that each Level's native record or saved parameters hold it, and that the
//! levels of those rooms differ in it.
//!
//! Output, one record per line:
//!   level <id> <class> object <n> <object class> <fields json>
//!   room <id> <class> object <n> <object class> <fields json>
//!   param <id> <parameter id> <value json> <source element id> <name>
//!
//! A `room` line is each object of the records of [`ROOMS`], RE-374's four
//! stepped rooms of Snowdon Towers Architectural, for their Level and base.
//!
//! Every object of every `Level` record is printed, each with its own fields: a
//! history table as its length under `<name>#`, a pointer member left out. Then
//! come the saved parameters of each Level, with their names where the file's
//! definitions give one.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1589_level_fields -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_element_parameters;
use serde_json::{Map, Value};

const CLIP: usize = 8000;

/// RE-374's stepped rooms of Snowdon Towers Architectural (#1589).
const ROOMS: [u64; 4] = [828537, 828548, 2181757, 2181762];

fn trimmed(fields: &Value) -> String {
    let text = match fields.as_object() {
        Some(members) => {
            let mut kept = Map::new();
            for (name, member) in members {
                if name.ends_with("HistTable") {
                    let count = member.as_array().map_or(0, Vec::len);
                    kept.insert(format!("{name}#"), Value::from(count));
                } else if member.get("pointer_token").is_none() {
                    kept.insert(name.clone(), member.clone());
                }
            }
            Value::Object(kept).to_string()
        }
        None => fields.to_string(),
    };
    text.chars().take(CLIP).collect()
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: BTreeSet::from(["Level".to_string()]),
        releases: rvt::research::native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    let mut levels: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        levels.insert(id);
        for (index, object) in graph.objects.iter().enumerate() {
            println!(
                "level {id} {class} object {index} {} {}",
                object.class_name,
                trimmed(&object.fields.to_json())
            );
        }
        Ok(())
    })?;
    let rooms = Options {
        selected_ids: ROOMS.into_iter().collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &rooms, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate().take(4) {
            println!(
                "room {id} {class} object {index} {} {}",
                object.class_name,
                trimmed(&object.fields.to_json())
            );
        }
        Ok(())
    })?;
    let mut with_rooms = levels.clone();
    with_rooms.extend(ROOMS);
    let saved = native_element_parameters::read(&mut rf, &with_rooms)?;
    for (id, parameters) in &saved.elements {
        for parameter in parameters {
            println!(
                "param {id} {} {} {} {:?}",
                parameter.parameter_id,
                parameter.value,
                parameter.source_element_id,
                parameter.name.as_deref().unwrap_or("-"),
            );
        }
    }
    eprintln!("levels {}", levels.len());
    Ok(())
}
