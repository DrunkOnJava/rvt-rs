//! #1713: do the walls and spaces of the BIM4LCA permit files that rvt-rs
//! leaves without a predefined type hold two copies of the parameter that
//! disagree, and which copy is the current one?
//!
//! FACT 1 to check: `scan_export_parameters` dropped a key an element sets to
//! two different values, in one partition or across partitions. On the three
//! permit files (housing concrete, office, timber office) Revit's export
//! writes `PARTITIONING`, `SOLIDWALL` and the like on 40 walls, and
//! `USERDEFINED` on 42 family-instance spaces of the timber office, where
//! rvt-rs read no value; the as-built files read all of them.
//!
//! FACT 2 to check: the current copy of such a key is the one
//! `native_document::current_partitions` finds among the partitions where the
//! element's data object appears at all, and choosing among only the
//! partitions that hold the key, or taking the latest of them, gives the same
//! answer on these files. An empty value is no entry (the scan reads none), so
//! a parameter cleared in the current partition would leave that partition
//! with the element's data object and no entry for the key: the probe counts
//! those, and the pairs whose answer depends on the rule.
//!
//! For each of the four IFC export parameters of a declared element the probe
//! counts the owners with entries in one partition, in several partitions
//! that agree, and in several that disagree (a partition with two different
//! values is a copy in doubt). For the disagreeing pairs it prints where the
//! routed partition lies and what each rule answers.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1713_export_parameter_copies -- FILE.rvt

use rvt::RevitFile;
use rvt::research::native_document::current_partitions;
use rvt::research::partition_ifc_export_overrides::{
    IFC_EXPORT_ELEMENT_AS, IFC_EXPORT_ELEMENT_TYPE_AS, IFC_EXPORT_PREDEFINEDTYPE,
    IFC_EXPORT_PREDEFINEDTYPE_TYPE,
};
use rvt::research::partition_room_parameters::DATA_OBJECT_HEADER;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const KEYS: [(i64, &str); 4] = [
    (IFC_EXPORT_ELEMENT_AS, "export as"),
    (IFC_EXPORT_ELEMENT_TYPE_AS, "type export as"),
    (IFC_EXPORT_PREDEFINEDTYPE, "predefined type"),
    (IFC_EXPORT_PREDEFINEDTYPE_TYPE, "type predefined type"),
];

/// The most disagreeing pairs printed whole, per key.
const SAMPLE: usize = 8;

/// What each partition holds of one key of one element: its value, or `None`
/// where the partition sets the key to two different values.
type Copies = BTreeMap<u32, Option<String>>;

/// The UTF-16 string at `at` (`u32` length, then the units), when every unit
/// is printable ASCII.
fn utf16_value(buf: &[u8], at: usize) -> Option<String> {
    let units = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=64).contains(&units) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + units * 2)?;
    bytes
        .chunks_exact(2)
        .map(|pair| {
            u8::try_from(u16::from_le_bytes([pair[0], pair[1]]))
                .ok()
                .filter(|byte| byte.is_ascii_graphic() || *byte == b' ')
                .map(char::from)
        })
        .collect()
}

