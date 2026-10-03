//! RE-160 (probe): whether the elements Revit's export leaves out differ in
//! their phases (B38, #328).
//!
//! rvt-rs writes RE1 Plumbing's water closet tank 442378, which Revit's IFC4
//! export of the same file does not hold; Snowdon's #328 slabs are left out
//! by phase. Revit keeps an element's Phase Created and Phase Demolished as
//! the BuiltInParameters `PHASE_CREATED` (-1012100) and `PHASE_DEMOLISHED`
//! (-1012101).
//!
//! For every record of every partition's leading chain (RE-35) whose frame
//! decodes, the probe reads, in the element's own verified data objects
//! (RE-153), the `u32` after each of those two ids. It tallies, by category
//! and by whether Revit's export holds an element with that `Tag`, how many
//! elements carry each `(created, demolished)` pair, and prints the elements
//! Revit leaves out in a category where it exports others.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re160_phase_entries -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const PHASE_CREATED: i64 = -1_012_100;
const PHASE_DEMOLISHED: i64 = -1_012_101;
/// Left-out elements printed per model.
const SHOWN: usize = 20;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut s) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += u32::from(x);
            s += a;
        }
        a %= 65521;
        s %= 65521;
    }
    (s << 16) | a
}

/// Every verified data object in `b` (RE-153), by ElementId: `(start, end)`.
fn objects(b: &[u8]) -> BTreeMap<u32, Vec<(usize, usize)>> {
    let mut out: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    let mut p = 0;
    while p + 20 <= b.len() {
        if u32_at(b, p + 4) == Some(0) {
            if let (Some(id), Some(sum), Some(size), Some(class)) = (
                u32_at(b, p),
                u32_at(b, p + 8),
                u32_at(b, p + 12),
                u32_at(b, p + 16),
            ) {
                let size = size as usize;
                let end = p + 20 + size;
                if size >= 4 && end <= b.len() && u32_at(b, end - 4) == Some(size as u32) {
                    let mut data = class.to_le_bytes().to_vec();
                    data.extend_from_slice(&b[p + 20..end]);
                    data.truncate(size);
                    if adler32(&data) == sum {
                        out.entry(id).or_default().push((p, end));
                    }
                }
            }
        }
        p += 1;
    }
    out
}

/// The `u32` after the first `parameter` entry in `b[start..end]`.
fn value_after(b: &[u8], start: usize, end: usize, parameter: i64) -> Option<u32> {
    let needle = parameter.to_le_bytes();
    let at = memchr::memmem::find(b.get(start..end)?, &needle)?;
    u32_at(b, start + at + 8)
}

/// Every numeric `Tag` of the reference export next to the model.
fn exported_tags(path: &str) -> Option<BTreeSet<u64>> {
    let model = Path::new(path);
    let stem = model.file_stem()?.to_string_lossy().to_string();
    let dir = model.parent()?;
    let reference = [format!("{stem}_slim.ifc"), format!("{stem}.ifc")]
        .into_iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists())?;
    let step = std::fs::read_to_string(reference).ok()?;
    let mut out = BTreeSet::new();
    for line in step.lines() {
        // The Tag is a quoted number before the entity's last arguments; any
        // quoted all-digit string is taken, which over-counts harmlessly.
        for piece in line.split('\'').skip(1).step_by(2) {
            if !piece.is_empty() && piece.len() < 12 && piece.bytes().all(|b| b.is_ascii_digit()) {
                if let Ok(tag) = piece.parse() {
                    out.insert(tag);
                }
            }
        }
    }
    Some(out)
}

type Tally = BTreeMap<(i64, bool, Option<u32>, Option<u32>), usize>;

fn probe(path: &str) -> anyhow::Result<Vec<String>> {
    let Some(exported) = exported_tags(path) else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no reference export\"}}"
        )]);
    };
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let Some(marker) = per::file_bbox_marker(&mut rf, revit) else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no bbox marker\"}}"
        )]);
    };
    let mut tally: Tally = BTreeMap::new();
    let mut left_out = Vec::new();
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut exported_categories: BTreeSet<i64> = BTreeSet::new();
    let mut rows: Vec<(i64, u64, bool, Option<u32>, Option<u32>)> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let objs = objects(buf);
        for span in per::partition_record_chain(buf, &marker) {
            if !seen.insert(span.element_id) {
                continue;
            }
            let Ok(own) = u32::try_from(span.element_id) else {
                continue;
            };
            let Some(frame) = per::decode_frame_as(&stream, buf, span.start, own, &marker) else {
                continue;
            };
            let spans = objs.get(&own).cloned().unwrap_or_default();
            let read = |parameter| {
                spans
                    .iter()
                    .find_map(|(s, e)| value_after(buf, *s, *e, parameter))
            };
            let (created, demolished) = (read(PHASE_CREATED), read(PHASE_DEMOLISHED));
            let is_exported = exported.contains(&span.element_id);
            if is_exported {
                exported_categories.insert(frame.builtin_category);
            }
            rows.push((
                frame.builtin_category,
                span.element_id,
                is_exported,
                created,
                demolished,
            ));
        }
    }
    for (category, id, is_exported, created, demolished) in &rows {
        *tally
            .entry((*category, *is_exported, *created, *demolished))
            .or_default() += 1;
        if !is_exported && exported_categories.contains(category) && left_out.len() < SHOWN {
            left_out.push(format!(
                "{{\"category\":{category},\"element\":{id},\"created\":{created:?},\"demolished\":{demolished:?}}}"
            ));
        }
    }
    let mut out = vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"records\":{},\"exported_categories\":{}}}",
        rows.len(),
        exported_categories.len()
    )];
    for ((category, is_exported, created, demolished), n) in &tally {
        if !exported_categories.contains(category) {
            continue;
        }
        out.push(format!(
            "{{\"category\":{category},\"exported\":{is_exported},\"created\":{created:?},\"demolished\":{demolished:?},\"elements\":{n}}}"
        ));
    }
    out.push(format!("{{\"left_out\":[{}]}}", left_out.join(",")));
    Ok(out)
}

fn main() {
    // Measure passes flags such as `--records` after the paths.
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        match probe(path) {
            Ok(lines) => lines.iter().for_each(|l| println!("{l}")),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
