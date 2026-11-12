//! RE-329 (#1144, #1247): which faces keep a saved mesh from decoding whole,
//! and how many elements each one blocks.
//!
//! FACT under test: `saved_meshes::attach` keeps an element's record box when
//! any non-view diagnostic is left in its saved graphics (RE-224), and the
//! graphics walk stops a `Geometry` at its first failing face, so
//! `GraphicsMeshes::diagnostics` names one failure per carrier. #1144 asked
//! for cone faces (landed, RE-311), spline-profile surfaces (RE-311 section
//! 3: no scored body gained a mesh) and cylinder trims (RE-304). This counts,
//! for the elements `attach` would replace and keeps as boxes, EVERY failing
//! face by surface class, profile curve class and trim shape, then gives each
//! element the set of causes over all its faces, so the kind that frees the
//! most elements can be told from one that only ever co-occurs with another.
//!
//! The candidate set is `attach_scoped_with_control`'s: a building element
//! exported with the record box as its body and not profile-resolved, whose
//! Tag no other entity carries. Elements whose graphics give no surface at all
//! are counted (they stay boxes). An element is blocked as `attach` decides:
//! some diagnostic that does not end with the nested-detail-filter or the
//! view-component message is left.
//!
//! A face's causes are read from the graph, independent of the order in which
//! the mesher happens to fail:
//! - `surface:<class>`: a surface class the mesher does not read;
//! - `profile:<class>`: a revolution's or ruled surface's profile curve other
//!   than `GLine` and `GArc`;
//! - `nonrect:<class>`: a revolution, ruled surface or cone whose single trim
//!   loop has an edge that lies on no side of its UV box;
//! - `multiloop:<class>`: a face with several trim loops;
//! - `err:<class>:<message>`: the mesher's message when it is none of the
//!   above (e.g. `disconnected parametric trim in model coordinates`).
//!
//! `traversal:<message>` is a diagnostic of the graphics walk itself and
//! `carrier:<message>` a failure of a carrier that is not a `Geometry` (a
//! `GPolyMesh`'s faceted topology) or of its transform.
//!
//! Output, counts only (no id, name or coordinate). `<scope>` is
//! `door_window` for IFCDOOR and IFCWINDOW (drawn from saved meshes by
//! default) and `other` for every other class (opt-in `--saved-meshes`):
//!   release <year>
//!   streams_not_read <n>
//!   census candidates <n> single <n> ambiguous_tags <n>
//!   census outcome <scope> <outcome> <n>
//!   census class_blocked <ifc type> <n>
//!   census cause_any <scope> <cause> <n>
//!   census cause_set <scope> <cause>+<cause>... <n>
//!   census face_ok <surface class> <n>      (faces of blocked elements)
//!   census face_fail <surface>[<profiles>] <trim> <message kind> <n>
//!   census unbounded_faces <n>
//! where <outcome> is `drawn`, `box_partial` (a mesh with a face missing),
//! `box_no_surface` (no face decoded), `empty_view_only` (nothing but view
//! diagnostics) or `empty_none` (no saved graphics selected), and <trim> is
//! `unread`, `1loop-rect`, `1loop-nonrect` or `2+loops`.
//!
//! The file is opened with `--large-model`'s limits, as `rvt-ifc
//! --large-model` opens it, so that a partition over the default limits is
//! read; `streams_not_read` says whether any part still could not be read.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re329_saved_graphics_census -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_graphics_traversal::{
    NESTED_DETAIL_FILTER_DIAGNOSTIC, VIEW_COMPONENT_DIAGNOSTIC,
    select_graphics_with_resolver_at_detail,
};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{face, graphics_with_resolver_at_detail, pointer};

const RECORD_BOX: &str = "partition_element_record_bbox";
const READ_SURFACES: [&str; 5] = ["Plane", "CylSurf", "SurfRev", "RuledSurf", "ConeSurf"];
const PARAMETRIC: [&str; 3] = ["SurfRev", "RuledSurf", "ConeSurf"];
const READ_PROFILES: [&str; 2] = ["GLine", "GArc"];
const EXPLAINED: [&str; 6] = [
    "unsupported surface",
    "unsupported parametric surface",
    "unsupported parametric profile curve",
    "nonrectangular parametric trim",
    "multiple parametric trim loops unsupported",
    "multiple cylinder trim loops unsupported",
];

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

