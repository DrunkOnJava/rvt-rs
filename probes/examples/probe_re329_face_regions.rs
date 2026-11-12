//! RE-329 (#1144, #1247): what a face with regions holds.
//!
//! FACT under test: `native_saved_mesh::face` and `cylinder_face` refuse a
//! face whose `m_faceRegions` is not empty (`face regions need subdivision`),
//! and the census (`probe_re329_saved_graphics_census`, Measure run
//! 41786625424) finds it the only reason three `IfcWindow` bodies of each of
//! six Revit 2025 models (r2025-12 to r2025-17, drawn from saved meshes by
//! default) keep their box: 24 plane faces in each file. This prints what such
//! a face holds, so that a reader can tell whether its regions only split the
//! face's area (for material or paint) or change it.
//!
//! NOT FOR PRIVATE FILES. It prints raw graph fields, the objects they point
//! to and UV and model coordinates of the faces it shows, so its output
//! describes the model's geometry. Run it on the public models of a Measure run
//! (r2025-12, r2025-17); for a file that must stay private use
//! `probe_re329_saved_graphics_census`, which prints counts and class names
//! only.
//!
//! Output, for every graph of the file: for the first faces with regions their
//! fields (pointers followed two deep) and first loop, for the first six
//! owners holding any the count of faces with and without regions, then
//! `census regions <n> faces <m>` lines (how many faces hold n regions).
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re329_face_regions -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::pointer;

fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    for key in ["m_id", "m_id64"] {
        if let Some(inner) = value.get(key) {
            return identifier(inner);
        }
    }
    None
}

fn describe(g: &ObjectGraph, oi: usize, depth: usize) -> String {
    let Some(o) = g.objects.get(oi) else {
        return "?".into();
    };
    let mut parts = Vec::new();
    if let Some(map) = o.fields.as_object() {
        for (k, v) in map {
            parts.push(format!("{k}={}", value_text(g, oi, v, depth)));
        }
    }
    format!("{}#{oi}{{{}}}", o.class_name, parts.join(" "))
}

fn value_text(g: &ObjectGraph, oi: usize, v: &Fv, depth: usize) -> String {
    match v {
        Fv::Object(m) if m.contains_key("pointer_token") => match pointer(g, oi, v) {
            Ok(Some(t)) if depth > 0 => describe(g, t, depth - 1),
            Ok(Some(t)) => format!("->{}#{t}", g.objects[t].class_name),
            Ok(None) => "null".into(),
            Err(_) => "->?".into(),
        },
        Fv::Array(a) => {
            let items: Vec<String> = a
                .iter()
                .take(8)
                .map(|x| value_text(g, oi, x, depth))
                .collect();
            let more = if a.len() > 8 {
                format!(",..{}", a.len())
            } else {
                String::new()
            };
            format!("[{}{more}]", items.join(","))
        }
        Fv::Object(m) => {
            let items: Vec<String> = m
                .iter()
                .map(|(k, x)| format!("{k}:{}", value_text(g, oi, x, depth)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
        other => other.to_string(),
    }
}

fn clip(text: String, n: usize) -> String {
    text.chars().take(n).collect()
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re329_face_regions FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let table = rvt::elem_table::parse_records(&mut rf)?;
    let mut options = Options {
        selected_ids: rvt::elem_table::declared_ids(&table)
            .into_iter()
            .map(u64::from)
            .collect(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let mut records: BTreeMap<u64, Record> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |r| {
        records.insert(r.identity.element_id, r);
        Ok(())
    })?;
    let mut attempted: BTreeSet<u64> = records.keys().copied().collect();
    for _ in 0..64 {
        let mut wanted = BTreeSet::new();
        for r in records.values() {
            let Some(g) = &r.graph else { continue };
            for o in &g.objects {
                if o.class_name != "InstanceInfo" {
                    continue;
                }
                if let Some(id) = identifier(&o.fields["m_symbolId"])
                    .and_then(|id| u64::try_from(id).ok())
                    .filter(|id| *id > 0)
                {
                    if !attempted.contains(&id) {
                        wanted.insert(id);
                    }
                }
            }
        }
        if wanted.is_empty() {
            break;
        }
        attempted.extend(&wanted);
        options.selected_ids = wanted;
        extract_graphs(&mut rf, &options, |r| {
            records.insert(r.identity.element_id, r);
            Ok(())
        })?;
    }
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    let mut shown = 0usize;
    let mut owners = 0usize;
    for r in records.values() {
        let Some(g) = &r.graph else { continue };
        let mut with = 0usize;
        let mut without = 0usize;
        for (fi, face) in g.objects.iter().enumerate() {
            if face.class_name != "Face" {
                continue;
            }
            let regions = face.fields["m_faceRegions"]
                .as_array()
                .map_or(0, <[Fv]>::len);
            *histogram.entry(regions).or_default() += 1;
            if regions == 0 {
                without += 1;
                continue;
            }
            with += 1;
            if shown >= 4 {
                continue;
            }
            shown += 1;
            println!(
                "face of owner {} fi={fi}: {}",
                r.identity.element_id,
                clip(describe(g, fi, 2), 3000)
            );
            if let Ok(Some(li)) = pointer(g, fi, &face.fields["m_pFirstLoop"]) {
                println!("  first loop: {}", clip(describe(g, li, 1), 1500));
            }
        }
        if with > 0 && owners < 6 {
            owners += 1;
            println!(
                "owner {} faces with regions {with} without {without}",
                r.identity.element_id
            );
        }
    }
    for (regions, faces) in &histogram {
        println!("census regions {regions} faces {faces}");
    }
    Ok(())
}
