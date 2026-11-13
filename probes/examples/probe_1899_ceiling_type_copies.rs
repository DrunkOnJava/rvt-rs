//! #1899: the copies of each ceiling type's compound structure, on the saves of
//! one project.
//!
//! FACT under test: on Autodesk's `rac_advanced` the 2023 save's export names
//! the furred ceilings 247617 and 247677 `Ceiling-<id>` where the 2022 save
//! names them `Compound Ceiling:Furred Ceiling:<id>`, and gives them no
//! `Pset_CoveringCommon`: their native records and the parameters the reader
//! finds in them are the same on both saves (`probe_1899_covering_parameters`),
//! so what differs is the type. Hypothesis: the type of the furred ceilings has
//! copies in several partitions that give different layers on the 2023 save,
//! and `partition_compound_structure::scan_type_layers` drops a type whose
//! copies disagree on Revit 2008 to 2023 (RE-446), so the ceilings have no type.
//! This prints every ceiling type's copies, as `probe_re446_type_layer_copies`
//! does for wall types, to say whether the copies of any type disagree on the 2023
//! save and not on the 2022 save.
//!
//! Output:
//!
//!   file <path> release <release>
//!   summary ceiling_types <n> copies_agree <n> copies_disagree <n> none_decoded <n>
//!   type <id> copies <k> decoded <d> verdict <agree|disagree|none|single> current_p <p|->
//!     copy <partition>/<offset> <layers|none>
//!
//! A layer prints as `<width mm>/<material id or ->/<function>`, layers joined
//! by `;`, top of the list first. Every type prints its copies, not only those
//! that disagree.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1899_ceiling_type_copies -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document;
use rvt::research::partition_compound_structure as pcs;
use rvt::research::partition_element_records::OST_CEILINGS;
use rvt::research::partition_names;
use rvt::research::partition_type_records as ptr;
use std::collections::{BTreeMap, BTreeSet};

/// A copy of a type's data: where it is and the layers it gives.
struct Copy {
    partition: u32,
    offset: usize,
    layers: Option<Vec<pcs::CompoundLayer>>,
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
        OST_CEILINGS,
        &declared,
    )?);
    let materials: BTreeSet<u32> =
        ptr::scan_type_records(&mut rf, version, ptr::OST_MATERIALS, &declared)?
            .iter()
            .map(|record| record.element_id)
            .collect();
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
            });
        }
    }
    let holders: BTreeMap<u32, BTreeSet<u32>> = copies
        .iter()
        .filter(|(_, held)| held.len() > 1)
        .map(|(id, held)| (*id, held.iter().map(|copy| copy.partition).collect()))
        .collect();
    let current = native_document::current_partitions(&mut rf, &holders);
    let (mut agree, mut disagree, mut none_decoded) = (0, 0, 0);
    let mut lines: Vec<String> = Vec::new();
    for (id, held) in &copies {
        let mut ordered: Vec<&Copy> = held.iter().collect();
        ordered.sort_by_key(|copy| (copy.partition, copy.offset));
        let decoded: Vec<&&Copy> = ordered
            .iter()
            .filter(|copy| copy.layers.is_some())
            .collect();
        let first = decoded.first().and_then(|copy| copy.layers.as_ref());
        let same = decoded.iter().all(|copy| copy.layers.as_ref() == first);
        let verdict = if decoded.is_empty() {
            none_decoded += 1;
            "none"
        } else if !same {
            disagree += 1;
            "disagree"
        } else if ordered.len() == 1 {
            "single"
        } else {
            agree += 1;
            "agree"
        };
        lines.push(format!(
            "type {id} copies {} decoded {} verdict {verdict} current_p {}",
            ordered.len(),
            decoded.len(),
            current.get(id).map_or("-".to_string(), |p| p.to_string()),
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
    }
    println!(
        "summary ceiling_types {} copies_agree {agree} copies_disagree {disagree} none_decoded {none_decoded}",
        definitions.len(),
    );
    for line in lines {
        println!("{line}");
    }
    Ok(())
}
