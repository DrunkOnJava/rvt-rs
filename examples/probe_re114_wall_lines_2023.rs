//! RE-114: a Revit 2023 wall's data holds its location line and its
//! orientation as 2024's does (RE-49, RE-53).
//!
//! FACT: after a 2023 wall's element-data header and `u32` ElementId, the
//! first bounded line record is the wall's centreline, and the words after
//! `ff ff ff ff 01 00 00 00` are its location-line setting, its
//! cross-section word and its flip flag, as on 2024.
//!
//! The probe prints each exported 2023 wall's line (feet, model axes) and
//! orientation; compare them with the `Axis` Revit's own export gives the
//! wall and the side its layer set's first layer lies on.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re114_wall_lines_2023 -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::WalkerLimits;
use std::collections::BTreeSet;

fn main() {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())
        .expect("partition records");
    let walls: BTreeSet<u32> = mvp.walls.iter().filter_map(|wall| wall.id).collect();
    let lines = rvt::partition_beam_axes::scan_first_bounded_lines(&mut rf, version, &walls)
        .expect("lines");
    let orientations =
        rvt::partition_compound_structure::scan_wall_orientations(&mut rf, version, &walls)
            .expect("orientations");
    for id in &walls {
        let line = lines.get(id).map(|line| {
            let at = |t: f64| {
                [
                    line.origin[0] + t * line.direction[0],
                    line.origin[1] + t * line.direction[1],
                    line.origin[2] + t * line.direction[2],
                ]
            };
            (at(line.start_parameter), at(line.end_parameter))
        });
        println!("{id}\t{line:?}\t{:?}", orientations.get(id));
    }
}
