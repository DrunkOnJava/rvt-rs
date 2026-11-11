//! #1507: why a window type's Fire Rating, held under a family parameter
//! named `Fire Rating`, does not reach the window on the R2025 models.
//!
//! FACT under test: on R2025 models 12 to 17 Revit's export writes
//! `Pset_WindowCommon.FireRating` `0.5h` on three windows and on their type,
//! and the text is a family type table entry under the family parameters
//! 12482 and 12220, both named `Fire Rating` (RE-185 section 3.3). rvt-rs
//! writes none of them. Three steps can lose it: the definitions are not
//! found by name (RE-93's reading takes a definition's name to be
//! `PARAMETER_NAME_OFFSET` bytes past a declared id), the entries under the
//! ids found are not attributed to the type, or the type's value is dropped
//! by RE-354's agreeing-copies guard. This prints each step.
//!
//! Output:
//!   release <version>
//!   definition <stream> <id> <name> offset <n> of <PARAMETER_NAME_OFFSET> header <id> declared <true|false>
//!       every definition RE-156's reading names, all ids kept, with the
//!       distance from the nearest `01 00 00 00 · u64 id` header before its
//!       name, that header's id, and whether the definition's id is declared
//!   named <stream> <id> declared <true|false>
//!       what `family_parameters_named` finds for `Fire Rating`
//!   entry <stream> <owner> <class> <parameter> <text>
//!       a table entry under one of those ids, before attribution
//!   type <id> <name> FireRating <text|->
//!       what `type_text_parameters` gives each window type
//!
//! On a model whose windows Revit rates `0.5h` the output must show a
//! `definition` line named `Fire Rating` at an offset other than
//! `PARAMETER_NAME_OFFSET`, or with `declared false`, if the name is what
//! fails; an `entry` line under that id in a window type's block if the
//! attribution is what fails; and `type <id> ... FireRating 0.5h` if nothing
//! fails here, which puts the loss after this reader.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1507_window_fire_rating_definitions -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::partition_room_parameters::{
    PARAMETER_GROUP_PREFIX, PARAMETER_NAME_WINDOW, enclosing_data_object,
};
use rvt::research::partition_schema_mvp::{
    TYPE_ID_FIELD, TYPE_NAME_FIELD, recover_partition_schema_mvp,
};
use rvt::research::partition_type_parameters::{
    FIRE_RATING_PARAMETER_NAME, PARAMETER_NAME_OFFSET, VALUE_BLOCK_MARK, family_parameters_named,
    type_text_parameters,
};
use rvt::walker::{InstanceField, WalkerLimits};

/// A table entry under a Fire Rating definition: the value block's owner and
/// the parameter id with the text before it.
type Entry = (u32, String);

