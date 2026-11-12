//! #1824: a Revit 2015 to 2023 export reads its walls' top joins once.
//!
//! FACT under test: `native_walls_and_top_joins_into` reads each wall's whole
//! `SWall` or `ArcWall` record and finds whether its top is joined
//! (`m_baseOrTop`). `WallReads::top_joins` reads the same member from a graph
//! of the same record built with only the fields its facts take. Both give the
//! same value on every wall. So the first read leaves the top joins in
//! `WallReads`, and the export's later readers of them make no pass of their
//! own. The walls with no type in the export, which that read draws no body
//! for, are read in it for their facts alone (`facts_only`): only a wall whose
//! record it could not read is left to a pass of the later readers.
//!
//! The probe:
//! 1. Makes the export `rvt-ifc` makes with no flags, and counts the passes
//!    `WallReads` made (`native_walls::wall_read_passes`).
//! 2. Reads the ids of every `SWall` and `ArcWall` record. On fresh handles it
//!    then reads their top joins both ways: the whole-record read
//!    (`native_walls_and_top_joins`) and the projected one
//!    (`WallReads::top_joins`).
//!
//! Output:
//!   export wall_read_passes=<n> facts_only=<n>
//!   top_joins walls=<n> whole=<n> projected=<n> joined=<n> equal=<bool>
//!   differ <id> whole=<value> projected=<value>   (the first 20, if any)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1824_wall_top_joins_once -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_walls::{
    WallReads, native_walls_and_top_joins, wall_facts_only_reads, wall_read_passes,
};

fn open(path: &str) -> anyhow::Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, false),
    )?)
}

fn main() -> anyhow::Result<()> {
    // Measure passes the samples `--records` too.
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_1824_wall_top_joins_once MODEL.rvt");

    let before = wall_read_passes();
    let facts_before = wall_facts_only_reads();
    rvt::ifc::run_export_with_limits(
        &mut open(&path)?,
        &rvt::ifc::ExportOptions::default(),
        rvt::walker::WalkerLimits::default(),
    )?;
    println!(
        "export wall_read_passes={} facts_only={}",
        wall_read_passes() - before,
        wall_facts_only_reads() - facts_before
    );

    let options = Options {
        selected_classes: ["SWall", "ArcWall"]
            .iter()
            .map(|class| (*class).to_string())
            .collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut walls: BTreeSet<u64> = BTreeSet::new();
    if let Err(error) = extract_graphs(&mut open(&path)?, &options, |record| {
        if matches!(record.class_name.as_deref(), Some("SWall" | "ArcWall")) {
            walls.insert(record.identity.element_id);
        }
        Ok(())
    }) {
        println!("walls read failed: {error:#}");
        return Ok(());
    }
    let whole = native_walls_and_top_joins(&mut open(&path)?, &walls)?.top_joined;
    let projected = WallReads::default().top_joins(&mut open(&path)?, &walls)?;
    let ids: BTreeSet<u64> = whole.keys().chain(projected.keys()).copied().collect();
    let differ: Vec<u64> = ids
        .into_iter()
        .filter(|id| whole.get(id) != projected.get(id))
        .collect();
    println!(
        "top_joins walls={} whole={} projected={} joined={} equal={}",
        walls.len(),
        whole.len(),
        projected.len(),
        whole.values().filter(|joined| **joined).count(),
        differ.is_empty()
    );
    for id in differ.iter().take(20) {
        println!(
            "differ {id} whole={:?} projected={:?}",
            whole.get(id),
            projected.get(id)
        );
    }
    Ok(())
}
