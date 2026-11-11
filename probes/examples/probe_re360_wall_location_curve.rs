//! #1475 (RE-360): why some host walls of `rac_advanced`'s 2027 save return no
//! native straight-line row, and which curve of their record is the location
//! curve.
//!
//! FACT under test: `native_walls` reads a wall only where its `SWall` record
//! holds exactly one `GLine` or `GArc` and that one is a level `GLine`. On
//! Revit 2027's save of `rac_advanced` eight host walls of 14 doors return no
//! row, where the 2024 save's do (the 2024 save's 8 hosts hold 15 doors; the
//! 15th's host is another wall on 2027, which is #1454's cause). Hypothesis (from #1471): the 2027 record holds a second `GLine`
//! the 2024 record does not, and the curve the record names as the wall's
//! location curve (`VWallDriver.m_pCrv`, as `location_curve` reads it, #1410)
//! is identifiable among them by that pointer alone.
//!
//! Result (RE-360, Measure run 42129732742): the hypothesis holds. The 18
//! two-line records of `rac_advanced`'s 2027 save hold a second `GLine` owned
//! by `WallRunTimeData.m_prevLocLine`; `m_pCrv` names exactly one `GLine` in
//! all 180 records of every save. The same member is in 5 `rac_basic` records
//! and 2 Snowdon Towers Architectural records of the 2024 to 2026 saves, so a
//! rule that reads the 2027 walls reads those too: `native_walls::record_line`
//! reads a record of two curves by `m_pCrv` when the other is only a
//! `m_prevLocLine` line, and the `native` line below is what it reads.
//!
//! Output, one JSON object per line:
//!
//!   kind "file"       release, the walls read, the doors and windows
//!   kind "openings"   the doors and windows whose two flips are read, by
//!                     whether their host wall gave a native line, and how
//!                     many were placed by the RE-347 rule
//!   kind "unread"     each host wall that gave no line: its decoded class and
//!                     the doors and windows set in it
//!   kind "census"     over every `SWall` record: the walls by the curves they
//!                     hold; the owner (class and field, array indices
//!                     dropped) of every `GLine` and `GArc`; what
//!                     `VWallDriver.m_pCrv` resolves to; and the walls the
//!                     current rule reads against the walls `m_pCrv` names one
//!                     level `GLine` for
//!   kind "swall"      one short line for every `SWall` of the save: its curves
//!                     (index, class, owners, level, length), what `m_pCrv`
//!                     names (token, class tag, the objects holding it), and
//!                     whether the current rule and `m_pCrv` each read it, so
//!                     that a wall is followed across the saves by its id
//!   (a plain line) `previous lines: ...`  counts of `SWall` records, those with a
//!                     previous location line, of those how many are offset from
//!                     the location curve, how many walls the rule reads that the
//!                     old one did not, and how many the old one read: numbers
//!                     beside fixed words, the only line a model without a
//!                     licence keeps (tools/ci/scrub-holdout-results.sh)
//!   kind "native"     the walls `native_walls` gives a row (count and ids), on
//!                     the build the probe is compiled with
//!   kind "wallfields" every field of each decoded wall that has an `SWall`
//!                     record, as text (a long value: its start, FNV-1a hash and
//!                     length)
//!   kind "opening"    each door and window: host, flips, placed by the rule,
//!                     and the placement X axis
//!   kind "wall"       one record in full: every object (index, class, token),
//!                     each curve's geometry, owners and the chain from the
//!                     root to it, `m_pCrv`, and the step classes. For each
//!                     unread host wall, walls 234869 and 428745 where present,
//!                     and the first walls only `m_pCrv` would read
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re360_wall_location_curve -- MODEL.rvt [--records]

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_walls::native_walls;
use rvt::research::partition_schema_mvp::{
    CURTAIN_WALL_CLASS, OPENING_HOST_FIELD, OPENING_PLACEMENT_X_AXIS_FIELDS,
    opening_flips_from_fields, wall_native_direction_from_fields,
};
use rvt::walker::{InstanceField, iter_elements};
use serde_json::{Value, json};

