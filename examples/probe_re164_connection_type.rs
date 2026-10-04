//! RE-164 (probe): where a pipe's ConnectionType `Generic` comes from (B47,
//! #35).
//!
//! RE-159: Revit's exports give each RE1 pipe `ConnectionType` `Generic`, a
//! text that no pipe's or pipe type's own data object holds. If it is the
//! name of an element (a pipe connection type), then a pipe, its type, or an
//! object between them holds that element's ElementId.
//!
//! Per file this prints, as one JSON line:
//!
//! 1. every element named `Generic`: by its name entry (RE-35, with its
//!    BuiltInCategory), and every data object (RE-153) whose payload holds the
//!    string `u32 7 · "Generic"` in UTF-16, by the object's ElementId;
//! 2. for each named id, and the first 40 objects holding the text, the data
//!    objects that hold it as a `u32` anywhere in their payload, by holder:
//!    its name entry and BuiltInCategory, and how many hits, up to 30 holders.
//!
//! Revit 2024 and later (data objects).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re164_connection_type -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::partition_room_parameters::{DATA_OBJECT_HEADER, data_objects};
use std::collections::{BTreeMap, BTreeSet};

const NAME: &str = "Generic";
const MAX_HOLDERS: usize = 30;
/// Objects holding the text whose holders are followed, after the named ones.
const MAX_TEXT_CANDIDATES: usize = 40;

fn json_string(s: &str) -> String {
    format!("{s:?}")
}

fn probe(path: &str) -> anyhow::Result<String> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    anyhow::ensure!(
        revit >= 2024,
        "data objects are read on Revit 2024 and later"
    );
    let names = rf.element_names();
    let named: BTreeMap<u32, i64> = names
        .entries
        .values()
        .filter(|entry| entry.name == NAME)
        .map(|entry| (entry.element_id, entry.builtin_category))
        .collect();

    // `u32 7` then "Generic" in UTF-16.
    let mut needle = (NAME.encode_utf16().count() as u32).to_le_bytes().to_vec();
    for unit in NAME.encode_utf16() {
        needle.extend(unit.to_le_bytes());
    }
    let streams = rf.partition_stream_names();
    // One pass per stream over its data objects (B70): which hold the text.
    let mut holding_text: BTreeSet<u32> = BTreeSet::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if memchr::memmem::find(&buf[p + DATA_OBJECT_HEADER..object.end], &needle).is_some() {
                holding_text.insert(object.element_id);
            }
        }
    }

    let candidates: BTreeSet<u32> = named
        .keys()
        .copied()
        .chain(holding_text.iter().copied().take(MAX_TEXT_CANDIDATES))
        .collect();
    let describe = |id: u32| -> String {
        match names.entries.get(&id) {
            Some(entry) => format!(
                "{{\"id\":{id},\"name\":{},\"category\":{}}}",
                json_string(&entry.name),
                entry.builtin_category
            ),
            None => format!("{{\"id\":{id},\"name\":null}}"),
        }
    };
    // candidate -> holder -> hits, and candidate -> its own objects.
    let mut held: BTreeMap<u32, BTreeMap<u32, usize>> = BTreeMap::new();
    let mut own: BTreeMap<u32, usize> = BTreeMap::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if candidates.contains(&object.element_id) {
                *own.entry(object.element_id).or_default() += 1;
            }
            for word in buf[p + DATA_OBJECT_HEADER..object.end].windows(4) {
                let id = u32::from_le_bytes(word.try_into().expect("4 bytes"));
                if id != object.element_id && candidates.contains(&id) {
                    *held
                        .entry(id)
                        .or_default()
                        .entry(object.element_id)
                        .or_default() += 1;
                }
            }
        }
    }
    let mut out = Vec::new();
    for &candidate in &candidates {
        let holders = held.get(&candidate).cloned().unwrap_or_default();
        let own_objects = own.get(&candidate).copied().unwrap_or(0);
        let listed: Vec<String> = holders
            .iter()
            .take(MAX_HOLDERS)
            .map(|(holder, count)| format!("{{\"holder\":{},\"hits\":{count}}}", describe(*holder)))
            .collect();
        out.push(format!(
            "{{\"candidate\":{},\"by_name_entry\":{},\"holds_text\":{},\"own_objects\":{own_objects},\"holders\":{},\"holders_listed\":[{}]}}",
            describe(candidate),
            named.contains_key(&candidate),
            holding_text.contains(&candidate),
            holders.len(),
            listed.join(",")
        ));
    }
    Ok(format!(
        "{{\"file\":{},\"revit\":{revit},\"named\":{},\"objects_holding_text\":{},\"candidates\":[{}]}}",
        json_string(&file),
        named.len(),
        holding_text.len(),
        out.join(",")
    ))
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re164_connection_type MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in paths.iter().filter(|p| !p.starts_with("--")) {
        match probe(path) {
            Ok(line) => println!("{line}"),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
