//! #2109: could the glTF exporter draw only the glass part of a door, window or lamp
//! see-through, from the materials its saved parts name?
//!
//! FACT under test: #2109 draws an element that keeps its category's colour whole at its
//! most transparent read material's alpha. The elements it reaches are those drawn in a
//! category colour whose type names a transparent material among its own. A saved part
//! (`native_saved_mesh::Primitive`) names a material ElementId on one of the reader's two
//! paths (a polymesh's `m_materialID`) and none on the other. This counts, for those
//! elements drawn from their own mesh, how many have saved parts naming two or more
//! materials the file names, a transparent one among them, so each part could be drawn in
//! its own material; and how many material ids the parts name that the file does not.
//!
//! The model is exported as `rvt-gltf` exports it (`run_export_with_limits_and_control`,
//! default saved meshes) and put to `gltf::build_gltf`, and the elements' saved graphics
//! are read with `native_saved_scene::extract_at_detail` at detail 3, hidden and filtered
//! parts left out, as the exporter leaves them out. Material names come from
//! `partition_materials::scan_material_names`, and a material is transparent when the
//! model's material of that name has a transparency above 0. It prints:
//!
//! ```text
//! reach class=<IFC class> shape=<cube|mesh> elements=<n>
//! parts class=<IFC class> elements=<n> no_graphics=<n> split=<n> split_partly=<n> one_named=<n> one_transparent=<n> none_named=<n> other=<n>
//! ids named=<n> joined=<n> unjoined=<n>
//! tags class=<IFC class> elements=<n> no_symbol=<n> no_map=<n> parts=<n> tagged=<n> mapped=<n> split=<n> one=<n> none=<n>
//! keys read=<n> unread=<n>
//! sample id=<ElementId> symbol=<ElementId> face_tags=[..] map_tags=[..]
//! ```
//!
//! The second route: a family type's `m_geomTag2MaterialId` maps geometry tags to material
//! ElementIds (RE-82, RE-203), and each saved part keeps a tag (`face_tag`, `m_GInfo.m_tag`).
//! `tags` reads each element's type from its `InstanceInfo.m_symbolId`, then each part's tag in
//! that type's map: `tagged` parts carry a tag other than -1, `mapped` ones find it in the map;
//! `split` elements map parts to a transparent and an opaque material, `one` to a single
//! material, `none` to no material. `keys` counts the map entries whose tag was read as an
//! integer. `sample` prints three elements' tags beside their type's map tags.
//!
//! `split`: every part names a material the file names, two or more of them, a transparent
//! one and an opaque one among them. `split_partly`: the same among the parts that name a
//! material, with some parts naming none. `one_named`: every named part names the same
//! material; `one_transparent`: of those, a transparent one. `none_named`: no part names a
//! material. `other`: two or more named, but one the file does not name, or none transparent
//! or none opaque. Counts only.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_2109_part_materials -- MODEL.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::gltf::build_gltf;
use rvt::ifc::{ExportOptions, SavedMeshChoice, run_export_with_limits_and_control};
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_scene::extract_at_detail;

