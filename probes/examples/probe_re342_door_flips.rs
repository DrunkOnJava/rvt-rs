//! RE-342 (#1357): what a door's or window's data holds of its hand flip and
//! facing flip.
//!
//! FACT under test: Revit's `FamilyInstance.HandFlipped` and `FacingFlipped`
//! (public RevitAPI names; revit-ifc's `DoorWindowInfo` reads them as its
//! `FlippedX` and `FlippedY`) are carried by the instance's data in a form
//! rvt-rs can read. Two places are candidates. The transform of RE-87 is read
//! only where its nine `f64` are a rotation of determinant 1
//! (`partition_instance_transforms`), so an instance stored as a reflection
//! (determinant -1) would be skipped or read from the wrong place; and the
//! instance's native `FamilyInstance` record names its placement fields
//! (`m_RefDir`, `m_zAxis`, `m_instOrigin`, and flip or mirror flags whose
//! names this probe lists rather than assumes). Neither is known to hold the
//! flips, and `m_opening_facing` is only the stored transform's Y axis.
//!
//! For every door and window of the export (the elements `RvtDocExporter`
//! gives an `IFCDOOR` or `IFCWINDOW` and the ElementId as its tag) the probe
//! prints one JSON object per line:
//!
//!   kind "model"    release and the counts of doors, windows and walls
//!   kind "fields"   a native record class and the names of its root fields,
//!                   once per class
//!   kind "door" or "window"
//!     id, host (the ElementId of the wall the export voids), read (the
//!     transform `scan_instance_transforms` takes: axes and origin), copies
//!     (every element-data copy: partition stream, offset, the first
//!     orthonormal 3 x 3 of either determinant within 16 KiB with its
//!     determinant, offset, nine values and the three values after them, the
//!     transform the reader takes from that copy, and the first 1,024 bytes of
//!     the copy's data in hex, unless `--records`), records (the root
//!     fields of each native `FamilyInstance` record of the element, with
//!     any value longer than 600 characters replaced by its length) and
//!     objects (the other non-graphics objects of the first records,
//!     for the first 40 elements of the model)
//!   kind "wall"     id, start, end, flipped and cross_section of each host
//!                   wall's native record (RE-222)
//!
//! With `--records` (the form Measure runs a probe on a sample project in)
//! only the first three elements print their records and objects.
//! `tools/re/door_flips_vs_ifc.py` pairs the lines with Revit's export.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re342_door_flips -- MODEL.rvt [--records]

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_walls::native_walls;
use rvt::research::partition_instance_transforms::{instance_transform, scan_instance_transforms};
use rvt::research::partition_names::element_data_header;
use serde_json::{Value, json};

const SEARCH: usize = 0x4000;
const TOLERANCE: f64 = 1e-9;
const LONG_VALUE: usize = 600;
const OBJECT_LIMIT: usize = 1500;
const FULL_ELEMENTS: usize = 40;
const COMPACT_ELEMENTS: usize = 3;
const HEAD_BYTES: usize = 1024;

fn f64_at(data: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        *data.get(at..at + 8)?.first_chunk::<8>()?,
    ))
}

type Orthonormal = (usize, f64, [f64; 9], Option<[f64; 3]>);

/// The first offset in `data` where nine `f64` form an orthonormal matrix of
/// either determinant, with its determinant, the nine values and the three
/// after them.
fn first_orthonormal(data: &[u8]) -> Option<Orthonormal> {
    (0..data.len().saturating_sub(72)).find_map(|at| {
        let mut flat = [0.0; 9];
        for (index, value) in flat.iter_mut().enumerate() {
            let v = f64_at(data, at + 8 * index)?;
            if !(v.is_finite() && v.abs() <= 1.0 + TOLERANCE) {
                return None;
            }
            *value = v;
        }
        let rows = [
            [flat[0], flat[1], flat[2]],
            [flat[3], flat[4], flat[5]],
            [flat[6], flat[7], flat[8]],
        ];
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        for i in 0..3 {
            for j in 0..3 {
                let want = if i == j { 1.0 } else { 0.0 };
                if (dot(rows[i], rows[j]) - want).abs() > TOLERANCE {
                    return None;
                }
            }
        }
        let [a, b, c] = rows;
        let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0]);
        let origin = match (
            f64_at(data, at + 72),
            f64_at(data, at + 80),
            f64_at(data, at + 88),
        ) {
            (Some(x), Some(y), Some(z)) if x.is_finite() && y.is_finite() && z.is_finite() => {
                Some([x, y, z])
            }
            _ => None,
        };
        Some((at, det, flat, origin))
    })
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    out
}