const NAMED: [u64; 2] = [234869, 428745];
const ONLY_POINTER_DETAIL: usize = 6;
const ID_LIST: usize = 60;
const FIELD_CLIP: usize = 900;
const VALUE_CLIP: usize = 240;

type Count = BTreeMap<String, usize>;

/// An object that names another: its index, class, token and the path of the
/// field.
type Owner = (usize, String, u32, String);

/// A decoded wall: its id, class and every field the export holds for it.
struct DecodedWall {
    id: u32,
    class: String,
    fields: Vec<(String, InstanceField)>,
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A field's value as text: exact where short, else its start, its FNV-1a hash
/// and its length, so a changed long value still reads as changed.
fn field_text(value: &InstanceField) -> String {
    let text = format!("{value:?}");
    if text.chars().count() <= VALUE_CLIP {
        return text;
    }
    let start: String = text.chars().take(VALUE_CLIP).collect();
    format!("{start}...#{:016x}/{}", fnv(&text), text.len())
}

fn bump(count: &mut Count, key: impl Into<String>) {
    *count.entry(key.into()).or_default() += 1;
}

fn triple(value: Option<&Value>) -> Option<[f64; 3]> {
    match value?.as_array()?.as_slice() {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?]),
        _ => None,
    }
}

/// The plan start and end of a level `GLine`, as `native_walls` reads it.
fn level_ends(fields: &Value) -> Option<([f64; 2], [f64; 2])> {
    let origin = triple(fields.get("m_origin"))?;
    let direction = triple(fields.get("m_dirVec"))?;
    let [a, b] = match fields.get("m_endParams")?.as_array()?.as_slice() {
        [a, b] => [a.as_f64()?, b.as_f64()?],
        _ => return None,
    };
    let at = |t: f64| [origin[0] + direction[0] * t, origin[1] + direction[1] * t];
    let (start, end) = (at(a), at(b));
    let finite = start.iter().chain(&end).all(|v| v.is_finite());
    (finite && direction[2].abs() <= 1e-9 && (end[0] - start[0]).hypot(end[1] - start[1]) > 1e-9)
        .then_some((start, end))
}

