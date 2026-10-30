//! #357: what a stair run's native records hold where its treads are not
//! straight (`StairsTreadInfo.m_runType` other than 0), for drawing a spiral
//! run as Revit draws it.
//!
//! FACT under test: a spiral run's `StairsRun` graph names its centre,
//! radius and swept angle, or its treads' origins and directions trace one
//! circle, so the run's treads can be laid out on it.
//!
//! Output, per `StairsRun` record with a tread whose `m_runType` is not 0:
//!   run <id> <root fields as JSON>
//!   object <id> <index> <class> <fields as JSON>
//! and per `StairsRunType`:
//!   type <id> <root fields as JSON>
//!
//! `tools/re/spiral_runs_vs_ifc.py` prints Revit's flight for each run.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_357_spiral_runs -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: ["StairsRun".to_string(), "StairsRunType".to_string()].into(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match class {
            "StairsRunType" => println!("type {id} {}", root.fields),
            "StairsRun" => {
                let spiral = graph.objects.iter().any(|o| {
                    o.class_name == "StairsTreadInfo"
                        && o.fields.get("m_runType").and_then(|v| v.as_i64()) != Some(0)
                });
                if !spiral {
                    return Ok(());
                }
                println!("run {id} {}", root.fields);
                for (index, object) in graph.objects.iter().enumerate().skip(1) {
                    println!(
                        "object {id} {index} {} {}",
                        object.class_name, object.fields
                    );
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(())
}
