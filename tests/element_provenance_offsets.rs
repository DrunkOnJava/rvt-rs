//! #1325: every offset an element's provenance reports is where the record
//! decodes to the same ElementId.
//!
//! The export's diagnostics list, for each decoded element, the carrier its
//! record was read from, the stream and the byte offset in that stream
//! inflated (the bytes `rvt-dump` writes as `Partitions_NN.decomp`, #291),
//! where its ElementId is stored, and the native record the native passes
//! indexed for it (`decoded.element_provenance`). This test exports each
//! model, then reads the bytes at every reported offset again, without the
//! reader that reported it:
//!
//! - a first-prologue partition element record decodes at its offset
//!   (`decode_at_with_marker`) to the same ElementId, category and class tag,
//!   and the id is the first bytes of the frame;
//! - a second-prologue frame carries no id at its first bytes, and the record
//!   header at the entry's `record_offset` starts with the ElementId, carries
//!   the release's header constant and a `u32` size that spans the frame's
//!   offset (read from the bytes, not through the reader that found the
//!   record); the frame decodes as that element's (`decode_frame_as`);
//! - a schema-walk instance decodes at its offset in `Global/Latest` to the
//!   same self ElementId, as the walk decodes it. No model below has such an
//!   element, so that check has not run on any: the carrier's offsets are
//!   unmeasured, and the walk inflates `Global/Latest` with
//!   `inflate_stream_auto` where `rvt-dump` has its own routine, which this
//!   test does not compare;
//! - an Area's chain record, and a native record, start with the ElementId.
//!
//! The table is off unless asked for (`ExportOptions::with_element_provenance`,
//! `rvt-ifc --element-provenance`): the test checks that an export without the
//! switch has no `element_provenance` in its diagnostics, then times what the
//! switch adds on the warm handle (`build`, then the table's JSON) and the
//! bytes it adds, so the cost is in the report beside the counts.
//!
//! Elements whose carrier this test cannot decode again (ArcWall records,
//! rows with no frame) are counted as not re-decoded, and so are elements with
//! no offset: nothing is skipped silently. Per model, the counts by carrier go
//! to `element-provenance/<model>.json` and `summary.txt` under
//! `CARGO_TARGET_TMPDIR`, which CI prints and uploads.
//!
//! Runs on every `.rvt` under `RVT_PROJECT_CORPUS_DIR` (Core Interior and the
//! RE1 models in CI) and on the BIM4LCA as-built office under
//! `RVT_NORDIC_LCA_DIR`; each is skipped when its variable is unset.

use crate::element_parameters::nordic_dir;
use rvt::ifc::element_provenance::{self, ElementCarrier, ElementProvenanceEntry};
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::partition_element_records::{
    carries_no_element_id, decode_at_with_marker, decode_frame_as, file_bbox_marker,
    record_prologue_constant,
};
use rvt::walker::{InstanceField, WalkerLimits, find_self_id_field};
use rvt::{RevitFile, compression, elem_table, elements, streams};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    buf.get(at..at.checked_add(2)?)
        .map(|b| u16::from_le_bytes(b.try_into().expect("2 bytes")))
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// The ElementId stored at `at`, `width` bytes wide.
fn id_at(buf: &[u8], at: usize, width: u8) -> Option<u64> {
    if width == 8 {
        u64_at(buf, at)
    } else {
        u32_at(buf, at).map(u64::from)
    }
}

#[derive(Default)]
struct Checked {
    checked: usize,
    confirmed: usize,
    not_checked: usize,
    mismatched: Vec<String>,
}

impl Checked {
    fn confirm(&mut self, id: u32, what: &str, holds: bool) {
        self.checked += 1;
        if holds {
            self.confirmed += 1;
        } else if self.mismatched.len() < 20 {
            self.mismatched.push(format!("{id}: {what}"));
        }
    }
}

fn project_models() -> Vec<(String, PathBuf)> {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR") else {
        eprintln!("skipping the project corpus: RVT_PROJECT_CORPUS_DIR is not set");
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut models: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvt"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some((name, path))
        })
        .collect();
    models.sort();
    models
}

fn office_model() -> Vec<(String, PathBuf)> {
    let Some(dir) = nordic_dir() else {
        return Vec::new();
    };
    let path = dir.join("ARK_NordicLCA_Office_Concrete_As-Built.rvt");
    if !path.is_file() {
        eprintln!("skipping the office: the as-built model is absent");
        return Vec::new();
    }
    vec![("nordic-office-as-built".to_string(), path)]
}

