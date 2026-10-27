//! RE-253 (#1048): how each wall's element record names its type, as counts
//! only, so it can run on a model whose data may not leave the runner.
//!
//! FACT under test: a wall's type is the one type-definition record of the
//! Walls category its reference list names (RE-28,
//! `partition_type_records::unique_type_reference`). On a held-out Revit
//! 2025 model one wall gets its sibling type's name and width; hypothesis:
//! its list names a second type that the type-record scan does not take for
//! a definition, so the join finds only one.
//!
//! Output, counts only: the walls; how many type definitions each one's
//! reference list names (0, 1, 2 or more); how many name a Walls type record
//! that is not a definition, and how many name an id with element-data
//! names that is no Walls type record; how many have a type picked that
//! is not the one definition their list names; and how many walls' native
//! `SWall` record, the current copy (RE-216), names another type in
//! `m_WallAttributesId` than the one picked, and how many of those types
//! are type definitions.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re253_wall_type_refs -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::{partition_element_records as per, partition_names, partition_type_records};
use rvt::walker::InstanceField;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        partition_type_records::scan_type_records(&mut rf, version, per::OST_WALLS, &declared)?;
    let definitions = partition_type_records::type_definition_ids(&records);
    let type_records: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    let mut lists: Vec<(Vec<u32>, Option<u32>)> = Vec::new();
    let mut picks: BTreeMap<u64, u32> = BTreeMap::new();
    for wall in &mvp.walls {
        let mut stream = None;
        let mut offset = None;
        let mut picked = None;
        for (name, value) in &wall.fields {
            match (name.as_str(), value) {
                ("m_source_stream", InstanceField::String(v)) => stream = Some(v.clone()),
                ("m_source_offset", InstanceField::Integer { value, .. }) => {
                    offset = usize::try_from(*value).ok();
                }
                (n, InstanceField::ElementId { id, .. })
                    if n == rvt::research::partition_schema_mvp::TYPE_ID_FIELD =>
                {
                    picked = Some(*id);
                }
                _ => {}
            }
        }
        let (Some(stream), Some(offset)) = (stream, offset) else {
            continue;
        };
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let references = offset
            .checked_add(per::REFERENCE_LIST_OFFSET)
            .and_then(|at| per::decode_reference_list(inflated.bytes(), at))
            .unwrap_or_default();
        let ids: Vec<u32> = references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .collect();
        if let (Some(id), Some(picked)) = (wall.id, picked) {
            picks.insert(u64::from(id), picked);
        }
        lists.push((ids, picked));
    }
    let wanted: BTreeSet<u32> = lists
        .iter()
        .flat_map(|(ids, _)| ids.iter().copied())
        .collect();
    let mut named: BTreeSet<u32> = BTreeSet::new();
    if let Some(header) = partition_names::element_data_header(version) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            named.extend(
                partition_names::find_element_data_names(inflated.bytes(), &header, &wanted)
                    .into_iter()
                    .map(|(id, _)| id),
            );
        }
    }
    let mut by_definitions: BTreeMap<usize, usize> = BTreeMap::new();
    let (mut other_records, mut named_other, mut picked_other) = (0, 0, 0);
    for (ids, picked) in &lists {
        let unique: BTreeSet<u32> = ids.iter().copied().collect();
        let defs: Vec<u32> = unique
            .iter()
            .copied()
            .filter(|id| definitions.contains(id))
            .collect();
        *by_definitions.entry(defs.len().min(2)).or_default() += 1;
        if unique
            .iter()
            .any(|id| type_records.contains(id) && !definitions.contains(id))
        {
            other_records += 1;
        }
        if unique
            .iter()
            .any(|id| named.contains(id) && !type_records.contains(id))
        {
            named_other += 1;
        }
        if let (Some(picked), [only]) = (picked, defs.as_slice()) {
            if picked != only {
                picked_other += 1;
            }
        }
    }
    let mut native_types: BTreeMap<u64, u32> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: picks.keys().copied().collect(),
            ..Options::default()
        },
        |record| {
            if let Some(kind) = record.graph.as_ref().and_then(|graph| {
                let id = graph
                    .objects
                    .first()?
                    .fields
                    .get("m_WallAttributesId")?
                    .pointer("/m_id/m_id64")?
                    .as_i64()?;
                u32::try_from(id).ok()
            }) {
                native_types.insert(record.identity.element_id, kind);
            }
            Ok(())
        },
    )?;
    let (mut native_read, mut native_other, mut native_other_definition) = (0, 0, 0);
    for (id, picked) in &picks {
        let Some(kind) = native_types.get(id) else {
            continue;
        };
        native_read += 1;
        if kind != picked {
            native_other += 1;
            if definitions.contains(kind) {
                native_other_definition += 1;
            }
        }
    }
    let count = |k: usize| by_definitions.get(&k).copied().unwrap_or(0);
    println!(
        "type refs: walls {}; naming 0 type definitions {}, 1 {}, 2 or more {}",
        lists.len(),
        count(0),
        count(1),
        count(2)
    );
    println!(
        "type refs: walls naming a Walls type record that is not a definition: {other_records}"
    );
    println!(
        "type refs: walls naming an id with element-data names that is no Walls type record: {named_other}"
    );
    println!(
        "type refs: walls whose picked type is not the one definition their list names: {picked_other}"
    );
    println!(
        "type refs: walls with a picked type whose native record names one: {native_read}; another than picked {native_other}, of them a type definition {native_other_definition}"
    );
    Ok(())
}
