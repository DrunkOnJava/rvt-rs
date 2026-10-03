//! RE-145 (probe): which partition the native path takes each element's
//! current record from, against the partitions that hold its records
//! (#548).
//!
//! `native_document::extract` routes every declared element through
//! `Global/DocumentIncrementTable` (`native_index::route_episode`, from the
//! element's stored revision) and skips a channel-101 record outside that
//! partition as historical. This prints, per file:
//!
//! - `single`: elements with one channel-101 record, and how many of them
//!   route to a partition that holds none of their records, by routed
//!   partition. The native path drops each of those (puzzbobb, #548,
//!   measured it on 2025 files; Core Interior, 2024, is the admitted file
//!   with several partitions).
//! - `repeated`: elements with records in several partitions, whether the
//!   copies are byte-identical, and for those that differ whether the route
//!   picks the latest partition's copy (RE-143 dated the later copy as the
//!   newer on the 2021 `rac_advanced` sample), the earliest one's, or another.
//! - `extract`: on releases the native path admits (2023, 2024, 2027), what
//!   `native_document::extract` itself reports for channel 101: records
//!   skipped as historical and elements emitted, so the count above is the
//!   real path's, not this probe's reading of it.
//!
//! A record's id is 4 bytes through Revit 2023 and 8 from 2024; the width
//! is taken per group, as the one at which the group's records walk
//! exactly. One JSON object per line.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re145_current_copy_routing -- MODEL.rvt ...

use rvt::compression::prepare_stream_for_inflate;
use rvt::{RevitFile, native_document, native_index, native_segments, schema_registry};
use std::collections::{BTreeMap, BTreeSet};

/// One channel-101 record: the partition and the record's body.
struct Copy {
    partition: u32,
    body: Vec<u8>,
}

/// Split a channel-101 group into `(id, body)` records, at the id width
/// (4 or 8 bytes) at which every record's two length words agree and the
/// group ends exactly; `None` when neither width walks it.
fn split_group(bytes: &[u8]) -> Option<Vec<(u64, Vec<u8>)>> {
    'width: for width in [8usize, 4] {
        let header = width + 4;
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < bytes.len() {
            if bytes.len() - pos < header + 4 {
                continue 'width;
            }
            let id = if width == 4 {
                u64::from(u32::from_le_bytes(bytes[pos..pos + 4].try_into().ok()?))
            } else {
                u64::from_le_bytes(bytes[pos..pos + 8].try_into().ok()?)
            };
            let length =
                u32::from_le_bytes(bytes[pos + header - 4..pos + header].try_into().ok()?) as usize;
            let start = pos + header;
            let Some(end) = start.checked_add(length) else {
                continue 'width;
            };
            if end + 4 > bytes.len()
                || u32::from_le_bytes(bytes[end..end + 4].try_into().ok()?) as usize != length
            {
                continue 'width;
            }
            out.push((id, bytes[start..end].to_vec()));
            pos = end + 4;
        }
        return Some(out);
    }
    None
}

fn top(map: &BTreeMap<u32, usize>) -> String {
    let parts: Vec<String> = map.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
    format!("{{{}}}", parts.join(","))
}

fn probe(path: &str) -> anyhow::Result<String> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let registry =
        schema_registry::parse(&native_document::read_single(&mut rf, "Formats/Latest")?)?;
    let episodes = native_index::creation_episodes(
        &native_document::read_single(&mut rf, "Global/History")?,
        &registry,
    )?;
    let index = native_index::parse(
        &native_document::read_single(&mut rf, "Global/ElemTable")?,
        &registry,
        &episodes,
    )?;
    let increments = native_index::storage_increments(
        &native_document::read_single(&mut rf, "Global/DocumentIncrementTable")?,
        &registry,
    )?;
    let mut names: Vec<String> = rf
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let present: BTreeSet<u32> = names.iter().filter_map(|n| n[11..].parse().ok()).collect();

    let mut copies: BTreeMap<u64, Vec<Copy>> = BTreeMap::new();
    let mut unsplit_groups = 0usize;
    for name in &names {
        let partition: u32 = name[11..].parse()?;
        let stored = rf.read_stream(name)?;
        let prepared = prepare_stream_for_inflate(name, &stored);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
            if source.content_key.is_some() || source.channel != 101 {
                return Ok(());
            }
            match split_group(bytes) {
                Some(records) => {
                    for (id, body) in records {
                        copies.entry(id).or_default().push(Copy { partition, body });
                    }
                }
                None => unsplit_groups += 1,
            }
            Ok(())
        })?;
    }

    let (mut single, mut single_routed_away) = (0usize, 0usize);
    let mut away_to: BTreeMap<u32, usize> = BTreeMap::new();
    let (mut repeated, mut identical, mut differing) = (0usize, 0usize, 0usize);
    let (mut to_latest, mut to_earliest, mut to_other) = (0usize, 0usize, 0usize);
    let mut route_errors = 0usize;
    let mut undeclared = 0usize;
    for (id, list) in &copies {
        let Some(identity) = index.identities.get(id) else {
            undeclared += 1;
            continue;
        };
        let Ok(routed) =
            native_index::route_episode(identity.stored_revision, &increments, &present)
        else {
            route_errors += 1;
            continue;
        };
        let partitions: BTreeSet<u32> = list.iter().map(|c| c.partition).collect();
        if partitions.len() == 1 {
            single += 1;
            if !partitions.contains(&routed) {
                single_routed_away += 1;
                *away_to.entry(routed).or_default() += 1;
            }
            continue;
        }
        repeated += 1;
        if list.iter().all(|c| c.body == list[0].body) {
            identical += 1;
            continue;
        }
        differing += 1;
        let latest = *partitions.last().expect("non-empty");
        let earliest = *partitions.first().expect("non-empty");
        if routed == latest {
            to_latest += 1;
        } else if routed == earliest {
            to_earliest += 1;
        } else {
            to_other += 1;
        }
    }

    let extract = if matches!(revit, 2023 | 2024 | 2027) {
        let options = native_document::Options {
            channels: BTreeSet::from([101]),
            ..native_document::Options::default()
        };
        let mut emitted = BTreeSet::new();
        match native_document::extract(&mut rf, &options, |record| {
            emitted.insert(record.identity.element_id);
            Ok(())
        }) {
            Ok(summary) => format!(
                "{{\"skipped_historical_records\":{},\"emitted_elements\":{},\"indexed_elements\":{}}}",
                summary.skipped_historical_records,
                emitted.len(),
                summary.indexed_elements
            ),
            Err(error) => format!("{{\"error\":{:?}}}", format!("{error:#}")),
        }
    } else {
        "null".to_string()
    };

    Ok(format!(
        "{{\"file\":{file:?},\"revit\":{revit},\"partitions\":{},\"indexed\":{},\
         \"ids_with_records\":{},\"undeclared_ids_with_records\":{undeclared},\"unsplit_groups\":{unsplit_groups},\
         \"route_errors\":{route_errors},\
         \"single\":{{\"elements\":{single},\"routed_away\":{single_routed_away},\"routed_to\":{}}},\
         \"repeated\":{{\"elements\":{repeated},\"identical\":{identical},\"differing\":{differing},\
         \"to_latest\":{to_latest},\"to_earliest\":{to_earliest},\"to_other\":{to_other}}},\
         \"extract\":{extract}}}",
        present.len(),
        index.identities.len(),
        copies.len(),
        top(&away_to),
    ))
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re145_current_copy_routing MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        match probe(path) {
            Ok(line) => println!("{line}"),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
