//! RE-86: a wall type stores its face angles, and a tapered wall carries 2
//! in its orientation word.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, every
//! "Basic Wall" type's data holds `u32 2`, eight zero bytes, an `f64` 0.7
//! and a count of 3 (`WALL_FACE_ANGLES_FRAME`), after a word that varies by
//! document, then three `f64` angles in radians. All three are 0 on 40 of
//! the 43 wall types Revit's IFC4 export holds. The three "Solar Wall" types
//! store 10 degrees in the middle one. Every wall whose word after its
//! location-line setting (RE-54) is 2 is of one of those types, and each of
//! the 21 Revit exports is tapered: its exterior face leans 10.000 degrees
//! from vertical, wider at the base, and its interior face is vertical.
//! The type's thickness is the body's at the wall's top, centred on its
//! line. Core Interior, RE1 and the MIT house store no such frame.
//!
//! The probe prints one JSON object per type that stores the frame (`type`,
//! `angles`) and one per wall with an orientation (`wall`, `word`, `flip`,
//! `location_line`). `tools/re/tapered_walls_vs_ifc.py` joins them to
//! Revit's export.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re86_tapered_walls -- MODEL.rvt > walls.jsonl

use rvt::RevitFile;
use rvt::partition_compound_structure as pcs;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let walls: BTreeSet<u32> =
        per::scan_category_records(&mut rf, version, per::OST_WALLS, &declared)?
            .into_iter()
            .map(|record| record.element_id)
            .collect();
    // Every declared element is a candidate type: the frame is looked for
    // in each one's data.
    let angles: BTreeMap<u32, [f64; 3]> =
        pcs::scan_wall_type_face_angles(&mut rf, version, &declared)?;
    for (id, [first, exterior, third]) in &angles {
        println!("{{\"type\":{id},\"angles\":[{first},{exterior},{third}]}}");
    }
    let orientations = pcs::scan_wall_orientations(&mut rf, version, &walls)?;
    for (id, o) in &orientations {
        println!(
            "{{\"wall\":{id},\"word\":{},\"flip\":{},\"location_line\":{}}}",
            o.word, o.flip, o.location_line
        );
    }
    let tapered = angles.values().filter(|a| a[1] != 0.0).count();
    let word2 = orientations.values().filter(|o| o.word == 2).count();
    eprintln!(
        "Revit {version}: {} types store face angles, {tapered} of them non-zero; {word2} of {} walls carry word 2",
        angles.len(),
        orientations.len()
    );
    Ok(())
}
