//! #824: every object of the BIM4LCA office roof hatches' sketch planes,
//! of the hatch symbol and of roof 2991742, as untruncated JSON, to find
//! what links a face-hosted hatch to the roof whose face it is on.
//!
//! FACT under test: the hatches' `m_hostId` names a `SketchPlane` whose
//! `FacePointRef` lists roof 2991742's top-face corners, and RE-221's dump
//! (which kept only scalars, short arrays and bare ElementIds) showed no
//! element id there. This prints every field of every object, so a
//! reference held in a struct is not dropped.
//!
//! Output, per object: `json <id> <record class> <object index> <object class> <fields as JSON>`
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_824_sketch_plane_json -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

const IDS: [u64; 4] = [3079516, 3073092, 1107673, 2991742];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: IDS.iter().copied().collect::<BTreeSet<u64>>(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let class = record.class_name.as_deref().unwrap_or("-");
        let id = record.identity.element_id;
        let limit = if id == 2991742 { 12 } else { 400 };
        for (index, object) in graph.objects.iter().enumerate().take(limit) {
            println!(
                "json {id} {class} {index} {} {}",
                object.class_name, object.fields
            );
        }
        Ok(())
    })?;
    Ok(())
}
