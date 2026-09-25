//! RE-75: a curved wall's data stores its location arc.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a wall's
//! element data opens its curves with the record RE-49 reads as a bounded
//! line (`04 00 08 01`). A `u64` in the 8 bytes before the tag says which
//! curve it is: 0 for a line, 1 for an arc. An arc's record holds its start
//! and end angle, unit X and Y axes, radius and centre. On the 24 walls
//! Revit's IFC4 export draws with an arc axis, the arc's centre and radius
//! are that axis's to 1e-5 ft. Read as a line, the arc's two axes gave a
//! line 0.15 to 17 ft long near the model's origin.
//!
//! The probe prints one JSON object per wall whose first curve record is an
//! arc: `id`, `centre` and `radius` in model feet, `angles` (start and end,
//! radians), the plan `x_axis` and `y_axis`, and the element record's
//! `box`. `tools/re/wall_arcs_vs_ifc.py` compares them with Revit's axes.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re75_wall_arcs -- MODEL.rvt > arcs.jsonl

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let boxes: BTreeMap<u32, [f64; 6]> =
        per::scan_category_records(&mut rf, version, per::OST_WALLS, &declared)?
            .into_iter()
            .map(|record| (record.element_id, record.bbox_feet))
            .collect();
    let walls: BTreeSet<u32> = boxes.keys().copied().collect();
    let arcs = rvt::partition_compound_structure::scan_wall_arcs(&mut rf, version, &walls)?;
    for (id, arc) in &arcs {
        let b = boxes[id];
        println!(
            "{{\"id\":{id},\"centre\":[{},{},{}],\"radius\":{},\"angles\":[{},{}],\"x_axis\":[{},{}],\"y_axis\":[{},{}],\"box\":[{},{},{},{},{},{}]}}",
            arc.centre[0],
            arc.centre[1],
            arc.centre[2],
            arc.radius,
            arc.start_angle,
            arc.end_angle,
            arc.x_axis[0],
            arc.x_axis[1],
            arc.y_axis[0],
            arc.y_axis[1],
            b[0],
            b[1],
            b[2],
            b[3],
            b[4],
            b[5]
        );
    }
    eprintln!(
        "Revit {version}: {} of {} walls store an arc",
        arcs.len(),
        walls.len()
    );
    Ok(())
}
