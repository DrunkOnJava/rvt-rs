//! Multi-hypothesis Level ElementId recovery probe (fail-closed research).
//!
//! Tries several independent signals on magnetar corpora and reports
//! whether any yield a stable Level name/elevation → ElementId map.
//!
//! Magnetar Einhoven / Core Interior: **INSUFFICIENT** — see
//! `reports/element-framing/RE-20-level-elementid-negative.md`.
//!
//! Usage:
//!   cargo run --release --example probe_level_elementid_recovery -- FILE.rvt

use rvt::RevitFile;
use rvt::compression;
use rvt::elem_table;
use rvt::formats;
use rvt::object_graph;
use rvt::partition_arc_walls::{
    recover_storeys_from_arc_walls, scan_partition_arc_walls_with_limits,
};
use rvt::partition_name_candidates::building_storey_name_candidates;
use rvt::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::WalkerLimits;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(
        env::args()
            .nth(1)
            .expect("usage: probe_level_elementid_recovery <file.rvt>"),
    );
    let mut rf = RevitFile::open(&path)?;
    let bfi = rf.basic_file_info()?;
    println!("file={} revit={}", path.display(), bfi.version);

    let limits = WalkerLimits::default();
    let walls = scan_partition_arc_walls_with_limits(&mut rf, bfi.version, limits)?.walls;
    let wall_ids: BTreeSet<u32> = walls.iter().filter_map(|w| w.element_id()).collect();
    let strings = object_graph::string_records_from_partitions(&mut rf).unwrap_or_default();
    let names = building_storey_name_candidates(strings.iter().map(|s| s.value.as_str()));
    let storeys = recover_storeys_from_arc_walls(&walls, &names);
    println!(
        "arcwalls={} wall_ids={} storey_names={:?} recovered_storeys={}",
        walls.len(),
        wall_ids.len(),
        names,
        storeys.storeys.len()
    );
    for s in &storeys.storeys {
        println!("  storey name={:?} elev={:.4}", s.name, s.elevation_feet);
    }

    let mvp = recover_partition_schema_mvp(&mut rf, bfi.version, limits)?;
    println!(
        "mvp levels={} floors={} rooms={}",
        mvp.levels.len(),
        mvp.floors.len(),
        mvp.rooms.len()
    );

    let elem_recs = elem_table::parse_records(&mut rf).unwrap_or_default();
    let elem_ids: BTreeSet<u32> = elem_recs.iter().map(|r| r.id_primary).collect();
    println!("elem_table_ids={}", elem_ids.len());

    // --- H1: Level-like string records + nearby ElemTable ids ---
    println!("\n=== H1 string-proximity Level name → nearby ElemTable ids ===");
    let level_strings: Vec<_> = strings
        .iter()
        .filter(|s| names.iter().any(|n| n == &s.value) || is_levelish(&s.value))
        .collect();
    println!("levelish_string_records={}", level_strings.len());
    for s in level_strings.iter().take(20) {
        println!("  offset={} tag={:#x} value={:?}", s.offset, s.tag, s.value);
    }

    // Concatenate all partitions (same as string extractor joins one
    // partition stream; also scan every Partitions/* stream).
    let mut h1_votes: BTreeMap<(String, u32), u32> = BTreeMap::new();
    let mut h1_name_to_ids: BTreeMap<String, BTreeMap<u32, u32>> = BTreeMap::new();
    for stream in partition_streams(&mut rf) {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
            .into_iter()
            .flatten()
            .collect();
        if concat.len() < 64 {
            continue;
        }
        for s in &level_strings {
            // String extractor offsets are from one joined partition;
            // search for UTF-16LE of the value instead (robust).
            let needle = utf16le(&s.value);
            let mut search = 0usize;
            while let Some(rel) = find_subslice(&concat[search..], &needle) {
                let at = search + rel;
                let lo = at.saturating_sub(96);
                let hi = (at + needle.len() + 96).min(concat.len());
                let mut j = lo;
                while j + 4 <= hi {
                    if j >= at && j < at + needle.len() {
                        j += 1;
                        continue;
                    }
                    let cand = u32::from_le_bytes(concat[j..j + 4].try_into().unwrap());
                    if cand != 0
                        && cand != u32::MAX
                        && elem_ids.contains(&cand)
                        && !wall_ids.contains(&cand)
                    {
                        *h1_votes.entry((s.value.clone(), cand)).or_insert(0) += 1;
                        *h1_name_to_ids
                            .entry(s.value.clone())
                            .or_default()
                            .entry(cand)
                            .or_insert(0) += 1;
                    }
                    j += 1;
                }
                search = at + 2;
                if search >= concat.len() {
                    break;
                }
            }
        }
    }
    println!("h1 unique (name,id) pairs={}", h1_votes.len());
    let mut h1_sorted: Vec<_> = h1_votes.iter().collect();
    h1_sorted.sort_by(|a, b| b.1.cmp(a.1));
    for ((name, id), n) in h1_sorted.iter().take(30) {
        println!("  name={name:?} id={id} votes={n}");
    }
    let h1_unique = count_unique_dominant(&h1_name_to_ids, 3);
    println!(
        "H1_VERDICT unique_dominant_names={}/{} (need >=2)",
        h1_unique,
        h1_name_to_ids.len()
    );

    // --- H2: elevation f64 near ElemTable ids (not ArcWall) ---
    println!("\n=== H2 elevation f64 neighbourhood → ElemTable ids ===");
    let elevs: Vec<f64> = storeys.storeys.iter().map(|s| s.elevation_feet).collect();
    let mut h2_elev_ids: BTreeMap<i64, BTreeMap<u32, u32>> = BTreeMap::new();
    for stream in partition_streams(&mut rf) {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
            .into_iter()
            .flatten()
            .collect();
        if concat.len() < 32 {
            continue;
        }
        let mut i = 0usize;
        while i + 8 <= concat.len() {
            let v = f64::from_le_bytes(concat[i..i + 8].try_into().unwrap());
            if let Some(elev) = elevs.iter().find(|e| (**e - v).abs() < 1e-6) {
                let key = (*elev * 1000.0).round() as i64;
                let lo = i.saturating_sub(48);
                let hi = (i + 48).min(concat.len().saturating_sub(3));
                let mut j = lo;
                while j + 4 <= hi {
                    if j + 4 > i && j < i + 8 {
                        j += 1;
                        continue;
                    }
                    let cand = u32::from_le_bytes(concat[j..j + 4].try_into().unwrap());
                    if cand != 0
                        && cand != u32::MAX
                        && elem_ids.contains(&cand)
                        && !wall_ids.contains(&cand)
                    {
                        *h2_elev_ids.entry(key).or_default().entry(cand).or_insert(0) += 1;
                    }
                    j += 1;
                }
            }
            i += 1;
        }
    }
    let mut h2_singletons = 0u64;
    for (elev_milli, ids) in &h2_elev_ids {
        let elev = *elev_milli as f64 / 1000.0;
        let mut votes: Vec<_> = ids.iter().collect();
        votes.sort_by(|a, b| b.1.cmp(a.1));
        let top = votes.first().map(|(id, n)| (**id, **n));
        let uniqueish =
            votes.len() == 1 || votes.get(1).map(|(_, n)| **n).unwrap_or(0) * 3 < *votes[0].1;
        if uniqueish && top.map(|(_, n)| n >= 3).unwrap_or(false) {
            h2_singletons += 1;
        }
        println!(
            "  elev={elev:.4} top={:?} n_distinct_ids={} uniqueish={uniqueish}",
            top,
            ids.len()
        );
    }
    println!(
        "H2_VERDICT singleton_elev_clusters={h2_singletons}/{}",
        h2_elev_ids.len()
    );

    // --- H3: schema Level class (tagless) + tagged relatives ---
    println!("\n=== H3 schema Level / Level-related tags ===");
    let schema = {
        let formats_raw = rf.read_stream(rvt::streams::FORMATS_LATEST)?;
        let formats_d = compression::inflate_at(&formats_raw, 0)?;
        formats::parse_schema(&formats_d)?
    };
    let level_class = schema.classes.iter().find(|c| c.name == "Level");
    match level_class {
        Some(c) => {
            println!(
                "Level schema tag={:?} fields={} parent={:?} tagged_ancestor={:?}",
                c.tag,
                c.fields.len(),
                c.parent,
                schema.tagged_ancestor("Level")
            );
            for f in c.fields.iter().take(16) {
                println!("  field {} type={:?}", f.name, f.field_type);
            }
        }
        None => println!("Level class missing from schema"),
    }
    println!("tagged Level/Datum-related classes:");
    for c in &schema.classes {
        if let Some(t) = c.tag {
            if c.name.contains("Level") || c.name.contains("Datum") || c.name == "HostObjAttr" {
                println!("  {} tag={t:#06x} fields={}", c.name, c.fields.len());
            }
        }
    }

    // --- H5: Floor plan-loop neighbourhood ElementIds ---
    println!("\n=== H5 Floor plan-loop neighbourhood ElemTable ids ===");
    let mut h5_ids: BTreeMap<u32, u32> = BTreeMap::new();
    for floor in &mvp.floors {
        let stream = floor
            .fields
            .iter()
            .find(|(n, _)| n == "m_source_stream")
            .and_then(|(_, v)| match v {
                rvt::walker::InstanceField::String(s) => Some(s.as_str()),
                _ => None,
            });
        let offset = floor
            .fields
            .iter()
            .find(|(n, _)| n == "m_source_offset")
            .and_then(|(_, v)| match v {
                rvt::walker::InstanceField::Integer {
                    value,
                    signed: false,
                    ..
                } => Some(*value as usize),
                _ => None,
            });
        let (Some(stream), Some(offset)) = (stream, offset) else {
            continue;
        };
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
            .into_iter()
            .flatten()
            .collect();
        let lo = offset.saturating_sub(128);
        let hi = (offset + 128).min(concat.len().saturating_sub(3));
        let mut j = lo;
        while j + 4 <= hi {
            let cand = u32::from_le_bytes(concat[j..j + 4].try_into().unwrap());
            if cand != 0
                && cand != u32::MAX
                && elem_ids.contains(&cand)
                && !wall_ids.contains(&cand)
            {
                *h5_ids.entry(cand).or_insert(0) += 1;
            }
            j += 1;
        }
    }
    let mut h5_sorted: Vec<_> = h5_ids.iter().collect();
    h5_sorted.sort_by(|a, b| b.1.cmp(a.1));
    println!("h5 candidate ids near floors (top 15):");
    for (id, n) in h5_sorted.iter().take(15) {
        println!("  id={id} votes={n}");
    }
    println!(
        "H5_VERDICT distinct_ids={} (Floor→Level join needs stable shared id)",
        h5_ids.len()
    );

    // --- H6: ContentDocuments id list vs Level string count ---
    println!("\n=== H6 ContentDocuments overview ===");
    if let Ok(raw) = rf.read_stream("Global/ContentDocuments") {
        let chunks = compression::inflate_all_chunks(&raw);
        let decomp: Vec<u8> = chunks.into_iter().flatten().collect();
        println!("ContentDocuments decompressed_bytes={}", decomp.len());
        // Count how many elem ids appear as u32 LE in CD body
        let mut cd_hits = 0u64;
        let mut i = 0usize;
        while i + 4 <= decomp.len() {
            let v = u32::from_le_bytes(decomp[i..i + 4].try_into().unwrap());
            if elem_ids.contains(&v) {
                cd_hits += 1;
            }
            i += 4; // stride rough
        }
        println!("aligned u32 ElemTable hits≈{cd_hits}");
    } else {
        println!("ContentDocuments missing");
    }

    // --- Final combined verdict ---
    let promising = h1_unique >= 2 || h2_singletons >= 2;
    println!(
        "\nVERDICT level_elementid_recovery={}",
        if promising {
            "PROMISING"
        } else {
            "INSUFFICIENT"
        }
    );
    Ok(())
}