fn tag_of(entity: &IfcEntity) -> Option<u32> {
    match entity {
        IfcEntity::BuildingElement {
            type_guid: Some(tag),
            ..
        } => tag.parse().ok(),
        _ => None,
    }
}

/// `value`, or its length where it prints longer than `LONG_VALUE`.
fn shortened(value: &Value) -> Value {
    let text = value.to_string();
    if text.len() > LONG_VALUE {
        json!({ "long": text.len() })
    } else {
        value.clone()
    }
}

/// A record's root fields whose names speak of a flip, a mirror, a hand, a
/// facing, an orientation, an axis or a placement, for the compact form.
fn flag_fields(record: &Value) -> Value {
    const WORDS: [&str; 10] = [
        "flip", "mirror", "hand", "facing", "orient", "refdir", "zaxis", "inst", "host", "symbol",
    ];
    let kept: serde_json::Map<String, Value> = record
        .get("root")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .filter(|(name, _)| {
                    let lower = name.to_ascii_lowercase();
                    WORDS.iter().any(|word| lower.contains(word))
                })
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default();
    json!({ "stream": record.get("stream"), "root": kept })
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: MODEL.rvt [--records]");
    let compact = args.any(|arg| arg == "--records");
    let element_limit = if compact {
        COMPACT_ELEMENTS
    } else {
        FULL_ELEMENTS
    };
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;

    // Each door or window by ElementId: its kind and the ElementId of the
    // wall the export voids; and every wall's ElementId.
    let mut kinds: BTreeMap<u32, (&'static str, Option<u32>)> = BTreeMap::new();
    let mut walls: BTreeSet<u32> = BTreeSet::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            host_element_index,
            ..
        } = entity
        else {
            continue;
        };
        let Some(id) = tag_of(entity) else {
            continue;
        };
        let upper = ifc_type.to_ascii_uppercase();
        if upper.starts_with("IFCWALL") {
            walls.insert(id);
        } else if upper == "IFCDOOR" || upper == "IFCWINDOW" {
            let kind = if upper == "IFCDOOR" { "door" } else { "window" };
            let host = host_element_index
                .and_then(|index| model.entities.get(index))
                .and_then(tag_of);
            kinds.insert(id, (kind, host));
        }
    }
    let ids: BTreeSet<u32> = kinds.keys().copied().collect();
    let door_count = kinds.values().filter(|(kind, _)| *kind == "door").count();
    println!(
        "{}",
        json!({
            "kind": "model",
            "release": version,
            "doors": door_count,
            "windows": kinds.len() - door_count,
            "walls": walls.len(),
        })
    );

    let read = scan_instance_transforms(&mut rf, version, &ids)?;

    // Every element-data copy of each door and window.
    let mut copies: BTreeMap<u32, Vec<Value>> = BTreeMap::new();
    if let Some(header) = element_data_header(version) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
            for (index, &hit) in hits.iter().enumerate() {
                let id_at = hit + header.len();
                let Some(id) = buf
                    .get(id_at..id_at + 8)
                    .and_then(|b| b.first_chunk::<8>())
                    .map(|b| u64::from_le_bytes(*b))
                    .and_then(|id| u32::try_from(id).ok())
                    .filter(|id| ids.contains(id))
                else {
                    continue;
                };
                let next = hits.get(index + 1).copied().unwrap_or(buf.len());
                let data = buf
                    .get(id_at + 8..next.min(id_at + 8 + SEARCH).min(buf.len()))
                    .unwrap_or_default();
                let first = first_orthonormal(data).map(|(at, det, flat, origin)| {
                    json!({ "at": at, "det": det, "values": flat, "after": origin })
                });
                let taken =
                    instance_transform(data).map(|t| json!({ "axes": t.axes, "origin": t.origin }));
                let head = (!compact).then(|| hex(&data[..data.len().min(HEAD_BYTES)]));
                copies.entry(id).or_default().push(json!({
                    "stream": stream,
                    "hit": hit,
                    "gap": next.saturating_sub(id_at + 8),
                    "first": first,
                    "taken": taken,
                    "head": head,
                }));
            }
        }
    }

    // The native `FamilyInstance` record of each door and window.
    let mut records: BTreeMap<u32, Vec<Value>> = BTreeMap::new();
    let mut objects: BTreeMap<u32, Vec<Value>> = BTreeMap::new();
    let mut shown: BTreeSet<String> = BTreeSet::new();
    let options = Options {
        selected_ids: ids.iter().map(|&id| u64::from(id)).collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let summary = extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let Ok(id) = u32::try_from(record.identity.element_id) else {
            return Ok(());
        };
        if shown.insert(class.to_string()) {
            let names: Vec<&String> = root
                .fields
                .as_object()
                .map(|fields| fields.keys().collect())
                .unwrap_or_default();
            println!(
                "{}",
                json!({ "kind": "fields", "class": class, "names": names })
            );
        }
        if class != "FamilyInstance" {
            return Ok(());
        }
        let fields: serde_json::Map<String, Value> = root
            .fields
            .as_object()
            .map(|fields| {
                fields
                    .iter()
                    .map(|(name, value)| (name.clone(), shortened(value)))
                    .collect()
            })
            .unwrap_or_default();
        records
            .entry(id)
            .or_default()
            .push(json!({ "stream": record.source.stream, "root": fields }));
        let others = objects.entry(id).or_default();
        for object in graph.objects.iter().skip(1) {
            if object.class_name.starts_with('G') && object.class_name != "GInstance" {
                continue;
            }
            let text = object.fields.to_string();
            let fields = if text.len() <= OBJECT_LIMIT {
                object.fields.clone()
            } else {
                let names: Vec<&String> = object
                    .fields
                    .as_object()
                    .map(|fields| fields.keys().collect())
                    .unwrap_or_default();
                json!({ "long": text.len(), "names": names })
            };
            others.push(json!({ "class": object.class_name, "fields": fields }));
        }
        Ok(())
    });
    match summary {
        Ok(summary) => eprintln!(
            "native records: {} emitted, {} graphs complete, {} unsupported",
            summary.emitted_records,
            summary.complete_graph_records,
            summary.unsupported_graph_records
        ),
        Err(error) => eprintln!("native records: not read: {error}"),
    }

    // The host walls' native records.
    let host_ids: BTreeSet<u64> = kinds
        .values()
        .filter_map(|(_, host)| host.map(u64::from))
        .collect();
    let host_walls = native_walls(&mut rf, &host_ids).unwrap_or_else(|error| {
        eprintln!("host walls: not read: {error}");
        BTreeMap::new()
    });
    for (id, wall) in &host_walls {
        println!(
            "{}",
            json!({
                "kind": "wall",
                "id": id,
                "start": wall.start,
                "end": wall.end,
                "flipped": wall.flipped,
                "cross_section": wall.cross_section,
            })
        );
    }

    for (position, (id, (kind, host))) in kinds.iter().enumerate() {
        let shown_fully = position < element_limit;
        let mut element_records = records.get(id).cloned().unwrap_or_default();
        if compact && !shown_fully {
            for record in &mut element_records {
                *record = flag_fields(record);
            }
        }
        let element_objects = if shown_fully {
            objects.get(id).cloned().unwrap_or_default()
        } else {
            Vec::new()
        };
        let element_copies = copies.get(id).cloned().unwrap_or_default();
        let element_read = read
            .get(id)
            .map(|t| json!({ "axes": t.axes, "origin": t.origin }));
        println!(
            "{}",
            json!({
                "kind": kind,
                "id": id,
                "host": host,
                "read": element_read,
                "copies": element_copies,
                "records": element_records,
                "objects": element_objects,
            })
        );
    }
    eprintln!(
        "{} doors and windows, {} with a transform read, {} with a native record, {} host walls read",
        kinds.len(),
        read.len(),
        records.len(),
        host_walls.len()
    );
    Ok(())
}
