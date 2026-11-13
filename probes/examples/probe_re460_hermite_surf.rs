//! RE-460 (#1191): what a `HermiteSurf` record holds in the saved graphics of the
//! elements that keep their record box, and whether it has the layout a second reader
//! of the format gives it.
//!
//! FACT under test (mahach666's RvtIO, Discussion #1241, 2026-11-13): `HermiteSurf`
//! derives from `Surface` and holds `m_periodic`, `m_constructedOK`, a node array of
//! `SplineSrfNode` (`m_iPoint`, `m_iTangent[2]`, `m_iMixedDer`: point, both tangents
//! and the mixed derivative, 12 f64 a node) and the parameter arrays `m_uParams` and
//! `m_vParams`, with `n = nu * nv` nodes and node `(iu, iv)` at `iv * nu + iu`. It is
//! the one surface class no reader of ours decodes (RE-329: `unsupported surface
//! HermiteSurf`, 512 faces on the BIM4LCA office as built).
//!
//! The candidate set is `probe_re329_saved_graphics_census`'s: a building element
//! exported with the record box as its body and not profile-resolved, whose Tag no
//! other entity carries, and the symbols its instances name. This prints, over every
//! `HermiteSurf` object of their graphs, counts only (no id, name or coordinate):
//!
//!   release <year>
//!   census hermite_objects <n> in <m> records
//!   census hermite_first <the shape of the first object: field names, and for an array
//!     its first item's shape and its length>
//!   census hermite_shape <n> <the shape with every array length replaced by n>
//!   census hermite_layout <n> <product_matches=<bool> u_increasing=<bool>
//!     v_increasing=<bool> periodic=<the flags>>
//!   census hermite_sizes nu <min>..<max> nv <min>..<max> nodes <min>..<max>
//!
//! Stage 2 (row 2 of #1191): the surface at an edge's stored (u, v) meets the face on the
//! other side of the edge. For each `Edge` with a `HermiteSurf` on one side and a `Plane` or
//! another `HermiteSurf` on the other, every sample of the edge (its two ends in
//! `m_firstAndLastEdgePnts`, the rest in `m_interiorEdgePnts`) holds the (u, v) it has on each
//! side (`uv[side]`). The point of the Hermite surface at its (u, v) is set beside the plane's
//! point at the plane's (u, v), `origin + u xVec + v yVec`, or beside the other Hermite
//! surface's at its own, and the distance between them is counted in feet. The surface is the
//! bicubic Hermite patch of a cell of the grid: its four corner nodes' point, tangents and
//! mixed derivative, with the layout of the discussion (the node of `(iu, iv)` at
//! `iv * nu + iu`, `m_iTangent[0]` along u and `[1]` along v) and the three other ways to read it
//! printed beside it, so a layout the data does not support shows as one:
//!
//!   edges borders <n> edges border a HermiteSurf: plane <n> hermite <n> other <n> unread <n>
//!   edges edge_first <field names of the first such Edge>
//!   edges <vs plane|vs hermite> <variant> ends <n> <distance buckets> interior <n> <buckets> unevaluated <n>
//!
//! where a variant reads `u-fastest|v-fastest tangent0=u|tangent0=v` and the buckets are the
//! counts of distances at most 1e-12, 1e-9, 1e-6, 1e-5, 1e-4, 1e-3 and over.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re460_hermite_surf -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_saved_mesh::pointer;

const RECORD_BOX: &str = "partition_element_record_bbox";
const NODE_FIELDS: [&str; 3] = ["m_NodeArray", "m_nodeArray", "m_nodes"];

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

