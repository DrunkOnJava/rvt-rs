//! RE-141 (probe): the counted lists of two references whose first is a duct,
//! a pipe or a fitting, wherever they are in a partition (#528).
//!
//! RE-138 reads a duct's or pipe's joins from the bytes after its connector
//! manager's anchor, in the form
//!
//! ```text
//! +0   u32 2
//! +4   u64 own element   u32 connector index   u32 1
//! +20  u64 joined element  u32 connector index   u32 1
//! ```
//!
//! and a fitting's own record holds only the sorted ids of the elements it
//! refers to, with no connector index. This looks for that list form
//! anywhere in every partition, with a duct, pipe or fitting as the first
//! reference, and prints one `{"pair":[element,index,other,other_index]}` for
//! each distinct list (and the ducts, pipes and fittings as `{"curve":id}`),
//! the form `tools/re/connector_pairs_vs_ifc.py` scores against the
//! `IfcRelConnectsPorts` of Revit's own export, and where the first lists of
//! fittings are.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re141_list_scan -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

/// A list holds two references of 16 bytes after its count.
const LIST_BYTES: usize = 4 + 2 * 16;
/// The `u32` after a reference's index.
const REFERENCE_FLAG: u32 = 1;
/// A connector index is a small number.
const MAX_INDEX: u32 = 8;
/// Fitting lists shown with the bytes before them.
const SHOWN: usize = 4;
/// Bytes shown before a list.
const CONTEXT: usize = 48;

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let ids = |records: Vec<per::PartitionElementRecord>| -> BTreeSet<u64> {
        records
            .iter()
            .map(|record| u64::from(record.element_id))
            .collect()
    };
    let curves = ids(per::scan_category_records_multi(
        &mut rf,
        version,
        &[per::OST_DUCT_CURVES, per::OST_PIPE_CURVES],
        &declared,
    )?);
    let fittings = ids(per::scan_category_records_multi(
        &mut rf,
        version,
        &[per::OST_DUCT_FITTING, per::OST_PIPE_FITTING],
        &declared,
    )?);
    let owners: BTreeSet<u64> = curves.union(&fittings).copied().collect();
    println!(
        "{{\"revit\":{version},\"curves\":{},\"fittings\":{}}}",
        curves.len(),
        fittings.len()
    );
    for id in &owners {
        println!("{{\"curve\":{id}}}");
    }
    if owners.is_empty() {
        return Ok(());
    }

    // (own, own index, other, other index) -> how many times it is written
    let mut lists: BTreeMap<(u64, u32, u64, u32), usize> = BTreeMap::new();
    let mut shown = 0usize;
    let mut context: Vec<String> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in 0..buf.len().saturating_sub(LIST_BYTES) {
            if u32_at(buf, at) != Some(2) {
                continue;
            }
            let (Some(own), Some(own_index), Some(own_flag)) = (
                u64_at(buf, at + 4),
                u32_at(buf, at + 12),
                u32_at(buf, at + 16),
            ) else {
                continue;
            };
            let (Some(other), Some(other_index), Some(other_flag)) = (
                u64_at(buf, at + 20),
                u32_at(buf, at + 28),
                u32_at(buf, at + 32),
            ) else {
                continue;
            };
            if !owners.contains(&own)
                || own_index >= MAX_INDEX
                || other_index >= MAX_INDEX
                || own_flag != REFERENCE_FLAG
                || other_flag != REFERENCE_FLAG
                || other == own
                || !u32::try_from(other).is_ok_and(|id| declared.contains(&id))
            {
                continue;
            }
            let written = lists
                .entry((own, own_index, other, other_index))
                .or_insert(0);
            *written += 1;
            if fittings.contains(&own) && shown < SHOWN && *written == 1 {
                shown += 1;
                let from = at.saturating_sub(CONTEXT);
                context.push(format!(
                    "{{\"fitting_list\":{{\"own\":{own},\"index\":{own_index},\"other\":{other},\"other_index\":{other_index},\"stream\":{stream:?},\"offset\":{at},\"before\":\"{}\"}}}}",
                    hex(&buf[from..at])
                ));
            }
        }
    }
    for (own, own_index, other, other_index) in lists.keys() {
        println!("{{\"pair\":[{own},{own_index},{other},{other_index}]}}");
    }
    for line in &context {
        println!("{line}");
    }
    let (mut by_curves, mut by_fittings) = (0usize, 0usize);
    let mut copies: BTreeMap<usize, usize> = BTreeMap::new();
    for ((own, ..), count) in &lists {
        if fittings.contains(own) {
            by_fittings += 1;
        } else {
            by_curves += 1;
        }
        *copies.entry(*count).or_default() += 1;
    }
    println!(
        "{{\"lists\":{{\"distinct\":{},\"of_curves\":{by_curves},\"of_fittings\":{by_fittings},\"by_copies\":\"{copies:?}\"}}}}",
        lists.len()
    );
    Ok(())
}
