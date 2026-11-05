//! RE-350 probe: where the frames of each `BuiltInCategory` stop on the way to
//! being written, on every release the exporter reads (#1444, #1445).
//!
//! FACT (to be measured): a frame the sweep decodes is written only when it is
//! a standalone placed instance (the RE-21 rule), has a 3D body, is outside a
//! non-primary design option and is in the export phase. This probe says, for
//! every category a file holds and for each ElementId, the first of those
//! tests the id's frames fail, so a category whose frames are not written can
//! be told apart from one the sweep does not decode, and a count that differs
//! from another reader's can be placed. `tools/re/category_census_vs_ifc.py`
//! scores the sets against the `Tag`s of Revit's own export.
//!
//! Reads a 2008 to 2023 file through `partition_element_records_2023::
//! scan_records` and a 2024 to 2027 file through the product sweep over every
//! category the file's marker frames carry (the same readers as
//! `probe_re319_category_census`, whose `any`, `rule` and `category` lines this
//! probe prints in the same form). Per category:
//!
//! - `any <id> <name> <count> <ids>`: every ElementId a decoded frame carries;
//! - `rule <id> <name> <count> <ids>`: those that pass the instance rule;
//! - `category <id> <name> <count> <ids>`: those the exporter writes;
//! - `unplaced <id> <name> <reason> <count> <ids>`: ids with no frame that
//!   passes the rule, by the reason of their frames (`container`: a container
//!   reference is set; `symbol`: the type symbol's placement kind;
//!   `other_kind`: any other placement kind);
//! - `refused <id> <name> <reason> <count> <ids>`: ids that pass the rule and
//!   have no frame that is written, by the first test their frames fail
//!   (`no_body_flat`: one extent is not above `VOLUME_EPSILON_FEET`;
//!   `no_body_line`: two; `no_body_point`: three; `no_body_not_finite`: an
//!   extent is not a number; `design_option`: a non-primary design option;
//!   `phase`: outside the export phase). An id whose frames fail at
//!   different tests is listed under each;
//! - `census stages: <id> <name> any N placed N written N; <kind> <reason>
//!   N ...`: the same as counts, in the restricted alphabet that
//!   `tools/ci/scrub-holdout-results.sh` keeps for a model with no licence to
//!   redistribute it.
//!
//! With `--records` (Measure passes it for Autodesk's samples) a `record`
//! line also lists, for each frame of the categories in `WATCHED`, the stream
//! and offset, the container reference, the placement kind, the design option
//! and the three extents, so an id can be looked up. Every run also prints
//! `declared <n> framed <n> unframed <n> <ids>`: the ElementIds `Global/ElemTable`
//! declares, how many of them a decoded frame of any category carries, and the
//! ones none does, as runs (`a-b`) of consecutive ids. An id in no frame of any
//! category is either not declared or declared with no frame the sweep reads.
//! `chained without a frame <n> <ids>` lists the ids of the records the
//! partitions open with (an element the file holds) that no decoded frame
//! carries.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re350_category_stages -- model.rvt [--records]
//! ```

use rvt::RevitFile;
use rvt::research::partition_element_records::{self as per, PartitionElementRecord};
use rvt::research::partition_element_records_2023 as per32;
use std::collections::{BTreeMap, BTreeSet};

type Ids = BTreeMap<i64, BTreeSet<u32>>;
type IdsByReason = BTreeMap<(i64, &'static str), BTreeSet<u32>>;

/// Autodesk's `BuiltInCategory.OST_StructuralFramingSystem` (Revit's "Structural
/// Beam Systems"): a record of it lists the beams of the system.
const BEAM_SYSTEMS: i64 = -2_001_327;

/// The categories whose frames `--records` lists: the curtain panels, mullions
/// and doors of #1445, and the assemblies, furniture systems, beam systems and
/// rebar of #1444 (Autodesk's published `BuiltInCategory` values).
const WATCHED: [i64; 7] = [
    per::OST_CURTAIN_WALL_PANELS,
    per::OST_CURTAIN_WALL_MULLIONS,
    per::OST_DOORS,
    -2_000_267,
    -2_001_100,
    BEAM_SYSTEMS,
    -2_009_000,
];

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    let end = at.checked_add(8)?;
    buf.get(at..end)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// Every frame of the file's marker in the `BuiltInCategory` band, decoded
/// for any category: a 2024 to 2027 file's frames with a declared id or an
/// enclosing record's.
fn records_2024(rf: &mut RevitFile, release: u32) -> Vec<PartitionElementRecord> {
    let Some(marker) = per::file_bbox_marker(rf, release) else {
        println!("the release gives no record marker");
        return Vec::new();
    };
    let Ok(table) = rvt::elem_table::parse_records(rf) else {
        return Vec::new();
    };
    let declared = rvt::elem_table::declared_ids(&table);
    let assigned = per::compute_second_prologue_ids(rf);
    let none = BTreeMap::new();
    let mut out = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let mut categories: BTreeSet<i64> = BTreeSet::new();
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(start) = hit.checked_sub(per::BBOX_MARKER_OFFSET) else {
                continue;
            };
            if let Some(category) = i64_at(buf, start + per::CATEGORY_OFFSET) {
                if (per::BUILTIN_CATEGORY_MIN..=per::BUILTIN_CATEGORY_MAX).contains(&category) {
                    categories.insert(category);
                }
            }
        }
        let list: Vec<i64> = categories.into_iter().collect();
        for records in per::find_categories_records_assigned(
            &stream,
            buf,
            &list,
            &declared,
            &marker,
            assigned.get(&stream).unwrap_or(&none),
        ) {
            out.extend(records);
        }
    }
    out
}

