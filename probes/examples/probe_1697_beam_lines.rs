//! #1697: which beams carry a location line in their element data, which of
//! those the exporter attaches, and what the line says of the beam's length,
//! slope and roll.
//!
//! FACT TO VERIFY: Revit's own export writes `Qto_BeamBaseQuantities.Length`
//! and `Pset_BeamCommon.Span` equal to the length of the beam's `Axis`
//! representation, `Slope` as that line's angle to the horizontal and `Roll`
//! as its Cross-Section Rotation, on every beam of the BIM4LCA office and
//! housing (142 of 142 for `Length` and `Span`), whether Revit draws the
//! beam as an extrusion along the line or as a mesh. rvt-rs attaches the line
//! only where the line and the beam's record box make a beam solid
//! (`attach_beam_axes`), so a beam drawn from a saved mesh has none of the
//! four. The probe prints, per structural-framing element, the first bounded
//! line of its element data (the one `scan_bounded_lines` reads), whether the
//! exporter attached it, whether a column cuts the beam, the rotation read,
//! and whether the line lies inside the record box and a box along it closes,
//! so that `tools/re/beam_lines_vs_ifc.py` can hold each against Revit's
//! export of the same beam by ElementId.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1697_beam_lines -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::element_record_beam_cuts::BEAM_BODY_SOURCE_FIELD;
use rvt::research::partition_beam_axes::{END_TOLERANCE_FEET, beam_body, scan_bounded_lines};
use rvt::research::partition_schema_mvp::{
    BEAM_ROTATION_FIELD, beam_axis_from_fields, element_record_bbox, recover_partition_schema_mvp,
};
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn rotation(element: &DecodedElement) -> Option<f64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == BEAM_ROTATION_FIELD => Some(*value),
        _ => None,
    })
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: probe_1697_beam_lines MODEL.rvt"),
    );
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let beams: Vec<&DecodedElement> = mvp
        .products
        .iter()
        .filter(|element| element.class == "StructuralFraming" && element.id.is_some())
        .collect();
    let ids: BTreeSet<u32> = beams.iter().filter_map(|beam| beam.id).collect();
    let lines = scan_bounded_lines(&mut rf, version, &ids)?;
    println!(
        "release {version}: {} structural-framing elements, {} with a bounded line",
        beams.len(),
        lines.len()
    );
    for beam in &beams {
        let id = beam.id.expect("filtered");
        let attached = beam_axis_from_fields(&beam.fields).is_some();
        let cut = beam
            .fields
            .iter()
            .any(|(name, _)| name == BEAM_BODY_SOURCE_FIELD);
        let rotation = rotation(beam).map_or("none".to_string(), |value| format!("{value:.9}"));
        let Some(line) = lines.get(&id) else {
            println!(
                "beam {id} line 0 attached {} cut {}",
                u8::from(attached),
                u8::from(cut)
            );
            continue;
        };
        let (start, end) = (line.start(), line.end());
        let length = (0..3)
            .map(|axis| (end[axis] - start[axis]).powi(2))
            .sum::<f64>()
            .sqrt();
        let (inside, closes) = match element_record_bbox(beam) {
            Some(bbox) => {
                let inside = [start, end].iter().all(|p| {
                    (0..3).all(|axis| {
                        p[axis] >= bbox[axis] - END_TOLERANCE_FEET
                            && p[axis] <= bbox[axis + 3] + END_TOLERANCE_FEET
                    })
                });
                (
                    u8::from(inside).to_string(),
                    u8::from(beam_body(bbox, start, end).is_some()).to_string(),
                )
            }
            None => ("-".to_string(), "-".to_string()),
        };
        println!(
            "beam {id} line 1 attached {} cut {} rotation {rotation} inside {inside} closes {closes} \
             start {:.9} {:.9} {:.9} end {:.9} {:.9} {:.9} length {length:.9}",
            u8::from(attached),
            u8::from(cut),
            start[0],
            start[1],
            start[2],
            end[0],
            end[1],
            end[2],
        );
    }
    Ok(())
}
