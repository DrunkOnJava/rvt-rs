//! RE-170 (probe): where the joins of Revit's export that rvt-rs does not
//! read are stored (#528, B54).
//!
//! RE-138 reads a duct's or pipe's joins from the lists after its connector
//! manager's anchor, and RE-141 a fitting's from lists anywhere with the
//! fitting first:
//!
//! ```text
//! +0   u32 2
//! +4   u64 own element   u32 connector index   u32 1
//! +20  u64 joined element  u32 connector index   u32 1
//! ```
//!
//! On RE1 Plumbing six joins of a pipe's first end to a fixture are not read,
//! and on RE1 Mechanical two of an air terminal to a duct fitting. This
//! exports the model, takes the `IfcRelConnectsPorts` of Revit's export next
//! to it (`MODEL.ifc`) that the export does not write, and prints every list
//! of that form naming both ends of one, in either order, with its offset
//! from the nearest connector manager anchor before it.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re170_unread_joins -- MODEL.rvt

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};

/// The `u32` after a reference's index.
const REFERENCE_FLAG: u32 = 1;

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

/// The port-to-port connections of a STEP file, each as its two ends'
/// `(ElementId, connector index)` from the ports' names, in order.
fn connections(step: &str) -> BTreeSet<[(u64, u32); 2]> {
    let mut ports = BTreeMap::new();
    let mut rels = Vec::new();
    for line in step.lines() {
        let Some((id, body)) = line.strip_prefix('#').and_then(|rest| rest.split_once('=')) else {
            continue;
        };
        if let Some(args) = body.strip_prefix("IFCDISTRIBUTIONPORT(") {
            let name = args.split('\'').nth(3).unwrap_or_default();
            let rest = name
                .strip_prefix("InPort_")
                .or_else(|| name.strip_prefix("OutPort_"))
                .or_else(|| name.strip_prefix("Port_"));
            if let Some((element, index)) = rest.and_then(|rest| rest.split_once('_')) {
                if let (Ok(element), Ok(index)) = (element.parse(), index.parse()) {
                    ports.insert(id.trim().to_string(), (element, index));
                }
            }
        } else if let Some(args) = body.strip_prefix("IFCRELCONNECTSPORTS(") {
            let refs: Vec<String> = args
                .split('#')
                .skip(1)
                .map(|s| s.chars().take_while(char::is_ascii_digit).collect())
                .collect();
            if refs.len() >= 3 {
                rels.push((refs[1].clone(), refs[2].clone()));
            }
        }
    }
    rels.into_iter()
        .filter_map(|(a, b)| {
            let mut ends = [*ports.get(&a)?, *ports.get(&b)?];
            ends.sort();
            Some(ends)
        })
        .collect()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let reference = std::path::Path::new(&path).with_extension("ifc");
    let Ok(theirs) = std::fs::read_to_string(&reference) else {
        println!("{{\"skipped\":\"no {}\"}}", reference.display());
        return Ok(());
    };
    let theirs = connections(&theirs);
    let mut rf = RevitFile::open(&path)?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let ours = connections(&write_step(&result.model));
    let unread: Vec<[(u64, u32); 2]> = theirs.difference(&ours).copied().collect();
    println!(
        "{{\"revit_connections\":{},\"written\":{},\"unread\":{}}}",
        theirs.len(),
        ours.len(),
        unread.len()
    );
    if unread.is_empty() {
        return Ok(());
    }
    let wanted: BTreeMap<(u64, u64), [(u64, u32); 2]> = unread
        .iter()
        .flat_map(|ends| {
            [
                ((ends[0].0, ends[1].0), *ends),
                ((ends[1].0, ends[0].0), *ends),
            ]
        })
        .collect();
    let classes = rf.schema_classes()?;
    let anchor = classes
        .classes
        .iter()
        .find(|class| class.name == "RbsCurveConnectorManager")
        .map(|class| {
            let mut bytes = class.tag.to_le_bytes().to_vec();
            bytes.extend_from_slice(&[0xff; 4]);
            bytes.extend_from_slice(&class.tag.wrapping_add(1).to_le_bytes());
            bytes
        });
    let mut found: BTreeSet<[(u64, u32); 2]> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let anchors: Vec<usize> = anchor
            .as_ref()
            .map(|anchor| memchr::memmem::find_iter(buf, anchor).collect())
            .unwrap_or_default();
        for at in memchr::memmem::find_iter(buf, &2u32.to_le_bytes()) {
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
            if own_flag != REFERENCE_FLAG || other_flag != REFERENCE_FLAG {
                continue;
            }
            let Some(revit) = wanted.get(&(own, other)) else {
                continue;
            };
            found.insert(*revit);
            let before = anchors.partition_point(|&anchor| anchor < at);
            let from_anchor = before
                .checked_sub(1)
                .map(|i| (at - anchors[i]).to_string())
                .unwrap_or_else(|| "null".into());
            println!(
                "{{\"list\":{{\"stream\":{stream:?},\"offset\":{at},\"from_anchor\":{from_anchor},\"own\":{own},\"own_index\":{own_index},\"other\":{other},\"other_index\":{other_index},\"revit\":{revit:?}}}}}"
            );
        }
    }
    for ends in unread.iter().filter(|ends| !found.contains(*ends)) {
        println!("{{\"no_list\":{ends:?}}}");
    }
    Ok(())
}
