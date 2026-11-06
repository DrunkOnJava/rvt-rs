//! RE-362: why a Revit 2015 to 2023 beam keeps its record box, for BIMcollab's
//! Revit 2015 example (the one non-held-out model with Revit's own export that
//! holds beams, where Measure run 42134881703 drew 0 of its 91 beams along a line).
//!
//! FACT under test: a structural-framing element is drawn along its location
//! line only where its element data holds one bounded line (copies that agree)
//! and `partition_beam_axes::beam_body` builds a solid from that line and the
//! element's record box (RE-49, RE-362). This is `probe_re328_beam_rejections`
//! on the releases RE-362 reads (`beam_axis_read_on`, 2015 to 2027): it sorts
//! every structural-framing element the partition reader recovers by what its
//! element data holds (`no_bbox`, `no_data`, `no_line`, `disagree`, `one_line`
//! `accepted` or declined by its first failing test, with the same detail), and
//! prints counts only, each line as `native readers: beam lines: ...` in the
//! restricted alphabet `tools/ci/scrub-holdout-results.sh` keeps for a model
//! fetched without a licence. A beam a column cuts (RE-122) keeps its cut box and
//! is counted apart (`column_cut`). For a line the reader accepts it also counts
//! the lines the beam's data holds (`accepted lines_in_data`) and whether another
//! one also makes a beam solid (`accepted other_line_fits`): where another does,
//! the body could be along the wrong line, which no scorer here would see. No
//! element id, name or coordinate is printed.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re362_beam_lines_pre2023 -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::element_record_beam_cuts::BEAM_BODY_SOURCE_FIELD;
use rvt::research::partition_beam_axes::{
    BeamReject, BoundedLine, END_TOLERANCE_FEET, MIN_PLAN_CONDITION, all_line_copies,
    beam_axis_read_on, beam_body_in_box, beam_check, beam_section_fit, box_along, box_overshoot,
    box_overshoot_feet, line_copies,
};
use rvt::research::partition_schema_mvp::{
    BEAM_ROTATION_FIELD, beam_axis_from_fields, beam_i_section_from_fields, element_record_bbox,
    recover_partition_schema_mvp,
};
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};

fn say(text: std::fmt::Arguments) {
    println!("native readers: beam lines: {text}");
}

fn bump(counts: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *counts.entry(key.into()).or_default() += 1;
}

fn decade(feet: f64) -> String {
    if feet.is_nan() || feet <= 0.0 {
        return "0".to_string();
    }
    format!("<=1e{}", feet.log10().ceil().clamp(-30.0, 30.0) as i32)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// How a beam's other line lies against the line the reader takes: its
/// direction (`parallel`, `reversed`, `perpendicular`, `other`), its offset from
/// the first line (`no_offset`, `offset_z_only`, `offset_in_plan`), whether it is a
/// level line at the mid-depth of the record box, and whether it has the same
/// start and end parameters (RE-49: "a second line with the same parameters,
/// origin and direction at the section's mid-depth").
fn relation(first: &BoundedLine, other: &BoundedLine, bbox: [f64; 6]) -> String {
    let d = dot(first.direction, other.direction);
    let direction = if d > 1.0 - 1e-9 {
        "parallel"
    } else if d < -(1.0 - 1e-9) {
        "reversed"
    } else if d.abs() < 1e-6 {
        "perpendicular"
    } else {
        "other"
    };
    let delta = [
        other.origin[0] - first.origin[0],
        other.origin[1] - first.origin[1],
        other.origin[2] - first.origin[2],
    ];
    let along = dot(delta, first.direction);
    let across = [
        delta[0] - along * first.direction[0],
        delta[1] - along * first.direction[1],
        delta[2] - along * first.direction[2],
    ];
    let offset = if dot(across, across).sqrt() < 1e-6 {
        "no_offset"
    } else if across[0].hypot(across[1]) < 1e-6 {
        "offset_z_only"
    } else {
        "offset_in_plan"
    };
    let mid = (bbox[2] + bbox[5]) * 0.5;
    let level_at_mid = (other.origin[2] - mid).abs() < 1e-3 && other.direction[2].abs() < 1e-9;
    let at_mid = if level_at_mid {
        "at_mid_depth"
    } else {
        "not_at_mid_depth"
    };
    let same = if (other.start_parameter - first.start_parameter).abs() < 1e-9
        && (other.end_parameter - first.end_parameter).abs() < 1e-9
    {
        "same_parameters"
    } else {
        "other_parameters"
    };
    format!("{direction} {offset} {at_mid} {same}")
}

fn float_field(element: &DecodedElement, wanted: &str) -> Option<f64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

fn orientation(direction: [f64; 3]) -> &'static str {
    if direction[0].hypot(direction[1]) < 0.1 {
        "steep"
    } else if direction[2].abs() > 1e-9 {
        "sloped"
    } else if direction[0].abs() < 1e-9 || direction[1].abs() < 1e-9 {
        "level_on_axis"
    } else {
        "level_rotated"
    }
}

