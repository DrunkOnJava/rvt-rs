//! RE-369: each element's Level parameters, one line per parameter,
//! `ElementId kind Level`, sorted. `kind` is `level` (what the export writes,
//! by class, as `Level`, `Base Constraint`, `Base Level` or a beam's, brace's,
//! duct's or pipe's `Reference Level`) or `top` (a wall's `Top Constraint`, a
//! stair's `Top Level`). On Revit 2015 to 2023 they
//! are the Levels the element's own native record names; on 2024 the ones
//! its association record or reference list names (RE-172, RE-174, RE-175).
//! Compare a 2015 to 2023 save with the same project's 2024 save:
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re369_levels -- \
//!     samples/2023-racbasicsampleproject.rvt > 2023.txt
//! cargo run --profile ci -p rvt-probes --example probe_re369_levels -- \
//!     samples/2024-racbasicsampleproject.rvt > 2024.txt
//! diff 2023.txt 2024.txt
//! ```
//!
//! A parameter one save gives and the other does not shows as a line on one
//! side only.

use rvt::RevitFile;
use rvt::research::partition_level_parameters::{LEVEL_PARAMETER_FIELD, TOP_LEVEL_PARAMETER_FIELD};
use rvt::research::partition_schema_mvp::{
    REFERENCE_LEVEL_FIELD, REFERENCE_LEVEL_FROM_LIST_FIELD, recover_partition_schema_mvp,
};
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::BTreeSet;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re369_levels MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open the project");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())
        .expect("partition records");
    let mut lines = BTreeSet::new();
    for element in mvp.into_elements() {
        let Some(id) = element.id else {
            continue;
        };
        let text = |wanted: &str| {
            element.fields.iter().find_map(|(name, value)| match value {
                InstanceField::String(v) if name == wanted => Some(v.clone()),
                _ => None,
            })
        };
        // The export takes a beam's or brace's Reference Level from the
        // level field where it is set, and from the reference fields where not.
        if let Some(level) = text(LEVEL_PARAMETER_FIELD)
            .or_else(|| text(REFERENCE_LEVEL_FROM_LIST_FIELD))
            .or_else(|| text(REFERENCE_LEVEL_FIELD))
        {
            lines.insert((id, "level", level));
        }
        if let Some(top) = text(TOP_LEVEL_PARAMETER_FIELD) {
            lines.insert((id, "top", top));
        }
    }
    for (id, kind, level) in lines {
        println!("{id} {kind} {level}");
    }
}
