//! Probe: does a building element's partition element record name the
//! Revit `Level` that hosts it, in its counted reference list at
//! `+0x88`?
//!
//! Research probe for #219 / RE-27 — storey containment for the
//! elements the record base/top elevation join leaves unbound.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re27_level_reference -- <file.rvt>

use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records as per;
use rvt::partition_level_records;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;

const TOL: f64 = 1e-3;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(
        env::args()
            .nth(1)
            .expect("usage: probe_re27_level_reference <file.rvt>"),
    );
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();

    let levels = partition_level_records::scan_partition_levels(&mut rf, version, &declared)?;
    let level_by_id: BTreeMap<u32, f64> = levels
        .iter()
        .map(|l| (l.element_id, l.elevation_feet))
        .collect();
    let elevations: Vec<f64> = levels.iter().map(|l| l.elevation_feet).collect();
    println!("levels={}", levels.len());

    let categories = [
        ("OST_Floors", per::OST_FLOORS),
        ("OST_BuildingPad", per::OST_BUILDING_PAD),
        ("OST_Columns", per::OST_COLUMNS),
        ("OST_Walls", per::OST_WALLS),
        ("OST_Doors", per::OST_DOORS),
        ("OST_Windows", per::OST_WINDOWS),
    ];
    let cats: Vec<i64> = categories.iter().map(|(_, c)| *c).collect();
    let records = per::scan_category_records_multi(&mut rf, version, &cats, &declared)?;

    let plate = |cat: i64| cat == per::OST_FLOORS || cat == per::OST_BUILDING_PAD;
    let near = |a: f64, b: f64| (a - b).abs() < TOL;
    let is_storey = |v: f64| elevations.iter().any(|e| near(*e, v));

    println!(
        "\n{:<16} {:>5} {:>7} {:>7} {:>7} {:>8} {:>8} {:>8} {:>8}",
        "category", "inst", "no-ref", "one", "many", "elev-ok", "lvl=elev", "lvl!=elev", "conflict"
    );
    for (name, cat) in categories {
        let rows: Vec<&per::PartitionElementRecord> = records
            .iter()
            .filter(|r| r.builtin_category == cat && r.is_placed_instance())
            .collect();
        let (mut none, mut one, mut many) = (0usize, 0usize, 0usize);
        // Of the single-level-reference records: does the existing
        // elevation join already answer, and does the named level
        // agree with it?
        let (mut elev_ok, mut same, mut diff, mut conflict) = (0usize, 0usize, 0usize, 0usize);
        let mut offsets: BTreeMap<i64, usize> = BTreeMap::new();
        for r in &rows {
            let ids: BTreeSet<u32> = r
                .references
                .iter()
                .filter_map(|v| u32::try_from(*v).ok())
                .filter(|v| level_by_id.contains_key(v))
                .collect();
            match ids.len() {
                0 => {
                    none += 1;
                    continue;
                }
                1 => one += 1,
                _ => {
                    many += 1;
                    continue;
                }
            }
            let level_elev = level_by_id[ids.iter().next().unwrap()];
            let key = if plate(cat) {
                r.bbox_feet[5]
            } else {
                r.bbox_feet[2]
            };
            let joined = is_storey(key);
            if joined {
                elev_ok += 1;
                if near(key, level_elev) {
                    same += 1;
                } else {
                    conflict += 1;
                }
            } else if near(key, level_elev) {
                same += 1;
            } else {
                diff += 1;
            }
            let delta = ((key - level_elev) * 10_000.0).round() as i64;
            *offsets.entry(delta).or_default() += 1;
        }
        println!(
            "{name:<16} {:>5} {:>7} {:>7} {:>7} {:>8} {:>8} {:>8} {:>8}",
            rows.len(),
            none,
            one,
            many,
            elev_ok,
            same,
            diff,
            conflict
        );
        let mut shown: Vec<_> = offsets.into_iter().collect();
        shown.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        shown.truncate(6);
        let pretty: Vec<String> = shown
            .iter()
            .map(|(d, n)| format!("{:+.4}ft x{n}", *d as f64 / 10_000.0))
            .collect();
        println!("    key-minus-named-level offsets: {}", pretty.join(", "));
    }

    println!(
        "\nLegend: `one` = the reference list names exactly one recovered Level.\n\
         `elev-ok` = the existing elevation join already binds it.\n\
         `lvl=elev` = named level equals the elevation-join answer.\n\
         `conflict` = elevation join binds it to a DIFFERENT storey than the named level."
    );
    Ok(())
}
