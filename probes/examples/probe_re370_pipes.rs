//! RE-370: each pipe's type and the cylinder it is drawn as.
//!
//! By default, through `recover_partition_schema_mvp`, one line per pipe,
//! `ElementId type <name> length <ft> radius <ft>`, sorted, with `-` for what
//! is not given: the type and cylinder the export writes, from RE-131's
//! connector entries where they place the pipe (2024, 2025) and from the
//! pipe's own `RbsPipeCurve` record elsewhere. Compare one save with the same
//! project's 2024 save:
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re370_pipes -- \
//!     samples/2019-rme_basic_sample_project.rvt > 2019.txt
//! cargo run --profile ci -p rvt-probes --example probe_re370_pipes -- \
//!     samples/2024-rmebasicsampleproject.rvt > 2024.txt
//! diff 2019.txt 2024.txt
//! ```
//!
//! Lengths and radii are printed to 0.001 ft: an outside radius of 0.03125 ft
//! that RE-131 fits to a record box as 0.031251 would differ at 0.0001.
//!
//! With `--native`, every pipe's record alone, whatever else places it: one
//! line per pipe, `ElementId start x,y,z end x,y,z outer <ft>`, model feet,
//! the ends of its one `GLine` and its `m_dOuterDiameter`, for
//! `tools/re/pipes_vs_ifc.py` to hold against Revit's own export:
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re370_pipes -- \
//!     --native RE1-Plumbing.rvt > native.txt
//! python3 tools/re/pipes_vs_ifc.py native.txt RE1-Plumbing.ifc
//! ```

use rvt::RevitFile;
use rvt::research::native_document as nd;
use rvt::research::partition_schema_mvp::{
    TYPE_NAME_FIELD, pipe_body_from_fields, recover_partition_schema_mvp,
};
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::Value;
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (native, path) = match args.as_slice() {
        [flag, path] if flag == "--native" => (true, path),
        [path] => (false, path),
        _ => panic!("usage: probe_re370_pipes [--native] MODEL.rvt"),
    };
    let mut rf = RevitFile::open(path).expect("open the project");
    if native {
        print_native(&mut rf);
    } else {
        print_exported(&mut rf);
    }
}

fn print_exported(rf: &mut RevitFile) {
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let mvp = recover_partition_schema_mvp(rf, version, WalkerLimits::default())
        .expect("partition records");
    let feet = |value: Option<f64>| value.map_or_else(|| "-".to_string(), |v| format!("{v:.3}"));
    let mut lines = BTreeMap::new();
    for element in mvp.into_elements() {
        let (Some(id), "Pipe") = (element.id, element.class.as_str()) else {
            continue;
        };
        let type_name = element
            .fields
            .iter()
            .find_map(|(name, value)| match value {
                InstanceField::String(v) if name == TYPE_NAME_FIELD => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "-".into());
        let body = pipe_body_from_fields(&element.fields);
        let length = body.as_ref().map(|pipe| {
            (0..3)
                .map(|axis| (pipe.end[axis] - pipe.start[axis]).powi(2))
                .sum::<f64>()
                .sqrt()
        });
        lines.insert(
            id,
            format!(
                "type {type_name} length {} radius {}",
                feet(length),
                feet(body.map(|pipe| pipe.radius_feet)),
            ),
        );
    }
    for (id, line) in lines {
        println!("{id} {line}");
    }
}

fn print_native(rf: &mut RevitFile) {
    let options = nd::Options {
        selected_classes: ["RbsPipeCurve".to_string()].into(),
        releases: nd::NATIVE_RELEASES,
        ..nd::Options::default()
    };
    let numbers = |value: Option<&Value>| -> Option<Vec<f64>> {
        value?.as_array()?.iter().map(Value::as_f64).collect()
    };
    let mut lines = BTreeMap::new();
    nd::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let objects_of = |class: &str| {
            graph
                .objects
                .iter()
                .filter(|object| object.class_name == class)
                .collect::<Vec<_>>()
        };
        let (glines, designs) = (
            objects_of("GLine"),
            objects_of("PipeDomainDesignPropertyManager"),
        );
        let ([line], [design]) = (glines.as_slice(), designs.as_slice()) else {
            return Ok(());
        };
        let (Some(origin), Some(direction), Some(params), Some(outer)) = (
            numbers(line.fields.get("m_origin")),
            numbers(line.fields.get("m_dirVec")),
            numbers(line.fields.get("m_endParams")),
            design
                .fields
                .get("m_dOuterDiameter")
                .and_then(Value::as_f64),
        ) else {
            return Ok(());
        };
        if origin.len() != 3 || direction.len() != 3 || params.len() != 2 {
            return Ok(());
        }
        let point = |t: f64| {
            (0..3)
                .map(|axis| format!("{:.6}", origin[axis] + direction[axis] * t))
                .collect::<Vec<_>>()
                .join(",")
        };
        lines.insert(
            record.identity.element_id,
            format!(
                "start {} end {} outer {outer:.6}",
                point(params[0]),
                point(params[1])
            ),
        );
        Ok(())
    })
    .expect("native records");
    for (id, line) in lines {
        println!("{id} {line}");
    }
}
