//! RE-444 (probe): which frame of an element framed in several partitions is the
//! current one, against the one the exporter keeps.
//!
//! FACT under test: where an element is framed in more than one partition and the
//! frames disagree on the box, the frame in the partition its stored revision
//! routes to (`native_document::current_partitions`, RE-216), which is also the
//! newest frame, is the one Revit's export agrees with. The exporter keeps the
//! frame with the greatest vertical extent and, on a tie, the newest
//! (`select_instance_records`, RE-22), which on an element that was made shorter
//! after an earlier save is the older frame.
//!
//! Output, one line per element with more than one frame of an exported
//! instance (the frames the selector reads), then totals:
//!
//!   file <path> release <release>
//!   element <id> <category> box_disagree <0|1> z_disagree <0|1> selected <p> newest <p> current <p|-> copies <p>/<offset>/<dx>/<dy>/<dz>/<min z> ...
//!   summary elements <n> multi_framed <n> box_disagree <n> z_disagree <n> selected_newest <n> selected_current <n> newest_current <n>
//!
//! The box is in feet and in the model's frame; `p` is the number of the
//! `Partitions/NN` stream; `current` is `-` where the element index routes the
//! element to no partition that holds a frame of it. The totals of the last
//! line count the elements whose frames disagree on the vertical extent.
//!
//! `tools/re/frame_copies_vs_ifc.py` sets each frame's vertical extent against
//! Revit's body.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re444_frame_copies -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document;
use rvt::research::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

/// Categories read beside the product ones.
const EXTRA_CATEGORIES: [i64; 14] = [
    per::OST_WALLS,
    per::OST_FLOORS,
    per::OST_DOORS,
    per::OST_WINDOWS,
    per::OST_COLUMNS,
    per::OST_STRUCTURAL_COLUMNS,
    per::OST_STRUCTURAL_FRAMING,
    per::OST_CURTAIN_WALL_MULLIONS,
    per::OST_CURTAIN_WALL_PANELS,
    per::OST_CEILINGS,
    per::OST_ROOFS,
    per::OST_RAMPS,
    per::OST_STAIRS,
    per::OST_GENERIC_MODEL,
];

/// How far, feet, two frames' boxes may differ and still be one box.
const SAME_BOX_FEET: f64 = 1e-4;

/// The vertical extent of a frame as the selector compares it: quantised to 1e-4 ft.
fn z_key(record: &per::PartitionElementRecord) -> i64 {
    let (_, _, dz) = record.extents_feet();
    if dz.is_finite() {
        (dz * 10_000.0).round() as i64
    } else {
        i64::MIN
    }
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("file {path} release {version}");
    if !per::supports_revit_version(version) {
        println!("not read: release {version} is outside the element records' range");
        return Ok(());
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let mut categories: Vec<i64> = per::PRODUCT_RECORD_CATEGORIES
        .iter()
        .map(|(id, _)| *id)
        .collect();
    categories.extend(EXTRA_CATEGORIES);
    categories.sort_unstable();
    categories.dedup();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let mut by_id: BTreeMap<u32, Vec<per::PartitionElementRecord>> = BTreeMap::new();
    for record in records {
        if record.is_exported_instance() && record.has_volume() {
            by_id.entry(record.element_id).or_default().push(record);
        }
    }
    let holders: BTreeMap<u32, BTreeSet<u32>> = by_id
        .iter()
        .filter(|(_, frames)| frames.len() > 1)
        .map(|(id, frames)| {
            (
                *id,
                frames.iter().map(|frame| frame.frame_order().0).collect(),
            )
        })
        .collect();
    let current = native_document::current_partitions(&mut rf, &holders);
    let (mut box_disagree, mut z_disagree) = (0usize, 0usize);
    let (mut selected_newest, mut selected_current, mut newest_current) = (0usize, 0usize, 0usize);
    for (id, frames) in by_id.iter().filter(|(_, frames)| frames.len() > 1) {
        let first = &frames[0];
        let boxes_differ = frames.iter().any(|frame| {
            frame
                .bbox_feet
                .iter()
                .zip(&first.bbox_feet)
                .any(|(a, b)| (a - b).abs() > SAME_BOX_FEET)
        });
        let z_differs = frames.iter().any(|frame| {
            z_key(frame) != z_key(first)
                || (frame.bbox_feet[2] - first.bbox_feet[2]).abs() > SAME_BOX_FEET
        });
        let selected = frames
            .iter()
            .max_by(|a, b| {
                z_key(a)
                    .cmp(&z_key(b))
                    .then_with(|| a.frame_order().cmp(&b.frame_order()))
            })
            .expect("frames is not empty");
        let newest = frames
            .iter()
            .max_by(|a, b| a.frame_order().cmp(&b.frame_order()))
            .expect("frames is not empty");
        let routed = current.get(id).copied();
        if z_differs {
            z_disagree += 1;
            selected_newest += usize::from(selected.frame_order() == newest.frame_order());
            selected_current += usize::from(Some(selected.frame_order().0) == routed);
            newest_current += usize::from(Some(newest.frame_order().0) == routed);
        }
        box_disagree += usize::from(boxes_differ);
        let mut ordered: Vec<&per::PartitionElementRecord> = frames.iter().collect();
        ordered.sort_by(|a, b| a.frame_order().cmp(&b.frame_order()));
        let copies: Vec<String> = ordered
            .iter()
            .map(|frame| {
                let (dx, dy, dz) = frame.extents_feet();
                format!(
                    "{}/{}/{dx:.5}/{dy:.5}/{dz:.5}/{:.5}",
                    frame.frame_order().0,
                    frame.offset,
                    frame.bbox_feet[2]
                )
            })
            .collect();
        println!(
            "element {id} {} box_disagree {} z_disagree {} selected {} newest {} current {} copies {}",
            per::builtin_category_name(first.builtin_category)
                .unwrap_or("?")
                .replace(' ', "_"),
            u8::from(boxes_differ),
            u8::from(z_differs),
            selected.frame_order().0,
            newest.frame_order().0,
            routed.map_or("-".to_string(), |p| p.to_string()),
            copies.join(" ")
        );
    }
    let multi = holders.len();
    println!(
        "summary elements {} multi_framed {multi} box_disagree {box_disagree} z_disagree {z_disagree} selected_newest {selected_newest} selected_current {selected_current} newest_current {newest_current}",
        by_id.len()
    );
    Ok(())
}
