//! RE-45: Revit's IFC export overrides are parameter entries in an
//! element's serialised data.
//!
//! FACT: an element's "Export to IFC As" and "IFC Predefined Type", and a
//! type's "Export Type to IFC As" and "Type IFC Predefined Type", are entries
//! `i64 BuiltInParameter · u32 n · n UTF-16 units` in the element's (or the
//! type's) serialised data. The keys are Autodesk's public
//! `BuiltInParameter` values `IFC_EXPORT_ELEMENT_AS` (−1019014),
//! `IFC_EXPORT_ELEMENT_TYPE_AS` (−1019015), `IFC_EXPORT_PREDEFINEDTYPE`
//! (−1019016) and `IFC_EXPORT_PREDEFINEDTYPE_TYPE` (−1019017). An entry
//! belongs to the element whose data header is the last one before it.
//!
//! On `2024_Core_Interior.rvt` 26 declared elements carry one: 21
//! `IfcShadingDevice` (the 20 exported are Revit's 20 `IFCSHADINGDEVICE`
//! Tags), 3 `ifcSlab` with `ROOF` (the 2 exported are Revit's 2 `IFCSLAB …
//! .ROOF.` rows), one `IfcSpace` and one `ifcFooting`, neither exported. A
//! further 12 entries sit in loaded families' own documents, whose ids
//! `Global/ElemTable` does not declare, and are not read. On
//! `teste_export_2025.rvt` wall type 402 carries `IfcCoveringType` with
//! `CLADDING`, and Revit exports its three walls as `IFCCOVERING …
//! .CLADDING.`.
//!
//! The probe prints every element carrying an export parameter, with its
//! values. Compare an element's entity and `PredefinedType` with the row of
//! the same `Tag` in Revit's export, and a type's with the rows of the
//! elements that name it.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re45_ifc_export_parameters -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_ifc_export_overrides::scan_export_parameters;
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let parameters = scan_export_parameters(&mut rf, version, &declared)?;
    println!(
        "release {version}: {} elements carry IFC export parameters",
        parameters.len()
    );
    for (id, p) in parameters {
        let show = |v: &Option<String>| v.clone().unwrap_or_else(|| "-".into());
        println!(
            "  {id:>9}  export as {:<18} predefined {:<10} type export as {:<18} type predefined {}",
            show(&p.export_as),
            show(&p.predefined_type),
            show(&p.type_export_as),
            show(&p.type_predefined_type),
        );
    }
    Ok(())
}