fn name_of(category: i64) -> String {
    per::builtin_category_name(category)
        .map(|name| name.replace(' ', "_"))
        .unwrap_or_else(|| "unnamed".to_string())
}

fn print_ids(kind: &str, category: i64, ids: &BTreeSet<u32>) {
    let list: Vec<String> = ids.iter().map(u32::to_string).collect();
    println!(
        "{kind} {category} {} {} {}",
        name_of(category),
        ids.len(),
        list.join(",")
    );
}

/// Why a placed instance has no 3D body: how many of its three extents are not
/// above the volume epsilon, or that one is not a number.
fn no_body_reason(record: &PartitionElementRecord) -> &'static str {
    let (dx, dy, dz) = record.extents_feet();
    let extents = [dx, dy, dz];
    if extents.iter().any(|extent| !extent.is_finite()) {
        return "no_body_not_finite";
    }
    let flat = extents
        .iter()
        .filter(|extent| **extent <= per::VOLUME_EPSILON_FEET)
        .count();
    match flat {
        1 => "no_body_flat",
        2 => "no_body_line",
        _ => "no_body_point",
    }
}

/// The ids of each `(category, reason)` that `covered` does not hold for the
/// category: an id with another frame that got further is not listed.
fn uncovered(reasons: &IdsByReason, covered: &Ids) -> IdsByReason {
    let mut out = IdsByReason::new();
    for ((category, reason), ids) in reasons {
        let kept: BTreeSet<u32> = ids
            .iter()
            .copied()
            .filter(|id| covered.get(category).is_none_or(|set| !set.contains(id)))
            .collect();
        if !kept.is_empty() {
            out.insert((*category, *reason), kept);
        }
    }
    out
}

fn print_reasons(kind: &str, reasons: &IdsByReason) {
    for ((category, reason), ids) in reasons {
        let list: Vec<String> = ids.iter().map(u32::to_string).collect();
        println!(
            "{kind} {category} {} {reason} {} {}",
            name_of(*category),
            ids.len(),
            list.join(",")
        );
    }
}