fn reject_name(reject: &BeamReject) -> &'static str {
    match reject {
        BeamReject::NotFinite => "not_finite",
        BeamReject::InvertedBox => "inverted_box",
        BeamReject::EndOutside { .. } => "end_outside",
        BeamReject::ZeroLength => "zero_length",
        BeamReject::TooSteep => "too_steep",
        BeamReject::NoSection => "no_section",
        BeamReject::OpenClosure { .. } => "open_closure",
    }
}

/// What `beam_body_in_box` and the rotation test do with a line `beam_check`
/// declined for a section that does not close, saying the first condition
/// that stops the beam.
fn rule_outcome(bbox: [f64; 6], start: [f64; 3], end: [f64; 3], rotation: &str) -> &'static str {
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let length = dot(delta, delta).sqrt();
    let d = [delta[0] / length, delta[1] / length, delta[2] / length];
    if d[2].abs() > 1e-9 {
        return "sloped";
    }
    if (d[0] * d[0] - d[1] * d[1]).abs() < MIN_PLAN_CONDITION {
        return "plan_angle_near_45";
    }
    let Some(free) = box_along(bbox, d) else {
        return "free_solve_none";
    };
    if free[0] <= length {
        return "box_not_longer";
    }
    if beam_body_in_box(bbox, start, end).is_none() {
        return "line_not_in_body";
    }
    match rotation {
        "zero" => "drawn",
        "nonzero" => "rotation_nonzero",
        _ => "rotation_unread",
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, true))?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let beams: Vec<&DecodedElement> = mvp
        .products
        .iter()
        .filter(|element| element.class == "StructuralFraming" && element.id.is_some())
        .collect();
    say(format_args!("release {version}"));
    say(format_args!("layout_read {}", beam_axis_read_on(version)));
    say(format_args!(
        "streams_not_read {}",
        rf.unread_streams().len()
    ));
    say(format_args!("beams {}", beams.len()));
    let along_line = beams
        .iter()
        .filter(|beam| beam_axis_from_fields(&beam.fields).is_some())
        .count();
    say(format_args!("along_line {along_line}"));
    if !beam_axis_read_on(version) {
        return Ok(());
    }
    let ids: BTreeSet<u32> = beams.iter().filter_map(|beam| beam.id).collect();
    let copies = line_copies(&mut rf, version, &ids)?;
    let all_lines = all_line_copies(&mut rf, version, &ids)?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for beam in &beams {
        let Some(id) = beam.id else { continue };
        // A beam a column cuts (RE-122) keeps its cut box: its record box is the
        // cut box, so no line is read for it.
        if beam
            .fields
            .iter()
            .any(|(name, _)| name == BEAM_BODY_SOURCE_FIELD)
        {
            bump(&mut counts, "column_cut");
            continue;
        }
        let attached = beam_axis_from_fields(&beam.fields).is_some();
        let Some(bbox) = element_record_bbox(beam) else {
            bump(&mut counts, "no_bbox");
            continue;
        };
        let line: BoundedLine = match copies.get(&id) {
            None => {
                bump(&mut counts, "no_data");
                continue;
            }
            Some(found) if found.lines.is_empty() => {
                bump(&mut counts, "no_line");
                continue;
            }
            Some(found) if found.lines.iter().any(|(_, l)| *l != found.lines[0].1) => {
                bump(&mut counts, "disagree");
                continue;
            }
            Some(found) => found.lines[0].1,
        };
        let (start, end) = (line.start(), line.end());
        let verdict = beam_check(bbox, start, end);
        let why = match &verdict {
            Ok(_) => "accepted",
            Err(reject) => reject_name(reject),
        };
        let orient = orientation(line.direction);
        bump(&mut counts, format!("one_line {why}"));
        bump(&mut counts, format!("one_line {why} {orient}"));
        let held = all_lines
            .get(&id)
            .and_then(|found| found.first())
            .map_or(0, |(_, lines)| lines.len());
        let other_fits = all_lines.get(&id).is_some_and(|found| {
            found
                .iter()
                .flat_map(|(_, lines)| lines.iter())
                .filter(|other| **other != line)
                .any(|other| beam_check(bbox, other.start(), other.end()).is_ok())
        });
        if verdict.is_ok() {
            // Whether the first line, the one the reader takes, is the only line
            // in the data that makes a beam solid: where another does too, the
            // body could be along the wrong one.
            bump(
                &mut counts,
                format!("one_line accepted lines_in_data {}", held.min(4)),
            );
            bump(
                &mut counts,
                format!("one_line accepted other_line_fits {other_fits}"),
            );
            // Every other line of the copy the reader took, against the first.
            let copy_lines = all_lines
                .get(&id)
                .and_then(|found| found.first())
                .map_or(&[][..], |(_, lines)| lines.as_slice());
            for other in copy_lines.iter().filter(|other| **other != line) {
                bump(
                    &mut counts,
                    format!("accepted second_line {}", relation(&line, other, bbox)),
                );
            }
            if !attached {
                bump(&mut counts, "MISMATCH accepted_but_not_attached");
            }
            continue;
        }
        let rotation = match float_field(beam, BEAM_ROTATION_FIELD) {
            Some(0.0) => "zero",
            Some(_) => "nonzero",
            None => "unread",
        };
        bump(&mut counts, format!("one_line {why} rotation {rotation}"));
        let section = if beam_i_section_from_fields(&beam.fields).is_some() {
            "known"
        } else {
            "unknown"
        };
        bump(&mut counts, format!("one_line {why} i_section {section}"));
        bump(
            &mut counts,
            format!("one_line {why} lines_in_data {}", held.min(4)),
        );
        bump(
            &mut counts,
            format!("one_line {why} other_line_fits {other_fits}"),
        );
        if let Some(free) = box_along(bbox, line.direction) {
            let line_length = line.end_parameter - line.start_parameter;
            let gap = free[0] - line_length;
            let sign = if gap < 0.0 {
                "box_shorter"
            } else {
                "box_longer"
            };
            bump(
                &mut counts,
                format!("one_line {why} free_length {sign} {}", decade(gap.abs())),
            );
        } else {
            bump(&mut counts, format!("one_line {why} free_length none"));
        }
        let mut drawn_by_rule = false;
        match verdict {
            Err(BeamReject::EndOutside { feet }) => {
                let outside = [start, end]
                    .iter()
                    .filter(|p| box_overshoot_feet(bbox, **p) > END_TOLERANCE_FEET)
                    .count();
                bump(&mut counts, format!("end_outside ends {outside}"));
                bump(&mut counts, format!("end_outside size {}", decade(feet)));
                let worst = [start, end]
                    .map(|p| box_overshoot(bbox, p))
                    .into_iter()
                    .max_by(|a, b| dot(*a, *a).total_cmp(&dot(*b, *b)))
                    .unwrap_or([0.0; 3]);
                let along = dot(worst, line.direction);
                let across = (dot(worst, worst) - along * along).max(0.0).sqrt();
                let kind = if across <= 1e-6 {
                    "along_line"
                } else {
                    "across_line"
                };
                bump(
                    &mut counts,
                    format!("end_outside overshoot {orient} {kind}"),
                );
                let then = match beam_section_fit(bbox, start, end) {
                    Ok(_) => "fits",
                    Err(other) => reject_name(&other),
                };
                bump(
                    &mut counts,
                    format!("end_outside without_ends_test {orient} {then}"),
                );
            }
            Err(BeamReject::OpenClosure { feet }) => {
                bump(&mut counts, format!("open_closure miss {}", decade(feet)));
                let outcome = rule_outcome(bbox, start, end, rotation);
                bump(&mut counts, format!("open_closure rule {outcome}"));
                bump(&mut counts, format!("open_closure rule {orient} {outcome}"));
                drawn_by_rule = outcome == "drawn";
            }
            _ => {}
        }
        if attached != drawn_by_rule {
            bump(&mut counts, "MISMATCH attached_differs_from_rule");
        }
    }
    for (key, n) in &counts {
        say(format_args!("{key} {n}"));
    }
    Ok(())
}
