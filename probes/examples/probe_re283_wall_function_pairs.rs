//! RE-283 (probe): a wall's type Function on Revit 2015 to 2023, against the
//! same project saved by a later release (#1158).
//!
//! FACT under test: `native_wall_types::wall_type_functions`, admitted to
//! Revit 2015 to 2022 (RE-283), reads the type Function (`m_function`) of
//! the walls whose records name a type, and a wall both files declare gets
//! the Function its copy gives it, wherever both read one.
//!
//! One file prints the totals. Two files (the older first, as Measure's
//! paired probe runs them) compare the walls both declare:
//!
//!   native readers: wall Function ... count lines (walls read, by value;
//!     shared walls read in both, equal, different, read in one only);
//!   native readers example: ... a different pair, by ElementId (dropped
//!     from the counts a model without a licence uploads).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re283_wall_function_pairs -- OLDER.rvt [NEWER.rvt]

use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::native_wall_types::wall_type_functions;
use std::collections::{BTreeMap, BTreeSet};

struct Side {
    release: u32,
    declared: BTreeSet<u64>,
    functions: BTreeMap<u64, i64>,
}

fn side(path: &str) -> anyhow::Result<Side> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    let declared: BTreeSet<u64> = elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .map(u64::from)
        .collect();
    let functions = wall_type_functions(&mut rf, &declared)?;
    Ok(Side {
        release,
        declared,
        functions,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let result = match paths.as_slice() {
        [one] => single(one),
        [older, newer] => pair(older, newer),
        _ => {
            eprintln!("usage: probe_re283_wall_function_pairs OLDER.rvt [NEWER.rvt]");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        println!("native readers: wall Function failed {error:#}");
    }
}

fn values(functions: &BTreeMap<u64, i64>) -> String {
    let mut by_value: BTreeMap<i64, usize> = BTreeMap::new();
    for value in functions.values() {
        *by_value.entry(*value).or_default() += 1;
    }
    by_value
        .iter()
        .map(|(value, n)| format!("{value} {n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe(side: &Side) {
    println!(
        "native readers: wall Function release {}: declared elements {}, walls with a Function {}, by value ({})",
        side.release,
        side.declared.len(),
        side.functions.len(),
        values(&side.functions)
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
        match (a.functions.get(id), b.functions.get(id)) {
            (Some(x), Some(y)) => {
                both += 1;
                if x == y {
                    equal += 1;
                } else {
                    different += 1;
                    if shown < 10 {
                        shown += 1;
                        println!("native readers example: wall {id} Function older {x} newer {y}");
                    }
                }
            }
            (Some(_), None) => only_older += 1,
            (None, Some(_)) => only_newer += 1,
            (None, None) => {}
        }
    }
    println!(
        "native readers: wall Function pair: older {} newer {}: shared declared elements {}, walls read in both {both}, equal {equal}, different {different}, read in the older only {only_older}, in the newer only {only_newer}",
        a.release,
        b.release,
        shared.len()
    );
    Ok(())
}
