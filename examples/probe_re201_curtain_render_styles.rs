//! RE-201: a mullion or curtain panel type's material is its
//! `m_renderStyleId`.
//!
//! FACT: every mullion type (`SysMullionFamSym`) and panel type
//! (`SysPanelFamSym`) holds its material's ElementId in `m_renderStyleId`.
//! RE-166 read it at a fixed +135 (+139 for a panel) in the type's data
//! object; an optional field before it moves it to +133 on 10 of Snowdon
//! Towers' 13 exported mullion types, and those types' 1,065 mullions got no
//! material or `<Unnamed>`. Read by name, it is Revit's material on all 1,773
//! members and plates of Snowdon Towers and on RE1 Architecture's.
//!
//! The probe prints one JSON line per mullion and panel type read from its
//! native record: ElementId, class, name (empty on a per-instance copy),
//! master type, `m_renderStyleId` and that material's name. A last line
//! counts them. Verify against Revit's export with
//! `tools/re/element_materials_vs_ifc.py`, by the types the `IfcMember` and
//! `IfcPlate` elements name.
//!
//! Usage:
//!   cargo run --release --example probe_re201_curtain_render_styles -- MODEL.rvt

use rvt::{RevitFile, research::native_document};
use serde_json::json;
use std::collections::BTreeSet;

fn id(value: Option<&serde_json::Value>) -> Option<i64> {
    value
        .and_then(|v| v.pointer("/m_id/m_id64"))
        .and_then(serde_json::Value::as_i64)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re201_curtain_render_styles MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&records);
    let names =
        rvt::research::partition_materials::scan_material_names(&mut rf, version, &declared)?;
    let options = native_document::Options {
        selected_classes: BTreeSet::from(["SysMullionFamSym".into(), "SysPanelFamSym".into()]),
        ..native_document::Options::default()
    };
    let (mut types, mut named) = (0usize, 0usize);
    native_document::extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let name = graph
            .objects
            .iter()
            .find(|o| o.class_name == "SymbolInfo")
            .and_then(|o| o.fields.get("m_name")?.as_str())
            .unwrap_or("");
        let style = id(root.fields.get("m_renderStyleId"));
        let material = style
            .and_then(|s| u32::try_from(s).ok())
            .and_then(|s| names.get(&s));
        types += 1;
        named += usize::from(material.is_some());
        println!(
            "{}",
            json!({
                "id": record.identity.element_id,
                "class": root.class_name,
                "name": name,
                "master": id(root.fields.get("m_masterId")),
                "render_style": style,
                "material": material,
            })
        );
        Ok(())
    })?;
    println!("{}", json!({"types": types, "with_material": named}));
    Ok(())
}