/// The shape of a value: field names, and an array's first item and its length (or `n`).
fn shape(value: &Fv, depth: usize, lengths: bool) -> String {
    if let Some(items) = value.as_array() {
        let first = items
            .first()
            .map_or("empty".to_string(), |item| shape(item, depth + 1, lengths));
        let length = if lengths {
            items.len().to_string()
        } else {
            "n".to_string()
        };
        return format!("[{first}; {length}]");
    }
    if let Some(object) = value.as_object() {
        if depth > 5 {
            return "{..}".to_string();
        }
        let fields: Vec<String> = object
            .iter()
            .map(|(name, inner)| format!("{name}: {}", shape(inner, depth + 1, lengths)))
            .collect();
        return format!("{{{}}}", fields.join(", "));
    }
    if value.is_boolean() {
        "bool".to_string()
    } else if value.is_f64() {
        "f64".to_string()
    } else if value.is_i64() || value.is_u64() {
        "int".to_string()
    } else if value.is_string() {
        "str".to_string()
    } else if value.is_null() {
        "null".to_string()
    } else {
        "other".to_string()
    }
}

fn increasing(value: Option<&Fv>) -> Option<bool> {
    let items = value?.as_array()?;
    let numbers: Option<Vec<f64>> = items.iter().map(Fv::as_f64).collect();
    let numbers = numbers?;
    Some(numbers.windows(2).all(|pair| pair[0] < pair[1]))
}

/// The buckets of a distance in feet: at most these, then over the last.
const BUCKETS: [f64; 6] = [1e-12, 1e-9, 1e-6, 1e-5, 1e-4, 1e-3];

fn bucket(distance: f64) -> usize {
    BUCKETS
        .iter()
        .position(|limit| distance <= *limit)
        .unwrap_or(BUCKETS.len())
}

fn array<const N: usize>(value: &Fv) -> Option<[f64; N]> {
    let items = value.as_array()?;
    if items.len() != N {
        return None;
    }
    let mut out = [0.0; N];
    for (slot, item) in out.iter_mut().zip(items) {
        let number = item.as_f64()?;
        if !number.is_finite() {
            return None;
        }
        *slot = number;
    }
    Some(out)
}

#[derive(Clone, Copy)]
struct Node {
    point: [f64; 3],
    tangent: [[f64; 3]; 2],
    mixed: [f64; 3],
}

/// A `HermiteSurf`: the parameters of its grid and a node at each crossing.
struct Hermite {
    u: Vec<f64>,
    v: Vec<f64>,
    nodes: Vec<Node>,
}

impl Hermite {
    fn read(fields: &Fv) -> Option<Self> {
        let params = |name: &str| -> Option<Vec<f64>> {
            let values: Option<Vec<f64>> = fields
                .get(name)?
                .as_array()?
                .iter()
                .map(Fv::as_f64)
                .collect();
            let values = values?;
            (values.len() >= 2 && values.windows(2).all(|w| w[0] < w[1])).then_some(values)
        };
        let (u, v) = (params("m_uParams")?, params("m_vParams")?);
        let list = NODE_FIELDS
            .iter()
            .find_map(|name| fields.get(*name))?
            .as_array()?;
        if list.len() != u.len() * v.len() {
            return None;
        }
        let mut nodes = Vec::with_capacity(list.len());
        for node in list {
            let tangents = node.get("m_iTangent")?.as_array()?;
            if tangents.len() != 2 {
                return None;
            }
            nodes.push(Node {
                point: array::<3>(node.get("m_iPoint")?)?,
                tangent: [array::<3>(&tangents[0])?, array::<3>(&tangents[1])?],
                mixed: array::<3>(node.get("m_iMixedDer")?)?,
            });
        }
        Some(Self { u, v, nodes })
    }

    /// The cell of `params` that holds `x`, the last one at the far end.
    fn cell(params: &[f64], x: f64) -> Option<usize> {
        let (first, last) = (params[0], params[params.len() - 1]);
        if !(first - 1e-9..=last + 1e-9).contains(&x) {
            return None;
        }
        Some(
            params
                .partition_point(|p| *p <= x)
                .saturating_sub(1)
                .min(params.len() - 2),
        )
    }

