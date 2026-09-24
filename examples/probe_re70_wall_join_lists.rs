//! RE-70: a wall's own data lists the walls it runs through at a join.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a wall's
//! element data (`element_data_header` + its ElementId, up to the next
//! element's header) holds join lists of the form
//! `07 00 00 00 · u32 tag · 02 00 00 00 · u32 count ·
//! count × (u32 · u64 ElementId · u32)`, normally two of them; Revit 2025
//! puts a zero `u32` before the count. The tag is one value per document
//! (`0x0b9f7d` on Snowdon Towers and Core Interior, `0x039f3d` on the MIT
//! tutorial house), in no schema class, so the probe takes the one tag
//! whose frames name only walls. At a
//! butt join of two walls whose centrelines end at one point, Revit's own
//! IFC export runs one wall on to the other's far face and stops the other
//! at its near face. The wall that runs through names the other in its
//! lists and is not named back.
//!
//! The probe prints one JSON object per wall with a centreline:
//! - `id`, the centreline's plan `start` and `end`, and the type's
//!   `thickness`, in feet;
//! - `layers`, its type's layers of non-zero width, and `box`, its
//!   element-record box;
//! - `lists`, each copy's join lists, every entry as `[word, id, k]`.
//!
//! Join them to Revit's export by `Tag` to label each end.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re70_wall_join_lists -- \
//!     MODEL.rvt > walls.jsonl

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const MAX_ENTRIES: usize = 64;

/// One copy of a wall's data: its join lists, each as its tag and entries.
type CopyLists = Vec<(u32, Vec<(u32, u64, u32)>)>;
const DATA_WINDOW: usize = 0x1_0000;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Every `07 00 00 00 · tag · 02 00 00 00 · count · entries` frame in
/// `data`, by tag, whose entries all name another wall in `walls`.
fn join_lists(data: &[u8], walls: &BTreeSet<u32>, count_at: usize) -> CopyLists {
    let mut out = Vec::new();
    for at in memchr::memmem::find_iter(data, &[0x07, 0x00, 0x00, 0x00]) {
        let (Some(tag), Some(2), Some(count)) = (
            u32_at(data, at + 4),
            u32_at(data, at + 8),
            u32_at(data, at + count_at),
        ) else {
            continue;
        };
        if count_at > 12 && u32_at(data, at + 12) != Some(0) {
            continue;
        }
        let count = count as usize;
        if tag < 0x100 || count > MAX_ENTRIES {
            continue;
        }
        let entries: Option<Vec<(u32, u64, u32)>> = (0..count)
            .map(|index| {
                let entry = at + count_at + 4 + index * 16;
                Some((
                    u32_at(data, entry)?,
                    u64_at(data, entry + 4)?,
                    u32_at(data, entry + 12)?,
                ))
            })
            .collect();
        let Some(entries) = entries else { continue };
        if entries
            .iter()
            .all(|(_, id, _)| u32::try_from(*id).is_ok_and(|id| walls.contains(&id)))
        {
            out.push((tag, entries));
        }
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
    // Revit 2025 puts a zero `u32` between the `2` and the count.
    let count_at = if version >= 2025 { 16 } else { 12 };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
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
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let boxes: BTreeMap<u32, [f64; 6]> =
        per::scan_category_records(&mut rf, version, per::OST_WALLS, &declared)?
            .into_iter()
            .map(|record| (record.element_id, record.bbox_feet))
            .collect();
    let layers: BTreeMap<u32, usize> = mvp
        .walls
        .iter()
        .filter_map(|wall| {
            let count = wall.fields.iter().find_map(|(name, value)| match value {
                rvt::walker::InstanceField::Vector(bands)
                    if name == rvt::partition_schema_mvp::WALL_LAYERS_FIELD =>
                {
                    Some(bands.len())
                }
                _ => None,
            })?;
            Some((wall.id?, count))
        })
        .collect();
    let ids: BTreeSet<u32> = axes.keys().copied().collect();
    let walls: BTreeSet<u32> = mvp.walls.iter().filter_map(|wall| wall.id).collect();
    let mut lists: BTreeMap<u32, Vec<CopyLists>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| ids.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(DATA_WINDOW))
                .min(buf.len());
            if let Some(data) = buf.get(id_at + 8..end) {
                lists
                    .entry(id)
                    .or_default()
                    .push(join_lists(data, &walls, count_at));
            }
        }
    }
    let mut votes: BTreeMap<u32, usize> = BTreeMap::new();
    for (tag, list) in lists.values().flatten().flatten() {
        if !list.is_empty() {
            *votes.entry(*tag).or_default() += 1;
        }
    }
    let mut ranked: Vec<(u32, usize)> = votes.into_iter().collect();
    ranked.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let tag = ranked.first().map(|(tag, _)| *tag);
    eprintln!(
        "Revit {version}: {} walls with a centreline, {} with element data; join-list tags {:x?}",
        axes.len(),
        lists.len(),
        &ranked[..ranked.len().min(4)]
    );
    for (id, axis) in &axes {
        let mut row = String::new();
        let _ = write!(
            row,
            "{{\"id\":{id},\"start\":[{},{}],\"end\":[{},{}],\"thickness\":{}",
            axis.start[0], axis.start[1], axis.end[0], axis.end[1], axis.thickness_feet
        );
        if let Some(count) = layers.get(id) {
            let _ = write!(row, ",\"layers\":{count}");
        }
        if let Some(b) = boxes.get(id) {
            let _ = write!(
                row,
                ",\"box\":[{},{},{},{},{},{}]",
                b[0], b[1], b[2], b[3], b[4], b[5]
            );
        }
        row.push_str(",\"lists\":[");
        for (copy, copy_lists) in lists.get(id).into_iter().flatten().enumerate() {
            if copy > 0 {
                row.push(',');
            }
            row.push('[');
            let kept = copy_lists
                .iter()
                .filter(|(t, _)| Some(*t) == tag)
                .map(|(_, list)| list);
            for (index, list) in kept.enumerate() {
                if index > 0 {
                    row.push(',');
                }
                row.push('[');
                for (entry, (word, other, k)) in list.iter().enumerate() {
                    if entry > 0 {
                        row.push(',');
                    }
                    let _ = write!(row, "[{word},{other},{k}]");
                }
                row.push(']');
            }
            row.push(']');
        }
        row.push_str("]}");
        println!("{row}");
    }
    Ok(())
}