/// Export `path`, read every reported offset again, write the report, and
/// return the counts by carrier, with the mismatches of each.
fn check_model(name: &str, path: &Path) -> BTreeMap<String, Checked> {
    let mut rf = RevitFile::open(path).expect("open the model");
    // Without the switch the diagnostics have no table.
    let started = Instant::now();
    let plain = run_export(&mut rf, &ExportOptions::default()).expect("export the model");
    let export_ms = started.elapsed().as_millis();
    assert!(
        plain.diagnostics.decoded.element_provenance.is_none(),
        "{name}: the table is built without the switch"
    );
    let plain_json = serde_json::to_string(&plain.diagnostics).expect("diagnostics json");
    assert!(
        !plain_json.contains("element_provenance"),
        "{name}: the diagnostics name the table without the switch"
    );
    // What the switch adds, on the warm handle, as the pipeline adds it: the
    // table, then its JSON.
    let started = Instant::now();
    let built = element_provenance::build(&mut rf, WalkerLimits::default());
    let build_ms = started.elapsed().as_millis();
    let started = Instant::now();
    let table_json = serde_json::to_vec(&built).expect("serialise the table");
    let json_ms = started.elapsed().as_millis();
    let run = run_export(
        &mut rf,
        &ExportOptions::default().with_element_provenance(true),
    )
    .expect("export the model with the table");
    let table = run
        .diagnostics
        .decoded
        .element_provenance
        .as_ref()
        .expect("the table when asked for");
    assert_eq!(
        table, &built,
        "{name}: the switch builds the table build gives"
    );
    let with_table_bytes = serde_json::to_string(&run.diagnostics)
        .expect("diagnostics json")
        .len();
    let version = rf.layout_release().expect("layout release");
    let width: u8 = table.record_id_bytes.expect("the record id width");
    assert_eq!(
        width == 8,
        version >= 2024,
        "{name}: id width for {version}"
    );

    let declared: BTreeSet<u32> =
        elem_table::declared_ids(&elem_table::parse_records(&mut rf).expect("ElemTable"));
    let marker = file_bbox_marker(&mut rf, version);

    // The schema walk's instances decode from Global/Latest inflated as the
    // walk inflates it.
    let schema = rf.schema().ok();
    let global_latest = rf
        .read_stream(streams::GLOBAL_LATEST)
        .ok()
        .and_then(|raw| compression::inflate_stream_auto(streams::GLOBAL_LATEST, &raw).ok())
        .map(|(_, bytes)| bytes);
    let mut class_by_name = HashMap::new();
    if let Some(schema) = schema.as_ref() {
        for class in &schema.classes {
            class_by_name.insert(class.name.as_str(), class);
        }
    }

    let mut by_carrier: BTreeMap<String, Checked> = BTreeMap::new();
    let mut native = Checked::default();

    for (&id, entry) in &table.elements {
        let stat = by_carrier
            .entry(entry.carrier.as_str().to_string())
            .or_default();
        if !entry.is_located() {
            stat.not_checked += 1;
        } else {
            let stream = entry
                .stream
                .as_deref()
                .expect("a located entry has a stream");
            let offset = entry.offset.expect("a located entry has an offset");
            match entry.carrier {
                ElementCarrier::GlobalLatestSchemaWalk => {
                    let class = entry
                        .class_name
                        .as_deref()
                        .and_then(|name| class_by_name.get(name).copied());
                    match (global_latest.as_deref(), class) {
                        (Some(bytes), Some(class)) => {
                            // The self id the walk reads: the class's own id
                            // field, else the id the typed decoder sets.
                            let own = elements::decode_instance_prefer_typed(bytes, offset, class)
                                .and_then(|decoded| {
                                    find_self_id_field(class)
                                        .and_then(|index| decoded.fields.get(index))
                                        .and_then(|(_, field)| match field {
                                            InstanceField::ElementId { id, .. } if *id != 0 => {
                                                Some(*id)
                                            }
                                            _ => None,
                                        })
                                        .or(decoded.id.filter(|id| *id != 0))
                                });
                            stat.confirm(
                                id,
                                &format!("Global/Latest:{offset} decodes to {own:?}"),
                                own == Some(id),
                            );
                        }
                        _ => stat.not_checked += 1,
                    }
                }
                ElementCarrier::PartitionElementRecordFirstPrologue
                | ElementCarrier::PartitionElementRecordSecondPrologue => {
                    match rf.inflated_partition(stream) {
                        Ok(inflated) => check_partition_frame(
                            stat,
                            id,
                            entry,
                            (stream, offset),
                            inflated.bytes(),
                            (width, marker),
                            &declared,
                        ),
                        Err(_) => stat.not_checked += 1,
                    }
                }
                ElementCarrier::PartitionAreaRecord => {
                    let holds = rf.inflated_partition(stream).is_ok_and(|inflated| {
                        id_at(inflated.bytes(), offset, width) == Some(u64::from(id))
                    });
                    stat.confirm(id, &format!("{stream}:{offset} starts with the id"), holds);
                }
                _ => stat.not_checked += 1,
            }
        }
        if let Some(record) = entry.native.as_ref() {
            match (record.stream.as_deref(), record.offset) {
                (Some(stream), Some(offset)) => {
                    let holds = rf.inflated_partition(stream).is_ok_and(|inflated| {
                        id_at(inflated.bytes(), offset, width) == Some(u64::from(id))
                    });
                    native.confirm(id, &format!("native {stream}:{offset}"), holds);
                }
                _ => native.not_checked += 1,
            }
        }
    }

    // The counts by carrier cover every element, the listed and the omitted.
    let counted: usize = table.by_carrier.values().map(|count| count.elements).sum();
    assert_eq!(
        counted,
        table.elements.len() + table.omitted_entries,
        "{name}: by_carrier counts every element"
    );
    for (carrier, count) in &table.by_carrier {
        assert_eq!(
            count.located + count.unlocated,
            count.elements,
            "{name}: {carrier}"
        );
    }

    let size = table_json.len();
    let mut report = json!({
        "model": name,
        "layout_release": table.layout_release,
        "partition_count": table.partition_count,
        "elements_listed": table.elements.len(),
        "omitted_entries": table.omitted_entries,
        "unkeyed_elements": table.unkeyed_elements,
        "table_json_bytes": size,
        "diagnostics_json_bytes_without_table": plain_json.len(),
        "diagnostics_json_bytes_with_table": with_table_bytes,
        "export_ms_without_table": export_ms,
        "table_build_ms": build_ms,
        "table_json_ms": json_ms,
        "native_record_index": table.native_record_index,
        "native": {
            "with_record": table.native.with_record,
            "located": table.native.located,
            "unlocated": table.native.unlocated,
            "several_copies": table.native.several_copies,
            "copy_unresolved": table.native.copy_unresolved,
            "offsets_checked": native.checked,
            "offsets_confirmed": native.confirmed,
        },
    });
    let mut carriers = serde_json::Map::new();
    for (carrier, stat) in &by_carrier {
        let count = table.by_carrier.get(carrier);
        carriers.insert(
            carrier.clone(),
            json!({
                "elements": count.map(|c| c.elements),
                "located": count.map(|c| c.located),
                "unlocated": count.map(|c| c.unlocated),
                "offsets_checked": stat.checked,
                "offsets_confirmed": stat.confirmed,
                "not_re_decoded": stat.not_checked,
            }),
        );
    }
    report["by_carrier"] = serde_json::Value::Object(carriers);
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("element-provenance");
    std::fs::create_dir_all(&out).expect("report folder");
    std::fs::write(
        out.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&report).expect("report json"),
    )
    .expect("write the report");
    let mut line = format!(
        "{name}: Revit {version}, {} elements ({} omitted, {} unkeyed), {} partitions, table {size} bytes as JSON ({} bytes of diagnostics without it, {} with); export {export_ms} ms without the table, table built in {build_ms} ms and written as JSON in {json_ms} ms\n",
        table.elements.len(),
        table.omitted_entries,
        table.unkeyed_elements,
        table.partition_count,
        plain_json.len(),
        with_table_bytes
    );
    for (carrier, stat) in &by_carrier {
        let count = table.by_carrier.get(carrier);
        line.push_str(&format!(
            "  {carrier}: {} elements, {} with an offset, {} without; {} offsets re-decoded, {} confirmed, {} not re-decoded\n",
            count.map_or(0, |c| c.elements),
            count.map_or(0, |c| c.located),
            count.map_or(0, |c| c.unlocated),
            stat.checked,
            stat.confirmed,
            stat.not_checked
        ));
    }
    line.push_str(&format!(
        "  native records: {} elements, {} with an offset, {} without ({} in several partitions, {} with no current copy); {} offsets re-decoded, {} confirmed\n",
        table.native.with_record,
        table.native.located,
        table.native.unlocated,
        table.native.several_copies,
        table.native.copy_unresolved,
        native.checked,
        native.confirmed
    ));
    eprint!("{line}");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out.join("summary.txt"))
        .and_then(|mut summary| summary.write_all(line.as_bytes()))
        .expect("write the summary");

    by_carrier.insert("native".to_string(), native);
    by_carrier
}

