//! #1719 (RE-428 residual): why does a beam's element data give no location line?
//!
//! FACT under test: `scan_bounded_lines` finds a bounded line for 291 of the 300
//! beams of the ten BIM4LCA models and none for 9, yet Revit's export writes
//! `Length`, `Span`, `Slope` and `Roll` on each of the 9. The reader drops a beam
//! for one of four reasons, and each leaves a different trace:
//!
//! - **no element data found**: no partition holds an element-data header that
//!   names the beam, so the beam is stored as another record kind;
//! - **data without a line**: a partition holds the data and it has no bounded
//!   line record at all;
//! - **first curve record is an arc**: `first_bounded_line` returns nothing where
//!   the data's first curve record is an arc, so a line that comes later is
//!   never read (`all_lines` shows it);
//! - **copies disagree**: the beam's data sits in several partitions and its
//!   copies give different lines, so `Copies::Agreeing` drops it. The copy the
//!   file routes to (RE-216) is `current`.
//!
//! The probe prints a summary count of the four, and for each beam with no line
//! one line naming its reason and, per partition copy, the lines it holds. It
//! prints the first beam that does read a line as a control, the same way.
//! Lengths are in feet; no name is printed.
//!
//! Output (one model per run):
//!   release <n>: <beams> structural-framing elements, <n> with a line
//!   summary no_data <n> data_without_line <n> arc_first <n> copies_disagree <n> other <n>
//!   beam <id> reason <name> holders <n> partitions <p,p,..> current <p|-> box <dx> <dy> <dz>
//!     copy <partition> first_lines <n> start <x y z> end <x y z> length <ft>
//!     all_lines <partition> <n> lengths <ft ft ..>
//!   control <id> partitions <p,p,..> ...
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_1719_beam_line_copies -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::current_partitions;
use rvt::research::partition_beam_axes::{
    BoundedLine, all_line_copies, line_copies, scan_bounded_lines, scan_first_bounded_arcs,
};
use rvt::research::partition_schema_mvp::{element_record_bbox, recover_partition_schema_mvp};
use rvt::walker::{DecodedElement, WalkerLimits};

fn length(line: &BoundedLine) -> f64 {
    let (start, end) = (line.start(), line.end());
    (0..3)
        .map(|axis| (end[axis] - start[axis]).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn join<T: ToString>(items: impl IntoIterator<Item = T>) -> String {
    let parts: Vec<String> = items.into_iter().map(|item| item.to_string()).collect();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(",")
    }
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_1719_beam_line_copies FILE.rvt");
    };
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
    let missing: BTreeSet<u32> = ids
        .iter()
        .copied()
        .filter(|id| !lines.contains_key(id))
        .collect();
    let control: BTreeSet<u32> = ids
        .iter()
        .copied()
        .find(|id| lines.contains_key(id))
        .into_iter()
        .collect();
    let wanted: BTreeSet<u32> = missing.union(&control).copied().collect();
    let copies = line_copies(&mut rf, version, &wanted)?;
    let every = all_line_copies(&mut rf, version, &wanted)?;
    let arcs = scan_first_bounded_arcs(&mut rf, version, &missing)?;
    let disagreeing: BTreeMap<u32, BTreeSet<u32>> = missing
        .iter()
        .filter_map(|id| {
            let held = copies.get(id)?;
            let (_, first) = held.lines.first()?;
            held.lines
                .iter()
                .any(|(_, line)| line != first)
                .then(|| (*id, held.partitions.clone()))
        })
        .collect();
    let current = current_partitions(&mut rf, &disagreeing);

    println!(
        "release {version}: {} structural-framing elements, {} with a line",
        beams.len(),
        lines.len()
    );
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut detail = Vec::new();
    for id in &missing {
        let held = copies.get(id);
        let reason = match held {
            None => "no_data",
            Some(_) if disagreeing.contains_key(id) => "copies_disagree",
            Some(_) if arcs.contains_key(id) => "arc_first",
            Some(held) if held.lines.is_empty() => "data_without_line",
            Some(_) => "other",
        };
        *counts.entry(reason).or_default() += 1;
        let beam = beams.iter().find(|beam| beam.id == Some(*id));
        let size = beam.and_then(|beam| element_record_bbox(beam)).map_or_else(
            || "-".to_string(),
            |[x0, y0, z0, x1, y1, z1]| format!("{:.4} {:.4} {:.4}", x1 - x0, y1 - y0, z1 - z0),
        );
        let partitions = held.map_or_else(|| "-".to_string(), |held| join(&held.partitions));
        let routed = current
            .get(id)
            .map_or_else(|| "-".to_string(), |partition| partition.to_string());
        detail.push(format!(
            "beam {id} reason {reason} holders {} partitions {partitions} current {routed} box {size}",
            held.map_or(0, |held| held.partitions.len()),
        ));
        if let Some(held) = held {
            for (partition, line) in &held.lines {
                let (start, end) = (line.start(), line.end());
                detail.push(format!(
                    "  copy {partition} first_lines 1 start {:.6} {:.6} {:.6} end {:.6} {:.6} {:.6} length {:.6}",
                    start[0], start[1], start[2], end[0], end[1], end[2], length(line)
                ));
            }
        }
        for (partition, all) in every.get(id).into_iter().flatten() {
            detail.push(format!(
                "  all_lines {partition} {} lengths {}",
                all.len(),
                join(all.iter().map(|line| format!("{:.6}", length(line))))
            ));
        }
    }
    println!(
        "summary no_data {} data_without_line {} arc_first {} copies_disagree {} other {}",
        counts.get("no_data").copied().unwrap_or(0),
        counts.get("data_without_line").copied().unwrap_or(0),
        counts.get("arc_first").copied().unwrap_or(0),
        counts.get("copies_disagree").copied().unwrap_or(0),
        counts.get("other").copied().unwrap_or(0),
    );
    for line in detail {
        println!("{line}");
    }
    for id in &control {
        let held = copies.get(id);
        println!(
            "control {id} partitions {} lines {}",
            held.map_or_else(|| "-".to_string(), |held| join(&held.partitions)),
            held.map_or(0, |held| held.lines.len())
        );
        for (partition, all) in every.get(id).into_iter().flatten() {
            println!(
                "  all_lines {partition} {} lengths {}",
                all.len(),
                join(all.iter().map(|line| format!("{:.6}", length(line))))
            );
        }
    }
    Ok(())
}
