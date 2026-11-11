//! RE-371: each duct's type, section and line, and each MEP connector's
//! direction and system.
//!
//! By default, through `recover_partition_schema_mvp`, one line per duct,
//! `ElementId type <name> length <ft> section <w>x<h> start x,y,z end x,y,z`,
//! sorted, with `-` for what is not given: what the export draws. Compare one
//! save with the same project's 2024 save:
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re371_ducts -- \
//!     samples/2019-rme_basic_sample_project.rvt > 2019.txt
//! cargo run --profile ci -p rvt-probes --example probe_re371_ducts -- \
//!     samples/2024-rmebasicsampleproject.rvt > 2024.txt
//! diff 2019.txt 2024.txt
//! ```
//!
//! With `--native`, every duct's and pipe's record alone, whatever else
//! places it, for `tools/re/ducts_vs_ifc.py` to hold against Revit's own
//! export:
//!
//! - `duct <id> type <type id> <type name> start x,y,z end x,y,z width <ft>
//!   height <ft> normal x,y,z level <id> offsets <start>,<end> justify <h>,<v>`:
//!   the root of its `RbsDuctCurve` record (the level its `m_assocLevelId`,
//!   or before 2024 its `m_idLevelStart`), the ends of its one `GLine` and
//!   its type's `SymbolInfo.m_name`, model feet;
//! - `port <element id> <connector index> <class> direction <d> system <s>
//!   flow <ft³/s> at <t> system_id <id>`: each connector calculation of
//!   every duct, pipe and family instance, `m_eDirection` (1 In, 2 Out, 0
//!   bidirectional, RE-212), `m_eSystemType`, `m_dFlow` and, on a duct or
//!   pipe, the parameter along its line its `SegmentConnectorPosition` puts
//!   the connector at (0 its start, 1 its end, between them a tap).
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re371_ducts -- \
//!     --native RE1-Mechanical.rvt > native.txt
//! python3 tools/re/ducts_vs_ifc.py native.txt RE1-Mechanical.ifc
//! ```

use rvt::RevitFile;
use rvt::research::native_document as nd;
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_parameters::pointer_token;
use rvt::research::partition_schema_mvp::{
    TYPE_NAME_FIELD, curve_size_from_fields, duct_ends_from_fields, recover_partition_schema_mvp,
};
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::Value;
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (native, path) = match args.as_slice() {
        [flag, path] if flag == "--native" => (true, path),
        [path] => (false, path),
        _ => panic!("usage: probe_re371_ducts [--native] MODEL.rvt"),
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
    let point = |p: [f64; 3]| format!("{:.3},{:.3},{:.3}", p[0], p[1], p[2]);
    let mut lines = BTreeMap::new();
    for element in mvp.into_elements() {
        let (Some(id), "Duct") = (element.id, element.class.as_str()) else {
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
        let ends = duct_ends_from_fields(&element.fields);
        let length = ends.map(|[start, end]| {
            (0..3)
                .map(|axis| (end[axis] - start[axis]).powi(2))
                .sum::<f64>()
                .sqrt()
        });
        let section = curve_size_from_fields(&element.fields).map_or_else(
            || "-".to_string(),
            |(width, height)| format!("{width:.3}x{height:.3}"),
        );
        let (start, end) = ends.map_or_else(
            || ("-".to_string(), "-".to_string()),
            |[start, end]| (point(start), point(end)),
        );
        lines.insert(
            id,
            format!(
                "type {type_name} length {} section {section} start {start} end {end}",
                feet(length)
            ),
        );
    }
    for (id, line) in lines {
        println!("{id} {line}");
    }
}

/// The one object of `class` in `graph`.
fn only<'a>(graph: &'a ObjectGraph, class: &str) -> Option<&'a Value> {
    let mut found = graph
        .objects
        .iter()
        .filter(|object| object.class_name == class);
    match (found.next(), found.next()) {
        (Some(object), None) => Some(&object.fields),
        _ => None,
    }
}

/// An `Identifier`: 8 bytes from Revit 2024 (`m_id64`), 4 before (`m_id`).
fn id_of(value: Option<&Value>) -> Option<i64> {
    let identifier = value?.get("m_id")?;
    identifier
        .get("m_id64")
        .or_else(|| identifier.get("m_id"))
        .and_then(Value::as_i64)
}

fn numbers(value: Option<&Value>) -> Option<Vec<f64>> {
    value?.as_array()?.iter().map(Value::as_f64).collect()
}

