//! RE-284 (probe): whether a wall's top is joined to a roof or floor on Revit 2015 to 2023, against the same project
//! saved by a later release (#1158).
//!
//! FACT under test: `native_walls::wall_top_joins`, admitted to Revit 2015 to 2022
//! (RE-284), reads whether the top of a wall is joined to a roof or floor (a `JoinToRoofGStep` with `m_baseOrTop` 1) of the elements whose records it reads, and an
//! element both files declare gets the value its copy gives it, wherever
//! both read one.
//!
//! One file prints the totals. Two files (the older first, as Measure's
//! paired probe runs them) compare the elements both declare:
//!
//!   native readers: wall top join ... count lines (elements read, by value;
//!     shared elements read in both, equal, different, read in one only);
//!   native readers example: ... a different pair, by ElementId (dropped
//!     from the counts a model without a licence uploads).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re284_wall_top_join_pairs -- OLDER.rvt [NEWER.rvt]

use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::native_walls::wall_top_joins;
use std::collections::{BTreeMap, BTreeSet};

struct Side {
    release: u32,
    declared: BTreeSet<u64>,
    values: BTreeMap<u64, String>,
}

fn side(path: &str) -> anyhow::Result<Side> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    let declared: BTreeSet<u64> = elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .map(u64::from)
        .collect();
    let values = read(&mut rf, &declared)?;
    Ok(Side {
        release,
        declared,
        values,
    })
}

fn read(rf: &mut RevitFile, ids: &BTreeSet<u64>) -> anyhow::Result<BTreeMap<u64, String>> {
    Ok(wall_top_joins(rf, ids)?
        .into_iter()
        .map(|(id, joined)| (id, joined.to_string()))
        .collect())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let result = match paths.as_slice() {
        [one] => single(one),
        [older, newer] => pair(older, newer),
        _ => {
            eprintln!("usage: probe_re284_wall_top_join_pairs OLDER.rvt [NEWER.rvt]");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        println!("native readers: wall top join failed {error:#}");
    }
}

fn by_value(values: &BTreeMap<u64, String>) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for value in values.values() {
        *counts.entry(value.as_str()).or_default() += 1;
    }
    counts
        .iter()
        .map(|(value, n)| format!("{value} {n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe(side: &Side) {
    println!(
        "native readers: wall top join release {}: declared elements {}, walls read {}, by value ({})",
        side.release,
        side.declared.len(),
        side.values.len(),
        by_value(&side.values)
    );
}

fn single(path: &str) -> anyhow::Result<()> {
    describe(&side(path)?);
    Ok(())
}

fn pair(older: &str, newer: &str) -> anyhow::Result<()> {
    let (a, b) = (side(older)?, side(newer)?);
    describe(&a);
    describe(&b);
    let shared: BTreeSet<u64> = a.declared.intersection(&b.declared).copied().collect();
    let (mut both, mut equal, mut different, mut only_older, mut only_newer) = (0, 0, 0, 0, 0);
    let mut shown = 0;
    for id in &shared {
        match (a.values.get(id), b.values.get(id)) {
            (Some(x), Some(y)) => {
                both += 1;
                if x == y {
                    equal += 1;
                } else {
                    different += 1;
                    if shown < 10 {
                        shown += 1;
                        println!("native readers example: walls {id} older {x} newer {y}");
                    }
                }
            }
            (Some(_), None) => only_older += 1,
            (None, Some(_)) => only_newer += 1,
            (None, None) => {}
        }
    }
    println!(
        "native readers: wall top join pair: older {} newer {}: shared declared elements {}, walls read in both {both}, equal {equal}, different {different}, read in the older only {only_older}, in the newer only {only_newer}",
        a.release,
        b.release,
        shared.len()
    );
    Ok(())
}