    /// The bicubic Hermite patch of the cell holding (u, v); `v_fastest` reads the node of
    /// `(iu, iv)` at `iu * nv + iv` and `swap` reads `m_iTangent[1]` as the u tangent.
    fn eval(&self, v_fastest: bool, swap: bool, uv: [f64; 2]) -> Option<[f64; 3]> {
        let (nu, nv) = (self.u.len(), self.v.len());
        let (i, j) = (Self::cell(&self.u, uv[0])?, Self::cell(&self.v, uv[1])?);
        let (du, dv) = (self.u[i + 1] - self.u[i], self.v[j + 1] - self.v[j]);
        let (t, s) = ((uv[0] - self.u[i]) / du, (uv[1] - self.v[j]) / dv);
        // The value and tangent basis of a cell's end 0 or 1.
        let value = |end: usize, x: f64| {
            if end == 0 {
                2.0 * x.powi(3) - 3.0 * x * x + 1.0
            } else {
                -2.0 * x.powi(3) + 3.0 * x * x
            }
        };
        let slope = |end: usize, x: f64| {
            if end == 0 {
                x.powi(3) - 2.0 * x * x + x
            } else {
                x.powi(3) - x * x
            }
        };
        let mut out = [0.0f64; 3];
        for a in 0..2 {
            for b in 0..2 {
                let (iu, iv) = (i + a, j + b);
                let node = self.nodes[if v_fastest {
                    iu * nv + iv
                } else {
                    iv * nu + iu
                }];
                let (tu, tv) = if swap {
                    (node.tangent[1], node.tangent[0])
                } else {
                    (node.tangent[0], node.tangent[1])
                };
                for (k, slot) in out.iter_mut().enumerate() {
                    *slot += node.point[k] * value(a, t) * value(b, s)
                        + tu[k] * du * slope(a, t) * value(b, s)
                        + tv[k] * dv * value(a, t) * slope(b, s)
                        + node.mixed[k] * du * dv * slope(a, t) * slope(b, s);
                }
            }
        }
        Some(out)
    }
}

/// The four readings of a Hermite grid: (v_fastest, swap, label).
const VARIANTS: [(bool, bool, &str); 4] = [
    (false, false, "u-fastest tangent0=u"),
    (false, true, "u-fastest tangent0=v"),
    (true, false, "v-fastest tangent0=u"),
    (true, true, "v-fastest tangent0=v"),
];

/// What the distances of one reading of one kind of edge come to.
#[derive(Default, Clone)]
struct Gaps {
    ends: [usize; 7],
    interior: [usize; 7],
    unevaluated: usize,
}