/// The path of object keys and array indices to the pointer at `offset`.
fn path_to(value: &Value, offset: usize, path: &mut Vec<String>) -> bool {
    if let Some((_, _, at)) = rvt::research::native_parameters::pointer_parts(value) {
        return Some(at) == u64::try_from(offset).ok();
    }
    match value {
        Value::Object(map) => {
            for (name, member) in map {
                path.push(name.clone());
                if path_to(member, offset, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(format!("[{index}]"));
                if path_to(item, offset, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        _ => false,
    }
}

/// Each way an object of the graph is named: the owner's class, token and the
/// path of the field, indices kept.
fn owners(graph: &ObjectGraph, index: usize) -> Vec<Owner> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.target_object_index == index)
        .map(|edge| {
            let source = &graph.objects[edge.source_object_index];
            let mut path = Vec::new();
            let found = path_to(&source.fields, edge.pointer_offset, &mut path);
            let field = if found {
                path.join(".").replace(".[", "[")
            } else {
                format!("@{}", edge.pointer_offset)
            };
            (
                edge.source_object_index,
                source.class_name.clone(),
                source.token,
                field,
            )
        })
        .collect()
}

/// `field` with its array indices dropped.
fn without_indices(field: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for c in field.chars() {
        match c {
            '[' => skipping = true,
            ']' => skipping = false,
            _ if !skipping => out.push(c),
            _ => {}
        }
    }
    out
}

/// The classes and fields from the root down to object `index`, by the first
/// way each object is named.
fn chain(graph: &ObjectGraph, index: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = index;
    for _ in 0..24 {
        out.push(graph.objects[at].class_name.clone());
        let Some(first) = owners(graph, at).into_iter().next() else {
            break;
        };
        out.push(first.3);
        at = first.0;
    }
    out.reverse();
    out
}

fn clip(value: &Value) -> Value {
    let text = value.to_string();
    if text.len() > FIELD_CLIP {
        json!({ "long": text.len() })
    } else {
        value.clone()
    }
}

/// The object `VWallDriver.m_pCrv` names: its token and the indices of the
/// objects holding it.
fn named_curve(graph: &ObjectGraph) -> (Option<u64>, Vec<usize>) {
    let token = graph
        .objects
        .iter()
        .find(|object| object.class_name == "VWallDriver")
        .and_then(|driver| {
            rvt::research::native_parameters::pointer_token(driver.fields.get("m_pCrv")?)
        });
    let indices: Vec<usize> = token
        .map(|token| {
            graph
                .objects
                .iter()
                .enumerate()
                .filter(|(_, object)| u64::from(object.token) == token)
                .map(|(index, _)| index)
                .collect()
        })
        .unwrap_or_default();
    (token, indices)
}

/// The class tag the `m_pCrv` pointer itself carries, which a pointer that
/// names no object still has.
fn pointer_tag(graph: &ObjectGraph) -> Option<u64> {
    graph
        .objects
        .iter()
        .find(|object| object.class_name == "VWallDriver")
        .and_then(|driver| driver.fields.pointer("/m_pCrv/class_tag")?.as_u64())
}

/// One short line of a wall: its curves, who names each, and what `m_pCrv`
/// names, for every `SWall` of a save so that the same wall is read across the
/// saves by its id.
fn wall_line(id: u64, graph: &ObjectGraph) -> Value {
    let curves: Vec<Value> = graph
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| matches!(object.class_name.as_str(), "GLine" | "GArc"))
        .map(|(index, object)| {
            let named: Vec<String> = owners(graph, index)
                .into_iter()
                .map(|(_, class, _, field)| format!("{class}.{}", without_indices(&field)))
                .collect();
            let ends = level_ends(&object.fields);
            json!({
                "i": index,
                "c": object.class_name,
                "o": named,
                "level_line": ends.is_some(),
                "len": ends.map(|(s, e)| (e[0] - s[0]).hypot(e[1] - s[1])),
            })
        })
        .collect();
    let (token, indices) = named_curve(graph);
    json!({
        "kind": "swall",
        "id": id,
        "pcrv_token": token,
        "pcrv_tag": pointer_tag(graph),
        "pcrv_at": indices,
        "curves": curves,
        "current": current_line(graph).is_some(),
        "pointer": pointer_line(graph).is_some(),
    })
}

/// The index of the first `GLine` that only `WallRunTimeData.m_prevLocLine`
/// names.
fn previous_line_index(graph: &ObjectGraph) -> Option<usize> {
    graph
        .objects
        .iter()
        .enumerate()
        .find_map(|(index, object)| {
            let named = owners(graph, index);
            (object.class_name == "GLine"
                && !named.is_empty()
                && named.iter().all(|(_, class, _, field)| {
                    class == "WallRunTimeData" && field == "m_prevLocLine"
                }))
            .then_some(index)
        })
}

/// How far apart two level lines are, the larger distance between their ends
/// taken either way round, in feet.
fn line_gap(a: ([f64; 2], [f64; 2]), b: ([f64; 2], [f64; 2])) -> f64 {
    let end = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).hypot(p[1] - q[1]);
    let same = end(a.0, b.0).max(end(a.1, b.1));
    let turned = end(a.0, b.1).max(end(a.1, b.0));
    same.min(turned)
}

/// The level line `m_pCrv` names, where it names exactly one object and that is
/// a level `GLine` (what `location_curve` reads).
fn pointer_line(graph: &ObjectGraph) -> Option<([f64; 2], [f64; 2])> {
    let (token, indices) = named_curve(graph);
    token?;
    let [index] = indices.as_slice() else {
        return None;
    };
    let curve = &graph.objects[*index];
    if curve.class_name != "GLine" {
        return None;
    }
    level_ends(&curve.fields)
}

