//! RE-315 probe: the record chain of a Revit 2008 to 2023 partition, and the
//! element frames that sit in it without an ElementId of their own (#1261).
//!
//! FACT (to be measured): a 32-bit partition is runs of back-to-back records
//! (`u32` ElementId, `u32` size, the tag of `ElementHeader`, a `u16` count,
//! the body, and a 12-byte trailer of a `u32`, a flag word and the size
//! again) with stretches between the runs where no record's trailer echoes
//! its size. The 2024 walk's flag test (`0` or `0x0100_0000`) stops at the
//! first record with another flag word, and only the leading run is walked;
//! a walk that tests the tag and the echo alone, and resumes at the next
//! record that validates, reaches the later runs. A placed-instance frame
//! that does not open with its declared id sits inside one declared record,
//! and its reference list names that record's id. `rvt::research::
//! partition_element_records_2023::scan_records` gives such a frame that id.
//!
//! For each partition it prints the records the walk finds against every
//! place the tag sits 8 bytes into a validating record, the runs they form,
//! where the 2024 flag test would stop, and the frames of the file's marker
//! in the `BuiltInCategory` band by the id they open with, whether they are
//! placed instances, and whether a declared record encloses them and their
//! list names it. Then the instances `scan_records` selects, one per
//! ElementId as the 2023 export selects them, as `category <id> <class>
//! <count> <ids>` lines that `tools/re/instances_vs_ifc_tags.py` scores
//! against Revit's own export; a class is suffixed `.enclosed` where the id
//! came from the enclosing record.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re315_32bit_record_chain -- model.rvt
//! ```

use rvt::RevitFile;
use rvt::research::partition_element_records::{
    self as per, PLACEMENT_KIND_INSTANCE, RECOVERED_CATEGORIES,
};
use rvt::research::partition_element_records_2023 as per32;
use std::collections::{BTreeMap, BTreeSet};

/// Bytes from the ElementId to the marker.
const ID_BEFORE_MARKER: usize = 52;
/// Bytes from the `BuiltInCategory` to the marker.
const CATEGORY_BEFORE_MARKER: usize = 38;
/// Bytes from the container reference to the marker.
const CONTAINER_BEFORE_MARKER: usize = 22;
/// Bytes from the placement kind to the marker.
const PLACEMENT_BEFORE_MARKER: usize = 14;
/// Bytes from the marker to the counted list of `u32` references.
const LIST_AFTER_MARKER: usize = 8 + 48;

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    let end = at.checked_add(4)?;
    buf.get(at..end)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    let end = at.checked_add(2)?;
    buf.get(at..end)
        .map(|b| u16::from_le_bytes(b.try_into().expect("2 bytes")))
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    let end = at.checked_add(8)?;
    buf.get(at..end)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// The record at `at` as `(element_id, end, flag word)` when its header
/// carries `header_tag` and its trailer echoes its size.
fn record_at(buf: &[u8], at: usize, header_tag: u16) -> Option<(u32, usize, u32)> {
    let id = u32_at(buf, at)?;
    let size = u32_at(buf, at.checked_add(4)?)? as usize;
    if u16_at(buf, at.checked_add(8)?)? != header_tag || size < 12 {
        return None;
    }
    let end = at.checked_add(size)?;
    let flag = u32_at(buf, end.checked_add(4)?)?;
    (u32_at(buf, end.checked_add(8)?)? as usize == size).then_some((id, end, flag))
}