impl Gaps {
    fn show(&self, name: &str, label: &str) -> String {
        let row = |counts: &[usize; 7]| {
            format!(
                "{} [{}]",
                counts.iter().sum::<usize>(),
                counts
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };
        format!(
            "edges {name} {label} ends {} interior {} unevaluated {}",
            row(&self.ends),
            row(&self.interior),
            self.unevaluated
        )
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
    let mut by_id: BTreeMap<u64, usize> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
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
            *by_id.entry(id).or_default() += 1;
        }
    }
    let mut options = Options {
        selected_ids: by_id
            .iter()
            .filter(|(_, count)| **count == 1)
            .map(|(id, _)| *id)
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

    let mut total = 0usize;
    let mut holders = BTreeSet::new();
    let mut first: Option<String> = None;
    let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
    let mut layouts: BTreeMap<String, usize> = BTreeMap::new();
    let (mut nu_range, mut nv_range, mut node_range) =
        ((usize::MAX, 0), (usize::MAX, 0), (usize::MAX, 0));
    for (id, record) in &records {
        let Some(graph) = &record.graph else { continue };
        for object in &graph.objects {
            if object.class_name != "HermiteSurf" {
                continue;
            }
            total += 1;
            holders.insert(*id);
            let fields = &object.fields;
            first.get_or_insert_with(|| shape(fields, 0, true));
            *shapes.entry(shape(fields, 0, false)).or_default() += 1;
            let nodes = NODE_FIELDS
                .iter()
                .find_map(|name| fields.get(*name))
                .and_then(Fv::as_array)
                .map(<[Fv]>::len);
            let u_params = fields
                .get("m_uParams")
                .and_then(Fv::as_array)
                .map(<[Fv]>::len);
            let v_params = fields
                .get("m_vParams")
                .and_then(Fv::as_array)
                .map(<[Fv]>::len);
            let matches = match (nodes, u_params, v_params) {
                (Some(n), Some(nu), Some(nv)) => (n == nu * nv).to_string(),
                _ => "unread".to_string(),
            };
            if let (Some(n), Some(nu), Some(nv)) = (nodes, u_params, v_params) {
                nu_range = (nu_range.0.min(nu), nu_range.1.max(nu));
                nv_range = (nv_range.0.min(nv), nv_range.1.max(nv));
                node_range = (node_range.0.min(n), node_range.1.max(n));
            }
            let periodic = fields.get("m_periodic").and_then(Fv::as_array).map_or(
                "unread".to_string(),
                |flags| {
                    flags
                        .iter()
                        .map(|flag| flag.as_bool().map_or("?".to_string(), |b| b.to_string()))
                        .collect::<Vec<_>>()
                        .join(",")
                },
            );
            let label = format!(
                "product_matches={matches} u_increasing={:?} v_increasing={:?} periodic={periodic}",
                increasing(fields.get("m_uParams")),
                increasing(fields.get("m_vParams")),
            );
            *layouts.entry(label).or_default() += 1;
        }
    }
    println!(
        "census hermite_objects {total} in {} records",
        holders.len()
    );
    if let Some(first) = first {
        println!("census hermite_first {first}");
    }
    for (label, n) in &shapes {
        println!("census hermite_shape {n} {label}");
    }
    for (label, n) in &layouts {
        println!("census hermite_layout {n} {label}");
    }
    if total > 0 && nu_range.0 != usize::MAX {
        println!(
            "census hermite_sizes nu {}..{} nv {}..{} nodes {}..{}",
            nu_range.0, nu_range.1, nv_range.0, nv_range.1, node_range.0, node_range.1
        );
    }
    edge_stage(&records);
    Ok(())
}

/// An edge's samples in order: its first end, those between and its last end.
fn edge_samples(edge: &Fv) -> Option<Vec<&Fv>> {
    let ends = edge
        .get("m_firstAndLastEdgePnts")?
        .as_array()
        .filter(|ends| ends.len() == 2)?;
    let mut samples = vec![&ends[0]];
    if let Some(inner) = edge.get("m_interiorEdgePnts").and_then(Fv::as_array) {
        samples.extend(inner);
    }
    samples.push(&ends[1]);
    Some(samples)
}

fn distance(p: [f64; 3], q: [f64; 3]) -> f64 {
    (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f64>().sqrt()
}

/// Adds one sample's distance to a reading's counts: an end of the edge to `ends`, any
/// other to `interior`, and a sample that could not be evaluated to `unevaluated`.
fn record_gap(slot: &mut Gaps, end: bool, gap: Option<f64>) {
    match gap {
        Some(gap) => {
            let counts = if end {
                &mut slot.ends
            } else {
                &mut slot.interior
            };
            counts[bucket(gap)] += 1;
        }
        None => slot.unevaluated += 1,
    }
}

/// Stage 2: the distance between a Hermite surface and the face across each of its edges.
fn edge_stage(records: &BTreeMap<u64, Record>) {
    let mut borders = [0usize; 4]; // plane, hermite, other, unread
    let mut others: BTreeMap<String, usize> = BTreeMap::new();
    let mut edge_first: Option<String> = None;
    let mut vs_plane = vec![Gaps::default(); VARIANTS.len()];
    let mut vs_hermite = vec![Gaps::default(); VARIANTS.len()];
    for record in records.values() {
        let Some(graph) = &record.graph else { continue };
        let surface_of = |edge_index: usize, face: &Fv| -> Option<usize> {
            let face_index = pointer(graph, edge_index, face).ok().flatten()?;
            let surface = graph.objects.get(face_index)?.fields.get("m_pSurf")?;
            pointer(graph, face_index, surface).ok().flatten()
        };
        for (edge_index, edge) in graph.objects.iter().enumerate() {
            if edge.class_name != "Edge" {
                continue;
            }
            let Some(pair) = edge.fields.get("m_pFace").and_then(Fv::as_array) else {
                continue;
            };
            if pair.len() != 2 {
                continue;
            }
            let (Some(first), Some(second)) = (
                surface_of(edge_index, &pair[0]),
                surface_of(edge_index, &pair[1]),
            ) else {
                continue;
            };
            let surfaces = [first, second];
            let class = |side: usize| graph.objects[surfaces[side]].class_name.as_str();
            let hermite_sides: Vec<usize> = (0..2)
                .filter(|side| class(*side) == "HermiteSurf")
                .collect();
            if hermite_sides.is_empty() {
                continue;
            }
            edge_first.get_or_insert_with(|| shape(&edge.fields, 0, false));
            let Some(samples) = edge_samples(&edge.fields) else {
                borders[3] += 1;
                continue;
            };
            let uv_of = |sample: &Fv, side: usize| array::<2>(&sample["uv"][side]);
            let hermite = |side: usize| Hermite::read(&graph.objects[surfaces[side]].fields);
            if hermite_sides.len() == 2 {
                borders[1] += 1;
                let (Some(a), Some(b)) = (hermite(0), hermite(1)) else {
                    borders[3] += 1;
                    continue;
                };
                for (slot, (v_fastest, swap, _)) in vs_hermite.iter_mut().zip(VARIANTS) {
                    for (n, sample) in samples.iter().copied().enumerate() {
                        let gap = uv_of(sample, 0).zip(uv_of(sample, 1)).and_then(|(ua, ub)| {
                            let p = a.eval(v_fastest, swap, ua)?;
                            let q = b.eval(v_fastest, swap, ub)?;
                            Some(distance(p, q))
                        });
                        record_gap(slot, n == 0 || n + 1 == samples.len(), gap);
                    }
                }
                continue;
            }
            let hermite_side = hermite_sides[0];
            let other_side = 1 - hermite_side;
            if class(other_side) != "Plane" {
                borders[2] += 1;
                *others.entry(class(other_side).to_string()).or_default() += 1;
                continue;
            }
            let plane = &graph.objects[surfaces[other_side]].fields;
            let (Some(origin), Some(x), Some(y), Some(surface)) = (
                plane.get("m_origin").and_then(array::<3>),
                plane.get("m_xVec").and_then(array::<3>),
                plane.get("m_yVec").and_then(array::<3>),
                hermite(hermite_side),
            ) else {
                borders[3] += 1;
                continue;
            };
            borders[0] += 1;
            for (slot, (v_fastest, swap, _)) in vs_plane.iter_mut().zip(VARIANTS) {
                for (n, sample) in samples.iter().copied().enumerate() {
                    let gap = uv_of(sample, hermite_side)
                        .zip(uv_of(sample, other_side))
                        .and_then(|(on_surface, [pu, pv])| {
                            let q = surface.eval(v_fastest, swap, on_surface)?;
                            let p: [f64; 3] =
                                std::array::from_fn(|k| origin[k] + pu * x[k] + pv * y[k]);
                            Some(distance(p, q))
                        });
                    record_gap(slot, n == 0 || n + 1 == samples.len(), gap);
                }
            }
        }
    }
    println!(
        "edges borders {} edges border a HermiteSurf: plane {} hermite {} other {} unread {}",
        borders.iter().sum::<usize>(),
        borders[0],
        borders[1],
        borders[2],
        borders[3]
    );
    for (class, n) in &others {
        println!("edges other_surface {class} {n}");
    }
    if let Some(fields) = edge_first {
        println!("edges edge_first {fields}");
    }
    for ((_, _, label), gaps) in VARIANTS.iter().zip(&vs_plane) {
        println!("{}", gaps.show("vs plane", label));
    }
    for ((_, _, label), gaps) in VARIANTS.iter().zip(&vs_hermite) {
        println!("{}", gaps.show("vs hermite", label));
    }
}
