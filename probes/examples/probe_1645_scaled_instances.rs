//! #1645: the `GInstance`s whose `m_bHasScale` is set, and the matrix each one carries.
//!
//! FACT under test: the saved-graphics walk refuses a `GInstance` with `m_bHasScale`
//! set (`scaled instance mode is not qualified`), which keeps, among others, the
//! BIM4LCA housing's `Suihku 1` showers and 10 of the office's sanitary terminals as
//! record boxes once the instances forbidden the targets 10 are drawn (RE-408). What
//! the flag means is not read. This lists every such instance of every exported
//! element's graphics and of the graphics of the symbols they name (at most four
//! rounds, as `probe_1645_cylinder_trim_loops` follows them), with the singular
//! shape of the 3x3 matrix of its `InstanceInfo.m_Trf`: whether it is a rotation, a
//! mirror, a uniform scale, a scale that differs by axis, or has a shear, and the
//! column lengths, so that a reading of the flag can be tested on the meshes whose
//! export Revit holds.
//!
//! Output, tab separated:
//!   scaled <class> kind=<identity|rotation|mirror|uniform|axes|shear> det=<sign> cols=<x,y,z lengths>
//!     max_dev=<largest |column i . column j - identity| of the group> instances=<n> records=<n> first=<ids>
//!   total <class> instances=<n> scaled=<n> with_category=<n> scaled_with_category=<n>
//!   sample <class> flagged=<bool> id=<record> instance=<the GInstance's fields> info=<its InstanceInfo's>
//! where the sample lines show the fields of the first two flagged instances of each class and of the
//! first unflagged one, so that what the flag adds to an instance can be read from the bytes.
//! where <class> is the IFC classes of the exported elements that reach the record
//! (joined with `+`).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1645_scaled_instances -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::pointer;

/// The columns of the 3x3 of an `InstanceInfo.m_Trf`, or none where it is unreadable.
fn columns(trf: &serde_json::Value) -> Option<[[f64; 3]; 3]> {
    let rows = trf["m_3x3"].as_array().filter(|rows| rows.len() == 3)?;
    let mut m = [[0.0; 3]; 3];
    for (i, row) in rows.iter().enumerate() {
        let row = row.as_array().filter(|row| row.len() == 3)?;
        for (j, value) in row.iter().enumerate() {
            m[i][j] = value.as_f64().filter(|v| v.is_finite())?;
        }
    }
    // Column j is the image of the local axis j.
    Some(std::array::from_fn(|j| std::array::from_fn(|i| m[i][j])))
}

