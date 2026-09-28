//! RE-106: a curtain mullion's or panel's body is the box along its own
//! axes whose world box is its record box.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, every
//! curtain mullion and panel whose transform (RE-87) is read has a record
//! box that is the axis-aligned box of a box along the transform's X, Y and
//! Z axes, centred on the record box: the half-extents `e` along those axes
//! solve `sum_i |axis_i| e_i = h`, `h` the record box's half-extents, and
//! Revit's own IFC4 mesh has exactly those extents along those axes
//! (`tools/re/oriented_boxes_vs_ifc.py`).
//!
//! The probe counts, per class, the instances whose transform is read, those
//! on the model's axes (whose record box is already their body's box), and
//! those turned or tilted off them, split by whether the solve gives
//! positive half-extents.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re106_oriented_boxes -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::{element_record_bbox, recover_partition_schema_mvp};
use rvt::walker::WalkerLimits;
use std::collections::{BTreeMap, BTreeSet};

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let elements: Vec<_> = mvp
        .products
        .iter()
        .filter(|e| e.class == "CurtainWallMullion" || e.class == "CurtainWallPanel")
        .collect();
    let ids: BTreeSet<u32> = elements.iter().filter_map(|e| e.id).collect();
    let transforms =
        rvt::partition_instance_transforms::scan_instance_transforms(&mut rf, version, &ids)?;
    let mut census: BTreeMap<(String, &str), usize> = BTreeMap::new();
    for element in &elements {
        let key = (|| {
            let bbox = element_record_bbox(element)?;
            let Some(transform) = element.id.and_then(|id| transforms.get(&id)) else {
                return Some("no transform read");
            };
            let axes = transform.axes;
            if axes
                .iter()
                .all(|axis| axis.iter().filter(|v| v.abs() > 1e-4).count() == 1)
            {
                return Some("on the model's axes");
            }
            let m = [0, 1, 2].map(|row| [0, 1, 2].map(|col| axes[col][row].abs()));
            let det = det3(m);
            if det.abs() < 0.1 {
                return Some("turned or tilted; axes leave the box undetermined");
            }
            let h = [0, 1, 2].map(|axis| (bbox[axis + 3] - bbox[axis]) / 2.0);
            let half = [0, 1, 2].map(|col| {
                let mut replaced = m;
                for row in 0..3 {
                    replaced[row][col] = h[row];
                }
                det3(replaced) / det
            });
            Some(if half.iter().all(|e| *e > 0.0) {
                "turned or tilted; solved"
            } else {
                "turned or tilted; no positive solve"
            })
        })()
        .unwrap_or("no record box");
        *census.entry((element.class.clone(), key)).or_default() += 1;
    }
    println!(
        "release {version}: {} curtain mullions and panels",
        elements.len()
    );
    for ((class, what), count) in &census {
        println!("  {count:>5}  {class}: {what}");
    }
    Ok(())
}
