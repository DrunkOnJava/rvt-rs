//! RE-87: a family instance's data holds its transform.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a family
//! instance's element data holds twelve `f64`: a 3 × 3 rotation stored row
//! by row, then its origin in model feet. The first place in the data where
//! nine `f64` are orthonormal with determinant 1 is it. Where its Z axis is
//! the model's, its X axis is Revit's IFC4 placement X, its reverse, or a
//! right angle from it, on every such instance in Revit's export; so is
//! RE1 Architecture's (Revit 2025).
//!
//! The probe prints one JSON object per record-backed element whose data
//! holds a transform: `id`, `x`, `y`, `z` (unit axes) and `origin`.
//! `tools/re/instance_transforms_vs_ifc.py` compares them with Revit's
//! placements.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re87_instance_transforms -- MODEL.rvt > transforms.jsonl

use rvt::RevitFile;
use rvt::partition_instance_transforms as pit;
use std::collections::BTreeSet;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let transforms = pit::scan_instance_transforms(&mut rf, version, &declared)?;
    let ids: BTreeSet<u32> = transforms.keys().copied().collect();
    for (id, t) in &transforms {
        let [x, y, z] = t.axes;
        println!(
            "{{\"id\":{id},\"x\":[{},{},{}],\"y\":[{},{},{}],\"z\":[{},{},{}],\"origin\":[{},{},{}]}}",
            x[0],
            x[1],
            x[2],
            y[0],
            y[1],
            y[2],
            z[0],
            z[1],
            z[2],
            t.origin[0],
            t.origin[1],
            t.origin[2]
        );
    }
    let upright = transforms.values().filter(|t| t.is_upright()).count();
    eprintln!(
        "Revit {version}: {} elements hold a transform, {upright} upright",
        ids.len()
    );
    Ok(())
}
