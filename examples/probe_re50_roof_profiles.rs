//! RE-50: a roof's sketched outline is in its sketch lines, as a floor's is,
//! and a sketch line's own data holds its exact ends.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, the
//! `OST_SketchLines` records whose owner reference names a placed roof close
//! into that roof's outline under RE-25's box solve for 9 of its 20 roofs.
//! Each sketch line's data also carries the bounded line record RE-49 reads
//! for beams (`04 00 08 01`), with the segment's exact ends; chaining those
//! closes 4 more roofs whose boxes leave the diagonals ambiguous. Every
//! outline's plan box is the roof's record box, and 11 of the 13 outlines
//! have the area of Revit's own export of the roof (5 of them per layer, as
//! Revit writes a layered roof as stacked layer solids).
//!
//! The probe lists every placed roof with its record box and, when its
//! sketch closes under the box solve, the outline's area, vertex count,
//! voids and pieces, and how the outline's plan box sits against the record
//! box. For a roof it does not close, it counts the sketch lines the roof
//! owns and says whether their recorded ends close.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re50_roof_profiles -- FILE.rvt

use rvt::RevitFile;
use rvt::element_record_plan_profiles::plan_profiles_from_sketch_line_records;
use rvt::partition_element_records::{OST_SKETCH_LINES, scan_category_records_multi};
use rvt::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The record box `[min x, min y, min z, max x, max y, max z]` an element
/// record gives: its location fields hold the plan centre and the base.
fn record_box(element: &DecodedElement) -> Option<[f64; 6]> {
    let field = |wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let (x, y, z) = (
        field("m_locationX")?,
        field("m_locationY")?,
        field("m_locationZ")?,
    );
    let (w, d, h) = (
        field("m_bboxWidth")?,
        field("m_bboxDepth")?,
        field("m_bboxHeight")?,
    );
    Some([x - w / 2.0, y - d / 2.0, z, x + w / 2.0, y + d / 2.0, z + h])
}

fn area(ring: &[(f64, f64)]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let roofs: Vec<_> = mvp
        .products
        .iter()
        .filter(|element| element.class == "Roof")
        .collect();
    let declared: BTreeSet<u32> =
        rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let sketch_lines =
        scan_category_records_multi(&mut rf, version, &[OST_SKETCH_LINES], &declared)?;
    let profiles = plan_profiles_from_sketch_line_records(&sketch_lines);
    let closed = roofs
        .iter()
        .filter(|roof| roof.id.is_some_and(|id| profiles.contains_key(&id)))
        .count();
    println!(
        "release {version}: {} placed roofs, {closed} with a closed sketch",
        roofs.len()
    );
    for roof in roofs {
        let Some(id) = roof.id else {
            continue;
        };
        let Some(bbox) = record_box(roof) else {
            continue;
        };
        let extents = format!(
            "{:.3} x {:.3} x {:.3} ft",
            bbox[3] - bbox[0],
            bbox[4] - bbox[1],
            bbox[5] - bbox[2]
        );
        let Some(profile) = profiles.get(&id) else {
            let owned: Vec<_> = sketch_lines
                .iter()
                .filter(|line| line.owner_reference == Some(id))
                .collect();
            let ids: BTreeSet<u32> = owned.iter().map(|line| line.element_id).collect();
            let diagonal = owned
                .iter()
                .filter(|l| {
                    (l.bbox_feet[3] - l.bbox_feet[0]).abs() > 1e-6
                        && (l.bbox_feet[4] - l.bbox_feet[1]).abs() > 1e-6
                })
                .count();
            let sloped = owned
                .iter()
                .filter(|l| (l.bbox_feet[5] - l.bbox_feet[2]).abs() > 1e-6)
                .count();
            // The exact ends each owned sketch line's data carries, and
            // whether they chain into closed loops (every end shared by
            // exactly two segments).
            let lines = rvt::partition_beam_axes::scan_bounded_lines(&mut rf, version, &ids)?;
            let key = |p: [f64; 3]| ((p[0] * 1e4).round() as i64, (p[1] * 1e4).round() as i64);
            let mut degree: std::collections::BTreeMap<(i64, i64), usize> = Default::default();
            for line in lines.values() {
                *degree.entry(key(line.start())).or_default() += 1;
                *degree.entry(key(line.end())).or_default() += 1;
            }
            let closes =
                lines.len() == ids.len() && !degree.is_empty() && degree.values().all(|&d| d == 2);
            println!(
                "  roof {id:>9}: box {extents}; no closed sketch ({} sketch lines it owns, {} records, {diagonal} not on an axis, {sloped} not level; {} with a line record, ends {})",
                ids.len(),
                owned.len(),
                lines.len(),
                if closes { "close" } else { "do not close" }
            );
            continue;
        };
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in profile
            .outer_xy
            .iter()
            .chain(profile.pieces.iter().flat_map(|p| p.outer_xy.iter()))
        {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        let voids: f64 = profile.inner_xy.iter().map(|ring| area(ring)).sum();
        let pieces: f64 = profile.pieces.iter().map(|p| area(&p.outer_xy)).sum();
        println!(
            "  roof {id:>9}: box {extents}; outline {:.3} ft2 ({} vertices, {} voids of {:.3} ft2, {} more pieces of {:.3} ft2); outline box off the record box by {:.3} {:.3} {:.3} {:.3} ft",
            area(&profile.outer_xy),
            profile.vertex_count(),
            profile.inner_xy.len(),
            voids,
            profile.pieces.len(),
            pieces,
            x0 - bbox[0],
            y0 - bbox[1],
            bbox[3] - x1,
            bbox[4] - y1,
        );
    }
    Ok(())
}
