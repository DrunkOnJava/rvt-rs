//! RE-330: the curve an area boundary line stores, beside its record box.
//!
//! FACT under test: an area boundary line's element record holds its bounding
//! box and nothing of its curve (RE-317), so a line that is not along a plan
//! axis (a slanted line, an arc) cannot be told apart from another by its
//! record. Its curve is in its element data, as a sketch line's or a wall's is
//! (RE-49, RE-50, RE-96): a record that opens with `04 00 08 01`, read as a
//! bounded line or as a circular arc. Autodesk's `rac_advanced` has four such
//! lines on 2015 to 2024 saves (177363, 177377 and 178310 on its first Level,
//! 178788 on its second, ids of the 2024 save), and Revit's own export of the
//! project draws the Areas around them (RE-330).
//!
//! This probe prints, tab-separated, for every area boundary line of the file:
//!
//! - `line`: the element record, as RE-317's probe prints it (id, category,
//!   stream, offset, class tag, container, placement kind, box, owner,
//!   reference list, design option);
//! - `copy`: each copy of the line's element data: partition, offset of its
//!   header, the window searched (to the next header, at most 64 KiB) and the
//!   number of `04 00 08 01` records in it;
//! - `curve`: each such record: its offset in the window, the `u64` before it
//!   (1 marks an arc, RE-75; a sketch line's arc is not marked, RE-96), its
//!   reading as a bounded line (start and end points, then the parameters,
//!   origin and direction) and as an arc (angles, radius, centre, axes), and
//!   whether the line reading's ends lie in the record box (1e-6, RE-49);
//! - `data`: the first bytes of the first copy's data window after the ElementId
//!   of the lines whose box is not a line along an axis, and of the first four
//!   lines;
//! - `nodata`: a line with no element data found.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re330_area_line_curves -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_beam_axes::{BOUNDED_LINE_TAG, arc_values_at, bounded_line_at};
use rvt::research::partition_element_records::{
    PartitionElementRecord, scan_category_records_multi,
};
use rvt::research::partition_element_records_2023 as per32;
use rvt::research::partition_level_records::recover_partition_levels;
use rvt::research::partition_names::element_data_layout;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const OST_AREA_SCHEME_LINES: i64 = -2_000_079;

