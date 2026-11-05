//! #1430: the elements CSV lists no material for an `IfcSpace`, as the default
//! IFC export associates none.
//!
//! FACT under test: `rvt::ifc::schedule_csv::elements_csv` leaves the
//! `material` cell of every `IFCSPACE` row empty, where it used to list the
//! material the model holds for the space (a `Turning Free Space` family
//! instance, which Revit's export associates a material with and IFC4's
//! `IfcRelAssociatesMaterial.AllowedElements` does not allow). The rows of
//! every other class keep their material.
//!
//! The probe builds the model `rvt-schedule` builds (the document exporter
//! alone), counts the spaces the model holds a material for, and reads the
//! CSV `elements_csv` writes. Counts only: no name, id or coordinate is
//! printed.
//!
//! Output, one line per item:
//!   spaces <n> with-material-in-model <n>   the model's spaces, and those it holds a material for
//!   csv-space-rows <n> with-material <n>    the CSV's IFCSPACE rows, and those listing a material
//!   csv-other-rows <n> with-material <n>    the CSV's other rows, and those listing a material
//!   check <name> ok | FAIL <detail>         space-materials: no space row lists a material
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1430_space_materials -- MODEL.rvt

use anyhow::{Context, Result, bail};
use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::schedule_csv::{CsvOptions, LengthUnit, element_columns, elements_csv};
use rvt::ifc::{Exporter, RvtDocExporter};

fn records(csv: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = csv.trim_start_matches('\u{FEFF}').chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => record.push(std::mem::take(&mut cell)),
            '\r' if !quoted && chars.peek() == Some(&'\n') => {
                chars.next();
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            other => cell.push(other),
        }
    }
    records
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("usage: probe_1430_space_materials MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export(&mut rf)?;

    let mut spaces = 0usize;
    let mut spaces_with_material = 0usize;
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            material_index,
            material_layer_set_index,
            material_profile_set_index,
            ..
        } = entity
        else {
            continue;
        };
        if !ifc_type.eq_ignore_ascii_case("IFCSPACE") {
            continue;
        }
        spaces += 1;
        let in_constituent_set = model
            .material_constituent_sets
            .iter()
            .any(|set| set.elements.contains(&index));
        if material_index.is_some()
            || material_layer_set_index.is_some()
            || material_profile_set_index.is_some()
            || in_constituent_set
        {
            spaces_with_material += 1;
        }
    }
    println!("spaces {spaces} with-material-in-model {spaces_with_material}");

    let columns = element_columns(LengthUnit::Feet);
    let type_column = columns
        .iter()
        .position(|c| c == "ifc_type")
        .context("no ifc_type column")?;
    let material_column = columns
        .iter()
        .position(|c| c == "material")
        .context("no material column")?;
    let csv = elements_csv(&model, &CsvOptions::default());
    let rows = records(&csv);
    let (mut space_rows, mut space_listed) = (0usize, 0usize);
    let (mut other_rows, mut other_listed) = (0usize, 0usize);
    for row in rows.iter().skip(1) {
        let is_space = row
            .get(type_column)
            .is_some_and(|t| t.eq_ignore_ascii_case("IFCSPACE"));
        let listed = row.get(material_column).is_some_and(|m| !m.is_empty());
        if is_space {
            space_rows += 1;
            space_listed += usize::from(listed);
        } else {
            other_rows += 1;
            other_listed += usize::from(listed);
        }
    }
    println!("csv-space-rows {space_rows} with-material {space_listed}");
    println!("csv-other-rows {other_rows} with-material {other_listed}");

    if space_listed == 0 {
        println!("check space-materials ok");
        Ok(())
    } else {
        println!("check space-materials FAIL {space_listed} space rows list a material");
        bail!("a space row lists a material")
    }
}
