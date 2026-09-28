//! RE-129 (probe): where a pipe's or duct's type is named.
//!
//! Revit's export names a pipe `Pipe Types:<type>:<id>` and a duct
//! `Rectangular Duct:<type>:<id>`, but rvt-rs finds no type for them: the
//! type rule (`partition_names::resolve_type`) needs exactly one id in the
//! record's reference list with a name entry of the element's own category.
//!
//! FACT (negative, RE1 Mechanical and Plumbing): nothing read so far links
//! a pipe or duct to its type. The probe prints:
//! - every name entry of the pipe and duct categories (none on RE1);
//! - each pipe and duct record's reference list, each id with its name
//!   entry (none of them has one);
//! - where the type names Revit exports ("221116-WTR", "233113-DUCT-Tees")
//!   are stored, and the element whose data holds each;
//! - the declared ids named in at least 90% of the pipes' (or ducts') own
//!   data, and whether each one's data holds a type name (none does).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re129_mep_curve_types -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let names = rf.element_names();
    let categories = [per::OST_PIPE_CURVES, per::OST_DUCT_CURVES];
    println!("Revit {version}: {} name entries", names.entries.len());
    for entry in names.entries.values() {
        if categories.contains(&entry.builtin_category) {
            println!(
                "entry {} {:?} category {}",
                entry.element_id, entry.name, entry.builtin_category
            );
        }
    }
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    for record in &records {
        let Ok(inflated) = rf.inflated_partition(&record.stream) else {
            continue;
        };
        let references = per::decode_reference_list(
            inflated.bytes(),
            record.offset + per::REFERENCE_LIST_OFFSET,
        )
        .unwrap_or_default();
        println!(
            "record {} category {} class {} references {}",
            record.element_id,
            record.builtin_category,
            record.class_tag,
            references.len()
        );
        for id in references {
            let named = u32::try_from(id)
                .ok()
                .and_then(|id| names.entries.get(&id))
                .map(|e| format!("{:?} category {}", e.name, e.builtin_category))
                .unwrap_or_default();
            println!("    {id} {named}");
        }
    }
    // Where the type names Revit's export gives RE1's pipes and ducts are
    // stored: each UTF-16 occurrence, with the declared ids within 64
    // bytes before it.
    for needle in ["221116-WTR", "233113-DUCT-Tees"] {
        let units: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for at in memchr::memmem::find_iter(buf, &units).take(8) {
                let from = at.saturating_sub(64);
                let hex: Vec<String> = buf[from..at].iter().map(|b| format!("{b:02x}")).collect();
                let ids: Vec<String> = (from..at.saturating_sub(7))
                    .filter_map(|i| {
                        let v = u64::from_le_bytes(buf[i..i + 8].try_into().ok()?);
                        let id = u32::try_from(v).ok()?;
                        declared.contains(&id).then(|| format!("{id}@-{}", at - i))
                    })
                    .collect();
                println!("name {needle:?} {stream} @{at}: ids {ids:?}");
                println!("    {}", hex.join(" "));
            }
        }
    }
    // Which element's data holds each name: the nearest element-data
    // header (`element_data_header` + ElementId) before it.
    if let Some(layout) = rvt::partition_names::element_data_layout(version) {
        for needle in ["221116-WTR", "233113-DUCT-Tees"] {
            let units: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
            for stream in rf.partition_stream_names() {
                let Ok(inflated) = rf.inflated_partition(&stream) else {
                    continue;
                };
                let buf = inflated.bytes();
                for at in memchr::memmem::find_iter(buf, &units).take(8) {
                    let owner = memchr::memmem::rfind(&buf[..at], &layout.header).and_then(|hit| {
                        let id = layout.id_at(buf, hit + layout.header.len())?;
                        Some((id, at - hit))
                    });
                    let after: Vec<String> = buf
                        [at + units.len()..(at + units.len() + 48).min(buf.len())]
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    println!(
                        "owner {needle:?} @{at}: element {owner:?}; after: {}",
                        after.join(" ")
                    );
                }
            }
        }
    }
    // Declared ids in every pipe's (and every duct's) own element data, and
    // whether each such id's own data holds a type name.
    if let Some(layout) = rvt::partition_names::element_data_layout(version) {
        let category_of: std::collections::BTreeMap<u32, i64> = records
            .iter()
            .map(|record| (record.element_id, record.builtin_category))
            .collect();
        let mut named: std::collections::BTreeMap<i64, std::collections::BTreeMap<u32, usize>> =
            Default::default();
        let mut counted: std::collections::BTreeMap<i64, usize> = Default::default();
        let mut data_of: std::collections::BTreeMap<u32, Vec<u8>> = Default::default();
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            let starts: Vec<usize> = memchr::memmem::find_iter(buf, &layout.header).collect();
            for (index, &hit) in starts.iter().enumerate() {
                let id_at = hit + layout.header.len();
                let Some(id) = layout.id_at(buf, id_at) else {
                    continue;
                };
                let end = starts
                    .get(index + 1)
                    .copied()
                    .unwrap_or(buf.len())
                    .min(hit + 0x1_0000);
                let data = &buf[id_at + 8..end];
                data_of.entry(id).or_insert_with(|| data.to_vec());
                let Some(category) = category_of.get(&id) else {
                    continue;
                };
                *counted.entry(*category).or_default() += 1;
                let found: std::collections::BTreeSet<u32> = (0..data.len().saturating_sub(8))
                    .filter_map(|at| {
                        let v = u64::from_le_bytes(data[at..at + 8].try_into().ok()?);
                        u32::try_from(v).ok().filter(|id| declared.contains(id))
                    })
                    .collect();
                for other in found {
                    *named
                        .entry(*category)
                        .or_default()
                        .entry(other)
                        .or_default() += 1;
                }
            }
        }
        for (category, ids) in &named {
            let total = counted[category];
            for (id, n) in ids {
                if *n * 10 < total * 9 {
                    continue;
                }
                let holds: Vec<&str> = ["221116-WTR", "233113-DUCT-Tees"]
                    .into_iter()
                    .filter(|needle| {
                        let units: Vec<u8> =
                            needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
                        data_of
                            .get(id)
                            .is_some_and(|d| memchr::memmem::find(d, &units).is_some())
                    })
                    .collect();
                let name = names.entries.get(id).map(|e| e.name.clone());
                println!(
                    "common {category} {id} in {n} of {total}; name entry {name:?}; data holds {holds:?}"
                );
            }
        }
    }
    Ok(())
}
