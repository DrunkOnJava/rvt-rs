//! #227: why some doors and windows with a cut loop (RE-215) read no
//! instance transform (RE-87).
//!
//! FACT under test: `scan_instance_transforms` takes the first nine `f64`
//! within 4 KiB of an instance's element-data header that form a rotation
//! (orthonormal, determinant 1), followed by its origin, and drops an
//! instance whose copies disagree. 8 of Core Interior's 132 doors have a cut
//! loop and no transform, so their opening stays the element's box. For each
//! such door or window this prints every header hit with its id, the first
//! orthonormal matrix after it of either determinant within 16 KiB, and the
//! transform `instance_transform` reads there, and its FamilyInstance's
//! placement fields. That tells apart a door with no data header, one whose
//! copies disagree, one stored as a reflection (determinant -1, a mirrored
//! instance) and one whose transform lies past the window.
//!
//! Output:
//!   missing <id>   a door or window with a cut loop and no transform
//!   hit <id> <stream> <offset> <bytes to the next hit> <first orthonormal: offset det | -> <transform | ->
//!   copy <id> <stream> <loop min> <loop max> <axes> <origin>   each copy's transform, for
//!     `tools/re/opening_copies_vs_ifc.py`
//!   place <id> <the stream native_document reads its current record from> <m_instOrigin,
//!     m_RefDir, m_zAxis, m_flippedX, m_flippedY, m_mirrored, m_insertOrientation>
//!
//! Usage:
//!   cargo run --profile ci --example probe_227_missing_transforms -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_symbol_cuts::instance_cut_loops;
use rvt::research::partition_instance_transforms::{instance_transform, scan_instance_transforms};
use rvt::research::partition_names::element_data_header;
use serde_json::Value;

const SEARCH: usize = 0x4000;
const TOLERANCE: f64 = 1e-9;

fn f64_at(data: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        *data.get(at..at + 8)?.first_chunk::<8>()?,
    ))
}

/// The first offset in `data` where nine `f64` form an orthonormal matrix,
/// with its determinant.
fn first_orthonormal(data: &[u8]) -> Option<(usize, f64)> {
    (0..data.len().saturating_sub(72)).find_map(|at| {
        let mut rows = [[0.0; 3]; 3];
        for (index, value) in rows.iter_mut().flatten().enumerate() {
            let v = f64_at(data, at + 8 * index)?;
            if !(v.is_finite() && v.abs() <= 1.0 + TOLERANCE) {
                return None;
            }
            *value = v;
        }
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
        Some((at, det))
    })
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let ids: BTreeSet<u32> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCDOOR")
                || ifc_type.eq_ignore_ascii_case("IFCWINDOW") =>
            {
                tag.parse().ok()
            }
            _ => None,
        })
        .collect();
    let loops = instance_cut_loops(&mut rf, &ids.iter().map(|&id| u64::from(id)).collect())?;
    let transforms = scan_instance_transforms(&mut rf, version, &ids)?;
    let missing: BTreeSet<u32> = ids
        .iter()
        .copied()
        .filter(|id| loops.contains_key(&u64::from(*id)) && !transforms.contains_key(id))
        .collect();
    for id in &missing {
        println!("missing {id}");
    }
    eprintln!(
        "{} doors and windows, {} with a cut loop, {} with a transform, {} with a loop and no transform",
        ids.len(),
        loops.len(),
        transforms.len(),
        missing.len()
    );
    let Some(header) = element_data_header(version) else {
        eprintln!("no element-data header for Revit {version}");
        return Ok(());
    };
    let mut hit_counts: BTreeMap<u32, usize> = BTreeMap::new();
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
                .filter(|id| missing.contains(id))
            else {
                continue;
            };
            *hit_counts.entry(id).or_default() += 1;
            let next = hits.get(index + 1).copied().unwrap_or(buf.len());
            let data = buf
                .get(id_at + 8..next.min(id_at + 8 + SEARCH).min(buf.len()))
                .unwrap_or_default();
            let first = first_orthonormal(data)
                .map_or("-".to_string(), |(at, det)| format!("{at} {det:.6}"));
            let read = instance_transform(data);
            let transform =
                read.map_or("-".to_string(), |t| format!("{:?} {:?}", t.axes, t.origin));
            println!(
                "hit {id} {stream} {hit} {} {first} {transform}",
                next.saturating_sub(id_at + 8)
            );
            if let (Some(t), Some(cut)) = (read, loops.get(&u64::from(id))) {
                println!(
                    "copy {id} {stream} {} {} {} {}",
                    serde_json::to_string(&cut.min)?,
                    serde_json::to_string(&cut.max)?,
                    serde_json::to_string(&t.axes)?,
                    serde_json::to_string(&t.origin)?
                );
            }
        }
    }
    for id in &missing {
        if !hit_counts.contains_key(id) {
            println!("hit {id} - - - no element-data header names it");
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let options = Options {
        selected_ids: missing.iter().map(|&id| u64::from(id)).collect(),
        selected_classes: BTreeSet::new(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some("FamilyInstance"), Some(graph)) =
            (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let place: serde_json::Map<String, Value> = [
            "m_instOrigin",
            "m_RefDir",
            "m_zAxis",
            "m_flippedX",
            "m_flippedY",
            "m_mirrored",
            "m_insertOrientation",
        ]
        .iter()
        .filter_map(|name| Some((name.to_string(), root.fields.get(*name)?.clone())))
        .collect();
        println!(
            "place {} {} {}",
            record.identity.element_id,
            record.source.stream,
            serde_json::to_string(&place)?
        );
        Ok(())
    })?;
    Ok(())
}