/// The leading run as the 2024 walk reads it, with a flag word of `0` or
/// `0x0100_0000` only: its records, where it stops, and the flag word that
/// stopped it, when a record validated but for its flag.
fn flag_rule_walk(buf: &[u8], header_tag: u16) -> (usize, usize, Option<u32>) {
    let (mut at, mut records) = (0usize, 0usize);
    while let Some((_, end, flag)) = record_at(buf, at, header_tag) {
        if !matches!(flag, 0 | 0x0100_0000) {
            return (records, at, Some(flag));
        }
        records += 1;
        at = end + 12;
    }
    (records, at, None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or("usage: probe_re315_32bit_record_chain <file.rvt>")?;
    let mut rf = RevitFile::open(&path)?;
    let saved = rf.basic_file_info()?.version;
    let release = rf.layout_release()?;
    println!("release saved {saved} layout {release}");
    if !per32::is_32bit_release(release) {
        println!("not a 32-bit release: nothing to measure");
        return Ok(());
    }
    let Some((marker, header_tag)) = per32::record_marker(&mut rf) else {
        println!("the schema gives no record marker");
        return Ok(());
    };
    println!(
        "marker {} header tag {header_tag}",
        marker
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let declared: BTreeSet<u32> =
        rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    println!("declared ids {}", declared.len());
    let shortfall = per32::envelope_shortfall(release);
    let mut with_record: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            println!("{stream}: cannot be inflated");
            continue;
        };
        let buf = inflated.bytes();
        let spans = per32::record_spans(buf, header_tag, &declared);
        let hits: BTreeSet<usize> = memchr::memmem::find_iter(buf, &header_tag.to_le_bytes())
            .filter_map(|hit| hit.checked_sub(8))
            .filter(|&at| {
                record_at(buf, at, header_tag).is_some_and(|(id, _, _)| declared.contains(&id))
            })
            .collect();
        let walked: BTreeSet<usize> = spans.iter().map(|span| span.start).collect();
        let mut runs: Vec<(usize, usize, usize)> = Vec::new();
        for span in &spans {
            match runs.last_mut() {
                Some(run) if run.1 + 12 == span.start => {
                    run.1 = span.end;
                    run.2 += 1;
                }
                _ => runs.push((span.start, span.end, 1)),
            }
        }
        let (legacy, legacy_stop, legacy_flag) = flag_rule_walk(buf, header_tag);
        let mut flags: BTreeMap<u32, usize> = BTreeMap::new();
        for span in &spans {
            if let Some((_, _, flag)) = record_at(buf, span.start, header_tag) {
                *flags.entry(flag).or_default() += 1;
            }
        }
        println!("{stream}: {} bytes", buf.len());
        println!(
            "  records walked {}, at every validating tag {}, only walked {}, only at a tag {}",
            walked.len(),
            hits.len(),
            walked.difference(&hits).count(),
            hits.difference(&walked).count()
        );
        let longest = runs.iter().map(|run| run.2).max().unwrap_or(0);
        println!("  runs {}, longest {longest} records", runs.len());
        for run in runs.iter().take(6) {
            println!("    run {}..{}: {} records", run.0, run.1, run.2);
        }
        println!(
            "  flag rule walk: {legacy} records, stops at {legacy_stop}{}",
            legacy_flag
                .map(|flag| format!(" on flag word 0x{flag:08x}"))
                .unwrap_or_default()
        );
        let mut flag_list: Vec<_> = flags.iter().collect();
        flag_list.sort_by_key(|entry| std::cmp::Reverse(*entry.1));
        println!(
            "  trailer flag words: {}",
            flag_list
                .iter()
                .take(6)
                .map(|(flag, count)| format!("0x{flag:08x} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        with_record.extend(spans.iter().map(|span| span.element_id));
        // Every frame of the marker in the category band, by how its id is
        // known.
        let mut frames: BTreeMap<String, usize> = BTreeMap::new();
        for at in memchr::memmem::find_iter(buf, &marker) {
            let Some(id_at) = at.checked_sub(ID_BEFORE_MARKER - shortfall) else {
                continue;
            };
            let Some(category) = at
                .checked_sub(CATEGORY_BEFORE_MARKER - shortfall)
                .and_then(|from| i64_at(buf, from))
            else {
                continue;
            };
            if !(per::BUILTIN_CATEGORY_MIN..=per::BUILTIN_CATEGORY_MAX).contains(&category) {
                continue;
            }
            let Some(leading) = u32_at(buf, id_at) else {
                continue;
            };
            let kind = if declared.contains(&leading) && u16_at(buf, id_at + 8) == Some(header_tag)
            {
                "opens with its declared id".to_string()
            } else {
                let instance = at
                    .checked_sub(CONTAINER_BEFORE_MARKER)
                    .and_then(|from| u32_at(buf, from))
                    == Some(u32::MAX)
                    && at
                        .checked_sub(PLACEMENT_BEFORE_MARKER)
                        .and_then(|from| u32_at(buf, from))
                        == Some(PLACEMENT_KIND_INSTANCE);
                let word = if leading == u32::MAX {
                    "ff ff ff ff"
                } else {
                    "a count or other word"
                };
                if !instance {
                    format!("no id, not a placed instance ({word})")
                } else {
                    match per32::enclosing_span(&spans, id_at) {
                        None => format!("no id, placed instance, in no declared record ({word})"),
                        Some(span) => {
                            let list_at = at + LIST_AFTER_MARKER;
                            let named = u32_at(buf, list_at).is_some_and(|count| {
                                count <= 1024
                                    && (0..count as usize).any(|slot| {
                                        u32_at(buf, list_at + 4 + slot * 4) == Some(span.element_id)
                                    })
                            });
                            if named {
                                format!(
                                    "no id, placed instance, in a record whose id its list names ({word})"
                                )
                            } else {
                                format!(
                                    "no id, placed instance, in a record whose id its list does not name ({word})"
                                )
                            }
                        }
                    }
                }
            };
            *frames.entry(kind).or_default() += 1;
        }
        for (kind, count) in &frames {
            println!("  frames {kind}: {count}");
        }
    }
    println!(
        "declared ids with a record in some partition: {} of {}",
        with_record.len(),
        declared.len()
    );

    // What scan_records selects, one per ElementId, newest first as the 2023
    // export picks.
    let records = per32::scan_records(&mut rf, release);
    let enclosed_total = records
        .iter()
        .filter(|record| record.id_from_enclosing_record)
        .count();
    println!(
        "scan_records: {} records, {} with the enclosing record's id",
        records.len(),
        enclosed_total
    );
    let partition = |stream: &str| {
        stream
            .rsplit('/')
            .next()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let mut selected: BTreeMap<u32, &per::PartitionElementRecord> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.is_exported_instance() && record.has_volume())
    {
        let newer = selected.get(&record.element_id).is_none_or(|held| {
            (partition(&record.stream), record.offset) > (partition(&held.stream), held.offset)
        });
        if newer {
            selected.insert(record.element_id, record);
        }
    }
    let mut by_class: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for record in selected.values() {
        let class = RECOVERED_CATEGORIES
            .iter()
            .find(|(category, _)| *category == record.builtin_category)
            .map(|(_, class)| (*class).to_string())
            .unwrap_or_else(|| format!("OST{}", -record.builtin_category));
        let name = if record.id_from_enclosing_record {
            format!("{class}.enclosed")
        } else {
            class
        };
        by_class.entry(name).or_default().push(record.element_id);
    }
    println!("selected instances {}", selected.len());
    for (name, ids) in &by_class {
        let list: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        println!("category 0 {} {} {}", name, ids.len(), list.join(","));
    }
    Ok(())
}
