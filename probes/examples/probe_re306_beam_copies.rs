//! RE-306: why a beam keeps its record box (#1247).
//!
//! FACT under test: a structural-framing element is drawn along its location
//! line only where `partition_beam_axes::scan_bounded_lines` finds one line
//! for it and `beam_body` builds a solid from that line and its record box
//! (RE-49). Since RE-216 an element whose copies of its element data in
//! different partitions hold different lines gets none, and keeps its box.
//! #1247 reports 341 beams of a Revit 2024 project drawn as boxes where
//! v0.6.1, before RE-216, drew them. This sorts every beam the default export
//! draws as its record box by what the line scan finds for it:
//! - `no_data`: no partition's element data names it;
//! - `no_line`: its element data holds no bounded line;
//! - `one_line`: its copies hold one line, so `beam_body` rejected it;
//! - `disagree`: its copies hold different lines, and RE-216 drops it.
//!
//! For `disagree` it also counts the beams whose current partition
//! (`native_document::current_partitions`) holds one line, as RE-216's
//! current-copy rule would take, and gives the largest distance between two
//! copies' line ends.
//!
//! Output: `census <class> <count>` lines, then `disagree current_one_line
//! <count>`, `disagree no_current <count>` and `disagree spread <decade> <count>`
//! lines, counts only. No element id, name or coordinate is printed.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re306_beam_copies -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::native_document::current_partitions;
use rvt::research::partition_beam_axes::line_copies;

const RECORD_BOX: &str = "partition_element_record_bbox";

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut boxed = BTreeSet::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        if ifc_type != "IfcBeam" {
            continue;
        }
        let is_box = set.properties.iter().any(|p| {
            p.name == "BodySource" && matches!(&p.value, PropertyValue::Text(t) if t == RECORD_BOX)
        });
        if let (true, Ok(id)) = (is_box, tag.parse::<u32>()) {
            boxed.insert(id);
        }
    }
    let copies = line_copies(&mut rf, version, &boxed)?;
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    let mut disagreeing: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut spreads: BTreeMap<i32, usize> = BTreeMap::new();
    for id in &boxed {
        let class = match copies.get(id) {
            None => "no_data",
            Some(found) if found.lines.is_empty() => "no_line",
            Some(found)
                if found
                    .lines
                    .iter()
                    .all(|(_, line)| *line == found.lines[0].1) =>
            {
                "one_line"
            }
            Some(found) => {
                disagreeing.insert(*id, found.partitions.clone());
                let mut spread = 0f64;
                for (_, a) in &found.lines {
                    for (_, b) in &found.lines {
                        spread = spread
                            .max(distance(a.start(), b.start()))
                            .max(distance(a.end(), b.end()));
                    }
                }
                let decade = if spread > 0. {
                    spread.log10().ceil() as i32
                } else {
                    i32::MIN
                };
                *spreads.entry(decade).or_default() += 1;
                "disagree"
            }
        };
        *census.entry(class).or_default() += 1;
    }
    println!("census boxed_beams {}", boxed.len());
    for (class, n) in &census {
        println!("census {class} {n}");
    }
    let current = current_partitions(&mut rf, &disagreeing);
    let mut current_one_line = 0;
    for (id, partition) in &current {
        let Some(found) = copies.get(id) else {
            continue;
        };
        let mut in_current = found.lines.iter().filter(|(p, _)| p == partition);
        if let Some((_, first)) = in_current.next() {
            if in_current.all(|(_, line)| line == first) {
                current_one_line += 1;
            }
        }
    }
    println!("disagree current_one_line {current_one_line}");
    println!(
        "disagree no_current {}",
        disagreeing.len() - current.len().min(disagreeing.len())
    );
    for (decade, n) in &spreads {
        if *decade == i32::MIN {
            println!("disagree spread 0 {n}");
        } else {
            println!("disagree spread 1e{decade} {n}");
        }
    }
    Ok(())
}
