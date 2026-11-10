//! RE-395: whose value block a family type table's Fire Rating and
//! Manufacturer entries lie in, and whether the copies that disagree are one
//! type's copies.
//!
//! FACT under test: RE-354 reads a `Fire Rating` or `Manufacturer` entry in a
//! family type table's form as the value of the type whose value block holds
//! it, and leaves out a type whose entries give different values, with a
//! warning. On Core Interior (Revit 2024) the export warned of one such
//! parameter, while Revit's own export of the file has no fire rating. This
//! lists every owner of a table entry with its element class (from its
//! record's class alone), what each partition's entries agree on, and
//! whether the copies across partitions agree.
//!
//! Output:
//!   entry <stream> <owner> <class> <parameter> <text | (disagree)>
//!   owner <owner> <class> <parameter> <agree <text> | disagree>
//!   classes <class>=<owners> ... disagreeing <class>=<owners> ...
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re395_family_type_table_owners -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::partition_type_parameters::scan_partition;

/// What a partition's table entries agree on, by owner and parameter.
type Table = BTreeMap<(u32, &'static str), Option<String>>;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re395_family_type_table_owners MODEL.rvt"))?;
    let mut rf = RevitFile::open(&path)?;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&records);
    let mut per_stream: Vec<(String, Table)> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let found = scan_partition(inflated.bytes(), &declared);
        if !found.table_form.is_empty() {
            per_stream.push((stream, found.table_form));
        }
    }
    let owners: BTreeSet<u64> = per_stream
        .iter()
        .flat_map(|(_, table)| table.keys().map(|(owner, _)| u64::from(*owner)))
        .collect();
    let mut class_of: BTreeMap<u32, String> = BTreeMap::new();
    if !owners.is_empty() {
        let options = Options {
            selected_ids: owners,
            graph_classes: Some(BTreeSet::new()),
            releases: NATIVE_RELEASES,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            if let (Some(class), Ok(id)) = (
                record.class_name.clone(),
                u32::try_from(record.identity.element_id),
            ) {
                class_of.insert(id, class);
            }
            Ok(())
        })?;
    }
    let class = |owner: u32| class_of.get(&owner).map_or("?", String::as_str);
    let mut merged: Table = BTreeMap::new();
    for (stream, table) in &per_stream {
        for ((owner, name), value) in table {
            println!(
                "entry {stream} {owner} {} {name} {}",
                class(*owner),
                value.as_deref().unwrap_or("(disagree)")
            );
            merged
                .entry((*owner, *name))
                .and_modify(|seen| {
                    if seen != value {
                        *seen = None;
                    }
                })
                .or_insert_with(|| value.clone());
        }
    }
    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
    let mut disagreeing: BTreeMap<&str, usize> = BTreeMap::new();
    for ((owner, name), value) in &merged {
        let c = class(*owner);
        *classes.entry(c).or_default() += 1;
        match value {
            Some(text) => println!("owner {owner} {c} {name} agree {text}"),
            None => {
                *disagreeing.entry(c).or_default() += 1;
                println!("owner {owner} {c} {name} disagree");
            }
        }
    }
    let list = |m: &BTreeMap<&str, usize>| {
        m.iter()
            .map(|(c, n)| format!("{c}={n}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    println!(
        "classes {} disagreeing {}",
        list(&classes),
        list(&disagreeing)
    );
    Ok(())
}