fn message_kind(message: &str) -> String {
    let text = match message.strip_prefix("object ") {
        Some(rest) => rest.split_once(": ").map_or(rest, |(_, kind)| kind),
        None => message,
    };
    let words: Vec<String> = text
        .split_whitespace()
        .map(|word| {
            let numeric = word.chars().any(|c| c.is_ascii_digit())
                && word
                    .chars()
                    .all(|c| c.is_ascii_digit() || ".-+eE".contains(c));
            if numeric {
                "#".to_string()
            } else {
                word.chars()
                    .map(|c| if c.is_ascii_digit() { '#' } else { c })
                    .collect()
            }
        })
        .collect();
    words.join(" ")
}

fn ignorable(diagnostic: &str) -> bool {
    diagnostic.ends_with(NESTED_DETAIL_FILTER_DIAGNOSTIC)
        || diagnostic.ends_with(VIEW_COMPONENT_DIAGNOSTIC)
}

fn uv(value: &Fv) -> Option<[f64; 2]> {
    let a = value.as_array()?;
    if a.len() != 2 {
        return None;
    }
    Some([a[0].as_f64()?, a[1].as_f64()?])
}

struct Trim {
    loops: usize,
    rect: Option<bool>,
}

impl Trim {
    fn label(&self) -> &'static str {
        match (self.loops, self.rect) {
            (0, _) => "unread",
            (1, Some(true)) => "1loop-rect",
            (1, Some(false)) => "1loop-nonrect",
            (1, None) => "unread",
            _ => "2+loops",
        }
    }
}

fn loop_is_rectangle(g: &ObjectGraph, fi: usize, li: usize) -> Option<bool> {
    let l = g.objects.get(li)?;
    let mut cur = pointer(g, li, &l.fields["m_next"]).ok()??;
    let mut seen = BTreeSet::new();
    let mut polylines: Vec<Vec<[f64; 2]>> = Vec::new();
    while cur != li {
        if !seen.insert(cur) {
            return None;
        }
        let e = g.objects.get(cur)?;
        if e.class_name != "Edge" {
            return None;
        }
        let faces = e.fields["m_pFace"].as_array()?;
        let mut side = None;
        for (i, p) in faces.iter().enumerate() {
            if pointer(g, cur, p).ok()? == Some(fi) {
                if side.is_some() {
                    return None;
                }
                side = Some(i);
            }
        }
        let side = side?;
        let ends = e.fields["m_firstAndLastEdgePnts"].as_array()?;
        if ends.len() != 2 {
            return None;
        }
        let mut points = vec![uv(&ends[0]["uv"][side])?];
        for p in e.fields["m_interiorEdgePnts"].as_array()? {
            points.push(uv(&p["uv"][side])?);
        }
        points.push(uv(&ends[1]["uv"][side])?);
        polylines.push(points);
        cur = pointer(g, cur, &e.fields["m_next"][side]).ok()??;
    }
    let all: Vec<[f64; 2]> = polylines.iter().flatten().copied().collect();
    if all.is_empty() {
        return None;
    }
    let min: [f64; 2] =
        std::array::from_fn(|k| all.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let max: [f64; 2] =
        std::array::from_fn(|k| all.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max));
    let near = |a: f64, b: f64| (a - b).abs() < 1e-7;
    let on_side = |a: [f64; 2], b: [f64; 2]| {
        (near(a[1], min[1]) && near(b[1], min[1]))
            || (near(a[0], max[0]) && near(b[0], max[0]))
            || (near(a[1], max[1]) && near(b[1], max[1]))
            || (near(a[0], min[0]) && near(b[0], min[0]))
    };
    Some(polylines.iter().all(|line| {
        line.windows(2).all(|w| {
            ((w[0][0] - w[1][0]).abs() < 1e-12 && (w[0][1] - w[1][1]).abs() < 1e-12)
                || on_side(w[0], w[1])
        })
    }))
}

fn trim_of(g: &ObjectGraph, fi: usize) -> Trim {
    let unread = Trim {
        loops: 0,
        rect: None,
    };
    let Some(f) = g.objects.get(fi) else {
        return unread;
    };
    let Ok(Some(first)) = pointer(g, fi, &f.fields["m_pFirstLoop"]) else {
        return unread;
    };
    let mut seen = BTreeSet::from([first]);
    let mut current = first;
    while let Ok(Some(next)) = pointer(g, current, &g.objects[current].fields["m_nextLoop"]) {
        if !seen.insert(next) {
            break;
        }
        current = next;
    }
    let loops = seen.len();
    let rect = if loops == 1 {
        loop_is_rectangle(g, fi, first)
    } else {
        None
    };
    Trim { loops, rect }
}

