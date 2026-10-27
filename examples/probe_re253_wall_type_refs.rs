//! RE-253 (#1048): how each wall's records name its type, so it can run on
//! a model whose data may not leave the runner: its `type refs:` and `type
//! copies:` lines are counts, which the held-out scrub keeps, and its other
//! lines feed `tools/re/export_after_save.py` and are deleted before upload.
//!
//! FACT under test: a wall's type is the one type-definition record of the
//! Walls category its reference list names (RE-28,
//! `partition_type_records::unique_type_reference`). On a held-out Revit
//! 2025 model one wall gets its sibling type's name and width; the
//! hypothesis that its list names a second type the type-record scan does
//! not take for a definition is refuted: every reading of the saved file
//! gives it the type rvt-rs picks (RE-253).
//!
//! Output, as counts: the walls; how many type definitions each one's
//! reference list names (0, 1, 2 or more); how many name a Walls type record
//! that is not a definition, and how many name an id with element-data
//! names that is no Walls type record; how many have a type picked that
//! is not the one definition their list names; and how many walls' native
//! `SWall` record, the current copy (RE-216), names another type in
//! `m_WallAttributesId` than the one picked, and how many of those types
//! are type definitions. Then, for the picked types, the copies of their
//! data (RE-216): how many have a name in more than one partition, more than
//! one name, and a name in their latest partition (which the export takes,
//! `type_data_names`) other than their routed current copy's; and how many
//! have a type record in more than one partition. Then `walltype <name>`
//! for each name the file's Walls type records give themselves, which
//! `tools/re/export_after_save.py` looks for among the export's wall types
//! (this output is not uploaded for a held-out model), and `wallbox <tag>
//! <feet>`, the smaller side in plan of each wall's element-record box,
//! which does not depend on how rvt-rs reads its type, and `wallfaces <tag>
//! <feet>`, the distance between the outermost two face planes of its
//! native `SWall` record (RE-252), its thickness as saved whichever way the
//! wall runs. Last, `saved <time>`,
//! the file's save time (its `ProjectInformation` Atom `updated`), which
//! `tools/re/export_after_save.py` compares with Revit's export's time.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re253_wall_type_refs -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, current_partitions, extract_graphs};
use rvt::research::{partition_element_records as per, partition_names, partition_type_records};
use rvt::walker::InstanceField;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let saved = rf.metadata()?.last_saved;
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
                    .into_keys(),
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
    let picked_types: BTreeSet<u32> = picks.values().copied().collect();
    let read_types: BTreeSet<u32> = type_records.union(&picked_types).copied().collect();
    let mut copies: BTreeMap<u32, BTreeMap<u32, String>> = BTreeMap::new();
    if let Some(header) = partition_names::element_data_header(version) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let partition = stream
                .rsplit('/')
                .next()
                .and_then(|n| n.parse::<u32>().ok())
                .unwrap_or(0);
            for (id, name) in
                partition_names::find_element_data_names(inflated.bytes(), &header, &read_types)
            {
                copies.entry(id).or_default().insert(partition, name);
            }
        }
    }
    let file_type_names: BTreeSet<&String> = copies.values().flat_map(|by| by.values()).collect();
    for name in &file_type_names {
        println!("walltype {name}");
    }
    let copies: BTreeMap<u32, BTreeMap<u32, String>> = copies
        .iter()
        .filter(|(id, _)| picked_types.contains(id))
        .map(|(id, by)| (*id, by.clone()))
        .collect();
    let holders: BTreeMap<u32, BTreeSet<u32>> = copies
        .iter()
        .map(|(&id, by)| (id, by.keys().copied().collect()))
        .collect();
    let current = current_partitions(&mut rf, &holders);
    let (mut several, mut names_differ, mut latest_not_current, mut no_route) = (0, 0, 0, 0);
    let mut stale: BTreeSet<u32> = BTreeSet::new();
    for (id, by) in &copies {
        if by.len() > 1 {
            several += 1;
        }
        if by.values().collect::<BTreeSet<_>>().len() > 1 {
            names_differ += 1;
        }
        let latest = by.values().next_back();
        match current.get(id).and_then(|partition| by.get(partition)) {
            Some(name) if Some(name) != latest => {
                latest_not_current += 1;
                stale.insert(*id);
            }
            Some(_) => {}
            None => no_route += 1,
        }
    }
    let stale_walls = picks.values().filter(|kind| stale.contains(kind)).count();
    let mut record_streams: BTreeMap<u32, BTreeSet<&str>> = BTreeMap::new();
    for record in &records {
        if picked_types.contains(&record.element_id) {
            record_streams
                .entry(record.element_id)
                .or_default()
                .insert(record.stream.as_str());
        }
    }
    let records_several = record_streams.values().filter(|s| s.len() > 1).count();
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
    println!(
        "type copies: picked types {}; named {}; in more than one partition {several}; with more than one name {names_differ}; latest name differs from the current copy {latest_not_current} (walls {stale_walls}); no routed copy {no_route}",
        picked_types.len(),
        copies.len()
    );
    println!(
        "type copies: picked types with a type record {}; in more than one partition {records_several}",
        record_streams.len()
    );
    for wall in &mvp.walls {
        let side = |wanted: &str| {
            wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        if let (Some(id), Some(width), Some(depth)) =
            (wall.id, side("m_bboxWidth"), side("m_bboxDepth"))
        {
            println!("wallbox {id} {}", width.min(depth));
        }
    }
    let ids: BTreeSet<u64> = mvp
        .walls
        .iter()
        .filter_map(|w| w.id.map(u64::from))
        .collect();
    for (id, wall) in rvt::research::native_walls::native_walls(&mut rf, &ids)? {
        if let Some([a, b]) = wall.faces {
            println!("wallfaces {id} {}", (b - a).abs());
        }
    }
    println!("saved {}", saved.as_deref().unwrap_or("-"));
    Ok(())
}