/// `u32 n · UTF-16 × n` at `at`, 1 to `limit` code units, no control character.
fn utf16_entry(buf: &[u8], at: usize, limit: usize) -> Option<String> {
    let n = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=limit).contains(&n) {
        return None;
    }
    let units: Vec<u16> = buf
        .get(at + 4..at + 4 + 2 * n)?
        .chunks(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    (!text.chars().any(char::is_control)).then_some(text)
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

/// Every definition RE-156's reading names, keeping every id: two definitions
/// of one name stay apart (RE-185 section 3.3). Each with where its name is.
fn definitions(buf: &[u8]) -> Vec<(u32, String, usize)> {
    let needle: Vec<u8> = PARAMETER_GROUP_PREFIX
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut out = Vec::new();
    for hit in memchr::memmem::find_iter(buf, &needle) {
        let Some(group) = hit.checked_sub(4).and_then(|at| utf16_entry(buf, at, 256)) else {
            continue;
        };
        let after = hit + 2 * group.encode_utf16().count();
        let name_at = (after..after + PARAMETER_NAME_WINDOW)
            .find(|&at| utf16_entry(buf, at, 256).is_some_and(|n| n.encode_utf16().count() >= 2));
        let (Some(name_at), Some(id)) = (name_at, enclosing_data_object(buf, hit)) else {
            continue;
        };
        let Some(name) = utf16_entry(buf, name_at, 256) else {
            continue;
        };
        out.push((id, name, name_at));
    }
    out
}

/// How far `at` lies past the nearest `01 00 00 00 · u64 id` header before it,
/// with that header's id.
fn header_before(buf: &[u8], at: usize) -> Option<(usize, u64)> {
    (1..=0x100).find_map(|back| {
        let id_at = at.checked_sub(back)?;
        if buf.get(id_at.checked_sub(4)?..id_at)? != [1, 0, 0, 0] {
            return None;
        }
        Some((back, u64_at(buf, id_at)?))
    })
}

/// Every value block mark in `buf` whose owner is declared (RE-77).
fn blocks(buf: &[u8], declared: &BTreeSet<u32>) -> Vec<(usize, u32)> {
    memchr::memmem::find_iter(buf, &VALUE_BLOCK_MARK)
        .filter(|&at| at >= 8 && buf[at - 1] != 0xff)
        .filter_map(|at| {
            let id = u32::try_from(u64_at(buf, at - 8)?).ok()?;
            declared.contains(&id).then_some((at, id))
        })
        .collect()
}

/// The text of the table entry whose `-1` element id is at `at` (RE-354).
fn table_text_before(buf: &[u8], at: usize) -> Option<String> {
    let flag_ends = at.checked_sub(8)?;
    let text_ends = if u32_at(buf, flag_ends.checked_sub(4)?)? == 0 {
        flag_ends - 4
    } else if buf.get(flag_ends.checked_sub(6)?..flag_ends.checked_sub(2)?)? == [0xff; 4] {
        flag_ends - 6
    } else {
        return None;
    };
    (1..=60).find_map(|units| {
        let start = text_ends.checked_sub(4 + 2 * units)?;
        if u32_at(buf, start)? as usize != units {
            return None;
        }
        utf16_entry(buf, start, 60)
    })
}

/// The table entries under `parameter` in `buf`, by the value block they lie
/// in (RE-354).
fn table_entries(buf: &[u8], parameter: i64, blocks: &[(usize, u32)]) -> Vec<(u32, String)> {
    let mut mark = [0xffu8; 16];
    mark[8..].copy_from_slice(&parameter.to_le_bytes());
    let mut out = Vec::new();
    for at in memchr::memmem::find_iter(buf, &mark) {
        let index = blocks.partition_point(|(start, _)| *start < at);
        let Some(&(_, owner)) = index.checked_sub(1).and_then(|i| blocks.get(i)) else {
            continue;
        };
        if let Some(text) = table_text_before(buf, at) {
            out.push((owner, text));
        }
    }
    out
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("usage: probe_1507_window_fire_rating_definitions MODEL.rvt")
    })?;
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("release {version}");
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&records);
    let mut fire_ids: BTreeSet<i64> = BTreeSet::new();
    let mut owners: BTreeSet<u64> = BTreeSet::new();
    let mut per_stream: Vec<(String, Vec<Entry>)> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (id, name, name_at) in definitions(buf) {
            if name != FIRE_RATING_PARAMETER_NAME {
                continue;
            }
            let (offset, header) = header_before(buf, name_at).unwrap_or((0, 0));
            println!(
                "definition {stream} {id} {name} offset {offset} of {PARAMETER_NAME_OFFSET} \
                 header {header} declared {}",
                declared.contains(&id)
            );
            fire_ids.insert(i64::from(id));
        }
        for id in family_parameters_named(buf, &declared, FIRE_RATING_PARAMETER_NAME) {
            println!("named {stream} {id} declared {}", declared.contains(&id));
            fire_ids.insert(i64::from(id));
        }
        let blocks = blocks(buf, &declared);
        let mut entries: Vec<Entry> = Vec::new();
        for &parameter in &fire_ids {
            for (owner, text) in table_entries(buf, parameter, &blocks) {
                owners.insert(u64::from(owner));
                entries.push((owner, format!("{parameter} {text}")));
            }
        }
        if !entries.is_empty() {
            per_stream.push((stream, entries));
        }
    }
    let mut class_of: BTreeMap<u32, String> = BTreeMap::new();
    if !owners.is_empty() {
        let options = Options {
            selected_ids: owners,
            graph_classes: Some(BTreeSet::new()),
            releases: NATIVE_RELEASES,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            if let (Some(class), Ok(id)) = (
                record.class_name.clone(),
                u32::try_from(record.identity.element_id),
            ) {
                class_of.insert(id, class);
            }
            Ok(())
        })?;
    }
    for (stream, entries) in &per_stream {
        for (owner, rest) in entries {
            let class = class_of.get(owner).map_or("?", String::as_str);
            println!("entry {stream} {owner} {class} {rest}");
        }
    }
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut types: BTreeMap<u32, String> = BTreeMap::new();
    for window in &mvp.windows {
        let (mut id, mut name) = (None, None);
        for (field, value) in &window.fields {
            match (field.as_str(), value) {
                (TYPE_ID_FIELD, InstanceField::ElementId { id: type_id, .. }) => {
                    id = Some(*type_id)
                }
                (TYPE_NAME_FIELD, InstanceField::String(text)) => name = Some(text.clone()),
                _ => {}
            }
        }
        if let Some(id) = id {
            types.insert(id, name.unwrap_or_default());
        }
    }
    let texts = type_text_parameters(&mut rf, version);
    for (id, name) in &types {
        let rating = texts
            .get(id)
            .and_then(|parameters| parameters.get(FIRE_RATING_PARAMETER_NAME))
            .map_or("-", String::as_str);
        println!("type {id} {name} FireRating {rating}");
    }
    Ok(())
}
