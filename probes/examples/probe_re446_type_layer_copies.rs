//! RE-446 (probe): the copies of a wall type's compound structure, set against
//! what the export reads of them.
//!
//! FACT under test: the export gives a wall type its layers from the copies of
//! the type's data in the `Partitions/*` streams, and drops a type whose copies
//! do not give one list (`partition_compound_structure::scan_type_layers`). On
//! the BIM4LCA timber office at building permit the types `VS-2.CLTpinta`
//! (3408088) and `SPR-1` (2435488) get no layers, so their walls have no
//! `IfcWallType`, `Width` or `Reference` (#1802). This prints each wall type's
//! copies, to say whether they disagree or do not decode.
//!
//! Output:
//!
//!   file <path> release <release>
//!   summary wall_types <n> used <n> layers_read <n> copies_agree <n> copies_disagree <n> none_decoded <n> disagree_latest_differs_from_first <n>
//!   type <id> walls <n> copies <k> decoded <d> verdict <agree|disagree|none|single> latest_p <p> current_p <p|-> total_mm <first>/<latest>/<current>
//!     copy <p>/<offset> <layers|none>      one line per copy, only for a type used by a wall whose copies do not give one list
//!     data <p>/<offset> <hex>               the first 192 bytes after a copy's id, for a type none of whose copies decode
//!
//! A layer prints as `<width mm>/<material id or ->/<function>`, layers joined
//! by `;`, top of the list first. `walls` counts the walls the partition
//! schema reader gives the type.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re446_type_layer_copies -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document;
use rvt::research::partition_compound_structure as pcs;
use rvt::research::partition_names;
use rvt::research::partition_type_records as ptr;
use rvt::walker::InstanceField;
use std::collections::{BTreeMap, BTreeSet};

/// A copy of a type's data: where it is and the layers it gives.
struct Copy {
    partition: u32,
    offset: usize,
    layers: Option<Vec<pcs::CompoundLayer>>,
    data: Vec<u8>,
}

fn partition_of(stream: &str) -> u32 {
    stream
        .rsplit('/')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

fn print_layers(layers: &[pcs::CompoundLayer]) -> String {
    layers
        .iter()
        .map(|layer| {
            format!(
                "{:.1}/{}/{}",
                layer.width_feet * 304.8,
                layer.material.map_or("-".to_string(), |id| id.to_string()),
                layer.function
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn total_mm(layers: &[pcs::CompoundLayer]) -> f64 {
    layers.iter().map(|layer| layer.width_feet).sum::<f64>() * 304.8
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
    let definitions = ptr::type_definition_ids(&ptr::scan_type_records(
        &mut rf,
        version,
        rvt::research::partition_element_records::OST_WALLS,
        &declared,
    )?);
    let materials: BTreeSet<u32> =
        ptr::scan_type_records(&mut rf, version, ptr::OST_MATERIALS, &declared)?
            .iter()
            .map(|record| record.element_id)
            .collect();
    // The walls each type is given by the partition schema reader.
    let mvp = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    let mut walls_of: BTreeMap<u32, usize> = BTreeMap::new();
    for wall in &mvp.walls {
        for (name, value) in &wall.fields {
            if let (
                rvt::research::partition_schema_mvp::TYPE_ID_FIELD,
                InstanceField::ElementId { id, .. },
            ) = (name.as_str(), value)
            {
                *walls_of.entry(*id).or_default() += 1;
            }
        }
    }
    let used: BTreeSet<u32> = walls_of.keys().copied().collect();
    let mut copies: BTreeMap<u32, Vec<Copy>> = BTreeMap::new();
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
                .filter(|id| definitions.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(pcs::LAYER_WINDOW))
                .min(buf.len());
            let data = buf.get(id_at + 8..end).unwrap_or(&[]);
            copies.entry(id).or_default().push(Copy {
                partition: partition_of(&stream),
                offset: hit,
                layers: pcs::find_layers(data, tag, layer_layout, &materials, &declared),
                data: data.iter().copied().take(192).collect(),
            });
        }
    }
    // The partition the element index routes each used type with several copies to, as the
    // readers of an element's data take it (RE-145, RE-216).
    let holders: BTreeMap<u32, BTreeSet<u32>> = copies
        .iter()
        .filter(|(id, held)| used.contains(id) && held.len() > 1)
        .map(|(id, held)| (*id, held.iter().map(|copy| copy.partition).collect()))
        .collect();
    let current = native_document::current_partitions(&mut rf, &holders);
    let (mut layers_read, mut agree, mut disagree, mut none_decoded, mut latest_differs) =
        (0, 0, 0, 0, 0);
    let mut lines: Vec<String> = Vec::new();
    for (id, held) in &copies {
        if !used.contains(id) {
            continue;
        }
        let mut ordered: Vec<&Copy> = held.iter().collect();
        ordered.sort_by_key(|copy| (copy.partition, copy.offset));
        let decoded: Vec<&&Copy> = ordered
            .iter()
            .filter(|copy| copy.layers.is_some())
            .collect();
        let first = decoded.first().and_then(|copy| copy.layers.as_ref());
        let latest = decoded.last().and_then(|copy| copy.layers.as_ref());
        let same = decoded.iter().all(|copy| copy.layers.as_ref() == first);
        let verdict = if decoded.is_empty() {
            none_decoded += 1;
            "none"
        } else if !same {
            disagree += 1;
            latest_differs += usize::from(latest != first);
            "disagree"
        } else if ordered.len() == 1 {
            layers_read += 1;
            "single"
        } else {
            layers_read += 1;
            agree += 1;
            "agree"
        };
        if verdict == "agree" || verdict == "single" {
            continue;
        }
        lines.push(format!(
            "type {id} walls {} copies {} decoded {} verdict {verdict} latest_p {} current_p {} total_mm {}/{}/{}",
            walls_of.get(id).copied().unwrap_or(0),
            ordered.len(),
            decoded.len(),
            ordered.last().map_or(0, |copy| copy.partition),
            current.get(id).map_or("-".to_string(), |p| p.to_string()),
            first.map_or("-".to_string(), |l| format!("{:.1}", total_mm(l))),
            latest.map_or("-".to_string(), |l| format!("{:.1}", total_mm(l))),
            current
                .get(id)
                .and_then(|p| {
                    ordered
                        .iter()
                        .rev()
                        .find(|copy| copy.partition == *p)
                        .and_then(|copy| copy.layers.as_ref())
                })
                .map_or("-".to_string(), |l| format!("{:.1}", total_mm(l))),
        ));
        for copy in &ordered {
            lines.push(format!(
                "  copy {}/{} {}",
                copy.partition,
                copy.offset,
                copy.layers
                    .as_ref()
                    .map_or("none".to_string(), |layers| print_layers(layers))
            ));
        }
        if decoded.is_empty() {
            for copy in &ordered {
                let hex: String = copy.data.iter().map(|byte| format!("{byte:02x}")).collect();
                lines.push(format!("  data {}/{} {hex}", copy.partition, copy.offset));
            }
        }
    }
    let missing = used.iter().filter(|id| !copies.contains_key(id)).count();
    println!(
        "summary wall_types {} used {} layers_read {layers_read} copies_agree {agree} copies_disagree {disagree} none_decoded {none_decoded} disagree_latest_differs_from_first {latest_differs} used_without_a_data_copy {missing}",
        definitions.len(),
        used.len(),
    );
    for line in lines {
        println!("{line}");
    }
    Ok(())
}
