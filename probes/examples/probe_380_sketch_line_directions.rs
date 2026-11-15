//! RE-380 follow-up (#1987): the direction each sketch line is drawn in, to
//! test whether it decides which line of a profile pocket's stretch Revit's
//! export names as the opening's Tag.
//!
//! FACT under test: RE-380 found that Revit tags a wall-profile pocket's
//! opening with the first line of its stretch in a counter-clockwise walk on 12
//! of 15 pockets and the last on the 3 of Snowdon Towers' wall 698364, with no
//! byte that told them apart (polygon winding, extrusion direction, profile
//! axis and the sketch's list order were checked). On 698364 every line of the
//! sketch is drawn in the walk's direction and Revit names the line whose own
//! end is the stretch's end. The hypothesis is that Revit names that line on
//! all 15: the stretch's last line in the direction its lines are drawn.
//!
//! Output, one line per `CurveElem` whose curve is a straight `GLine`:
//!   line <id> sketch <SketchMembership group id> a <x> <y> <z> b <x> <y> <z>
//! a and b are the line's ends in the order it is drawn (origin + direction x
//! each end parameter), model feet.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_380_sketch_line_directions -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

fn triple(value: Option<&Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    match items.as_slice() {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?]),
        _ => None,
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_380_sketch_line_directions MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: BTreeSet::from(["CurveElem".to_string()]),
        ..Options::default()
    };
    let mut count = 0usize;
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let mut sketch = -1i64;
        let mut ends = None;
        for object in &graph.objects {
            let fields = object.fields.to_json();
            match object.class_name.as_str() {
                "SketchMembership" => {
                    sketch = fields
                        .pointer("/m_groupId/m_id/m_id64")
                        .or_else(|| fields.pointer("/m_groupId/m_id/m_id"))
                        .and_then(Value::as_i64)
                        .unwrap_or(-1);
                }
                "GLine" if ends.is_none() => {
                    let origin = triple(fields.get("m_origin"));
                    let direction = triple(fields.get("m_dirVec"));
                    let params = fields.get("m_endParams").and_then(Value::as_array);
                    if let (Some(o), Some(d), Some([t0, t1])) =
                        (origin, direction, params.map(Vec::as_slice))
                    {
                        if let (Some(t0), Some(t1)) = (t0.as_f64(), t1.as_f64()) {
                            let at = |t: f64| [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t];
                            ends = Some((at(t0), at(t1)));
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some((a, b)) = ends {
            if sketch >= 0 {
                count += 1;
                println!(
                    "line {id} sketch {sketch} a {:.5} {:.5} {:.5} b {:.5} {:.5} {:.5}",
                    a[0], a[1], a[2], b[0], b[1], b[2]
                );
            }
        }
        Ok(())
    })?;
    eprintln!("lines {count}");
    Ok(())
}