/// The most bytes of an element's data searched for its curves.
const DATA_WINDOW: usize = 0x1_0000;
/// The most curve records printed for one copy.
const CURVES_PER_COPY: usize = 6;
/// Bytes of a window printed after the ElementId.
const HEX_BYTES: usize = 192;
/// Lines whose data is printed whatever their box.
const SAMPLED_LINES: usize = 4;
/// How far a line's end may lie outside the record box, feet.
const BOX_TOLERANCE: f64 = 1e-6;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn partition_number(stream: &str) -> u32 {
    stream
        .rsplit('/')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn join_f64(values: &[f64]) -> String {
    values
        .iter()
        .map(|v| format!("{v:.9}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Whether a box is a line along an axis, or a point: the two cases an area
/// boundary line's box is the line itself.
fn is_axis_box(bbox: &[f64; 6]) -> bool {
    let dx = (bbox[3] - bbox[0]).abs();
    let dy = (bbox[4] - bbox[1]).abs();
    dx <= 1e-5 || dy <= 1e-5
}

fn in_box(bbox: &[f64; 6], point: [f64; 3]) -> bool {
    (0..3).all(|axis| {
        point[axis] >= bbox[axis] - BOX_TOLERANCE && point[axis] <= bbox[axis + 3] + BOX_TOLERANCE
    })
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("release {version}");
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    for level in recover_partition_levels(&mut rf, version)? {
        println!(
            "level\t{}\t{}\t{:.6}\t{:?}",
            level.element_id, level.name, level.elevation_feet, level.building_story
        );
    }

    let records: Vec<PartitionElementRecord> = if per32::is_32bit_release(version) {
        per32::scan_records(&mut rf, version)
            .into_iter()
            .filter(|record| record.builtin_category == OST_AREA_SCHEME_LINES)
            .collect()
    } else {
        scan_category_records_multi(&mut rf, version, &[OST_AREA_SCHEME_LINES], &declared)?
    };
    println!("lines\t{}", records.len());
    let join = |ids: &[u64]| ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    let mut boxes: BTreeMap<u32, [f64; 6]> = BTreeMap::new();
    let mut hexed: BTreeSet<u32> = BTreeSet::new();
    for record in &records {
        println!(
            "line\t{}\t{}\t{}\t{}\t{}\t{}\t{:#x}\t{}\t{}\t{}\t{}",
            record.element_id,
            record.builtin_category,
            record.stream,
            record.offset,
            record.class_tag,
            record
                .container_element_id()
                .map_or("-".to_string(), |c| c.to_string()),
            record.placement_kind,
            record
                .bbox_feet
                .iter()
                .map(|v| format!("{v:.6}"))
                .collect::<Vec<_>>()
                .join(","),
            record
                .owner_reference
                .map_or("-".to_string(), |o| o.to_string()),
            join(&record.references),
            record
                .design_option
                .map_or("-".to_string(), |d| d.to_string()),
        );
        boxes.insert(record.element_id, record.bbox_feet);
    }
    for (index, (id, bbox)) in boxes.iter().enumerate() {
        if index < SAMPLED_LINES || !is_axis_box(bbox) {
            hexed.insert(*id);
        }
    }

    let Some(layout) = element_data_layout(version) else {
        println!("no element data layout for release {version}");
        return Ok(());
    };
    let ids: BTreeSet<u32> = boxes.keys().copied().collect();
    let mut with_data: BTreeSet<u32> = BTreeSet::new();
    let mut printed_hex: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &layout.header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + layout.header.len();
            let Some(id) = layout.id_at(buf, id_at).filter(|id| ids.contains(id)) else {
                continue;
            };
            with_data.insert(id);
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(DATA_WINDOW))
                .min(buf.len());
            let Some(window) = buf.get(id_at + 8..end) else {
                continue;
            };
            let tags: Vec<usize> = memchr::memmem::find_iter(window, &BOUNDED_LINE_TAG).collect();
            println!(
                "copy\t{id}\t{}\t{hit}\t{}\t{}",
                partition_number(&stream),
                window.len(),
                tags.len()
            );
            let bbox = boxes[&id];
            for &at in tags.iter().take(CURVES_PER_COPY) {
                let kind = at
                    .checked_sub(8)
                    .and_then(|from| u64_at(window, from))
                    .map_or("-".to_string(), |k| k.to_string());
                let line = bounded_line_at(window, at).map_or("-".to_string(), |l| {
                    format!(
                        "L {} | {} | {} | {}",
                        join_f64(&l.start()),
                        join_f64(&l.end()),
                        join_f64(&[
                            l.start_parameter,
                            l.end_parameter,
                            l.origin[0],
                            l.origin[1],
                            l.origin[2],
                            l.direction[0],
                            l.direction[1],
                            l.direction[2],
                        ]),
                        in_box(&bbox, l.start()) && in_box(&bbox, l.end())
                    )
                });
                let arc = arc_values_at(window, at).map_or("-".to_string(), |a| {
                    format!(
                        "A {} | {}",
                        join_f64(&[
                            a.start_angle,
                            a.end_angle,
                            a.radius,
                            a.centre[0],
                            a.centre[1],
                            a.centre[2],
                        ]),
                        join_f64(&[
                            a.x_axis[0],
                            a.x_axis[1],
                            a.x_axis[2],
                            a.y_axis[0],
                            a.y_axis[1],
                            a.y_axis[2],
                        ])
                    )
                });
                println!(
                    "curve\t{id}\t{}\t{at}\t{kind}\t{line}\t{arc}",
                    partition_number(&stream)
                );
            }
            if hexed.contains(&id) && printed_hex.insert(id) {
                println!(
                    "data\t{id}\t{}\t{}",
                    partition_number(&stream),
                    hex(&window[..window.len().min(HEX_BYTES)])
                );
            }
        }
    }
    for id in &ids {
        if !with_data.contains(id) {
            println!("nodata\t{id}");
        }
    }
    Ok(())
}
