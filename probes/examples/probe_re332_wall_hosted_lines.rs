//! RE-332: where an area boundary line lies against the wall its record names.
//!
//! FACT under test: an area boundary line that was picked from a wall names that
//! wall in its reference list (RE-317 saw an id past the Level and the scheme
//! that is not a line; RE-330 saw the same ids in the walls' footprints). The
//! line's record box then says where Revit draws the line against the wall's own
//! record box:
//!
//! - `centre`: the line lies on the wall's centre line (interior partitions,
//!   200 mm generic walls);
//! - `face`: the line lies half the wall's thickness off its centre line, on one
//!   of its two faces (the exterior masonry wall, the 225 mm concrete wall);
//! - `compound`: the line's box is as wide as half the wall's thickness, from
//!   the centre line to one face. Revit draws such a line on the face for one
//!   stretch and on the centre line for another, and its record keeps the
//!   union (RE-330 section 5: lines 177377 and 177363 of `rac_advanced`);
//! - `other`: neither (a curtain wall's line lies 0.041 ft off its axis).
//!
//! For a compound line the probe also prints the stretches of it that lie
//! beyond the host wall's extent, by more than one wall thickness: the stretches
//! where no wall face can move the line off its centre line.
//!
//! Prints, tab-separated:
//!
//! - `wall`: a wall record a line names: id, stream, box, class tag;
//! - `host`: a line, the wall it names, the wall's box and thickness, the
//!   line's two edges across the wall as offsets from the wall's centre line,
//!   and the kind;
//! - `compound`: a compound line with its extent along the wall, the wall's
//!   extent and the stretches beyond it;
//! - `nohost`: a line that names no wall record;
//! - `counts`: lines by kind, per wall type of the line's host.
//!
//! `tools/re/wall_lines_vs_offsets.py` joins this output with the offsets the file stores for
//! each line (`probe_re333_area_plan_topology`).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re332_wall_hosted_lines -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::{
    OST_WALLS, PartitionElementRecord, scan_category_records_multi,
};
use rvt::research::partition_element_records_2023 as per32;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const OST_AREA_SCHEME_LINES: i64 = -2_000_079;

/// A wall record's box, the stream it was found in and its class tag.
type WallRecord = ([f64; 6], String, u16);

/// How close two positions across a wall must be to be the same, feet. The
/// measured lines are within 5e-7 ft, and the nearest line that is neither on a
/// centre line nor on a face is 0.041 ft off (RE-332 section 6).
const SAME: f64 = 0.001;
/// A line this thin across is a line, not a box.
const THIN: f64 = 1e-5;

fn records(
    rf: &mut RevitFile,
    version: u32,
    declared: &BTreeSet<u32>,
    category: i64,
) -> rvt::Result<Vec<PartitionElementRecord>> {
    if per32::is_32bit_release(version) {
        Ok(per32::scan_records(rf, version)
            .into_iter()
            .filter(|record| record.builtin_category == category)
            .collect())
    } else {
        scan_category_records_multi(rf, version, &[category], declared)
    }
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("release {version}");
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let lines = records(&mut rf, version, &declared, OST_AREA_SCHEME_LINES)?;
    let walls = records(&mut rf, version, &declared, OST_WALLS)?;
    println!("lines\t{}\twalls\t{}", lines.len(), walls.len());

    let mut wall_boxes: BTreeMap<u32, WallRecord> = BTreeMap::new();
    let mut disagree: BTreeSet<u32> = BTreeSet::new();
    for wall in &walls {
        match wall_boxes.get(&wall.element_id) {
            None => {
                wall_boxes.insert(
                    wall.element_id,
                    (wall.bbox_feet, wall.stream.clone(), wall.class_tag),
                );
            }
            Some((held, _, _)) if *held != wall.bbox_feet => {
                disagree.insert(wall.element_id);
            }
            Some(_) => {}
        }
    }
    println!("walls with copies that disagree\t{}", disagree.len());

    let mut shown: BTreeSet<u32> = BTreeSet::new();
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut seen_lines: BTreeSet<u32> = BTreeSet::new();
    for line in &lines {
        if !seen_lines.insert(line.element_id) {
            continue;
        }
        let [x0, y0, _, x1, y1, _] = line.bbox_feet;
        let hosts: Vec<u32> = line
            .references
            .iter()
            .filter_map(|r| u32::try_from(*r).ok())
            .filter(|id| wall_boxes.contains_key(id))
            .collect();
        if hosts.is_empty() {
            println!("nohost\t{}", line.element_id);
            *counts.entry("nohost").or_default() += 1;
            continue;
        }
        for host in hosts {
            let (wall, stream, tag) = &wall_boxes[&host];
            if shown.insert(host) {
                println!(
                    "wall\t{host}\t{stream}\t{}\t{tag}",
                    wall.iter()
                        .map(|v| format!("{v:.6}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            let (wdx, wdy) = (wall[3] - wall[0], wall[4] - wall[1]);
            let wall_horizontal = wdx > wdy;
            let thickness = if wall_horizontal { wdy } else { wdx };
            let centre = if wall_horizontal {
                (wall[1] + wall[4]) / 2.0
            } else {
                (wall[0] + wall[3]) / 2.0
            };
            let (wall_lo, wall_hi) = if wall_horizontal {
                (wall[0], wall[3])
            } else {
                (wall[1], wall[4])
            };
            let (across_lo, across_hi, along_lo, along_hi) = if wall_horizontal {
                (y0.min(y1), y0.max(y1), x0.min(x1), x0.max(x1))
            } else {
                (x0.min(x1), x0.max(x1), y0.min(y1), y0.max(y1))
            };
            let width = across_hi - across_lo;
            let (low_off, high_off) = (across_lo - centre, across_hi - centre);
            let kind = if width <= THIN {
                if low_off.abs() <= SAME {
                    "centre"
                } else if (low_off.abs() - thickness / 2.0).abs() <= SAME {
                    "face"
                } else {
                    "other"
                }
            } else if (width - thickness / 2.0).abs() <= SAME
                && (low_off.abs() <= SAME || high_off.abs() <= SAME)
            {
                "compound"
            } else {
                "other"
            };
            *counts.entry(kind).or_default() += 1;
            println!(
                "host\t{}\t{host}\t{}\t{thickness:.6}\t{}\t{low_off:.6}\t{high_off:.6}\t{kind}",
                line.element_id,
                if wall_horizontal { "h" } else { "v" },
                line.bbox_feet
                    .iter()
                    .map(|v| format!("{v:.6}"))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            if kind == "compound" {
                let before = (along_lo, along_hi.min(wall_lo - thickness));
                let after = (along_lo.max(wall_hi + thickness), along_hi);
                let stretches: Vec<String> = [before, after]
                    .iter()
                    .filter(|(lo, hi)| hi - lo > THIN)
                    .map(|(lo, hi)| format!("{lo:.6}..{hi:.6}"))
                    .collect();
                println!(
                    "compound\t{}\t{host}\tline {along_lo:.6}..{along_hi:.6}\twall {wall_lo:.6}..{wall_hi:.6}\tbeyond one thickness\t{}",
                    line.element_id,
                    if stretches.is_empty() {
                        "-".to_string()
                    } else {
                        stretches.join(" ")
                    }
                );
            }
        }
    }
    for (kind, count) in counts {
        println!("counts\t{kind}\t{count}");
    }
    Ok(())
}
