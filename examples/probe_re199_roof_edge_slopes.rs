//! RE-199 (probe, #356): every roof sketch line's slope block (RE-56), on any
//! release whose element data header is known.
//!
//! For each placed roof the probe lists the sketch lines it owns, and for each
//! line every run of 32 `ff` bytes in its element data with the bytes around
//! it: the f64 at −0x18, the i64 at +0x20, the u32s at +0x28 and +0x2c, the
//! four flag bytes at +0x30, the f64 at +0x3b, and what
//! `partition_roof_slopes::edge_slope_in` makes of the block. It also prints the
//! line's bounded line record (RE-49) when one is found, the slope arrows'
//! heights, and each roof's rafter cut and fascia depth (read on 2025 only).
//!
//! Usage: probe_re199_roof_edge_slopes <model.rvt>

use rvt::RevitFile;
use rvt::partition_element_records::{OST_SKETCH_LINES, scan_category_records_multi};
use rvt::partition_roof_slopes::edge_slope_in;
use rvt::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let header = rvt::partition_names::element_data_header(version)
        .ok_or_else(|| anyhow::anyhow!("no element data header for {version}"))?;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let roofs: BTreeMap<u32, f64> = mvp
        .products
        .iter()
        .filter(|e| e.class == "Roof" || e.class == "FootPrintRoof")
        .filter_map(|e| {
            let height = e.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == "m_bboxHeight" => Some(*value),
                _ => None,
            });
            Some((e.id?, height.unwrap_or(f64::NAN)))
        })
        .collect();
    let declared: BTreeSet<u32> =
        rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let sketch_lines =
        scan_category_records_multi(&mut rf, version, &[OST_SKETCH_LINES], &declared)?;
    // RE-97: where a roof's Sketch lists its curves, that list is its
    // membership; a line's owner reference names another element once the
    // sketch is edited.
    let all_lines: BTreeSet<u32> = sketch_lines.iter().map(|l| l.element_id).collect();
    let lists =
        rvt::element_record_plan_profiles::scan_sketch_curve_lists(&mut rf, version, &all_lines);
    let mut owned: BTreeMap<u32, u32> = BTreeMap::new();
    for line in &sketch_lines {
        if let Some(owner) = line
            .owner_reference
            .filter(|o| roofs.contains_key(o) && !lists.contains_key(o))
        {
            owned.insert(line.element_id, owner);
        }
    }
    for (owner, curves) in lists.iter().filter(|(o, _)| roofs.contains_key(o)) {
        println!("sketch list of roof {owner}: {curves:?}");
        owned.extend(curves.iter().map(|&c| (c, *owner)));
    }
    let ids: BTreeSet<u32> = owned.keys().copied().collect();
    let bounded = rvt::partition_beam_axes::scan_sketch_bounded_lines(&mut rf, version, &ids)?;
    println!("release {version}: roofs {roofs:?}");
    let roof_ids: BTreeSet<u32> = roofs.keys().copied().collect();
    println!(
        "slope arrows: {:?}",
        rvt::partition_roof_slopes::scan_arrow_heights(&mut rf, version, &ids)?
    );
    println!(
        "rafter cuts: {:?}",
        rvt::partition_roof_slopes::scan_rafter_cuts(&mut rf, version, &roof_ids)
    );
    println!(
        "fascia depths: {:?}",
        rvt::partition_roof_slopes::scan_fascia_depths(&mut rf, version, &roof_ids)?
    );
    let mut seen: BTreeSet<(u32, Vec<u8>)> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = buf
                .get(id_at..id_at + 8)
                .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| owned.contains_key(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit + 0x1000)
                .min(buf.len());
            let data = &buf[id_at + 8..end];
            for at in memchr::memmem::find_iter(data, &[0xffu8; 32]) {
                let Some(block) = data.get(at.saturating_sub(0x18)..(at + 0x48).min(data.len()))
                else {
                    continue;
                };
                if !seen.insert((id, block.to_vec())) {
                    continue;
                }
                let angle = at.checked_sub(0x18).and_then(|p| f64_at(data, p));
                let small = data
                    .get(at + 0x20..at + 0x28)
                    .map(|s| i64::from_le_bytes(s.try_into().expect("8")));
                println!(
                    "roof {} line {id} @+{at:#x} ({stream}): angle {:?} rad ({:.4}°) i64 {:?} u32 {:?} {:?} flags {} +0x3b {:?} -> {:?}",
                    owned[&id],
                    angle,
                    angle.map(f64::to_degrees).unwrap_or(f64::NAN),
                    small,
                    data.get(at + 0x28..at + 0x2c)
                        .map(|s| u32::from_le_bytes(s.try_into().expect("4"))),
                    data.get(at + 0x2c..at + 0x30)
                        .map(|s| u32::from_le_bytes(s.try_into().expect("4"))),
                    data.get(at + 0x30..at + 0x34).map(hex).unwrap_or_default(),
                    f64_at(data, at + 0x3b),
                    edge_slope_in(data, version),
                );
                println!(
                    "    after: {}",
                    hex(&data[at + 0x20..(at + 0x60).min(data.len())])
                );
            }
        }
    }
    for (id, line) in &bounded {
        println!("bounded line {id}: {line:?}");
    }
    Ok(())
}
