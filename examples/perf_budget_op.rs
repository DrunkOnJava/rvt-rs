//! Single-operation runner for `tools/perf_budget.py`.
//!
//! The Python harness measures wall time and peak resident memory for this
//! process. Keeping one operation per process gives the budget gate a clean
//! memory boundary without adding platform-specific profiling code to the
//! library crate.

use std::{hint::black_box, path::PathBuf};

use anyhow::{Context, bail};
use rvt::{
    RevitFile, compression, formats,
    ifc::{
        RvtDocExporter,
        gltf::model_to_glb,
        scene_graph::{build_scene_graph, build_schedule},
        step_writer::write_step,
    },
    streams, walker,
};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let op = args
        .next()
        .context("usage: perf_budget_op <operation> <path>")?;
    let path = PathBuf::from(
        args.next()
            .context("usage: perf_budget_op <operation> <path>")?,
    );
    if args.next().is_some() {
        bail!("usage: perf_budget_op <operation> <path>");
    }

    let units = match op.as_str() {
        "open" => {
            let rf = RevitFile::open(&path)?;
            black_box(rf.stream_names().len())
        }
        "summarize" => {
            let mut rf = RevitFile::open(&path)?;
            let summary = rf.summarize_strict()?;
            black_box(summary.streams.len())
        }
        "schema_parse" => {
            let mut rf = RevitFile::open(&path)?;
            let raw = rf.read_stream(streams::FORMATS_LATEST)?;
            let (_, decomp) = compression::inflate_at_auto(&raw)?;
            let schema = formats::parse_schema(&decomp)?;
            black_box(schema.classes.len())
        }
        "element_decode" => {
            let mut rf = RevitFile::open(&path)?;
            let elements = walker::iter_elements(&mut rf)?;
            black_box(elements.count())
        }
        "ifc_export" => {
            let mut rf = RevitFile::open(&path)?;
            let export = RvtDocExporter.export_with_diagnostics(&mut rf)?;
            let step = write_step(&export.model);
            black_box(step.len())
        }
        "viewer_parse_render" => {
            let mut rf = RevitFile::open(&path)?;
            let export = RvtDocExporter.export_with_diagnostics(&mut rf)?;
            let scene = build_scene_graph(&export.model);
            let schedule = build_schedule(&export.model);
            let glb = model_to_glb(&export.model);
            black_box(scene.children.len() + glb.len() + schedule.rows.len())
        }
        other => bail!("unknown operation: {other}"),
    };

    let payload = serde_json::json!({
        "operation": op,
        "path": path,
        "units": units,
    });
    println!("{}", serde_json::to_string(&payload)?);
    Ok(())
}
