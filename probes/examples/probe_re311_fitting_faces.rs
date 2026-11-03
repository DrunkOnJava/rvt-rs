//! RE-311 (#1144, #96): what a saved fitting's graphics hold, face by face.
//!
//! FACT under test: with `--saved-meshes`, RE1-Plumbing's welded elbows and
//! tees come out at 1.2 to 1.5 times Revit's volume although every vertex is
//! within 0.01 ft of Revit's body, and RE1-Mechanical's three closet bends
//! (`Closet Bend Reducing`) come out with a negative or a wrong volume. Nine
//! more fittings keep their box for `unsupported surface ConeSurf` and two
//! panels for a spline profile curve (#1144). This prints, for every saved
//! element, each selected primitive (a `Geometry` of B-rep faces or a
//! `GPolyMesh`), each face's surface class, triangle count, area and signed
//! volume about the element's centre, and whether its triangles are wound
//! alike, so a face that is turned the wrong way, or one that does not
//! decode, can be told apart from a body that is only meshed finer than
//! Revit's export.
//!
//! Output, one line per fact (`L<n>` is the detail level, 1 to 3):
//!   elem <id> L<n> prims=<n> tris=<n> svol=<v> diagnostics=<n> unbounded=<n>
//!     edges(consistent=<n> samedir=<n> boundary=<n> other=<n>) <messages>
//!   prim <id> L<n> oi=<object> class=<Geometry|GPolyMesh> owner=<id|-> det=<+|->
//!     tris=<n> svol=<v> [facets=<n> opposed=<n> consistent=<n> samedir=<n>
//!     boundary=<n> other=<n>]
//!   face <id> L<n> oi=<object> fi=<face> surf=<class> ok tris=<n> area=<a> svol=<v>
//!   face <id> L<n> oi=<object> fi=<face> surf=<class> err=<message>
//!   surface <id> <class> <fields, pointers followed two deep>
//!   edge <id> <fields of the first two trim edges of a face that failed>
//!   object <owner> <class> parents=[..] <fields>   (ConeSurf and spline classes)
//!   tri <id> L<n> <label> x y z x y z x y z        (named elements and
//!     every element with a face on a surface the mesher does not read, or
//!     that failed, document feet, 6 decimals)
//!   census <what> <count>
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re311_fitting_faces -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_graphics_traversal::{
    GraphicsTransform, select_graphics_with_resolver_at_detail,
};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{self, pointer};
use serde_json::Value;

/// Elements of RE1-Plumbing and RE1-Mechanical whose bodies are dumped at
/// every detail level: welded elbows of three sizes, two welded tees, the
/// reducing closet bends of both files.
const NAMED: [u64; 12] = [
    445080, 447350, 444370, 447684, 444607, 444676, 447845, 444721, 447864, 442037, 442191, 442040,
];
const READ_SURFACES: [&str; 4] = ["Plane", "CylSurf", "SurfRev", "RuledSurf"];
const WATCHED: [&str; 4] = ["ConeSurf", "GNurbSpline", "GHermiteSpline", "HermiteSurf"];
const TRIANGLE_CAP: usize = 20_000;

type Vec3 = [f64; 3];
type Tri = [Vec3; 3];

struct Piece {
    label: String,
    tris: Vec<Tri>,
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn apply(m: &GraphicsTransform, p: Vec3) -> Vec3 {
    std::array::from_fn(|i| m[i][3] + (0..3).map(|j| m[i][j] * p[j]).sum::<f64>())
}

fn determinant(m: &GraphicsTransform) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn p3(v: &Value) -> Option<Vec3> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?])
}

fn identifier(value: &Value) -> Option<i64> {
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

fn signed_volume(tris: &[Tri], origin: Vec3) -> f64 {
    tris.iter()
        .map(|t| {
            let a = sub(t[0], origin);
            let b = sub(t[1], origin);
            let c = sub(t[2], origin);
            dot(a, cross(b, c)) / 6.0
        })
        .sum()
}

fn area(tris: &[Tri]) -> f64 {
    tris.iter()
        .map(|t| {
            let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            0.5 * dot(n, n).sqrt()
        })
        .sum()
}

/// Edges of triangles named by any hashable vertex key: `(consistent, samedir,
/// boundary, other)`. A closed, consistently wound mesh has every edge once in
/// each direction.
fn edge_stats<K: Ord + Copy>(facets: &[[K; 3]]) -> (usize, usize, usize, usize) {
    let mut directed: BTreeMap<(K, K), usize> = BTreeMap::new();
    for t in facets {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *directed.entry((a, b)).or_default() += 1;
        }
    }
    let mut seen = BTreeSet::new();
    let (mut consistent, mut samedir, mut boundary, mut other) = (0, 0, 0, 0);
    for &(a, b) in directed.keys() {
        let key = if a < b { (a, b) } else { (b, a) };
        if !seen.insert(key) {
            continue;
        }
        let forward = directed.get(&(a, b)).copied().unwrap_or(0);
        let back = directed.get(&(b, a)).copied().unwrap_or(0);
        match (forward, back) {
            (1, 1) => consistent += 1,
            (1, 0) | (0, 1) => boundary += 1,
            (2, 0) | (0, 2) => samedir += 1,
            _ => other += 1,
        }
    }
    (consistent, samedir, boundary, other)
}

