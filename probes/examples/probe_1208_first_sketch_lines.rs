//! Research probe for #1208 (not to land): the first-drawn sketch lines of
//! every element that owns sketch lines.
//!
//! FACT under test: Revit's export draws a slab's or covering's body as an
//! extrusion whose profile's X axis runs along the profile outline's first
//! edge (114 of 117 slabs and coverings with a `Width` or `Length` on the
//! BIM4LCA office as built). This prints, for each owner (a floor, roof,
//! ceiling and anything else whose sketch lines name it), its straight
//! sketch lines' count and its two lowest-ElementId lines with the ends each
//! is stored with (`GLine` origin, direction and end parameters), in model
//! feet, so the first edge can be matched to a line offline:
//!
//!   owner <id> lines <n> first <id> <x0> <y0> <x1> <y1> second <id> <x0> <y0> <x1> <y1>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1208_first_sketch_lines -- MODEL.rvt

use std::collections::BTreeMap;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::partition_element_records::{self as per};

const SKETCH_LINES: i64 = -2_000_045;

type Ends = ([f64; 2], [f64; 2]);

fn numbers<const N: usize>(value: Option<&Fv>) -> Option<[f64; N]> {
    let items = value?.as_array()?;
    if items.len() != N {
        return None;
    }
    let mut out = [0.0; N];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = item.as_f64()?;
    }
    Some(out)
}

fn line_ends(fields: &Fv) -> Option<Ends> {
    let origin = numbers::<3>(fields.get("m_origin"))?;
    let direction = numbers::<3>(fields.get("m_dirVec"))?;
    let params = numbers::<2>(fields.get("m_endParams"))?;
    let at = |t: f64| [origin[0] + direction[0] * t, origin[1] + direction[1] * t];
    Some((at(params[0]), at(params[1])))
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let revit = rf.basic_file_info()?.version;
    let mut owner_of: BTreeMap<u64, u64> = BTreeMap::new();
    if let Some(marker) = per::file_bbox_marker(&mut rf, revit) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for span in per::partition_record_chain(buf, &marker) {
                let Some(frame) =
                    per::decode_frame_as(&stream, buf, span.start, span.element_id as u32, &marker)
                else {
                    continue;
                };
                if frame.builtin_category != SKETCH_LINES {
                    continue;
                }
                if let Some(owner) = frame.owner_reference {
                    owner_of.entry(span.element_id).or_insert(u64::from(owner));
                }
            }
        }
    }
    let options = Options {
        selected_ids: owner_of.keys().copied().collect(),
        ..Options::default()
    };
    let mut geom: BTreeMap<u64, Ends> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record: Record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for object in &graph.objects {
            if object.class_name == "GLine" {
                if let Some(ends) = line_ends(&object.fields) {
                    geom.insert(id, ends);
                }
            }
        }
        Ok(())
    })?;
    let mut by_owner: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for (line, owner) in &owner_of {
        if geom.contains_key(line) {
            by_owner.entry(*owner).or_default().push(*line);
        }
    }
    let show = |id: u64| {
        geom.get(&id).map_or_else(String::new, |(a, b)| {
            format!("{id} {:.4} {:.4} {:.4} {:.4}", a[0], a[1], b[0], b[1])
        })
    };
    for (owner, mut lines) in by_owner {
        lines.sort_unstable();
        let first = lines.first().map_or_else(String::new, |id| show(*id));
        let second = lines.get(1).map_or_else(String::new, |id| show(*id));
        println!(
            "owner {owner} lines {} first {first} second {second}",
            lines.len()
        );
    }
    println!("owners with straight sketch lines: counted above, no ids follow");
    Ok(())
}
