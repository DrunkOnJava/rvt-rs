//! #1987: which records the Tags of three wall-cut openings name, on Snowdon
//! Towers Architectural.
//!
//! FACT to find: wall 698364 holds three openings that Revit's export writes
//! with the Tags 1993278, 1993305 and 1993365, and that rvt-rs writes in the
//! same places (each box equal to 0.001 ft) with the Tags 1993286, 1993355 and
//! 1993375, the cutters the wall's cut list names. The records of every id from
//! 1993270 to 1993380 are printed, with their class and their objects' fields,
//! to see what each Tag is and how the two are linked. So are the objects of
//! wall 698364 whose class names a cut, an opening or an embed.
//!
//! Output:
//!   record <id> <class> object <n> <object class> <fields json, clipped>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1987_cut_ids -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

const CLIP: usize = 3000;
const WALL: u64 = 698364;

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1987_cut_ids MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let mut ids: BTreeSet<u64> = (1_993_270..=1_993_380).collect();
    ids.insert(WALL);
    let options = Options {
        selected_ids: ids,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate() {
            let name = object.class_name.as_str();
            if id == WALL
                && !(name.contains("Cut") || name.contains("Opening") || name.contains("Embed"))
            {
                continue;
            }
            let text: String = object
                .fields
                .to_json()
                .to_string()
                .chars()
                .take(CLIP)
                .collect();
            println!("record {id} {class} object {index} {name} {text}");
        }
        Ok(())
    })?;
    Ok(())
}