/// What `native_walls` reads: exactly one `GLine` or `GArc`, a level `GLine`.
fn current_line(graph: &ObjectGraph) -> Option<([f64; 2], [f64; 2])> {
    let curves: Vec<_> = graph
        .objects
        .iter()
        .filter(|object| matches!(object.class_name.as_str(), "GLine" | "GArc"))
        .collect();
    match curves.as_slice() {
        [curve] if curve.class_name == "GLine" => level_ends(&curve.fields),
        _ => None,
    }
}

fn wall_detail(id: u64, openings: &[u32], graph: &ObjectGraph) -> Value {
    let objects: Vec<Value> = graph
        .objects
        .iter()
        .enumerate()
        .map(|(index, object)| format!("{index}:{}:{}", object.class_name, object.token))
        .map(Value::from)
        .collect();
    let curves: Vec<Value> = graph
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| matches!(object.class_name.as_str(), "GLine" | "GArc"))
        .map(|(index, object)| {
            let ends = level_ends(&object.fields);
            let length = ends.map(|(s, e)| (e[0] - s[0]).hypot(e[1] - s[1]));
            let named: Vec<Value> = owners(graph, index)
                .into_iter()
                .map(|(source, class, token, field)| {
                    json!({ "object": source, "class": class, "token": token, "field": field })
                })
                .collect();
            json!({
                "index": index,
                "class": object.class_name,
                "token": object.token,
                "level_line_ends": ends,
                "length": length,
                "owners": named,
                "chain": chain(graph, index),
                "fields": clip(&object.fields),
            })
        })
        .collect();
    let (token, indices) = named_curve(graph);
    let mut steps = Count::new();
    for object in &graph.objects {
        if object.class_name.ends_with("Step") {
            bump(&mut steps, object.class_name.clone());
        }
    }
    let driver_fields: Vec<&String> = graph
        .objects
        .iter()
        .find(|object| object.class_name == "VWallDriver")
        .and_then(|driver| driver.fields.as_object())
        .map(|fields| fields.keys().collect())
        .unwrap_or_default();
    json!({
        "kind": "wall",
        "id": id,
        "openings": openings,
        "objects": objects,
        "curves": curves,
        "pcrv": { "token": token, "indices": indices },
        "current_reads": current_line(graph).is_some(),
        "pointer_reads": pointer_line(graph).is_some(),
        "steps": steps,
        "driver_fields": driver_fields,
    })
}