fn norm(v: [f64; 3]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn describe(c: [[f64; 3]; 3]) -> (String, i8, String, f64) {
    let lengths = [norm(c[0]), norm(c[1]), norm(c[2])];
    let cross = [
        c[0][1] * c[1][2] - c[0][2] * c[1][1],
        c[0][2] * c[1][0] - c[0][0] * c[1][2],
        c[0][0] * c[1][1] - c[0][1] * c[1][0],
    ];
    let det = dot(cross, c[2]);
    let sign = if det >= 0. { 1 } else { -1 };
    let orthogonal = dot(c[0], c[1]).abs() <= 1e-6 * lengths[0] * lengths[1]
        && dot(c[0], c[2]).abs() <= 1e-6 * lengths[0] * lengths[2]
        && dot(c[1], c[2]).abs() <= 1e-6 * lengths[1] * lengths[2];
    let unit = lengths.iter().all(|l| (l - 1.).abs() <= 1e-6);
    let uniform = (lengths[0] - lengths[1]).abs() <= 1e-6 * lengths[0]
        && (lengths[0] - lengths[2]).abs() <= 1e-6 * lengths[0];
    let identity =
        (0..3).all(|a| (0..3).all(|b| (c[a][b] - f64::from(u8::from(a == b))).abs() <= 1e-9));
    let kind = if !orthogonal {
        "shear"
    } else if identity {
        "identity"
    } else if unit && sign > 0 {
        "rotation"
    } else if unit {
        "mirror"
    } else if uniform {
        "uniform"
    } else {
        "axes"
    };
    let dev = (0..3)
        .flat_map(|a| (0..3).map(move |b| (a, b)))
        .map(|(a, b)| (dot(c[a], c[b]) - f64::from(u8::from(a == b))).abs())
        .fold(0.0, f64::max);
    (
        kind.to_string(),
        sign,
        format!("{:.4},{:.4},{:.4}", lengths[0], lengths[1], lengths[2]),
        dev,
    )
}

/// The first 1,400 characters of a JSON text, so that a sample line stays a line.
fn clip(text: String) -> String {
    text.chars().take(1400).collect()
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
    // The exported classes that reach each record: an element's own, then its symbols'.
    let mut reached: BTreeMap<u64, BTreeSet<String>> = classes
        .iter()
        .map(|(id, class)| (*id, BTreeSet::from([class.clone()])))
        .collect();
    let mut visited: BTreeSet<u64> = BTreeSet::new();
    let mut frontier: BTreeSet<u64> = classes.keys().copied().collect();
    type Key = (String, String, i8, String);
    let mut census: BTreeMap<Key, (usize, BTreeSet<u64>, f64)> = BTreeMap::new();
    let mut shown: BTreeMap<(String, bool), usize> = BTreeMap::new();
    let mut totals: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    for _round in 0..4 {
        let options = Options {
            selected_ids: frontier.clone(),
            channels: BTreeSet::from([103]),
            max_graph_values: 1_000_000,
            max_graph_objects: 1_000_000,
            ..Options::default()
        };
        visited.extend(frontier.iter().copied());
        let mut named: BTreeSet<u64> = BTreeSet::new();
        let mut update: Vec<(u64, BTreeSet<String>)> = Vec::new();
        extract_graphs(&mut rf, &options, |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            let class = reached
                .get(&id)
                .map(|set| set.iter().cloned().collect::<Vec<_>>().join("+"))
                .unwrap_or_else(|| "unmapped".to_string());
            for object in &graph.objects {
                if object.class_name == "InstanceInfo" {
                    if let Some(symbol) = object.fields["m_symbolId"]["m_id"]["m_id64"].as_u64() {
                        if symbol != id {
                            named.insert(symbol);
                            if let Some(set) = reached.get(&id) {
                                update.push((symbol, set.clone()));
                            }
                        }
                    }
                }
            }
            for (index, object) in graph.objects.iter().enumerate() {
                if object.class_name != "GInstance" {
                    continue;
                }
                let total = totals.entry(class.clone()).or_default();
                total[0] += 1;
                let flagged = object.fields["m_bHasScale"].as_bool() == Some(true);
                let with_category = object.fields["m_GInfo"]["m_categoryId"]["m_id"]["m_id64"]
                    .as_i64()
                    .is_some_and(|id| id != -1);
                total[2] += usize::from(with_category);
                let info = pointer(graph, index, &object.fields["m_instanceInfo"])
                    .ok()
                    .flatten();
                let limit = if flagged { 2 } else { 1 };
                let seen = shown.entry((class.clone(), flagged)).or_default();
                if *seen < limit {
                    *seen += 1;
                    let info_json = info
                        .map(|i| graph.objects[i].fields.to_string())
                        .unwrap_or_default();
                    println!(
                        "sample\t{class}\tflagged={flagged}\tid={id}\tinstance={}\tinfo={}",
                        clip(object.fields.to_string()),
                        clip(info_json)
                    );
                }
                if !flagged {
                    continue;
                }
                total[1] += 1;
                total[3] += usize::from(with_category);
                let trf = info.and_then(|i| columns(&graph.objects[i].fields["m_Trf"]));
                let (kind, sign, lengths, dev) = match trf {
                    Some(c) => describe(c),
                    None => ("unreadable".to_string(), 0, String::new(), 0.0),
                };
                let entry = census
                    .entry((class.clone(), kind, sign, lengths))
                    .or_default();
                entry.0 += 1;
                entry.1.insert(id);
                entry.2 = entry.2.max(dev);
            }
            Ok(())
        })?;
        for (symbol, set) in update {
            reached.entry(symbol).or_default().extend(set);
        }
        frontier = named.difference(&visited).copied().collect();
        if frontier.is_empty() {
            break;
        }
    }
    for ((class, kind, sign, lengths), (instances, records, dev)) in &census {
        let first: Vec<String> = records.iter().take(3).map(u64::to_string).collect();
        println!(
            "scaled\t{class}\tkind={kind}\tdet={sign}\tcols={lengths}\tmax_dev={dev:.2e}\tinstances={instances}\trecords={}\tfirst={}",
            records.len(),
            first.join(",")
        );
    }
    for (class, [instances, scaled, with_category, scaled_with_category]) in &totals {
        println!(
            "total\t{class}\tinstances={instances}\tscaled={scaled}\twith_category={with_category}\tscaled_with_category={scaled_with_category}"
        );
    }
    Ok(())
}
