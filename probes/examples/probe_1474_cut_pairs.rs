//! #1474: the cut pairs of the walls at 2056705's end, on the BIM4LCA office
//! as built.
//!
//! FACT to find: Revit's export ends wall 2056705 at 179.298 ft over the strip
//! beside wall 1493966's face, the bottom of wall 3490093, which stands there
//! up to 183.071 ft. Neither wall's join lists name the other (3490093's are
//! empty, 2056705's name only 2056741). The hypothesis is that the two are
//! joined by geometry, which a wall saves as a `CutPairGStep`. Every object of
//! these walls' records whose class names a cut, a join or a pair is printed
//! with its fields.
//!
//! Output:
//!   record <id> <class> object <n> <object class> <fields json, clipped>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1474_cut_pairs -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

const CLIP: usize = 4000;
const WALLS: [u64; 5] = [2056705, 2056770, 3490093, 1493966, 2056741];

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1474_cut_pairs MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: WALLS.into_iter().collect::<BTreeSet<u64>>(),
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
            if !(name.contains("Cut") || name.contains("Join") || name.contains("Pair")) {
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