fn element_host(fields: &[(String, InstanceField)]) -> Option<u32> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. } if name == OPENING_HOST_FIELD => Some(*id),
        _ => None,
    })
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: MODEL.rvt [--records]");
    let mut rf = RevitFile::open(&path)?;
    let release = rf.basic_file_info()?.version;

    // The decoded walls, doors and windows, as the export reads them.
    let mut wall_class: BTreeMap<u32, (String, bool)> = BTreeMap::new();
    let mut openings: BTreeMap<u32, (String, Option<u32>, bool, bool)> = BTreeMap::new();
    let mut axes: BTreeMap<u32, [f64; 2]> = BTreeMap::new();
    let mut decoded_walls: Vec<DecodedWall> = Vec::new();
    for element in iter_elements(&mut rf)? {
        let Some(id) = element.id else { continue };
        match element.class.as_str() {
            "Door" | "Window" => {
                let axis = OPENING_PLACEMENT_X_AXIS_FIELDS.map(|wanted| {
                    element.fields.iter().find_map(|(name, value)| match value {
                        InstanceField::Float { value, .. } if name == wanted => Some(*value),
                        _ => None,
                    })
                });
                let placed = axis[0].is_some();
                if let [Some(x), Some(y)] = axis {
                    axes.insert(id, [x, y]);
                }
                openings.insert(
                    id,
                    (
                        element.class.clone(),
                        element_host(&element.fields),
                        opening_flips_from_fields(&element.fields).is_some(),
                        placed,
                    ),
                );
            }
            class if class.contains("Wall") && !class.contains("Type") => {
                let read = wall_native_direction_from_fields(&element.fields).is_some();
                wall_class.insert(id, (class.to_string(), read));
                decoded_walls.push(DecodedWall {
                    id,
                    class: class.to_string(),
                    fields: element.fields.clone(),
                });
            }
            _ => {}
        }
    }
    println!(
        "{}",
        json!({
            "kind": "file",
            "release": release,
            "walls": wall_class.len(),
            "walls_read": wall_class.values().filter(|(_, read)| *read).count(),
            "doors": openings.values().filter(|o| o.0 == "Door").count(),
            "windows": openings.values().filter(|o| o.0 == "Window").count(),
        })
    );

    // The doors and windows by whether their host gave a native line.
    let mut by_state = Count::new();
    let mut unread: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (id, (_, host, flips, placed)) in &openings {
        if !flips {
            bump(&mut by_state, "flips not read");
            continue;
        }
        let Some(host) = host else {
            bump(&mut by_state, "flips read, no host");
            continue;
        };
        let state = match wall_class.get(host) {
            None => "host is not a decoded wall".to_string(),
            Some((class, _)) if class == CURTAIN_WALL_CLASS => "host is a curtain wall".to_string(),
            Some((_, true)) => "host gave a line".to_string(),
            Some((_, false)) => {
                unread.entry(*host).or_default().push(*id);
                "host gave no line".to_string()
            }
        };
        bump(
            &mut by_state,
            format!("{state}, {}", if *placed { "placed" } else { "not placed" }),
        );
    }
    println!("{}", json!({ "kind": "openings", "states": by_state }));
    for (host, ids) in &unread {
        println!(
            "{}",
            json!({
                "kind": "unread",
                "wall": host,
                "class": wall_class.get(host).map(|(class, _)| class),
                "openings": ids,
            })
        );
    }

    // Every SWall record.
    let mut shapes = Count::new();
    let mut owners_of_curves = Count::new();
    let mut pointer_states = Count::new();
    let mut agreement = Count::new();
    let mut only_pointer: Vec<u64> = Vec::new();
    let mut only_current: Vec<u64> = Vec::new();
    let mut shown_only_pointer = 0usize;
    let mut swall_ids: BTreeSet<u64> = BTreeSet::new();
    let mut old_rule_ids: BTreeSet<u64> = BTreeSet::new();
    let mut with_previous = 0usize;
    let mut previous_offset = 0usize;
    let options = Options {
        selected_classes: BTreeSet::from(["SWall".to_string()]),
        ..Options::default()
    };
    let summary = extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        if class != "SWall" {
            return Ok(());
        }
        let id = record.identity.element_id;
        swall_ids.insert(id);
        let lines = graph
            .objects
            .iter()
            .filter(|object| object.class_name == "GLine")
            .count();
        let arcs = graph
            .objects
            .iter()
            .filter(|object| object.class_name == "GArc")
            .count();
        bump(&mut shapes, format!("GLine={lines} GArc={arcs}"));
        for (index, object) in graph.objects.iter().enumerate() {
            if !matches!(object.class_name.as_str(), "GLine" | "GArc") {
                continue;
            }
            let named = owners(graph, index);
            if named.is_empty() {
                bump(
                    &mut owners_of_curves,
                    format!("{}: no owner", object.class_name),
                );
            }
            for (_, owner_class, _, field) in named {
                bump(
                    &mut owners_of_curves,
                    format!(
                        "{}: {owner_class}.{}",
                        object.class_name,
                        without_indices(&field)
                    ),
                );
            }
        }
        let (token, indices) = named_curve(graph);
        let state = match (token, indices.as_slice()) {
            (None, _) => "no VWallDriver.m_pCrv".to_string(),
            (Some(0), _) => "m_pCrv null".to_string(),
            (Some(_), []) => format!(
                "m_pCrv names no object (tag {})",
                pointer_tag(graph).map_or_else(|| "?".to_string(), |tag| tag.to_string())
            ),
            (Some(_), [one]) => format!("m_pCrv names one {}", graph.objects[*one].class_name),
            (Some(_), _) => "m_pCrv names several objects".to_string(),
        };
        bump(
            &mut pointer_states,
            format!("{state}; {lines} GLine, {arcs} GArc"),
        );
        let (now, pointed) = (current_line(graph), pointer_line(graph));
        if now.is_some() {
            old_rule_ids.insert(id);
        }
        if let Some(previous) = previous_line_index(graph) {
            with_previous += 1;
            let previous_ends = graph
                .objects
                .get(previous)
                .and_then(|object| level_ends(&object.fields));
            if previous_ends
                .zip(pointed)
                .is_some_and(|(a, b)| line_gap(a, b) > 1e-6)
            {
                previous_offset += 1;
            }
        }
        match (now, pointed) {
            (Some(a), Some(b)) => {
                let same =
                    a.0.iter()
                        .chain(&a.1)
                        .zip(b.0.iter().chain(&b.1))
                        .all(|(x, y)| (x - y).abs() <= 1e-9);
                bump(
                    &mut agreement,
                    if same {
                        "read by both, same line"
                    } else {
                        "read by both, DIFFERENT line"
                    },
                );
            }
            (None, Some(_)) => {
                bump(&mut agreement, "read by m_pCrv only");
                only_pointer.push(id);
            }
            (Some(_), None) => {
                bump(&mut agreement, "read by the current rule only");
                only_current.push(id);
            }
            (None, None) => bump(&mut agreement, "read by neither"),
        }
        println!("{}", wall_line(id, graph));
        let openings_here: Vec<u32> = u32::try_from(id)
            .ok()
            .and_then(|id| unread.get(&id).cloned())
            .unwrap_or_default();
        let wanted = !openings_here.is_empty()
            || NAMED.contains(&id)
            || (now.is_none() && pointed.is_some() && {
                shown_only_pointer += 1;
                shown_only_pointer <= ONLY_POINTER_DETAIL
            });
        if wanted {
            println!("{}", wall_detail(id, &openings_here, graph));
        }
        Ok(())
    });
    match summary {
        Ok(summary) => eprintln!(
            "native records: {} emitted, {} graphs complete, {} unsupported",
            summary.emitted_records,
            summary.complete_graph_records,
            summary.unsupported_graph_records
        ),
        Err(error) => eprintln!("native records: not read: {error}"),
    }
    only_pointer.truncate(ID_LIST);
    only_current.truncate(ID_LIST);
    println!(
        "{}",
        json!({
            "kind": "census",
            "release": release,
            "shapes": shapes,
            "curve_owners": owners_of_curves,
            "pointer": pointer_states,
            "agreement": agreement,
            "only_pointer_ids": only_pointer,
            "only_current_ids": only_current,
        })
    );

    // What the library reads: the walls `native_walls` gives a row, and every
    // field of the decoded walls that have an `SWall` record and of the doors and
    // windows, so that two builds of the same file are compared field by field.
    let read = native_walls(&mut rf, &swall_ids).unwrap_or_default();
    println!(
        "{}",
        json!({
            "kind": "native",
            "release": release,
            "swall_records": swall_ids.len(),
            "read": read.len(),
            "ids": read.keys().collect::<Vec<_>>(),
        })
    );
    // Counts only, in the alphabet tools/ci/scrub-holdout-results.sh keeps for a
    // model without a licence to redistribute it.
    println!(
        "previous lines: swall records {}, with a previous location line {}, of those offset from the location curve {}, read through the rule {}, read by the old rule {}",
        swall_ids.len(),
        with_previous,
        previous_offset,
        read.keys().filter(|id| !old_rule_ids.contains(*id)).count(),
        old_rule_ids.len()
    );
    for wall in decoded_walls
        .iter()
        .filter(|wall| swall_ids.contains(&u64::from(wall.id)))
    {
        let fields: Vec<(&String, String)> = wall
            .fields
            .iter()
            .map(|(name, value)| (name, field_text(value)))
            .collect();
        println!(
            "{}",
            json!({ "kind": "wallfields", "id": wall.id, "class": wall.class, "fields": fields })
        );
    }
    for (id, (class, host, flips, placed)) in &openings {
        println!(
            "{}",
            json!({
                "kind": "opening",
                "id": id,
                "class": class,
                "host": host,
                "flips_read": flips,
                "placed": placed,
                "axis": axes.get(id),
            })
        );
    }
    Ok(())
}