fn print_native(rf: &mut RevitFile) {
    let options = nd::Options {
        selected_classes: [
            "RbsDuctCurve",
            "RbsPipeCurve",
            "RbsFlexDuctCurve",
            "RbsFlexPipeCurve",
            "FamilyInstance",
            "AbsDuctType",
        ]
        .map(str::to_string)
        .into(),
        releases: nd::NATIVE_RELEASES,
        ..nd::Options::default()
    };
    let mut type_names: BTreeMap<u64, String> = BTreeMap::new();
    let mut ducts: BTreeMap<u64, (i64, String)> = BTreeMap::new();
    let mut ports = BTreeMap::new();
    nd::extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if class == "AbsDuctType" {
            if let Some(name) = only(graph, "SymbolInfo")
                .and_then(|info| info.get("m_name"))
                .and_then(Value::as_str)
            {
                type_names.insert(id, name.to_string());
            }
            return Ok(());
        }
        // Each connector calculation, by its connector's index.
        let index_of_token: BTreeMap<u64, u64> = graph
            .objects
            .iter()
            .filter(|object| object.class_name == "Connector")
            .filter_map(|connector| {
                Some((
                    u64::from(connector.token),
                    connector.fields.get("m_nIndex")?.as_u64()?,
                ))
            })
            .collect();
        // Where each connector sits along a duct's or pipe's line.
        let positions: BTreeMap<u64, f64> = graph
            .objects
            .iter()
            .filter(|object| object.class_name == "SegmentConnectorPosition")
            .filter_map(|position| {
                let token = pointer_token(position.fields.get("m_pConnector")?)?;
                Some((
                    *index_of_token.get(&token)?,
                    position.fields.get("m_paramOnCurve")?.as_f64()?,
                ))
            })
            .collect();
        for calculation in graph.objects.iter().filter(|object| {
            object.class_name == "SegmentConnectorCalculation"
                || object
                    .class_name
                    .starts_with("MEPFamilyConnectorCalculation")
        }) {
            let Some(index) = calculation
                .fields
                .get("m_pConnector")
                .and_then(pointer_token)
                .and_then(|token| index_of_token.get(&token))
            else {
                continue;
            };
            let field = |name: &str| {
                calculation
                    .fields
                    .get(name)
                    .and_then(Value::as_i64)
                    .map_or_else(|| "-".to_string(), |v| v.to_string())
            };
            ports.insert(
                (id, *index, calculation.class_name.clone()),
                format!(
                    "direction {} system {} flow {} at {} system_id {}",
                    field("m_eDirection"),
                    field("m_eSystemType"),
                    calculation
                        .fields
                        .get("m_dFlow")
                        .and_then(Value::as_f64)
                        .map_or_else(|| "-".to_string(), |v| format!("{v:.6}")),
                    positions
                        .get(index)
                        .map_or_else(|| "-".to_string(), |v| format!("{v:.6}")),
                    id_of(calculation.fields.get("m_idSystem"))
                        .map_or_else(|| "-".to_string(), |v| v.to_string()),
                ),
            );
        }
        if class != "RbsDuctCurve" {
            return Ok(());
        }
        let Some(root) = only(graph, "RbsDuctCurve") else {
            return Ok(());
        };
        let line = only(graph, "GLine").and_then(|line| {
            let origin = numbers(line.get("m_origin"))?;
            let direction = numbers(line.get("m_dirVec"))?;
            let params = numbers(line.get("m_endParams"))?;
            let ([o0, o1, o2], [d0, d1, d2], [t0, t1]) =
                (origin.as_slice(), direction.as_slice(), params.as_slice())
            else {
                return None;
            };
            let point =
                |t: f64| format!("{:.6},{:.6},{:.6}", o0 + d0 * t, o1 + d1 * t, o2 + d2 * t);
            Some(format!("start {} end {}", point(*t0), point(*t1)))
        });
        let float = |name: &str| {
            root.get(name)
                .and_then(Value::as_f64)
                .map_or_else(|| "-".to_string(), |v| format!("{v:.6}"))
        };
        let int = |name: &str| {
            root.get(name)
                .and_then(Value::as_i64)
                .map_or_else(|| "-".to_string(), |v| v.to_string())
        };
        let normal = numbers(root.get("m_vNormal"))
            .filter(|n| n.len() == 3)
            .map_or_else(
                || "-".to_string(),
                |n| format!("{:.6},{:.6},{:.6}", n[0], n[1], n[2]),
            );
        let type_id = id_of(root.get("m_idType")).unwrap_or(-1);
        ducts.insert(
            id,
            (
                type_id,
                format!(
                    "{} width {} height {} normal {normal} level {} offsets {},{} justify {},{}",
                    line.unwrap_or_else(|| "start - end -".to_string()),
                    float("m_dWidthOrDiameter"),
                    float("m_dHeight"),
                    id_of(root.get("m_assocLevelId"))
                        .filter(|level| *level >= 0)
                        .or_else(|| id_of(root.get("m_idLevelStart")))
                        .unwrap_or(-1),
                    float("m_dOffsetStart"),
                    float("m_dOffsetEnd"),
                    int("m_eHorOffset"),
                    int("m_eVertOffset"),
                ),
            ),
        );
        Ok(())
    })
    .expect("native records");
    for (id, (type_id, line)) in ducts {
        let name = u64::try_from(type_id)
            .ok()
            .and_then(|type_id| type_names.get(&type_id))
            .map_or("-", String::as_str);
        // A type name may hold spaces; `|` stands for each.
        println!("duct {id} type {type_id} {} {line}", name.replace(' ', "|"));
    }
    for ((id, index, class), line) in ports {
        println!("port {id} {index} {class} {line}");
    }
}