fn surface_of(g: &ObjectGraph, fi: usize) -> (String, Vec<String>) {
    let Some(f) = g.objects.get(fi) else {
        return ("none".into(), Vec::new());
    };
    let Ok(Some(si)) = pointer(g, fi, &f.fields["m_pSurf"]) else {
        return ("none".into(), Vec::new());
    };
    let surface = &g.objects[si];
    let fields: &[&str] = match surface.class_name.as_str() {
        "SurfRev" => &["m_pProfileCurve"],
        "RuledSurf" => &["m_pProfileCurve1", "m_pProfileCurve2"],
        _ => &[],
    };
    let profiles: BTreeSet<String> = fields
        .iter()
        .map(|field| match pointer(g, si, &surface.fields[*field]) {
            Ok(Some(ci)) => g.objects[ci].class_name.clone(),
            _ => "none".to_string(),
        })
        .collect();
    (surface.class_name.clone(), profiles.into_iter().collect())
}

#[derive(Default)]
struct Analysis {
    ifc_type: String,
    door_window: bool,
    outcome: &'static str,
    causes: BTreeSet<String>,
    faces_ok: BTreeMap<String, usize>,
    face_fail: BTreeMap<String, usize>,
    unbounded: usize,
}

fn analyse(id: u64, records: &BTreeMap<u64, Record>, out: &mut Analysis) {
    let Some(record) = records.get(&id) else {
        out.outcome = "empty_none";
        return;
    };
    let Some(g) = record.graph.as_ref() else {
        out.outcome = "box_no_surface";
        out.causes.insert(format!(
            "carrier:{}",
            message_kind(
                record
                    .diagnostic
                    .as_deref()
                    .unwrap_or("graphics graph unavailable")
            )
        ));
        return;
    };
    let resolver = |rid: u64| records.get(&rid).and_then(|r| r.graph.as_ref());
    let meshes = graphics_with_resolver_at_detail(g, &resolver, 3);
    let blocking = meshes.diagnostics.iter().any(|d| !ignorable(d));
    out.outcome = match (meshes.primitives.is_empty(), blocking) {
        (false, false) => "drawn",
        (false, true) => "box_partial",
        (true, true) => "box_no_surface",
        (true, false) if !meshes.diagnostics.is_empty() => "empty_view_only",
        (true, false) => "empty_none",
    };
    if !blocking {
        return;
    }
    let selection = select_graphics_with_resolver_at_detail(g, &resolver, 3);
    let mut expected: HashMap<String, usize> = HashMap::new();
    for d in &selection.diagnostics {
        let text = format!("object {}: {}", d.object_index, d.message);
        if !ignorable(&text) {
            out.causes
                .insert(format!("traversal:{}", message_kind(&d.message)));
        }
        *expected.entry(text).or_default() += 1;
    }
    for s in &selection.selected {
        let sg = match s.source_owner_id {
            None => g,
            Some(owner) => match records.get(&owner).and_then(|r| r.graph.as_ref()) {
                Some(x) => x,
                None => continue,
            },
        };
        let oi = s.object_index;
        let Some(obj) = sg.objects.get(oi) else {
            continue;
        };
        if obj.class_name != "Geometry" {
            continue;
        }
        let faces = obj.fields["m_pFaces"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut first_error: Option<String> = None;
        for ptr in &faces {
            let fi = match pointer(sg, oi, ptr) {
                Ok(Some(fi)) => fi,
                Ok(None) => {
                    first_error.get_or_insert_with(|| "null geometry face".into());
                    continue;
                }
                Err(e) => {
                    first_error.get_or_insert_with(|| format!("{e:#}"));
                    continue;
                }
            };
            let (surface, profiles) = surface_of(sg, fi);
            match face(sg, fi) {
                Ok(Some(_)) => *out.faces_ok.entry(surface).or_default() += 1,
                Ok(None) => out.unbounded += 1,
                Err(e) => {
                    let message = format!("{e:#}");
                    let kind = message_kind(&message);
                    first_error.get_or_insert(message);
                    let trim = trim_of(sg, fi);
                    let profile_label = if profiles.is_empty() {
                        String::new()
                    } else {
                        format!("[{}]", profiles.join("+"))
                    };
                    *out.face_fail
                        .entry(format!("{surface}{profile_label} {} {kind}", trim.label()))
                        .or_default() += 1;
                    if !READ_SURFACES.contains(&surface.as_str()) {
                        out.causes.insert(format!("surface:{surface}"));
                    } else {
                        for profile in &profiles {
                            if !READ_PROFILES.contains(&profile.as_str()) {
                                out.causes.insert(format!("profile:{profile}"));
                            }
                        }
                        if PARAMETRIC.contains(&surface.as_str()) || surface == "CylSurf" {
                            if trim.loops > 1 {
                                out.causes.insert(format!("multiloop:{surface}"));
                            } else if PARAMETRIC.contains(&surface.as_str())
                                && trim.rect == Some(false)
                            {
                                out.causes.insert(format!("nonrect:{surface}"));
                            }
                        }
                        let explained = EXPLAINED.iter().any(|e| kind.starts_with(e));
                        if !explained {
                            out.causes.insert(format!("err:{surface}:{kind}"));
                        }
                    }
                }
            }
        }
        if let Some(message) = first_error {
            *expected
                .entry(format!("object {oi}: {message}"))
                .or_default() += 1;
        }
    }
    for d in &meshes.diagnostics {
        match expected.get_mut(d) {
            Some(n) if *n > 0 => *n -= 1,
            _ => {
                if !ignorable(d) {
                    out.causes.insert(format!("carrier:{}", message_kind(d)));
                }
            }
        }
    }
    if out.causes.is_empty() {
        out.causes.insert("unexplained".into());
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, true))?;
    println!("release {}", rf.basic_file_info()?.version);
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    let mut by_id: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            location_feet: Some(_),
            ..
        } = entity
        else {
            continue;
        };
        let boxed = set.properties.iter().any(|p| {
            p.name == "BodySource" && matches!(&p.value, PropertyValue::Text(t) if t == RECORD_BOX)
        });
        let resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        if !boxed || resolved {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            by_id.entry(id).or_default().push(ifc_type.to_string());
        }
    }
    let single: BTreeMap<u64, String> = by_id
        .iter()
        .filter(|(_, types)| types.len() == 1)
        .map(|(id, types)| (*id, types[0].clone()))
        .collect();
    println!(
        "census candidates {} single {} ambiguous_tags {}",
        by_id.len(),
        single.len(),
        by_id.len() - single.len()
    );
    let mut options = Options {
        selected_ids: single.keys().copied().collect(),
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
    for _ in 0..128 {
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
    println!("streams_not_read {}", rf.unread_streams().len());
    let roots: Vec<(u64, String)> = single.into_iter().collect();
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut results: Vec<(usize, Analysis)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some((id, ifc_type)) = roots.get(i) else {
                            break;
                        };
                        let mut analysis = Analysis {
                            ifc_type: ifc_type.clone(),
                            door_window: ifc_type.eq_ignore_ascii_case("IFCDOOR")
                                || ifc_type.eq_ignore_ascii_case("IFCWINDOW"),
                            ..Analysis::default()
                        };
                        analyse(*id, &records, &mut analysis);
                        done.push((i, analysis));
                    }
                    done
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    results.sort_by_key(|(i, _)| *i);
    type Counts = BTreeMap<String, usize>;
    let mut outcomes: Counts = BTreeMap::new();
    let mut classes: Counts = BTreeMap::new();
    let mut cause_any: Counts = BTreeMap::new();
    let mut cause_set: Counts = BTreeMap::new();
    let mut faces_ok: Counts = BTreeMap::new();
    let mut face_fail: Counts = BTreeMap::new();
    let mut unbounded = 0usize;
    for (_, a) in &results {
        let scope = if a.door_window {
            "door_window"
        } else {
            "other"
        };
        *outcomes
            .entry(format!("{scope} {}", a.outcome))
            .or_default() += 1;
        unbounded += a.unbounded;
        for (surface, n) in &a.faces_ok {
            *faces_ok.entry(surface.clone()).or_default() += n;
        }
        for (key, n) in &a.face_fail {
            *face_fail.entry(key.clone()).or_default() += n;
        }
        if a.outcome != "box_partial" && a.outcome != "box_no_surface" {
            continue;
        }
        *classes.entry(a.ifc_type.clone()).or_default() += 1;
        for cause in &a.causes {
            *cause_any.entry(format!("{scope} {cause}")).or_default() += 1;
        }
        let set: Vec<&str> = a.causes.iter().map(String::as_str).collect();
        *cause_set
            .entry(format!("{scope} {}", set.join("+")))
            .or_default() += 1;
    }
    for (key, n) in &outcomes {
        println!("census outcome {key} {n}");
    }
    for (key, n) in &classes {
        println!("census class_blocked {key} {n}");
    }
    for (key, n) in &cause_any {
        println!("census cause_any {key} {n}");
    }
    for (key, n) in &cause_set {
        println!("census cause_set {key} {n}");
    }
    for (key, n) in &faces_ok {
        println!("census face_ok {key} {n}");
    }
    for (key, n) in &face_fail {
        println!("census face_fail {key} {n}");
    }
    println!("census unbounded_faces {unbounded}");
    Ok(())
}
