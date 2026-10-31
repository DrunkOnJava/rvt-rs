//! RE-290 (#1046, W7): each wall's centreline cut back at its joins, beside
//! its curve, its confirmed `Length` and its record box.
//!
//! FACT under test: on a plain wall (a rectangle in plan whose native record
//! holds no step but its joins and a roof or floor join), the longest
//! vertical face of Revit's solid is as long as the wall's `Axis`, its
//! centreline cut back where its body meets the walls joined at its ends
//! (RE-252), and that is what Revit's export writes as the wall's `Length`
//! wherever it is not the curve's. The box of a wall whose joins the record
//! box cannot cut like Revit's is longer or shorter, and the cut-back length
//! is not.
//!
//! Output, one line per wall: `wallcut <id> curve <feet> cutback <feet>
//! length <feet> box <width> <depth>`, each `-` where the partition reader
//! gives none. `tools/re/wall_quantities_vs_ifc.py` prints Revit's own
//! quantities by the same `Tag`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re290_wall_cutbacks -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::{
    WALL_CURVE_LENGTH_FIELD, WALL_CUTBACK_LENGTH_FIELD, WALL_LENGTH_FIELD,
    recover_partition_schema_mvp,
};
use rvt::walker::{InstanceField, WalkerLimits};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    for wall in &mvp.walls {
        let Some(id) = wall.id else {
            continue;
        };
        let float = |wanted: &str| {
            wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let show = |value: Option<f64>| value.map_or_else(|| "-".to_string(), |v| v.to_string());
        println!(
            "wallcut {id} curve {} cutback {} length {} box {} {}",
            show(float(WALL_CURVE_LENGTH_FIELD)),
            show(float(WALL_CUTBACK_LENGTH_FIELD)),
            show(float(WALL_LENGTH_FIELD)),
            show(float("m_bboxWidth")),
            show(float("m_bboxDepth")),
        );
    }
    Ok(())
}
