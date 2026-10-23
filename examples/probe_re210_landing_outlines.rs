//! RE-210 (#824): whether a stair landing's own data holds its plan outline.
//!
//! FACT under test: on the BIM4LCA office, 21 `StairsLanding` elements are
//! drawn as their record box, where Revit's export gives each an outline
//! with a notch, or an arc, where its stair meets it. A straight run keeps
//! its plan sketch as bounded lines in its data (RE-52); this probe prints
//! every bounded line and arc in each landing's data, for
//! `tools/re/landing_outlines_vs_ifc.py` to compare with Revit's outline.
//!
//! Output:
//!   landing <id> box <x> <y> <z> <w> <d> <h>
//!   line <id> <x0> <y0> <z0> <x1> <y1> <z1>
//!   arc <id> <cx> <cy> <cz> <radius> <start> <end> <xx> <xy> <yx> <yy>
//!
//! Usage:
//!   cargo run --profile ci --example probe_re210_landing_outlines -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_beam_axes::{BOUNDED_LINE_TAG, bounded_arc_at, bounded_line_at};
use rvt::research::partition_names::element_data_header;
use rvt::walker::{InstanceField, iter_elements};
use std::collections::BTreeSet;

fn field(fields: &[(String, InstanceField)], name: &str) -> f64 {
    fields
        .iter()
        .find_map(|(n, v)| match (n.as_str(), v) {
            (n2, InstanceField::Float { value, .. }) if n2 == name => Some(*value),
            _ => None,
        })
        .unwrap_or(f64::NAN)
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: probe_re210_landing_outlines FILE.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().map(|info| info.version).unwrap_or(0);
    let mut landings = BTreeSet::new();
    for element in iter_elements(&mut rf).expect("elements") {
        if element.class != "StairsLanding" {
            continue;
        }
        let Some(id) = element.id else { continue };
        let f = &element.fields;
        println!(
            "landing {id} box {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            field(f, "m_locationX"),
            field(f, "m_locationY"),
            field(f, "m_locationZ"),
            field(f, "m_bboxWidth"),
            field(f, "m_bboxDepth"),
            field(f, "m_bboxHeight")
        );
        landings.insert(id);
    }
    let Some(header) = element_data_header(version) else {
        println!("no element data header for {version}");
        return;
    };
    let mut seen = BTreeSet::new();
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
                .and_then(|s| s.first_chunk::<8>())
                .map(|b| u64::from_le_bytes(*b))
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| landings.contains(id) && seen.insert(*id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(id_at + 0x4000);
            let data = &buf[id_at..end];
            for at in memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG) {
                if let Some(arc) = bounded_arc_at(data, at) {
                    let c = arc.centre;
                    println!(
                        "arc {id} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                        c[0],
                        c[1],
                        c[2],
                        arc.radius,
                        arc.start_angle,
                        arc.end_angle,
                        arc.x_axis[0],
                        arc.x_axis[1],
                        arc.y_axis[0],
                        arc.y_axis[1]
                    );
                } else if let Some(line) = bounded_line_at(data, at) {
                    let (s, e) = (line.start(), line.end());
                    println!(
                        "line {id} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                        s[0], s[1], s[2], e[0], e[1], e[2]
                    );
                }
            }
        }
    }
}
