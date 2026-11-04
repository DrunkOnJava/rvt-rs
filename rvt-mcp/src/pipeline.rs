//! The decode `rvt-ifc` runs, up to the model it writes the IFC from.
//!
//! The steps and their order are those of `src/bin/rvt_ifc.rs`: the document
//! exporter, the saved-mesh bodies, the optional Revit property sets and
//! property placement, the base-quantity switch, and the record of streams a
//! resource limit kept unread. Every tool answers from the model this builds,
//! and the exports are written from it, so an answer and the IFC cannot
//! disagree. The end-to-end test compares an export with `rvt-ifc`'s byte for
//! byte.

use rvt::RevitFile;
use rvt::control::WalkerControl;
use rvt::ifc::saved_meshes::{SavedMeshScope, attach_scoped_with_control, default_scope};
use rvt::ifc::{
    ExportDiagnostics, ExportQualityMode, IfcModel, RvtDocExporter, leave_out_base_quantities,
    record_unread_streams, recount_exported,
};
use rvt::reader::UnreadStream;

use crate::config::{Config, Failure};

/// Which bodies are drawn from the mesh Revit saved for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshScope {
    /// Doors and windows on Revit 2023 to 2027, as `rvt-ifc` does by default.
    Default,
    /// Every element whose body would be its record's bounding box.
    All,
    /// None: doors and windows keep their record's box.
    Off,
}

/// The `rvt-ifc` switches that change the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineOptions {
    pub mode: ExportQualityMode,
    pub saved_meshes: MeshScope,
    pub revit_property_sets: bool,
    pub revit_set_placement: bool,
    pub no_base_quantities: bool,
}

impl Default for PipelineOptions {
    /// `rvt-ifc model.rvt` with no switch.
    fn default() -> Self {
        Self {
            mode: ExportQualityMode::Scaffold,
            saved_meshes: MeshScope::Default,
            revit_property_sets: false,
            revit_set_placement: false,
            no_base_quantities: false,
        }
    }
}

/// The decoded model, its diagnostics and what the run noted.
pub struct Pipeline {
    pub model: IfcModel,
    pub diagnostics: ExportDiagnostics,
    /// Streams a resource limit kept from being read whole.
    pub unread: Vec<UnreadStream>,
    /// What `rvt-ifc` prints to stderr about steps that fell back or were
    /// skipped.
    pub notes: Vec<String>,
}

fn export_failure(error: rvt::Error) -> Failure {
    match error {
        rvt::Error::Cancelled => Failure::Cancelled,
        other => Failure::Message(format!("the export could not read the file: {other}")),
    }
}

/// Decode `rf` as `rvt-ifc` does with `options`. A cancelled `control` ends
/// the run with [`Failure::Cancelled`] and nothing is kept.
pub fn run_pipeline(
    rf: &mut RevitFile,
    config: &Config,
    options: &PipelineOptions,
    control: &WalkerControl,
) -> Result<Pipeline, Failure> {
    let result = RvtDocExporter
        .export_with_diagnostics_mode_limits_and_control(
            rf,
            options.mode,
            config.walker_limits,
            control,
        )
        .map_err(export_failure)?;
    let mut model = result.model;
    let mut diagnostics = result.diagnostics;
    let mut notes = Vec::new();

    let scope = match options.saved_meshes {
        MeshScope::Off => None,
        MeshScope::All => Some(SavedMeshScope::All),
        MeshScope::Default => default_scope(rf),
    };
    if let Some(scope) = scope {
        // The saved graphics only improve bodies: when they cannot be read,
        // the export goes ahead with the bodies it has, and says so.
        match attach_scoped_with_control(rf, &mut model, scope, control) {
            Ok(report) => {
                recount_exported(&mut diagnostics, &model);
                if report.candidates > 0 {
                    let what = if options.saved_meshes == MeshScope::All {
                        "bounding-box bodies"
                    } else {
                        "door and window bounding-box bodies"
                    };
                    notes.push(format!(
                        "{} of {} {what} drawn from their saved graphics ({} triangles); {} kept their box, their saved graphics not decoding whole",
                        report.replaced, report.candidates, report.triangles, report.partial
                    ));
                }
            }
            Err(error) => {
                if control.check().is_err() {
                    return Err(Failure::Cancelled);
                }
                notes.push(format!(
                    "saved graphics not read, bodies unchanged: {error}"
                ));
            }
        }
    }

    if options.revit_property_sets {
        // Like the saved graphics, the parameters only add to the export.
        match rvt::ifc::revit_property_sets::attach(rf, &mut model) {
            Ok(report) => notes.push(format!(
                "Revit property sets on {} elements and {} types, and type sets on {} elements of types with no type object ({} properties, {} named parameters left out)",
                report.elements,
                report.types,
                report.untyped_elements,
                report.properties,
                report.left_out
            )),
            Err(error) => {
                if control.check().is_err() {
                    return Err(Failure::Cancelled);
                }
                notes.push(format!("Revit property sets not written: {error:#}"));
            }
        }
    }

    if options.revit_set_placement {
        match rvt::ifc::revit_set_placement::attach(rf, &mut model) {
            Ok(report) => {
                recount_exported(&mut diagnostics, &model);
                notes.push(format!(
                    "{} properties on {} elements written where Revit's exporter writes them ({} parameters of a kind not written left out)",
                    report.properties, report.elements, report.left_out
                ));
            }
            Err(error) => {
                if control.check().is_err() {
                    return Err(Failure::Cancelled);
                }
                notes.push(format!(
                    "properties not placed as Revit's exporter places them: {error:#}"
                ));
            }
        }
    }

    if options.no_base_quantities {
        let left_out = leave_out_base_quantities(&mut model);
        recount_exported(&mut diagnostics, &model);
        notes.push(format!("{left_out} base quantities left out"));
    }

    // The steps above may have read more streams than the export did.
    record_unread_streams(&mut diagnostics, rf.unread_streams());
    let unread = rf.unread_streams().to_vec();
    Ok(Pipeline {
        model,
        diagnostics,
        unread,
        notes,
    })
}
