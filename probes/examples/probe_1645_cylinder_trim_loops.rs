//! #1645: the trim loops of the cylinder faces that `cylinder_face` refuses for
//! holding more than one, by IFC class.
//!
//! FACT under test: `native_saved_mesh::cylinder_face` meshes a cylinder face
//! with one trim loop and refuses one with several (`multiple cylinder trim
//! loops unsupported`), which is the reason for 700 or so record boxes in the
//! 2024 samples (air terminals, pipe and duct fittings, sanitary terminals,
//! doors). This lists, for each IFC class, the arrangement of the loops of every
//! such face: how many loops, how many are outer rings and how many holes (the
//! depth of each loop's first vertex in the others, as the plane path takes it),
//! the sign of each loop's area, whether a loop touches the surface's U range
//! (a seam crossing) and whether the loops share a vertex, so what the mesher
//! has to read is known before it is written.
//!
//! Output, tab separated:
//!   arrangement <class> <key> faces=<n> elements=<n> first=<ids>
//!   detail <class> <element> face=<index> <loops: vertices/area/uv box> u=<range> v=<range>
//! where <key> is `loops=<n> outers=<n> holes=<n> signs=<+-...> seam=<b> shared=<b>`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1645_cylinder_trim_loops -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::{face_trim_loops, pointer};

fn signed_area(r: &[[f64; 2]]) -> f64 {
    let mut sum = 0.;
    for i in 0..r.len() {
        let a = r[i];
        let b = r[(i + 1) % r.len()];
        sum += a[0] * b[1] - b[0] * a[1];
    }
    sum / 2.
}

/// The even-odd test `native_saved_mesh::inside` takes for a loop's first vertex.
fn inside(p: [f64; 2], r: &[[f64; 2]]) -> bool {
    let mut hit = false;
    for i in 0..r.len() {
        let a = r[i];
        let b = r[(i + 1) % r.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            hit = !hit
        }
    }
    hit
}

fn bbox(r: &[[f64; 2]]) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in r {
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
    }
    b
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        else {
            continue;
        };
        if let Ok(id) = tag.parse::<u64>() {
            classes.entry(id).or_insert_with(|| ifc_type.clone());
        }
    }
    let options = Options {
        selected_ids: classes.keys().copied().collect(),
        channels: BTreeSet::from([102, 103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    type Key = (String, String);
    let mut census: BTreeMap<Key, (usize, BTreeSet<u64>)> = BTreeMap::new();
    let mut details: BTreeMap<Key, Vec<String>> = BTreeMap::new();
    let mut faces_seen = 0usize;
    let mut multi = 0usize;
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let class = classes
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "unmapped".to_string());
        for (fi, face) in graph.objects.iter().enumerate() {
            if face.class_name != "Face" {
                continue;
            }
            let Ok(Some(surface)) = pointer(graph, fi, &face.fields["m_pSurf"]) else {
                continue;
            };
            let surface = &graph.objects[surface];
            if surface.class_name != "CylSurf" {
                continue;
            }
            faces_seen += 1;
            let key_of = |text: String| (class.clone(), text);
            let loops = match face_trim_loops(graph, fi) {
                Ok(loops) => loops,
                Err(error) => {
                    // A loop the ring reader refuses is a different question.
                    let key = key_of(format!("loops unreadable: {error:#}"));
                    let entry = census.entry(key).or_default();
                    entry.0 += 1;
                    entry.1.insert(id);
                    continue;
                }
            };
            if loops.len() < 2 {
                continue;
            }
            multi += 1;
            let corners = surface.fields["m_Envelope"]["m_corners"].as_array();
            let (u, v) = match corners {
                Some(c) if c.len() == 2 => (
                    [
                        c[0][0].as_f64().unwrap_or(f64::NAN),
                        c[1][0].as_f64().unwrap_or(f64::NAN),
                    ],
                    [
                        c[0][1].as_f64().unwrap_or(f64::NAN),
                        c[1][1].as_f64().unwrap_or(f64::NAN),
                    ],
                ),
                _ => ([f64::NAN; 2], [f64::NAN; 2]),
            };
            let depths: Vec<usize> = loops
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    loops
                        .iter()
                        .enumerate()
                        .filter(|(j, s)| *j != i && inside(r[0], s))
                        .count()
                })
                .collect();
            let outers = depths.iter().filter(|d| **d % 2 == 0).count();
            let signs: String = loops
                .iter()
                .map(|r| if signed_area(r) >= 0. { '+' } else { '-' })
                .collect();
            let seam = loops.iter().any(|r| {
                let b = bbox(r);
                (b[0] - u[0]).abs() < 1e-7 || (b[2] - u[1]).abs() < 1e-7
            });
            let shared = loops.iter().enumerate().any(|(i, first)| {
                loops[i + 1..].iter().any(|second| {
                    first.iter().any(|a| {
                        second
                            .iter()
                            .any(|b| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9)
                    })
                })
            });
            let key = key_of(format!(
                "loops={} outers={} holes={} signs={} seam={} shared={}",
                loops.len(),
                outers,
                loops.len() - outers,
                signs,
                seam,
                shared
            ));
            let entry = census.entry(key.clone()).or_default();
            entry.0 += 1;
            entry.1.insert(id);
            let list = details.entry(key).or_default();
            if list.len() < 3 {
                let shape: Vec<String> = loops
                    .iter()
                    .zip(&depths)
                    .map(|(r, d)| {
                        let b = bbox(r);
                        format!(
                            "{}v/{:.5}/depth{}/u[{:.4},{:.4}]v[{:.4},{:.4}]",
                            r.len(),
                            signed_area(r),
                            d,
                            b[0],
                            b[2],
                            b[1],
                            b[3]
                        )
                    })
                    .collect();
                list.push(format!(
                    "detail\t{class}\t{id}\tface={fi}\t{}\tu=[{:.4},{:.4}]\tv=[{:.4},{:.4}]\tradius={}",
                    shape.join(" "),
                    u[0],
                    u[1],
                    v[0],
                    v[1],
                    surface.fields["m_radius"]
                ));
            }
        }
        Ok(())
    })?;
    println!("cylinder faces={faces_seen} with_several_loops={multi}");
    for ((class, key), (faces, elements)) in &census {
        let first: Vec<String> = elements.iter().take(3).map(u64::to_string).collect();
        println!(
            "arrangement\t{class}\t{key}\tfaces={faces}\telements={}\tfirst={}",
            elements.len(),
            first.join(",")
        );
    }
    for lines in details.values() {
        for line in lines {
            println!("{line}");
        }
    }
    Ok(())
}
