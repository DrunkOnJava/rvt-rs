//! RE-122 probe: a Revit 2023 beam's reference list names the column each
//! of its ends lies in.
//!
//! FACT: on modelo_bim every beam end lies in the record box of a column the
//! beam names, and only one: its foundation beams name the columns below
//! their Level, not the ones standing on it whose boxes touch at the same
//! point. Revit's export stops each beam at the named column's near face.
//!
//! Prints every structural framing and column record of a 2023 file with
//! its box and reference list, and, for each beam end, the columns whose
//! plan box holds that end.
//!
//! ```bash
//! cargo run --profile ci --example probe_re122_beam_column_refs -- modelo_bim.rvt
//! ```

use rvt::RevitFile;
use rvt::partition_element_records::{OST_STRUCTURAL_COLUMNS, OST_STRUCTURAL_FRAMING};
use rvt::partition_element_records_2023::{REVIT_2023, scan_records};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: probe_re122_beam_column_refs <file.rvt>")?;
    let mut rf = RevitFile::open(&path)?;
    let records = scan_records(&mut rf, REVIT_2023);
    let columns: Vec<_> = records
        .iter()
        .filter(|r| r.builtin_category == OST_STRUCTURAL_COLUMNS && r.placement_kind != 0)
        .collect();
    for r in records.iter().filter(|r| {
        matches!(
            r.builtin_category,
            OST_STRUCTURAL_FRAMING | OST_STRUCTURAL_COLUMNS
        )
    }) {
        println!(
            "{} cat {} kind {} container {:#x} box {:?} refs {:?}",
            r.element_id,
            r.builtin_category,
            r.placement_kind,
            r.container,
            r.bbox_feet,
            r.references
        );
        if r.builtin_category != OST_STRUCTURAL_FRAMING {
            continue;
        }
        let b = r.bbox_feet;
        let axis = if b[3] - b[0] > b[4] - b[1] { 0 } else { 1 };
        let mid = [(b[0] + b[3]) / 2.0, (b[1] + b[4]) / 2.0];
        for end in [b[axis], b[axis + 3]] {
            let mut point = mid;
            point[axis] = end;
            let holding: Vec<u32> = columns
                .iter()
                .filter(|c| {
                    let cb = c.bbox_feet;
                    cb[0] <= point[0]
                        && point[0] <= cb[3]
                        && cb[1] <= point[1]
                        && point[1] <= cb[4]
                        && cb[2] < b[5]
                        && b[2] < cb[5] + 1e-6
                })
                .map(|c| c.element_id)
                .collect();
            let named: Vec<u32> = holding
                .iter()
                .copied()
                .filter(|id| r.references.contains(&u64::from(*id)))
                .collect();
            println!("  end {end:.4}: columns holding it {holding:?}, named in refs {named:?}");
        }
    }
    Ok(())
}