fn print_record(record: &PartitionElementRecord, stage: &str) {
    let (dx, dy, dz) = record.extents_feet();
    let container = if record.is_container_member() {
        record.container.to_string()
    } else {
        "none".to_string()
    };
    let option = record
        .design_option
        .map_or_else(|| "none".to_string(), |option| option.to_string());
    let references = if record.builtin_category == BEAM_SYSTEMS {
        let shown: Vec<String> = record
            .references
            .iter()
            .take(64)
            .map(u64::to_string)
            .collect();
        format!(
            " references {} {}",
            record.references.len(),
            shown.join(",")
        )
    } else {
        String::new()
    };
    println!(
        "record {} {} {stage} {} {} container {container} kind {:#x} option {option} extents {dx:.4} {dy:.4} {dz:.4} enclosing {}{references}",
        record.builtin_category,
        record.element_id,
        record.stream,
        record.offset,
        record.placement_kind,
        record.id_from_enclosing_record
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let records_wanted = args.iter().any(|arg| arg == "--records");
    let path = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .ok_or("usage: probe_re350_category_stages <file.rvt> [--records]")?;
    let mut rf = RevitFile::open(path)?;
    let saved = rf.basic_file_info()?.version;
    let release = rf.layout_release()?;
    println!("release saved {saved} layout {release}");
    let records: Vec<PartitionElementRecord> = if per32::is_32bit_release(release) {
        per32::scan_records(&mut rf, release)
    } else if per::supports_revit_version(release) {
        records_2024(&mut rf, release)
    } else {
        println!("a release whose records are not read: nothing to measure");
        return Ok(());
    };
    let filtered = per32::applies_export_filters(release) || per::supports_revit_version(release);
    let (phases, options) = if filtered {
        (Some(rf.phase_filter()), Some(rf.design_options()))
    } else {
        (None, None)
    };
    println!("records {}", records.len());

    let mut any = Ids::new();
    let mut rule = Ids::new();
    let mut written = Ids::new();
    let mut unplaced = IdsByReason::new();
    let mut refused = IdsByReason::new();
    for record in &records {
        let category = record.builtin_category;
        let id = record.element_id;
        any.entry(category).or_default().insert(id);
        let stage: &str = if !record.is_exported_instance() {
            let reason = if record.is_container_member() {
                "container"
            } else if record.is_type_symbol() {
                "symbol"
            } else {
                "other_kind"
            };
            unplaced.entry((category, reason)).or_default().insert(id);
            reason
        } else {
            rule.entry(category).or_default().insert(id);
            let reason = if !record.has_volume() {
                Some(no_body_reason(record))
            } else if options.as_ref().is_some_and(|o| o.excludes(record)) {
                Some("design_option")
            } else if phases.as_ref().is_some_and(|p| p.excludes(record)) {
                Some("phase")
            } else {
                None
            };
            match reason {
                Some(reason) => {
                    refused.entry((category, reason)).or_default().insert(id);
                    reason
                }
                None => {
                    written.entry(category).or_default().insert(id);
                    "written"
                }
            }
        };
        if records_wanted && WATCHED.contains(&category) {
            print_record(record, stage);
        }
    }

    let unplaced = uncovered(&unplaced, &rule);
    let refused = uncovered(&refused, &written);
    let empty = BTreeSet::new();
    for (category, any_ids) in &any {
        print_ids("any", *category, any_ids);
        print_ids("rule", *category, rule.get(category).unwrap_or(&empty));
        print_ids(
            "category",
            *category,
            written.get(category).unwrap_or(&empty),
        );
    }
    print_reasons("unplaced", &unplaced);
    print_reasons("refused", &refused);
    for (category, any_ids) in &any {
        let mut line = format!(
            "census stages: {category} {} any {} placed {} written {}",
            name_of(*category),
            any_ids.len(),
            rule.get(category).map_or(0, BTreeSet::len),
            written.get(category).map_or(0, BTreeSet::len)
        );
        for (kind, reasons) in [("unplaced", &unplaced), ("refused", &refused)] {
            for ((reason_category, reason), ids) in reasons {
                if reason_category == category {
                    line.push_str(&format!("; {kind} {reason} {}", ids.len()));
                }
            }
        }
        println!("{line}");
    }
    let declared = rvt::elem_table::parse_records(&mut rf)
        .map(|table| rvt::elem_table::declared_ids(&table))
        .unwrap_or_default();
    let framed: BTreeSet<u32> = any.values().flatten().copied().collect();
    let unframed: Vec<u32> = declared.difference(&framed).copied().collect();
    println!(
        "declared {} framed {} unframed {} {}",
        declared.len(),
        declared.intersection(&framed).count(),
        unframed.len(),
        runs(&unframed)
    );
    let bare: Vec<u32> = chained_ids(&mut rf, release, &declared)
        .difference(&framed)
        .copied()
        .collect();
    println!("chained without a frame {} {}", bare.len(), runs(&bare));
    Ok(())
}

/// The ElementIds of the records the file's partitions open with: the chain of
/// RE-35 on Revit 2024 to 2027, the declared records `record_spans` walks on
/// 2008 to 2023 (RE-315). A record here is an element the file holds, whether
/// or not it has a bounding-box frame the sweep decodes.
fn chained_ids(rf: &mut RevitFile, release: u32, declared: &BTreeSet<u32>) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    let layout32 = per32::is_32bit_release(release);
    let header_tag = per32::record_marker(rf).map(|(_, tag)| tag);
    let marker = per::file_bbox_marker(rf, release);
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        if layout32 {
            if let Some(tag) = header_tag {
                out.extend(
                    per32::record_spans(buf, tag, declared)
                        .iter()
                        .map(|span| span.element_id),
                );
            }
        } else if let Some(marker) = &marker {
            out.extend(
                per::partition_record_chain(buf, marker)
                    .iter()
                    .filter_map(|span| u32::try_from(span.element_id).ok()),
            );
        }
    }
    out
}

/// `a-b` for each run of consecutive ids and `a` for a single id, joined by
/// commas, for ids in ascending order.
fn runs(ids: &[u32]) -> String {
    let run = |start: u32, end: u32| {
        if start == end {
            start.to_string()
        } else {
            format!("{start}-{end}")
        }
    };
    let mut out: Vec<String> = Vec::new();
    let mut iter = ids.iter().copied();
    let Some(first) = iter.next() else {
        return String::new();
    };
    let (mut start, mut last) = (first, first);
    for id in iter {
        if last.checked_add(1) == Some(id) {
            last = id;
        } else {
            out.push(run(start, last));
            (start, last) = (id, id);
        }
    }
    out.push(run(start, last));
    out.join(",")
}