/// An ElementId or tag as the graph saves it: an integer, or one under `m_id` / `m_id64`.
fn ident(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(ident)
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2109_part_materials MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let run = run_export_with_limits_and_control(
        &mut rf,
        &ExportOptions::default().with_saved_meshes(SavedMeshChoice::Default),
        rvt::walker::WalkerLimits::default(),
        &rvt::control::WalkerControl::new(),
    )?;
    let model = &run.model;
    let transparent_names: BTreeSet<&str> = model
        .materials
        .iter()
        .filter(|m| m.transparency.is_some_and(|t| t > 0.0))
        .map(|m| m.name.as_str())
        .collect();
    // The materials each element is associated with through its type, as #2109 reads them.
    let mut associated: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for set in &model.material_constituent_sets {
        for &element in &set.elements {
            associated
                .entry(element)
                .or_default()
                .extend(set.material_indices.iter().copied());
        }
    }
    let (doc, _) = build_gltf(model);
    // Elements drawn in a category colour whose type names a transparent material: those
    // #2109 draws at that material's alpha.
    let mut reach: BTreeMap<(String, &str), usize> = BTreeMap::new();
    let mut meshed: BTreeMap<u64, String> = BTreeMap::new();
    for node in &doc.nodes {
        let (Some(mesh), Some(extras)) = (node.mesh, node.extras.as_ref()) else {
            continue;
        };
        let Some(primitive) = doc.meshes.get(mesh).and_then(|m| m.primitives.first()) else {
            continue;
        };
        let Some(material) = primitive.material.and_then(|i| doc.materials.get(i)) else {
            continue;
        };
        if !material
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("Category "))
        {
            continue;
        }
        let transparent = associated.get(&extras.entity_index).is_some_and(|list| {
            list.iter()
                .filter_map(|i| model.materials.get(*i))
                .any(|m| m.transparency.is_some_and(|t| t > 0.0))
        });
        if !transparent {
            continue;
        }
        // The shared unit cube's positions are accessor 0.
        let shape = if primitive.attributes.get("POSITION") == Some(&0) {
            "cube"
        } else {
            "mesh"
        };
        *reach.entry((extras.ifc_type.clone(), shape)).or_default() += 1;
        if shape == "mesh" {
            let id = match model.entities.get(extras.entity_index) {
                Some(IfcEntity::BuildingElement { type_guid, .. }) => {
                    type_guid.as_deref().and_then(|tag| tag.parse::<u64>().ok())
                }
                _ => None,
            };
            if let Some(id) = id {
                meshed.insert(id, extras.ifc_type.clone());
            }
        }
    }
    for ((class, shape), n) in &reach {
        println!("reach class={class} shape={shape} elements={n}");
    }
    let release = rf.basic_file_info().map(|b| b.version).unwrap_or(0);
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let names =
        rvt::research::partition_materials::scan_material_names(&mut rf, release, &declared)
            .unwrap_or_default();
    let scene = if meshed.is_empty() {
        None
    } else {
        let options = Options {
            selected_ids: meshed.keys().copied().collect(),
            max_graph_values: 1_000_000,
            max_graph_objects: 1_000_000,
            ..Options::default()
        };
        match extract_at_detail(&mut rf, &options, 3) {
            Ok(scene) => Some(scene),
            Err(error) => {
                println!("saved graphics: {error}");
                None
            }
        }
    };
    // Per class: elements, no graphics, split, split partly, one named, one transparent,
    // none named, other.
    let mut tally: BTreeMap<String, [usize; 8]> = BTreeMap::new();
    let mut found: BTreeSet<u64> = BTreeSet::new();
    let mut face_tags: BTreeMap<u64, Vec<i64>> = BTreeMap::new();
    let (mut named_ids, mut joined_ids) = (BTreeSet::new(), BTreeSet::new());
    if let Some(scene) = &scene {
        for element in &scene.elements {
            let id = element.identity.element_id;
            let Some(class) = meshed.get(&id) else {
                continue;
            };
            found.insert(id);
            let parts: Vec<_> = element
                .meshes
                .primitives
                .iter()
                .filter(|p| !p.hidden && !p.filtered)
                .collect();
            face_tags.insert(id, parts.iter().map(|p| p.face_tag).collect());
            let ids: BTreeSet<i64> = parts.iter().filter_map(|p| p.material_id).collect();
            let unnamed = parts.iter().filter(|p| p.material_id.is_none()).count();
            let mut transparent = 0usize;
            let mut opaque = 0usize;
            let mut joined = 0usize;
            for id in &ids {
                named_ids.insert(*id);
                let name = u32::try_from(*id).ok().and_then(|id| names.get(&id));
                if let Some(name) = name {
                    joined += 1;
                    joined_ids.insert(*id);
                    if transparent_names.contains(name.as_str()) {
                        transparent += 1;
                    } else {
                        opaque += 1;
                    }
                }
            }
            let t = tally.entry(class.clone()).or_default();
            t[0] += 1;
            if ids.is_empty() {
                t[6] += 1;
            } else if ids.len() == 1 {
                t[4] += 1;
                if transparent == 1 {
                    t[5] += 1;
                }
            } else if joined == ids.len() && transparent > 0 && opaque > 0 {
                if unnamed == 0 {
                    t[2] += 1;
                } else {
                    t[3] += 1;
                }
            } else {
                t[7] += 1;
            }
        }
    }
    for (id, class) in &meshed {
        if !found.contains(id) {
            let t = tally.entry(class.clone()).or_default();
            t[0] += 1;
            t[1] += 1;
        }
    }
    for (class, t) in &tally {
        println!(
            "parts class={class} elements={} no_graphics={} split={} split_partly={} one_named={} one_transparent={} none_named={} other={}",
            t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7]
        );
    }
    println!(
        "ids named={} joined={} unjoined={}",
        named_ids.len(),
        joined_ids.len(),
        named_ids.len() - joined_ids.len()
    );
    // The second route: each element's type, then the type's tag-to-material map.
    let mut symbol_of: BTreeMap<u64, u64> = BTreeMap::new();
    if !meshed.is_empty() {
        let options = Options {
            selected_ids: meshed.keys().copied().collect(),
            max_graph_values: 1_000_000,
            max_graph_objects: 1_000_000,
            ..Options::default()
        };
        let read = extract_graphs(&mut rf, &options, |record| {
            if let Some(symbol) = record.graph.as_ref().and_then(|graph| {
                graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "InstanceInfo")
                    .and_then(|object| object.fields.get("m_symbolId"))
                    .and_then(ident)
                    .and_then(|id| u64::try_from(id).ok())
            }) {
                symbol_of.insert(record.identity.element_id, symbol);
            }
            Ok(())
        });
        if let Err(error) = read {
            println!("instances: {error}");
        }
    }
    let symbols: BTreeSet<u64> = symbol_of.values().copied().collect();
    let mut maps: BTreeMap<u64, BTreeMap<i64, i64>> = BTreeMap::new();
    let (mut keys_read, mut keys_unread) = (0usize, 0usize);
    if !symbols.is_empty() {
        let options = Options {
            selected_ids: symbols.clone(),
            max_graph_values: 1_000_000,
            max_graph_objects: 1_000_000,
            ..Options::default()
        };
        let read = extract_graphs(&mut rf, &options, |record| {
            let Some(map) = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
                .and_then(|root| root.fields.get("m_geomTag2MaterialId"))
                .and_then(Fv::as_array)
            else {
                return Ok(());
            };
            let mut tags = BTreeMap::new();
            for entry in map {
                let tag = entry.get("first").and_then(ident);
                let material = entry.pointer("/second/m_id/m_id64").and_then(Fv::as_i64);
                match (tag, material) {
                    (Some(tag), Some(material)) => {
                        keys_read += 1;
                        tags.insert(tag, material);
                    }
                    _ => keys_unread += 1,
                }
            }
            maps.insert(record.identity.element_id, tags);
            Ok(())
        });
        if let Err(error) = read {
            println!("symbols: {error}");
        }
    }
    // Per class: elements, no symbol, no map, parts, tagged, mapped, split, one, none.
    let mut by_tag: BTreeMap<String, [usize; 9]> = BTreeMap::new();
    let mut samples = 0usize;
    for (id, class) in &meshed {
        let t = by_tag.entry(class.clone()).or_default();
        t[0] += 1;
        let Some(symbol) = symbol_of.get(id) else {
            t[1] += 1;
            continue;
        };
        let Some(map) = maps.get(symbol) else {
            t[2] += 1;
            continue;
        };
        let tags = face_tags.get(id).cloned().unwrap_or_default();
        if samples < 3 {
            samples += 1;
            let mut shown: Vec<i64> = tags.clone();
            shown.sort_unstable();
            shown.dedup();
            println!(
                "sample id={id} symbol={symbol} face_tags={:?} map_tags={:?}",
                shown.iter().take(12).collect::<Vec<_>>(),
                map.keys().take(12).collect::<Vec<_>>()
            );
        }
        let mut transparent = 0usize;
        let mut opaque = 0usize;
        let mut materials: BTreeSet<i64> = BTreeSet::new();
        for tag in &tags {
            t[3] += 1;
            if *tag == -1 {
                continue;
            }
            t[4] += 1;
            if let Some(material) = map.get(tag) {
                t[5] += 1;
                materials.insert(*material);
            }
        }
        for material in &materials {
            let name = u32::try_from(*material).ok().and_then(|m| names.get(&m));
            match name {
                Some(name) if transparent_names.contains(name.as_str()) => transparent += 1,
                Some(_) => opaque += 1,
                None => {}
            }
        }
        if transparent > 0 && opaque > 0 {
            t[6] += 1;
        } else if materials.len() == 1 {
            t[7] += 1;
        } else if materials.is_empty() {
            t[8] += 1;
        }
    }
    for (class, t) in &by_tag {
        println!(
            "tags class={class} elements={} no_symbol={} no_map={} parts={} tagged={} mapped={} split={} one={} none={}",
            t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7], t[8]
        );
    }
    println!("keys read={keys_read} unread={keys_unread}");
    Ok(())
}
