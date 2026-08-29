//! Probe Level ElementId bind evidence on project corpora (fail-closed research).
//!
//! Looks for ArcWall ElementIds and nearby ElemTable ids that co-occur with
//! a plausible f64 (LevelAssociationCell-shaped: levelId + offset).
//! Clusters candidate level ids by ArcWall base elevation.
//!
//! Usage:
//!   cargo run --release --example probe_level_elementid_bind -- \
//!     _project_corpus/Revit/Revit_IFC5_Einhoven.rvt

use rvt::RevitFile;
use rvt::compression;
use rvt::elem_table;
use rvt::object_graph;
use rvt::partition_arc_walls::{
    recover_storeys_from_arc_walls, scan_partition_arc_walls_with_limits,
};
use rvt::partition_name_candidates::building_storey_name_candidates;
use rvt::walker::WalkerLimits;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(
        env::args()
            .nth(1)
            .expect("usage: probe_level_elementid_bind <file.rvt>"),
    );
    let mut rf = RevitFile::open(&path)?;
    let bfi = rf.basic_file_info()?;
    println!("file={} revit={}", path.display(), bfi.version);

    let limits = WalkerLimits::default();
    let walls = scan_partition_arc_walls_with_limits(&mut rf, bfi.version, limits)?.walls;
    let wall_ids: BTreeSet<u32> = walls.iter().filter_map(|w| w.element_id()).collect();
    println!(
        "arcwalls={} with_element_id={}",
        walls.len(),
        wall_ids.len()
    );

    let elev_by_id: BTreeMap<u32, f64> = walls
        .iter()
        .filter_map(|w| {
            let id = w.element_id()?;
            let elev = w.base_elevation_feet()?;
            Some((id, elev))
        })
        .collect();

    let strings = object_graph::string_records_from_partitions(&mut rf).unwrap_or_default();
    let names = building_storey_name_candidates(strings.iter().map(|s| s.value.as_str()));
    let storeys = recover_storeys_from_arc_walls(&walls, &names);
    println!(
        "storeys={}",
        storeys
            .storeys
            .iter()
            .map(|s| format!("{}@{:.3}", s.name, s.elevation_feet))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let elem_ids: BTreeSet<u32> = match elem_table::parse_records(&mut rf) {
        Ok(recs) => recs.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => BTreeSet::new(),
    };
    println!("elem_table_ids={}", elem_ids.len());
    let wall_in_table = wall_ids.iter().filter(|id| elem_ids.contains(id)).count();
    println!("wall_ids_in_elem_table={wall_in_table}/{}", wall_ids.len());

    let mut level_id_votes: BTreeMap<u32, u32> = BTreeMap::new();
    let mut elev_to_level_ids: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut pair_hits = 0u64;

    for stream in partition_streams_largest_first(&mut rf) {
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
        println!("scan {} bytes={}", stream, concat.len());

        let mut i = 0usize;
        while i + 4 <= concat.len() {
            let v = u32::from_le_bytes(concat[i..i + 4].try_into().unwrap());
            if wall_ids.contains(&v) {
                let lo = i.saturating_sub(64);
                let hi = (i + 64).min(concat.len().saturating_sub(3));
                let mut j = lo;
                while j + 4 <= hi {
                    if j == i {
                        j += 1;
                        continue;
                    }
                    let cand = u32::from_le_bytes(concat[j..j + 4].try_into().unwrap());
                    if cand == 0 || cand == u32::MAX || cand == v {
                        j += 1;
                        continue;
                    }
                    if !elem_ids.contains(&cand) || wall_ids.contains(&cand) {
                        j += 1;
                        continue;
                    }
                    if !f64_near(&concat, j, 24) {
                        j += 1;
                        continue;
                    }
                    pair_hits += 1;
                    *level_id_votes.entry(cand).or_insert(0) += 1;
                    if let Some(&elev) = elev_by_id.get(&v) {
                        let key = (elev * 1000.0).round() as i64;
                        elev_to_level_ids.entry(key).or_default().insert(cand);
                    }
                    j += 1;
                }
            }
            i += 1;
        }
        if pair_hits > 0 {
            break;
        }
    }

    println!("proximity_pair_hits={pair_hits}");
    println!("candidate_level_ids_by_votes (top 20):");
    let mut votes: Vec<_> = level_id_votes.iter().collect();
    votes.sort_by(|a, b| b.1.cmp(a.1));
    for (id, n) in votes.iter().take(20) {
        println!("  level_id_candidate={id} votes={n}");
    }

    let mut singleton_elev_clusters = 0u64;
    println!("elevation → candidate level ids:");
    for (elev_milli, ids) in &elev_to_level_ids {
        let elev = *elev_milli as f64 / 1000.0;
        if ids.len() == 1 {
            singleton_elev_clusters += 1;
        }
        println!(
            "  elev={elev:.4} n_ids={} ids={:?}",
            ids.len(),
            ids.iter().take(8).collect::<Vec<_>>()
        );
    }
    println!(
        "singleton_elev_clusters={singleton_elev_clusters}/{}",
        elev_to_level_ids.len()
    );

    let bind_ready =
        singleton_elev_clusters >= 2 && votes.first().map(|(_, n)| **n >= 3).unwrap_or(false);
    println!(
        "VERDICT level_elementid_bind_evidence={}",
        if bind_ready {
            "PROMISING"
        } else {
            "INSUFFICIENT"
        }
    );
    Ok(())
}

fn f64_near(buf: &[u8], at: usize, radius: usize) -> bool {
    let lo = at.saturating_sub(radius);
    let hi = (at + radius).min(buf.len().saturating_sub(7));
    let mut i = lo;
    while i + 8 <= hi {
        if i + 4 > at && i < at + 4 {
            i += 1;
            continue;
        }
        let v = f64::from_le_bytes(buf[i..i + 8].try_into().unwrap());
        if v.is_finite() && v.abs() < 50.0 {
            return true;
        }
        i += 1;
    }
    false
}

fn partition_streams_largest_first(rf: &mut RevitFile) -> Vec<String> {
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