/// Read the frame of a partition element record again: `at` is its stream and
/// offset, `format` the width of an ElementId in the file and its bbox marker.
/// A second-prologue frame is checked against the record header's own bytes,
/// not against the reader that found the record: the header at `record_offset`
/// starts with the ElementId, its `u32` size follows the id, and the frame's
/// offset lies inside the `size` bytes from the header.
fn check_partition_frame(
    stat: &mut Checked,
    id: u32,
    entry: &ElementProvenanceEntry,
    at: (&str, usize),
    buf: &[u8],
    format: (u8, Option<[u8; 8]>),
    declared: &BTreeSet<u32>,
) {
    let (stream, offset) = at;
    let (width, marker) = format;
    let key = u64::from(id);
    let Some(record_offset) = entry.record_offset else {
        stat.confirm(
            id,
            &format!("{stream}:{offset} has no record offset"),
            false,
        );
        return;
    };
    let id_stored_at_record = id_at(buf, record_offset, width) == Some(key);
    if entry.carrier == ElementCarrier::PartitionElementRecordFirstPrologue {
        let mut holds = id_stored_at_record && record_offset == offset;
        if let Some(marker) = marker {
            holds &= decode_at_with_marker(stream, buf, offset, declared, &marker).is_some_and(
                |record| {
                    record.element_id == id
                        && !record.id_from_enclosing_record
                        && Some(record.class_tag) == entry.class_tag
                        && Some(record.builtin_category) == entry.builtin_category
                },
            );
        }
        stat.confirm(
            id,
            &format!("{stream}:{offset} first-prologue frame"),
            holds,
        );
        return;
    }
    let size = u32_at(buf, record_offset + usize::from(width)).map(|size| size as usize);
    let mut holds = id_stored_at_record
        && size.is_some_and(|size| record_offset < offset && offset < record_offset + size)
        && if width == 8 {
            u64_at(buf, offset).is_some_and(carries_no_element_id)
        } else {
            id_at(buf, offset, width) != Some(key)
        };
    if let (8, Some(marker)) = (width, marker) {
        // The release's record header constant, at +0x0c of every header.
        holds &= u16_at(buf, record_offset + 0x0c) == Some(record_prologue_constant(&marker));
        holds &= decode_frame_as(stream, buf, offset, id, &marker).is_some_and(|record| {
            record.element_id == id
                && Some(record.class_tag) == entry.class_tag
                && Some(record.builtin_category) == entry.builtin_category
        });
    }
    stat.confirm(
        id,
        &format!("{stream}:{offset} second-prologue frame"),
        holds,
    );
}

