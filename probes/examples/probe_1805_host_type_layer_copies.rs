//! #1805 (probe): every copy of a wall, floor, roof or ceiling type's layers
//! whose copies disagree, with the elements of that type, for a scorer to set
//! against the layer sets of Revit's export.
//!
//! FACT under test: RE-446 takes the current partition's copy of a host type's
//! layers where its copies disagree, on Revit 2024 and later only. Taken on
//! every release, the current copy gave four floors of a held-out Revit 2020
//! file materials Revit's export does not give them. This prints, for each
//! type an element names whose decoded copies disagree, every copy (its
//! partition, whether it is the first, the latest and the current one, and its
//! layers with their materials' names) and the elements of the type, so
//! `tools/re/layer_copies_vs_ifc.py` can say which copy, if any, holds Revit's
//! widths and materials on each release. RE-446's probe lists wall types only.
//!
//! Output:
//!
//!   file <path> release <release>
//!   summary host_types <n> copies_disagree <n> current_read <n>
//!   json {"type": id, "class": "Wall|Floor|Roof|Ceiling", "elements": [ids],
//!         "current": partition|null,
//!         "copies": [{"partition": p, "offset": o, "role": ["first", "latest", "current"],
//!                     "layers": [{"mm": width, "material": name|null, "function": f}]}]}
//!
//! Only decoded copies are listed. A layer's material is `null` where the
//! layer takes its category's, or where the material's name is not read.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1805_host_type_layer_copies -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document;
use rvt::research::partition_compound_structure as pcs;
use rvt::research::partition_names;
use rvt::research::partition_schema_mvp::TYPE_ID_FIELD;
use rvt::research::partition_type_records as ptr;
use rvt::walker::InstanceField;
use std::collections::{BTreeMap, BTreeSet};

/// The element classes whose types hold a compound structure.
const HOST_CLASSES: [&str; 4] = ["Wall", "Floor", "Roof", "Ceiling"];

/// Where RE-115's object-styles entry is lost on a 32-bit release: of the
/// places the category's `i32` is followed by `ff ff ff ff`, how many pass
/// each of `scan_category_material_2023`'s checks in turn, where
/// `CATEGORY_MATERIAL_MARK_2023` sits past the category when the frame does
/// not hold the release's `PatternHelper` tag, and how many materials the
/// entries that pass hold, set and unset. Counts and offsets only.
fn category_entry_trace(rf: &mut RevitFile, version: u32, category: i64) -> serde_json::Value {
    use rvt::research::partition_materials::CATEGORY_MATERIAL_MARK_2023 as MARK;
    let Some(tags) = rvt::research::partition_element_records_2023::schema_tags(version) else {
        return serde_json::json!({"layout": "64-bit"});
    };
    let Ok(category) = i32::try_from(category) else {
        return serde_json::json!({"layout": "category out of range"});
    };
    let mut pattern = category.to_le_bytes().to_vec();
    pattern.extend_from_slice(&[0xff; 4]);
    let mut frame = [0xff, 0xff, 0xff, 0xff, 0, 0];
    frame[4..].copy_from_slice(&tags.pattern_helper.to_le_bytes());
    let (mut hits, mut word, mut one, mut framed, mut other_tag, mut marked) = (0, 0, 0, 0, 0, 0);
    let mut mark_offsets: BTreeMap<usize, usize> = BTreeMap::new();
    let mut values: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &pattern) {
            hits += 1;
            let get = |offset: usize, len: usize| buf.get(at + offset..at + offset + len);
            if let Some(window) = get(4, 60) {
                for hit in memchr::memmem::find_iter(window, &MARK) {
                    *mark_offsets.entry(hit + 4).or_default() += 1;
                }
            }
            if !matches!(get(8, 4), Some([1 | 2, 0, 0, 0])) {
                continue;
            }
            word += 1;
            if get(12, 4) != Some(&[1, 0, 0, 0][..]) {
                continue;
            }
            one += 1;
            if get(16, 4) == Some(&[0xff; 4][..]) && get(16, 6) != Some(&frame[..]) {
                other_tag += 1;
            }
            if get(16, 6) != Some(&frame[..]) {
                continue;
            }
            framed += 1;
            if get(30, 4) != Some(&MARK[..]) {
                continue;
            }
            marked += 1;
            if let Some(bytes) = get(34, 4) {
                values.insert(u32::from_le_bytes(bytes.try_into().expect("four bytes")));
            }
        }
    }
    let set = values.iter().filter(|&&v| v != 0 && v != u32::MAX).count();
    serde_json::json!({
        "layout": "32-bit",
        "hits": hits, "word": word, "one": one, "frame": framed, "other_tag": other_tag,
        "mark": marked, "values_set": set, "values_unset": values.len() - set,
        "mark_offsets": mark_offsets.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>(),
    })
}

