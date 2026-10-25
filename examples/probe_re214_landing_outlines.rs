//! RE-214 (#824): whether a stair landing's own data holds its plan outline.
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
//!   landing-type <id> own <id|None>
//!   data-line <id> <x0> <y0> <z0> <x1> <y1> <z1>   (every line, 64 KiB window)
//!   gate <id> native <top and thickness|None> lines <n> nearest-to-top <ft>
//!   native <id> <index> <class> <member=value ...>
//!
//! The `native` lines are the native record objects of each landing, each
//! `StairsElement`, `StairsType` and `StairsLandingType`, and each landing
//! type a `StairsType` names (`m_landingTypeId`), read by member name: every
//! number, flag, string and element id (`@id`) they hold at their top level,
//! for the landing's top and thickness to be found by name.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re214_landing_outlines -- FILE.rvt

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_stairs::native_landings;
use rvt::research::partition_beam_axes::{BOUNDED_LINE_TAG, bounded_arc_at, bounded_line_at};
use rvt::research::partition_element_records::{
    OST_STAIRS_LANDINGS, REFERENCE_LIST_OFFSET, decode_reference_list,
};
use rvt::research::partition_names::element_data_header;
use rvt::research::partition_stairs::{scan_landing_lines, scan_run_lines};
use rvt::research::partition_type_records::{
    scan_type_records, type_definition_ids, unique_type_reference,
};
use rvt::walker::{InstanceField, iter_elements};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn field(fields: &[(String, InstanceField)], name: &str) -> f64 {
    fields
        .iter()
        .find_map(|(n, v)| match (n.as_str(), v) {
            (n2, InstanceField::Float { value, .. }) if n2 == name => Some(*value),
            _ => None,
        })
        .unwrap_or(f64::NAN)
}

/// Prints every object of each selected record's graph as a `native` line,
/// and returns the landing types its `StairsType` records name.
fn print_native(
    rf: &mut RevitFile,
    options: &Options,
    printed: &mut BTreeSet<u64>,
) -> BTreeSet<u64> {
    let mut landing_types = BTreeSet::new();
    extract_graphs(rf, options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        printed.insert(id);
        for (index, object) in graph.objects.iter().enumerate() {
            if index == 0 && class == "StairsType" {
                if let Some(t) = object
                    .fields
                    .get("m_landingTypeId")
                    .and_then(|v| v.get("m_id")?.get("m_id64")?.as_u64())
                {
                    landing_types.insert(t);
                }
            }
            let members: Vec<String> = object
                .fields
                .as_object()
                .into_iter()
                .flat_map(|members| members.iter())
                .filter_map(|(name, value)| match value {
                    Value::Number(_) | Value::Bool(_) => Some(format!("{name}={value}")),
                    Value::String(text) => Some(format!("{name}={}", text.replace(' ', "_"))),
                    _ => value
                        .get("m_id")?
                        .get("m_id64")
                        .map(|id| format!("{name}=@{id}")),
                })
                .collect();
            println!(
                "native {id} {index} {class}/{} {}",
                object.class_name,
                members.join(" ")
            );
        }
        Ok(())
    })
    .expect("native records");
    landing_types
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: probe_re214_landing_outlines FILE.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().map(|info| info.version).unwrap_or(0);
    let mut landings = BTreeSet::new();
    let elements: Vec<_> = iter_elements(&mut rf)
        .expect("elements")
        .filter(|element| element.class == "StairsLanding")
        .collect();
    let declared =
        rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf).expect("ElemTable"));
    let landing_types = type_definition_ids(
        &scan_type_records(&mut rf, version, OST_STAIRS_LANDINGS, &declared).expect("type records"),
    );
    let mut own_types: BTreeMap<u64, u64> = BTreeMap::new();
    for element in elements {
        let Some(id) = element.id else { continue };
        let f = &element.fields;
        println!(
            "landing {id} box {} {} {} {} {} {}",
            field(f, "m_locationX"),
            field(f, "m_locationY"),
            field(f, "m_locationZ"),
            field(f, "m_bboxWidth"),
            field(f, "m_bboxDepth"),
            field(f, "m_bboxHeight")
        );
        let mut stream = None;
        let mut offset = None;
        for (name, value) in f {
            match (name.as_str(), value) {
                ("m_source_stream", InstanceField::String(v)) => stream = Some(v.clone()),
                ("m_source_offset", InstanceField::Integer { value, .. }) => {
                    offset = usize::try_from(*value).ok();
                }
                _ => {}
            }
        }
        let references = match (stream, offset) {
            (Some(stream), Some(offset)) => rf
                .inflated_partition(&stream)
                .ok()
                .and_then(|inflated| {
                    decode_reference_list(inflated.bytes(), offset + REFERENCE_LIST_OFFSET)
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let own = unique_type_reference(&references, &landing_types);
        println!("landing-type {id} own {own:?}");
        if let Some(own) = own {
            own_types.insert(u64::from(id), u64::from(own));
        }
        landings.insert(id);
    }
    // What `attach_landing_slabs` reads, gate by gate: each landing's top and
    // monolithic thickness, and every line of its data, with how far each
    // line's ends lie from the top.
    let native = native_landings(&mut rf, &own_types).expect("native landings");
    let lines = scan_landing_lines(&mut rf, version, &landings).expect("landing lines");
    for (id, held) in scan_run_lines(&mut rf, version, &landings).expect("run lines") {
        for line in held {
            let (s, e) = (line.start(), line.end());
            println!(
                "data-line {id} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                s[0], s[1], s[2], e[0], e[1], e[2]
            );
        }
    }
    for id in &landings {
        let landing = native.get(&u64::from(*id));
        let held = lines.get(id);
        let nearest = match (landing, held) {
            (Some(landing), Some(held)) => held
                .iter()
                .map(|line| {
                    (line.start()[2] - landing.top_feet)
                        .abs()
                        .max((line.end()[2] - landing.top_feet).abs())
                })
                .fold(f64::INFINITY, f64::min),
            _ => f64::NAN,
        };
        println!(
            "gate {id} native {landing:?} lines {} nearest-to-top {nearest:e}",
            held.map_or(0, Vec::len)
        );
    }
    let options = Options {
        selected_ids: landings.iter().map(|&id| u64::from(id)).collect(),
        selected_classes: ["StairsLandingType", "StairsType", "StairsElement"]
            .map(String::from)
            .into(),
        ..Options::default()
    };
    let mut printed = BTreeSet::new();
    let landing_types = print_native(&mut rf, &options, &mut printed);
    let options = Options {
        selected_ids: landing_types.difference(&printed).copied().collect(),
        ..Options::default()
    };
    if !options.selected_ids.is_empty() {
        print_native(&mut rf, &options, &mut printed);
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