fn welded(tris: &[Tri]) -> Vec<[[i64; 3]; 3]> {
    let key = |p: Vec3| -> [i64; 3] { std::array::from_fn(|k| (p[k] * 1e5).round() as i64) };
    tris.iter()
        .map(|t| [key(t[0]), key(t[1]), key(t[2])])
        .collect()
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

fn value_text(g: &ObjectGraph, oi: usize, v: &Value, depth: usize) -> String {
    match v {
        Value::Object(m) if m.contains_key("pointer_token") => match pointer(g, oi, v) {
            Ok(Some(t)) if depth > 0 => describe(g, t, depth - 1),
            Ok(Some(t)) => format!("->{}#{t}", g.objects[t].class_name),
            Ok(None) => "null".into(),
            Err(_) => "->?".into(),
        },
        Value::Array(a) => {
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
        Value::Object(m) => {
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

fn message_kind(message: &str) -> String {
    let text = message.split_once(": ").map_or(message, |(_, rest)| rest);
    text.chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

struct Census {
    faces: BTreeMap<String, usize>,
    watched: BTreeMap<String, usize>,
    printed: BTreeMap<String, usize>,
}

fn element(
    id: u64,
    g: &ObjectGraph,
    records: &BTreeMap<u64, Record>,
    level: i64,
    named: bool,
    census: &mut Census,
) {
    let resolver = |rid: u64| records.get(&rid).and_then(|r| r.graph.as_ref());
    let selection = select_graphics_with_resolver_at_detail(g, &resolver, level);
    let mut pieces: Vec<Piece> = Vec::new();
    let mut prim_lines: Vec<String> = Vec::new();
    let mut face_lines: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let mut unusual = false;
    let mut surfaces: Vec<String> = Vec::new();
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
        let m = &s.world_transform;
        let det = determinant(m);
        let owner = s
            .source_owner_id
            .map_or_else(|| "-".to_string(), |o| o.to_string());
        let sign = if det < 0.0 { '-' } else { '+' };
        let place = |p: Vec3| apply(m, p);
        let flip = |t: [u32; 3]| if det < 0.0 { [t[0], t[2], t[1]] } else { t };
        if obj.class_name == "Geometry" {
            let pieces_before = pieces.len();
            let faces = obj.fields["m_pFaces"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            for ptr in &faces {
                let Ok(Some(fi)) = pointer(sg, oi, ptr) else {
                    continue;
                };
                let surf_index = sg
                    .objects
                    .get(fi)
                    .and_then(|f| pointer(sg, fi, &f.fields["m_pSurf"]).ok().flatten());
                let surf = surf_index.map_or_else(
                    || "none".to_string(),
                    |si| sg.objects[si].class_name.clone(),
                );
                match native_saved_mesh::face(sg, fi) {
                    Ok(Some(mesh)) => {
                        let tris: Vec<Tri> = mesh
                            .triangles
                            .iter()
                            .filter_map(|t| {
                                let t = flip(*t);
                                let a = mesh.vertices.get(t[0] as usize)?;
                                let b = mesh.vertices.get(t[1] as usize)?;
                                let c = mesh.vertices.get(t[2] as usize)?;
                                Some([place(*a), place(*b), place(*c)])
                            })
                            .collect();
                        *census.faces.entry(format!("{surf} ok")).or_default() += 1;
                        face_lines.push(format!(
                            "oi={oi} fi={fi} surf={surf} ok tris={} area={:.6}",
                            tris.len(),
                            area(&tris)
                        ));
                        if !READ_SURFACES.contains(&surf.as_str()) {
                            unusual = true;
                        }
                        if !READ_SURFACES.contains(&surf.as_str()) || named {
                            if let Some(si) = surf_index {
                                surfaces.push(clip(describe(sg, si, 2), 2400));
                            }
                        }
                        pieces.push(Piece {
                            label: format!("{oi}.{fi}.{surf}"),
                            tris,
                        });
                    }
                    Ok(None) => {
                        *census.faces.entry(format!("{surf} unbounded")).or_default() += 1;
                        face_lines.push(format!("oi={oi} fi={fi} surf={surf} err=unbounded"));
                    }
                    Err(e) => {
                        let kind = message_kind(&format!("{e:#}"));
                        *census
                            .faces
                            .entry(format!("{surf} err {}", clip(kind.clone(), 90)))
                            .or_default() += 1;
                        face_lines.push(format!(
                            "oi={oi} fi={fi} surf={surf} err={}",
                            clip(format!("{e:#}"), 160)
                        ));
                        failed.push(surf.clone());
                        if let Some(si) = surf_index {
                            surfaces.push(clip(describe(sg, si, 2), 2400));
                        }
                        if let Some(face) = sg.objects.get(fi) {
                            if let Ok(Some(li)) = pointer(sg, fi, &face.fields["m_pFirstLoop"]) {
                                let mut text = format!("face {}", clip(describe(sg, fi, 0), 700));
                                text.push_str(&format!(" loop {}", clip(describe(sg, li, 0), 500)));
                                if let Ok(Some(e)) =
                                    pointer(sg, li, &sg.objects[li].fields["m_next"])
                                {
                                    if e != li {
                                        text.push_str(&format!(
                                            " | edge {}",
                                            clip(describe(sg, e, 0), 1200)
                                        ));
                                    }
                                }
                                surfaces.push(format!("EDGE {text}"));
                            }
                        }
                    }
                }
            }
            let n_tris: usize = pieces[pieces_before..].iter().map(|p| p.tris.len()).sum();
            prim_lines.push(format!(
                "oi={oi} class=Geometry owner={owner} det={sign} faces={} tris={n_tris}",
                faces.len()
            ));
        } else if obj.class_name == "GPolyMesh" {
            let topology = pointer(sg, oi, &obj.fields["m_pFacetedTopology"])
                .ok()
                .flatten()
                .map(|ti| &sg.objects[ti].fields);
            let Some(f) = topology else {
                prim_lines.push(format!(
                    "oi={oi} class=GPolyMesh owner={owner} det={sign} no-topology"
                ));
                continue;
            };
            let points: Vec<Vec3> = f["m_pointsArr"]
                .as_array()
                .map(|a| a.iter().filter_map(p3).collect())
                .unwrap_or_default();
            let normals: Vec<Vec3> = f["m_normalsArr"]
                .as_array()
                .map(|a| a.iter().filter_map(p3).collect())
                .unwrap_or_default();
            let facets: Vec<[u32; 3]> = f["m_facetsArr"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            let t = v.as_array()?;
                            if t.len() != 3 {
                                return None;
                            }
                            Some([
                                u32::try_from(t[0].as_u64()?).ok()?,
                                u32::try_from(t[1].as_u64()?).ok()?,
                                u32::try_from(t[2].as_u64()?).ok()?,
                            ])
                        })
                        .collect()
                })
                .unwrap_or_default();
            let (consistent, samedir, boundary, other) = edge_stats(&facets);
            let mut opposed = 0usize;
            let mut tris: Vec<Tri> = Vec::new();
            for (i, t) in facets.iter().enumerate() {
                let (Some(a), Some(b), Some(c)) = (
                    points.get(t[0] as usize),
                    points.get(t[1] as usize),
                    points.get(t[2] as usize),
                ) else {
                    continue;
                };
                if normals.len() == facets.len() {
                    let n = cross(sub(*b, *a), sub(*c, *a));
                    if dot(n, normals[i]) < 0.0 {
                        opposed += 1;
                    }
                }
                let t = flip(*t);
                let a = points[t[0] as usize];
                let b = points[t[1] as usize];
                let c = points[t[2] as usize];
                tris.push([place(a), place(b), place(c)]);
            }
            *census.faces.entry("GPolyMesh ok".into()).or_default() += 1;
            prim_lines.push(format!(
                "oi={oi} class=GPolyMesh owner={owner} det={sign} tris={} facets={} opposed={opposed} consistent={consistent} samedir={samedir} boundary={boundary} other={other} stored-normals={}",
                tris.len(),
                facets.len(),
                normals.len()
            ));
            pieces.push(Piece {
                label: format!("{oi}.poly"),
                tris,
            });
        }
    }
    let total: usize = pieces.iter().map(|p| p.tris.len()).sum();
    let mut count = 0usize;
    let mut centre = [0.0; 3];
    for p in &pieces {
        for t in &p.tris {
            for v in t {
                count += 1;
                for (c, x) in centre.iter_mut().zip(v) {
                    *c += x;
                }
            }
        }
    }
    if count > 0 {
        for c in &mut centre {
            *c /= count as f64;
        }
    }
    let all_tris: Vec<Tri> = pieces.iter().flat_map(|p| p.tris.iter().copied()).collect();
    let (consistent, samedir, boundary, other) = edge_stats(&welded(&all_tris));
    let kinds: BTreeSet<String> = selection
        .diagnostics
        .iter()
        .map(|d| message_kind(&d.message))
        .collect();
    let messages = if kinds.is_empty() {
        "-".to_string()
    } else {
        kinds.into_iter().collect::<Vec<_>>().join(" | ")
    };
    if pieces.is_empty() && selection.selected.is_empty() {
        return;
    }
    println!(
        "elem {id} L{level} prims={} tris={total} svol={:.6} diagnostics={} edges(consistent={consistent} samedir={samedir} boundary={boundary} other={other}) {messages}",
        selection.selected.len(),
        signed_volume(&all_tris, centre),
        selection.diagnostics.len(),
    );
    for line in &prim_lines {
        println!("prim {id} L{level} {line}");
    }
    for line in &face_lines {
        println!("face {id} L{level} {line}");
    }
    for p in &pieces {
        println!(
            "piece {id} L{level} {} tris={} area={:.6} svol={:.6}",
            p.label,
            p.tris.len(),
            area(&p.tris),
            signed_volume(&p.tris, centre)
        );
    }
    let report_surfaces = named || unusual || !failed.is_empty();
    if report_surfaces {
        let mut seen = BTreeSet::new();
        for text in &surfaces {
            if seen.insert(text.clone()) {
                if let Some(kind) = text.strip_prefix("EDGE ") {
                    println!("edge {id} {kind}");
                } else {
                    println!("surface {id} {text}");
                }
            }
        }
    }
    if (named || unusual || !failed.is_empty()) && total <= TRIANGLE_CAP {
        for p in &pieces {
            for t in &p.tris {
                println!(
                    "tri {id} L{level} {} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6}",
                    p.label,
                    t[0][0],
                    t[0][1],
                    t[0][2],
                    t[1][0],
                    t[1][1],
                    t[1][2],
                    t[2][0],
                    t[2][1],
                    t[2][2]
                );
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re311_fitting_faces FILE.rvt");
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
    let roots: Vec<u64> = records.keys().copied().collect();
    let mut attempted: BTreeSet<u64> = roots.iter().copied().collect();
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
    println!("census roots {} records {}", roots.len(), records.len());
    let mut census = Census {
        faces: BTreeMap::new(),
        watched: BTreeMap::new(),
        printed: BTreeMap::new(),
    };
    for (owner, r) in &records {
        let Some(g) = &r.graph else { continue };
        let mut in_edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for edge in &g.edges {
            in_edges
                .entry(edge.target_object_index)
                .or_default()
                .push(edge.source_object_index);
        }
        for (index, o) in g.objects.iter().enumerate() {
            if !WATCHED.contains(&o.class_name.as_str()) {
                continue;
            }
            *census.watched.entry(o.class_name.clone()).or_default() += 1;
            let shown = census.printed.entry(o.class_name.clone()).or_default();
            if *shown >= 6 {
                continue;
            }
            *shown += 1;
            let parents: Vec<&str> = in_edges
                .get(&index)
                .map(|v| {
                    v.iter()
                        .map(|&s| g.objects[s].class_name.as_str())
                        .collect()
                })
                .unwrap_or_default();
            println!(
                "object {owner} {} parents={parents:?} {}",
                o.class_name,
                clip(describe(g, index, 1), 2400)
            );
        }
    }
    for &id in &roots {
        let Some(g) = records[&id].graph.as_ref() else {
            continue;
        };
        let named = NAMED.contains(&id);
        let levels: &[i64] = if named { &[1, 2, 3] } else { &[3] };
        for &level in levels {
            element(id, g, &records, level, named, &mut census);
        }
    }
    for (what, n) in &census.faces {
        println!("census face {what} {n}");
    }
    for (what, n) in &census.watched {
        println!("census object {what} {n}");
    }
    Ok(())
}
