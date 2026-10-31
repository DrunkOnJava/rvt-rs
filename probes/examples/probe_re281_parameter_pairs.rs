//! RE-281 (probe): an element's saved parameters on Revit 2015 to 2022,
//! by name and value, against the same project saved by a later release
//! (#1158).
//!
//! FACT under test: `native_element_parameters::read`, admitted to Revit
//! 2015 to 2022 (RE-281), gives an element the parameters, named by the
//! file's own definitions, and the values that the same project's copy
//! saved by Revit 2023 gives it: a parameter both files hold has the same
//! name, kind, storage type and value, and the ones one file holds and the
//! other does not are those of elements or parameters Revit added or
//! removed between the saves.
//!
//! One file prints the totals. Two files (the older first, as Measure's
//! paired probe runs them) compare, over the ElementIds both declare, every
//! parameter of each element by (level, kind, name or built-in id, storage
//! type):
//!
//!   parameters pair: ... count lines (elements, parameters equal, different,
//!     only in one);
//!   different <name or id> <storage type> xN, only older ..., only newer ...
//!     the most frequent causes, by name.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re281_parameter_pairs -- OLDER.rvt [NEWER.rvt]

use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::native_element_parameters::{ElementParameter, read};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type Key = (String, String, String, String);

struct Side {
    release: u32,
    declared: BTreeSet<u64>,
    parameters: BTreeMap<u64, Vec<ElementParameter>>,
    unresolved_slots: usize,
}

fn side(path: &str) -> anyhow::Result<Side> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    let declared: BTreeSet<u64> = elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .map(u64::from)
        .collect();
    let saved = read(&mut rf, &declared)?;
    Ok(Side {
        release,
        declared,
        unresolved_slots: saved.unresolved_family_slots,
        parameters: saved.elements,
    })
}

fn key(parameter: &ElementParameter) -> Key {
    let name = match &parameter.name {
        Some(name) => name.clone(),
        None => format!("#{}", parameter.parameter_id),
    };
    (
        format!("{:?}", parameter.level),
        format!("{:?}", parameter.kind),
        name,
        parameter.storage_type.clone(),
    )
}

fn keyed(parameters: &[ElementParameter]) -> BTreeMap<Key, &Value> {
    let mut out = BTreeMap::new();
    for parameter in parameters {
        out.entry(key(parameter)).or_insert(&parameter.value);
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let result = match paths.as_slice() {
        [one] => single(one),
        [older, newer] => pair(older, newer),
        _ => {
            eprintln!("usage: probe_re281_parameter_pairs OLDER.rvt [NEWER.rvt]");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        println!("parameters: failed {error:#}");
    }
}

fn single(path: &str) -> anyhow::Result<()> {
    let side = side(path)?;
    let values: usize = side.parameters.values().map(Vec::len).sum();
    let named = side
        .parameters
        .values()
        .flatten()
        .filter(|parameter| parameter.name.is_some())
        .count();
    println!(
        "parameters: release {}: declared elements {}, with parameters {}, parameters {values}, of them named {named}, unresolved family slots {}",
        side.release,
        side.declared.len(),
        side.parameters.len(),
        side.unresolved_slots
    );
    Ok(())
}

fn pair(older: &str, newer: &str) -> anyhow::Result<()> {
    let (a, b) = (side(older)?, side(newer)?);
    single(older)?;
    single(newer)?;
    let shared: BTreeSet<u64> = a.declared.intersection(&b.declared).copied().collect();
    let (mut with_both, mut identical, mut only_older_el, mut only_newer_el) = (0, 0, 0, 0);
    let (mut equal, mut different, mut only_older, mut only_newer) = (0usize, 0, 0, 0);
    let (mut named_equal, mut named_different, mut named_only) = (0usize, 0, 0);
    let mut causes: BTreeMap<String, usize> = BTreeMap::new();
    let mut examples: BTreeMap<String, String> = BTreeMap::new();
    for id in &shared {
        let (x, y) = (a.parameters.get(id), b.parameters.get(id));
        match (x, y) {
            (Some(x), Some(y)) => {
                with_both += 1;
                let (kx, ky) = (keyed(x), keyed(y));
                let mut same = true;
                for (k, v) in &kx {
                    let named = !k.2.starts_with('#');
                    match ky.get(k) {
                        Some(w) if w == v => {
                            equal += 1;
                            named_equal += usize::from(named);
                        }
                        Some(w) => {
                            same = false;
                            different += 1;
                            named_different += usize::from(named);
                            let cause = format!("different {} {}", k.2, k.3);
                            examples
                                .entry(cause.clone())
                                .or_insert_with(|| format!("id {id} older {v} newer {w}"));
                            *causes.entry(cause).or_default() += 1;
                        }
                        None => {
                            same = false;
                            only_older += 1;
                            named_only += usize::from(named);
                            *causes
                                .entry(format!("only older {} {}", k.2, k.3))
                                .or_default() += 1;
                        }
                    }
                }
                for k in ky.keys() {
                    if !kx.contains_key(k) {
                        same = false;
                        only_newer += 1;
                        named_only += usize::from(!k.2.starts_with('#'));
                        *causes
                            .entry(format!("only newer {} {}", k.2, k.3))
                            .or_default() += 1;
                    }
                }
                identical += usize::from(same);
            }
            (Some(_), None) => only_older_el += 1,
            (None, Some(_)) => only_newer_el += 1,
            (None, None) => {}
        }
    }
    println!(
        "parameters pair: older {} newer {}: shared declared elements {}, with parameters in both {with_both}, with parameters in the older only {only_older_el}, in the newer only {only_newer_el}, lists identical {identical}",
        a.release,
        b.release,
        shared.len()
    );
    println!(
        "parameters pair: parameters equal {equal}, different {different}, only in the older {only_older}, only in the newer {only_newer}; named equal {named_equal}, named different {named_different}, named in one only {named_only}"
    );
    let mut rows: Vec<_> = causes.iter().collect();
    rows.sort_by(|x, y| y.1.cmp(x.1).then_with(|| x.0.cmp(y.0)));
    for (cause, n) in rows.into_iter().take(25) {
        let example = examples.get(cause).map(String::as_str).unwrap_or("");
        println!("parameters pair: {cause} x{n} {example}");
    }
    Ok(())
}