fn partition_of(stream: &str) -> u32 {
    stream
        .rsplit('/')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("file {path} release {version}");
    let (Some(layout), Some(tag), Some(layer_layout)) = (
        partition_names::element_data_layout(version),
        pcs::layer_frame_tag(version),
        pcs::layer_layout(version),
    ) else {
        println!("not read: release {version} has no measured layer layout");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let mvp = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    // The material ids and names the export reads (`materials_and_names`): a 32-bit release's
    // from its `Material` records, a 64-bit release's from its type records and the schema
    // reader's materials. Revit 2008 to 2023 have no material type records, so their layers
    // would not decode against those.
    let (materials, names): (BTreeSet<u32>, BTreeMap<u32, String>) =
        if rvt::research::partition_element_records_2023::is_32bit_release(version) {
            rvt::research::partition_materials::scan_materials_32(&mut rf, version, &declared)
        } else {
            let ids = ptr::scan_type_records(&mut rf, version, ptr::OST_MATERIALS, &declared)?
                .iter()
                .map(|record| record.element_id)
                .collect();
            let names =
                mvp.materials
                    .iter()
                    .filter_map(|material| {
                        let id = material.id?;
                        material.fields.iter().find_map(|(name, value)| {
                            match (name.as_str(), value) {
                                ("m_name", InstanceField::String(text)) => Some((id, text.clone())),
                                _ => None,
                            }
                        })
                    })
                    .collect();
            (ids, names)
        };
    // The elements each type is given by the partition schema reader, and its class.
    let mut elements_of: BTreeMap<u32, (String, Vec<u32>)> = BTreeMap::new();
    for element in mvp
        .walls
        .iter()
        .chain(&mvp.slabs)
        .chain(&mvp.products)
        .filter(|element| HOST_CLASSES.contains(&element.class.as_str()))
    {
        let Some(id) = element.id else { continue };
        for (name, value) in &element.fields {
            if let (TYPE_ID_FIELD, InstanceField::ElementId { id: type_id, .. }) =
                (name.as_str(), value)
            {
                elements_of
                    .entry(*type_id)
                    .or_insert_with(|| (element.class.clone(), Vec::new()))
                    .1
                    .push(id);
            }
        }
    }
    // The decoded copies of each type, and every partition holding its data.
    let mut copies: BTreeMap<u32, Vec<(u32, usize, Vec<pcs::CompoundLayer>)>> = BTreeMap::new();
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &layout.header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + layout.header.len();
            let Some(id) = layout
                .id_at(buf, id_at)
                .filter(|id| elements_of.contains_key(id))
            else {
                continue;
            };
            holders.entry(id).or_default().insert(partition_of(&stream));
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(pcs::LAYER_WINDOW))
                .min(buf.len());
            let data = buf.get(id_at + 8..end).unwrap_or(&[]);
            if let Some(layers) = pcs::find_layers(data, tag, layer_layout, &materials, &declared) {
                copies
                    .entry(id)
                    .or_default()
                    .push((partition_of(&stream), hit, layers));
            }
        }
    }
    for held in copies.values_mut() {
        held.sort_by_key(|(partition, hit, _)| (*partition, *hit));
    }
    let disagreeing: BTreeMap<u32, BTreeSet<u32>> = copies
        .iter()
        .filter(|(_, held)| held.iter().any(|(_, _, layers)| *layers != held[0].2))
        .filter_map(|(id, _)| Some((*id, holders.get(id)?.clone())))
        .collect();
    let current = native_document::current_partitions(&mut rf, &disagreeing);
    println!(
        "summary host_types {} copies_disagree {} current_read {}",
        elements_of.len(),
        disagreeing.len(),
        disagreeing
            .keys()
            .filter(|id| current.contains_key(id))
            .count(),
    );
    for id in disagreeing.keys() {
        let (Some(held), Some((class, elements))) = (copies.get(id), elements_of.get(id)) else {
            continue;
        };
        let now = current.get(id).copied();
        let last = held.len() - 1;
        let listed: Vec<serde_json::Value> = held
            .iter()
            .enumerate()
            .map(|(index, (partition, offset, layers))| {
                let mut role = Vec::new();
                if index == 0 {
                    role.push("first");
                }
                if index == last {
                    role.push("latest");
                }
                // The copy the export would take: the last one in the current partition.
                if Some(*partition) == now
                    && !held[index + 1..].iter().any(|(p, _, _)| Some(*p) == now)
                {
                    role.push("current");
                }
                serde_json::json!({
                    "partition": partition,
                    "offset": offset,
                    "role": role,
                    "layers": layers.iter().map(|layer| serde_json::json!({
                        "mm": layer.width_feet * 304.8,
                        "material": layer.material.and_then(|m| names.get(&m)),
                        "function": layer.function,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        // The material the export gives a layer that takes its category's (RE-91, RE-115, RE-206).
        let category = match class.as_str() {
            "Wall" => Some(rvt::research::partition_element_records::OST_WALLS),
            "Floor" => Some(rvt::research::partition_element_records::OST_FLOORS),
            "Roof" => Some(rvt::research::partition_element_records::OST_ROOFS),
            "Ceiling" => Some(rvt::research::partition_element_records::OST_CEILINGS),
            _ => None,
        };
        let category_id = category.and_then(|category| {
            rvt::research::partition_materials::scan_category_material(&mut rf, version, category)
                .ok()
                .flatten()
        });
        let category_material = category_id.and_then(|material| names.get(&material).cloned());
        let category_trace = category.map(|category| {
            let mut trace = category_entry_trace(&mut rf, version, category);
            let styles = match rvt::research::partition_materials::native_category_material(
                &mut rf, category,
            ) {
                Ok(Some(_)) => "one",
                Ok(None) => "none",
                Err(_) => "error",
            };
            trace["styles"] = serde_json::json!(styles);
            trace["read"] = serde_json::json!(category_id.is_some());
            trace["named"] = serde_json::json!(category_material.is_some());
            trace
        });
        println!(
            "json {}",
            serde_json::json!({
                "type": id,
                "class": class,
                "category_material": category_material,
                "category_trace": category_trace,
                "elements": elements,
                "current": now,
                "copies": listed,
            })
        );
    }
    Ok(())
}
