//! #1385 (RE-335): where the cut partners of a wall that embeds a curtain wall
//! stand, so that a byte signal can tell the hosts whose end stubs Revit cuts
//! away from the hosts that keep them.
//!
//! FACT under test: a wall that embeds a curtain wall spanning nearly all of it
//! (`WallEmbedWallGStep`, RE-335) has its end stubs, the stretches the curtain
//! wall does not span, cut away by the elements it has `CutPairGStep` partners
//! with where those stubs lie inside the partners' bodies (RE-335 section 3:
//! 2098938 and the columns 1927726 and 1927727), and keeps them where they do
//! not (2103576 has no partner; 2177158 has a partner and keeps its stub).
//!
//! `probe_1067_native_dump` prints each wall's native steps and relations, and
//! each partner's native record. This prints what it does not: the model-space
//! boxes of the element-record frames of the walls, columns, structural
//! columns and structural framing of the file, so that a partner's footprint
//! can be set against the host's line and the curtain wall's span. One line a
//! frame, feet:
//!
//!   frame <id> <category> <stream> <offset> <min x> <min y> <min z> <max x> <max y> <max z> <placement kind> <container> <class tag> <references>
//!
//! `<references>` is the frame's counted reference list, comma separated.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1385_cut_partners -- MODEL.rvt

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::partition_element_records as per;

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1385_cut_partners MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    let table = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&table);
    let categories = [
        per::OST_WALLS,
        per::OST_COLUMNS,
        per::OST_STRUCTURAL_COLUMNS,
        per::OST_STRUCTURAL_FRAMING,
    ];
    let records = per::scan_category_records_multi(&mut rf, release, &categories, &declared)?;
    println!("release {release} frames {}", records.len());
    for record in &records {
        let b = record.bbox_feet;
        let references: Vec<String> = record.references.iter().map(u64::to_string).collect();
        println!(
            "frame {} {} {} {} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {} {} {} {}",
            record.element_id,
            record.builtin_category,
            record.stream,
            record.offset,
            b[0],
            b[1],
            b[2],
            b[3],
            b[4],
            b[5],
            record.placement_kind,
            record.container,
            record.class_tag,
            references.join(",")
        );
    }
    Ok(())
}
