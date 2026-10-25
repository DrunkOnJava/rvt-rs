//! W6: where a wall type's native record keeps its layers' core boundaries.
//!
//! FACT under test: at a T joint between layered walls on the BIM4LCA office,
//! Revit's stem stops one outer layer inside the host's face (0.0426 ft into
//! a type 460609 host, 0.0853 ft into a type 3347182 host, RE-222 §3), as if
//! it stopped at the host's core boundary. The partition data gives a type's
//! layers' widths, materials and functions, not its core (RE-53). This
//! prints every object of the native records of three wall types, 460609
//! (0.0427 / 0.2428 / 0.0427 ft), 3347182 (0.0853 / 0.2165 / 0.0853) and
//! 1495207 (US-1, five layers), so the core boundary can be found.
//!
//! Output:
//!   native <id> <object index> <class>/<object class> <member>=<json> ...
//!
//! Usage:
//!   cargo run --profile ci --example probe_wall_type_core -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

const TYPES: &[u64] = &[460609, 3347182, 1495207];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: TYPES.iter().copied().collect::<BTreeSet<u64>>(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate() {
            let members: Vec<String> = object
                .fields
                .as_object()
                .into_iter()
                .flatten()
                .map(|(name, value)| {
                    let text: String = value.to_string().chars().take(900).collect();
                    format!("{name}={text}")
                })
                .collect();
            println!(
                "native {id} {index} {class}/{} {}",
                object.class_name,
                members.join(" ")
            );
        }
        Ok(())
    })?;
    Ok(())
}