fn is_levelish(s: &str) -> bool {
    let t = s.trim();
    if t.eq_ignore_ascii_case("roof") || t.eq_ignore_ascii_case("ground floor") {
        return true;
    }
    let lower = t.to_ascii_lowercase();
    lower.starts_with("level ")
        || lower.starts_with("elevation ")
        || lower.starts_with("basement")
        || lower.starts_with("mez")
}

fn utf16le(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 2);
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn count_unique_dominant(map: &BTreeMap<String, BTreeMap<u32, u32>>, min_votes: u32) -> usize {
    let mut n = 0usize;
    for ids in map.values() {
        let mut votes: Vec<_> = ids.iter().collect();
        votes.sort_by(|a, b| b.1.cmp(a.1));
        if votes.is_empty() {
            continue;
        }
        let top = *votes[0].1;
        if top < min_votes {
            continue;
        }
        let second = votes.get(1).map(|(_, v)| **v).unwrap_or(0);
        if top >= second.saturating_mul(3) {
            n += 1;
        }
    }
    n
}

fn partition_streams(rf: &mut RevitFile) -> Vec<String> {
    let mut streams: Vec<(usize, String)> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .filter_map(|s| {
            let raw = rf.read_stream(&s).ok()?;
            Some((raw.len(), s))
        })
        .collect();
    streams.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    streams.into_iter().map(|(_, s)| s).collect()
}
