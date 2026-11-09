//! #1607: the sketch lines of every wall whose sketch lists curves, with their
//! ElementIds and ends, so Revit's wall-profile openings can be set against the
//! lines they are formed from.
//!
//! FACT under test: RE-151's rule reads a wall's edited elevation profile as
//! openings, one for each run of lines that leaves the profile's bounding
//! rectangle, tagged with the run's highest line id. Revit's export writes an
//! opening on the BIM4LCA terrain wall 1387731 tagged 1389018, which is not the
//! highest id of its top's four lines, and none on wall 1351524. This prints, per
//! wall, its record box and each listed sketch line's id and ends (model feet,
//! in the order the sketch lists them), so the ids can be read against the
//! shapes.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_1607_wall_profile_lines -- MODEL.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::scan_sketch_curve_lists;
use rvt::research::partition_element_records as per;

fn main() -> rvt::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_1607_wall_profile_lines <file.rvt>");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let walls = per::scan_category_records(&mut rf, version, per::OST_WALLS, &declared)?;
    let wall_boxes: BTreeMap<u32, [f64; 6]> = walls
        .iter()
        .map(|wall| (wall.element_id, wall.bbox_feet))
        .collect();
    let lines =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let line_ids: BTreeSet<u32> = lines.iter().map(|record| record.element_id).collect();
    let lists = scan_sketch_curve_lists(&mut rf, version, &line_ids);
    let wanted: BTreeSet<u32> = lists
        .iter()
        .filter(|(owner, _)| wall_boxes.contains_key(*owner))
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    let curves = rvt::research::partition_beam_axes::scan_sketch_curves(&mut rf, version, &wanted)?;
    let mut printed = 0usize;
    for (owner, ids) in &lists {
        let Some(wall_box) = wall_boxes.get(owner) else {
            continue;
        };
        printed += 1;
        println!(
            "wall {owner} box ({:.4}, {:.4}, {:.4}) ({:.4}, {:.4}, {:.4}) lines {}",
            wall_box[0],
            wall_box[1],
            wall_box[2],
            wall_box[3],
            wall_box[4],
            wall_box[5],
            ids.len()
        );
        for id in ids {
            match curves.get(id) {
                Some(curve) => {
                    if let Some(line) = curve.line {
                        let (a, b) = (line.start(), line.end());
                        println!(
                            "  line {id} ({:.4}, {:.4}, {:.4}) ({:.4}, {:.4}, {:.4})",
                            a[0], a[1], a[2], b[0], b[1], b[2]
                        );
                    } else if curve.arc.is_some() {
                        println!("  line {id} arc");
                    } else {
                        println!("  line {id} unread");
                    }
                }
                None => println!("  line {id} not found"),
            }
        }
    }
    println!("{printed} walls list sketch curves in release {version}");
    Ok(())
}
