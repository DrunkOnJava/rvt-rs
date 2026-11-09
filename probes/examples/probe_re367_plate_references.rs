//! RE-367 (#1520): what each curtain panel's reference list names, for a Revit
//! 2015 to 2023 save, one line per panel: its ElementId, then each id its list
//! names as `id`, `R` where the id is itself an element record and `-` where it
//! is not, and `:<name>` where a name entry holds one.
//!
//! The 67 plates of Vitoria's `ALD26 Before` get one type from the rule that
//! takes the one named id that is no record, where Revit's export gives them
//! five (Measure run 42534783950). This lists what their lists hold.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re367_plate_references -- \
//!     samples/2023-racadvancedsampleproject.rvt
//! ```

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::elem_table::{declared_ids, parse_records};
use rvt::research::partition_element_records::OST_CURTAIN_WALL_PANELS;
use rvt::research::partition_element_records_2023::{is_32bit_release, scan_records};
use rvt::research::partition_names::name_entries_2023;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or("usage: probe_re367_plate_references <file.rvt>")?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    println!("release layout {release}");
    if !is_32bit_release(release) {
        println!("not a 32-bit release: nothing to list");
        return Ok(());
    }
    let records = scan_records(&mut rf, release);
    let table = parse_records(&mut rf)?;
    let declared = declared_ids(&table);
    let names = name_entries_2023(&mut rf, &declared);
    let by_id: BTreeMap<u32, &_> = records.iter().map(|r| (r.element_id, r)).collect();
    let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|r| r.builtin_category == OST_CURTAIN_WALL_PANELS)
    {
        let mut line = format!("panel {}", record.element_id);
        let mut shape = Vec::new();
        for &reference in &record.references {
            let Ok(id) = u32::try_from(reference) else {
                line.push_str(&format!(" big({reference})"));
                continue;
            };
            let has_record = by_id.contains_key(&id);
            let name = names.get(&id);
            line.push_str(&format!(
                " {id}:{}{}",
                if has_record { "R" } else { "-" },
                name.map(|n| format!(":{n}")).unwrap_or_default()
            ));
            if id != record.element_id {
                shape.push(format!(
                    "{}{}",
                    if has_record { "R" } else { "-" },
                    if name.is_some() { "n" } else { "_" }
                ));
            }
        }
        println!("{line}");
        *shapes.entry(shape.join(",")).or_default() += 1;
    }
    for (shape, count) in shapes {
        println!("shape {count} panels: {shape}");
    }
    Ok(())
}
