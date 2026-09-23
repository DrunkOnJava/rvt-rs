//! RE-42: a family that nests others names them in its partition record.
//!
//! FACT: when a family type's partition record names more than one
//! same-category family (RE-38's family candidates, #324), exactly one of
//! them is the type's own family, and that family's own partition record
//! names every other candidate: the families nested in it.
//! `partition_names::resolve_family` picks it. On RE1 Electrical (MIT) the
//! 12 lighting fixtures it names carry exactly the names Revit's own IFC
//! export gives them. Where the type's `Global/ElemTable` owner (RE-31) is
//! one of the candidates, it is the family the rule picks in every case.
//!
//! The probe prints, over every named type with family candidates, how the
//! ElemTable owner relates to them, how the nesting rule agrees with the
//! owner on the types with several candidates, and those types.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re42_nested_families -- FILE.rvt

use rvt::RevitFile;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let names = rf.element_names();
    let records = rvt::elem_table::parse_records(&mut rf)?;
    // Owner by ElementId, under either id the record declares (RE-41).
    let mut owner: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for record in &records {
        owner.insert(record.id_primary, record.owner_id);
        if record.id_secondary != 0 {
            owner.entry(record.id_secondary).or_insert(record.owner_id);
        }
    }
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut ambiguous = Vec::new();
    for (type_id, candidates) in &names.type_family_candidates {
        if candidates.is_empty() {
            continue;
        }
        let owner_id = owner.get(type_id).copied().flatten();
        let key = match (candidates.len(), owner_id) {
            (1, None) => "one candidate, owner unset",
            (1, Some(o)) if candidates.contains(&o) => "one candidate, owner is it",
            (1, Some(_)) => "one candidate, owner is another id",
            (_, None) => "several candidates, owner unset",
            (_, Some(o)) if candidates.contains(&o) => "several candidates, owner is one",
            (_, Some(_)) => "several candidates, owner is another id",
        };
        *counts.entry(key.to_string()).or_insert(0) += 1;
        if candidates.len() > 1 {
            ambiguous.push((*type_id, candidates.clone(), owner_id));
        }
    }
    println!(
        "release {version}: {} named types with family candidates",
        counts.values().sum::<usize>()
    );
    for (key, count) in &counts {
        println!("  {key:<40} {count:>5}");
    }
    let name = |id: &u32| {
        names
            .entries
            .get(id)
            .map(|e| e.name.clone())
            .unwrap_or_default()
    };
    let mut agreement: BTreeMap<&str, usize> = BTreeMap::new();
    for (type_id, _, owner_id) in &ambiguous {
        let host = rvt::partition_names::resolve_family(&names, *type_id);
        let key = match (host, owner_id) {
            (None, _) => "nesting rule: none",
            (Some(_), None) => "nesting rule: a family, owner unset",
            (Some(h), Some(o)) if h == *o => "nesting rule: the owner",
            (Some(_), Some(_)) => "nesting rule: not the owner",
        };
        *agreement.entry(key).or_insert(0) += 1;
    }
    for (key, count) in &agreement {
        println!("  {key:<40} {count:>5}");
    }
    for (type_id, candidates, owner_id) in ambiguous {
        let listed: BTreeSet<String> = candidates
            .iter()
            .map(|c| format!("{c} {:?}", name(c)))
            .collect();
        let host = rvt::partition_names::resolve_family(&names, type_id);
        println!(
            "  type {type_id} {:?}: candidates {listed:?}, owner {owner_id:?} {:?}, nesting rule {host:?} {:?}",
            name(&type_id),
            owner_id.map(|o| name(&o)).unwrap_or_default(),
            host.map(|h| name(&h)).unwrap_or_default()
        );
    }
    Ok(())
}
