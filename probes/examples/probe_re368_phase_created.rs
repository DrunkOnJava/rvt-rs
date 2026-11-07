//! RE-368: each element's Phase Created, one line per element with a phase,
//! `ElementId phase`, sorted by ElementId. On Revit 2015 to 2023 the
//! phase is the one the element's own `m_createdPhaseId` names; on 2024 the
//! one its reference list names (#755). Compare a 2015 to 2023 save with the
//! same project's 2024 save:
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re368_phase_created -- \
//!     samples/2023-racbasicsampleproject.rvt > 2023.txt
//! cargo run --profile ci -p rvt-probes --example probe_re368_phase_created -- \
//!     samples/2024-racbasicsampleproject.rvt > 2024.txt
//! diff 2023.txt 2024.txt
//! ```
//!
//! An element one save holds and the other does not shows as a line on one
//! side only.

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::{PHASE_CREATED_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::BTreeMap;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re368_phase_created MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open the project");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())
        .expect("partition records");
    let mut phases = BTreeMap::new();
    for element in mvp.into_elements() {
        let (Some(id), Some(phase)) = (
            element.id,
            element.fields.iter().find_map(|(name, value)| match value {
                InstanceField::String(phase) if name == PHASE_CREATED_FIELD => Some(phase.clone()),
                _ => None,
            }),
        ) else {
            continue;
        };
        phases.insert(id, phase);
    }
    for (id, phase) in phases {
        println!("{id} {phase}");
    }
}