/// Check each model; the models named in `required` must have at least one
/// partition element record offset confirmed.
fn check_all(models: Vec<(String, PathBuf)>, required: &[&str]) {
    let mut checked_models = 0;
    for (name, path) in models {
        let stats = check_model(&name, &path);
        for (carrier, stat) in &stats {
            assert!(
                stat.mismatched.is_empty(),
                "{name}: {} of {} offsets of {carrier} do not decode to the element's id: {:?}",
                stat.checked - stat.confirmed,
                stat.checked,
                stat.mismatched
            );
        }
        if required.contains(&name.as_str()) {
            let frames: usize = stats
                .iter()
                .filter(|(carrier, _)| carrier.starts_with("partition_element_record"))
                .map(|(_, stat)| stat.confirmed)
                .sum();
            assert!(
                frames > 0,
                "{name}: no partition element record offset confirmed"
            );
        }
        checked_models += 1;
    }
    if checked_models == 0 {
        eprintln!("skipping: no model to read");
    }
}

#[test]
fn project_corpus_offsets_decode_to_the_same_element_id() {
    check_all(
        project_models(),
        &["2024_Core_Interior", "RE1-Architecture"],
    );
}

#[test]
fn nordic_office_offsets_decode_to_the_same_element_id() {
    check_all(office_model(), &["nordic-office-as-built"]);
}
