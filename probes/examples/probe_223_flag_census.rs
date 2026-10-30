//! #223: the element record's flag word at `+0x46`, per record, for a census
//! of its bits against what Revit's IFC export writes for the element.
//!
//! FACT under test: a bit of `+0x46` is a property Revit's export writes,
//! such as a wall's or slab's `LoadBearing` (Revit's Structural parameter),
//! exactly on every element of its category.
//!
//! Output, per first-prologue element record (one with its own ElementId at
//! `+0x00`) of the categories below:
//!   rec <id> <BuiltInCategory> <+0x46 as hex> <+0x42 placement kind>
//!
//! `tools/re/flag_bits_vs_ifc.py` pairs each with Revit's element of the
//! same Tag.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_223_flag_census -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records as per;

const FLAG_OFFSET: usize = 0x46;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = [
        per::OST_WALLS,
        per::OST_FLOORS,
        per::OST_ROOFS,
        per::OST_CEILINGS,
        per::OST_COLUMNS,
        per::OST_STRUCTURAL_COLUMNS,
        per::OST_STRUCTURAL_FRAMING,
        per::OST_STRUCTURAL_FOUNDATION,
        per::OST_DOORS,
        per::OST_WINDOWS,
    ];
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let mut printed = 0usize;
    for record in records.iter().filter(|r| !r.id_from_enclosing_record) {
        let stream = rf.inflated_partition(&record.stream)?;
        let Some(word) = stream
            .bytes()
            .get(record.offset + FLAG_OFFSET..record.offset + FLAG_OFFSET + 4)
        else {
            continue;
        };
        let flags = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        println!(
            "rec {} {} {:#010x} {}",
            record.element_id, record.builtin_category, flags, record.placement_kind
        );
        printed += 1;
    }
    eprintln!("{printed} records of {} scanned", records.len());
    Ok(())
}
