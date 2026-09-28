//! RE-127 (probe): the lists in a wall's data that name the walls it is
//! joined to, beside RE-70's join lists.
//!
//! OBSERVATION being tested: besides RE-70's lists (`07 00 00 00 · tag ·
//! 02 · count · count × (u32 · u64 ElementId · u32)`), a wall's element
//! data holds frames `u32 word · u32 tag · 02 00 00 00 · 00 00 00 00 ·
//! u32 count` whose entries are variable length: `u64 ElementId · u32 n ·
//! n × u32`. On the flowbim.ee Revit 2026 house the tag is the join lists'
//! tag + 2. Whether they carry the join decision RE-70 cannot read (#238)
//! is what this probe's output is scored for.
//!
//! One JSON object per wall with a centreline: `id`, the centreline's plan
//! `start` and `end` in feet, and `lists`, every frame of either kind whose
//! entries all name another recovered wall, as `{kind, word, tag, at,
//! entries}`; a join-list entry is `[k, id, j]` and a joined-wall entry is
//! `[id, [u32...]]`. `at` is the frame's offset in the wall's data.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re127_joined_wall_lists -- MODEL.rvt > walls.jsonl

use rvt::RevitFile;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const WINDOW: usize = 0x1_0000;
const MAX_ENTRIES: usize = 64;
const MAX_WORDS: usize = 256;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

fn wall_id(b: &[u8], at: usize, walls: &BTreeSet<u32>) -> Option<u32> {
    u64_at(b, at)
        .and_then(|id| u32::try_from(id).ok())
        .filter(|id| walls.contains(id))
}

/// Every frame at `at` = `word · tag · 2 · 0 · count · entries` whose
/// entries all name a wall, as a JSON object.
fn frames(data: &[u8], walls: &BTreeSet<u32>) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 20 <= data.len() {
        let (Some(word), Some(tag), Some(2), Some(zero), Some(count)) = (
            u32_at(data, at),
            u32_at(data, at + 4),
            u32_at(data, at + 8),
            u32_at(data, at + 12),
            u32_at(data, at + 16),
        ) else {
            break;
        };
        let count = count as usize;
        if tag < 0x100 || count == 0 || count > MAX_ENTRIES {
            at += 1;
            continue;
        }
        // RE-70 join list (2025 and later put a zero before the count).
        if word == 7 && zero == 0 {
            let entries: Option<Vec<String>> = (0..count)
                .map(|index| {
                    let entry = at + 20 + index * 16;
                    Some(format!(
                        "[{},{},{}]",
                        u32_at(data, entry)?,
                        wall_id(data, entry + 4, walls)?,
                        u32_at(data, entry + 12)?
                    ))
                })
                .collect();
            if let Some(entries) = entries {
                out.push(format!(
                    "{{\"kind\":\"join\",\"word\":{word},\"tag\":{tag},\"at\":{at},\"entries\":[{}]}}",
                    entries.join(",")
                ));
                at += 20;
                continue;
            }
        }
        if zero == 0 {
            let mut cursor = at + 20;
            let mut entries = Vec::new();
            for _ in 0..count {
                let Some(id) = wall_id(data, cursor, walls) else {
                    break;
                };
                let Some(n) = u32_at(data, cursor + 8).map(|n| n as usize) else {
                    break;
                };
                if n > MAX_WORDS {
                    break;
                }
                let words: Option<Vec<String>> = (0..n)
                    .map(|index| u32_at(data, cursor + 12 + 4 * index).map(|w| w.to_string()))
                    .collect();
                let Some(words) = words else {
                    break;
                };
                entries.push(format!("[{id},[{}]]", words.join(",")));
                cursor += 12 + 4 * n;
            }
            if entries.len() == count {
                out.push(format!(
                    "{{\"kind\":\"joined\",\"word\":{word},\"tag\":{tag},\"at\":{at},\"entries\":[{}]}}",
                    entries.join(",")
                ));
                at = cursor;
                continue;
            }
        }
        at += 1;
    }
    out
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(header) = rvt::partition_names::element_data_header(version) else {
        eprintln!("Revit {version}: element data is not read on this release");
        return Ok(());
    };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    let walls: BTreeSet<u32> = mvp.walls.iter().filter_map(|wall| wall.id).collect();
    let axes: BTreeMap<u32, rvt::partition_schema_mvp::WallAxis> = mvp
        .walls
        .iter()
        .filter_map(|wall| {
            Some((
                wall.id?,
                rvt::partition_schema_mvp::wall_axis_from_fields(&wall.fields)?,
            ))
        })
        .collect();
    let mut lists: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = wall_id(buf, id_at, &walls).filter(|id| axes.contains_key(id)) else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(WINDOW))
                .min(buf.len());
            if let Some(data) = buf.get(id_at + 8..end) {
                lists.entry(id).or_default().extend(frames(data, &walls));
            }
        }
    }
    eprintln!(
        "Revit {version}: {} walls, {} with a centreline, {} with lists",
        walls.len(),
        axes.len(),
        lists.values().filter(|l| !l.is_empty()).count()
    );
    for (id, axis) in &axes {
        let mut row = String::new();
        let _ = write!(
            row,
            "{{\"id\":{id},\"start\":[{},{}],\"end\":[{},{}],\"thickness\":{},\"lists\":[{}]}}",
            axis.start[0],
            axis.start[1],
            axis.end[0],
            axis.end[1],
            axis.thickness_feet,
            lists.get(id).map(|l| l.join(",")).unwrap_or_default()
        );
        println!("{row}");
    }
    Ok(())
}