fn partition_number(stream: &str) -> u32 {
    stream
        .rsplit('/')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

fn show(copies: &Copies) -> String {
    copies
        .iter()
        .map(|(partition, value)| match value {
            Some(value) => format!("{partition}:{value}"),
            None => format!("{partition}:(two values)"),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    println!("release {version}");

    // (owner, key) -> partition -> the values that partition's entries hold.
    let mut found: BTreeMap<(u32, i64), BTreeMap<u32, BTreeSet<String>>> = BTreeMap::new();
    // owner -> the partitions where its data object appears at all.
    let mut appears: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut undeclared = 0usize;
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let Ok(objects) = rf.partition_data_objects(&stream) else {
            continue;
        };
        let partition = partition_number(&stream);
        let buf = inflated.bytes();
        for (_, object) in objects.iter() {
            if declared.contains(&object.element_id) {
                appears
                    .entry(object.element_id)
                    .or_default()
                    .insert(partition);
            }
        }
        for (key, _) in KEYS {
            for hit in memchr::memmem::find_iter(buf, &key.to_le_bytes()) {
                let Some(value) = utf16_value(buf, hit + 8) else {
                    continue;
                };
                let Some(owner) = objects
                    .partition_point(|(p, _)| *p <= hit)
                    .checked_sub(1)
                    .map(|index| objects[index])
                    .filter(|(p, object)| hit >= p + DATA_OBJECT_HEADER && hit + 8 <= object.end)
                    .map(|(_, object)| object.element_id)
                else {
                    continue;
                };
                if !declared.contains(&owner) {
                    undeclared += 1;
                    continue;
                }
                found
                    .entry((owner, key))
                    .or_default()
                    .entry(partition)
                    .or_default()
                    .insert(value);
            }
        }
    }
    println!(
        "entries in a declared element's data object: {}",
        found
            .values()
            .flat_map(|by_partition| by_partition.values())
            .map(BTreeSet::len)
            .sum::<usize>()
    );
    println!("entries in an undeclared element's data object: {undeclared}");

    let copies: BTreeMap<(u32, i64), Copies> = found
        .into_iter()
        .map(|(pair, by_partition)| {
            let held: Copies = by_partition
                .into_iter()
                .map(|(partition, values)| {
                    let mut values = values.into_iter();
                    let first = values.next();
                    (
                        partition,
                        if values.next().is_none() { first } else { None },
                    )
                })
                .collect();
            (pair, held)
        })
        .collect();
    let disagree = |held: &Copies| {
        let mut values = held.values();
        let first = values.next();
        !values.all(|value| Some(value) == first)
    };

    // The pairs whose partitions disagree, and the partitions where each
    // owner's data object appears.
    let disagreeing: Vec<(u32, i64)> = copies
        .iter()
        .filter(|(_, held)| disagree(*held))
        .map(|(pair, _)| *pair)
        .collect();
    let owners: BTreeSet<u32> = disagreeing.iter().map(|&(owner, _)| owner).collect();
    let by_object_holders: BTreeMap<u32, BTreeSet<u32>> = owners
        .iter()
        .filter_map(|owner| Some((*owner, appears.get(owner)?.clone())))
        .collect();
    let by_object = current_partitions(&mut rf, &by_object_holders);
    println!(
        "owners with a disagreeing key: {}; routed by their data objects' partitions: {}",
        owners.len(),
        by_object.len()
    );

    for (key, label) in KEYS {
        let mine: Vec<(&u32, &Copies)> = copies
            .iter()
            .filter(|((_, k), _)| *k == key)
            .map(|((owner, _), held)| (owner, held))
            .collect();
        let one = mine.iter().filter(|(_, held)| held.len() == 1).count();
        let agree = mine
            .iter()
            .filter(|(_, held)| held.len() > 1 && !disagree(*held))
            .count();
        let pairs: Vec<u32> = disagreeing
            .iter()
            .filter(|(_, k)| *k == key)
            .map(|(owner, _)| *owner)
            .collect();
        println!(
            "{label}: {} owners; one partition {one}; several that agree {agree}; several that disagree {}",
            mine.len(),
            pairs.len()
        );
        if pairs.is_empty() {
            continue;
        }

        // The rule over only the partitions that hold the key, the rule the
        // first version of the fix used, and the latest partition.
        let by_key_holders: BTreeMap<u32, BTreeSet<u32>> = pairs
            .iter()
            .map(|owner| (*owner, copies[&(*owner, key)].keys().copied().collect()))
            .collect();
        let by_key = current_partitions(&mut rf, &by_key_holders);

        let (mut unrouted, mut holds, mut doubt, mut cleared) = (0usize, 0usize, 0usize, 0usize);
        let (mut key_differs, mut latest_differs, mut not_latest) = (0usize, 0usize, 0usize);
        let mut printed = 0usize;
        for owner in &pairs {
            let held = &copies[&(*owner, key)];
            let answer = |partition: Option<&u32>| {
                partition
                    .and_then(|partition| held.get(partition))
                    .cloned()
                    .flatten()
            };
            let object_answer = answer(by_object.get(owner));
            match by_object.get(owner).map(|partition| held.get(partition)) {
                None => unrouted += 1,
                Some(Some(Some(_))) => holds += 1,
                Some(Some(None)) => doubt += 1,
                Some(None) => cleared += 1,
            }
            let key_differs_here = answer(by_key.get(owner)) != object_answer;
            let latest = held.keys().next_back();
            let latest_differs_here = answer(latest) != object_answer;
            if by_object.get(owner).is_some() && by_object.get(owner) != latest {
                not_latest += 1;
            }
            key_differs += usize::from(key_differs_here);
            latest_differs += usize::from(latest_differs_here);
            if printed < SAMPLE || key_differs_here || latest_differs_here {
                printed += 1;
                println!(
                    "  {label} owner {owner}: routed {:?}; appears in {:?}; holds the key in {:?}; answer {:?}{}{}  [{}]",
                    by_object.get(owner),
                    appears
                        .get(owner)
                        .map(|set| set.iter().copied().collect::<Vec<_>>()),
                    held.keys().collect::<Vec<_>>(),
                    object_answer,
                    if key_differs_here {
                        " DIFFERS from the rule over the key's partitions"
                    } else {
                        ""
                    },
                    if latest_differs_here {
                        " DIFFERS from the latest partition"
                    } else {
                        ""
                    },
                    show(held)
                );
            }
        }
        println!(
            "  {label}: of {} disagreeing owners, not routed {unrouted}; the routed partition holds a value {holds}, holds the element's data with the key in doubt {doubt}, holds the element's data with no entry for the key {cleared}",
            pairs.len()
        );
        println!(
            "  {label}: the routed partition is not the latest that holds the key {not_latest}; the answer differs from the rule over the key's partitions {key_differs}, from the latest partition {latest_differs}"
        );
    }
    Ok(())
}
