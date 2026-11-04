//! The export every front end runs: [`run_export`].
//!
//! `rvt-ifc`, the Python package, the viewer's WebAssembly bindings and
//! `rvt-rs-mcp` all build their IFC model from this one function, so the
//! steps and their order are written once: the exporter, the phase, the
//! bodies drawn from saved graphics, Revit's own property sets and their
//! placement, the base-quantity switch, and the record of streams a resource
//! limit kept unread. What each front end adds on top (writing the file,
//! refusing an incomplete export, checking the quality mode) stays with it.

use super::saved_meshes::{SavedMeshScope, attach_scoped_with_control, default_scope};
use super::{
    DiagnosticRvtDocExporter, ExportDiagnostics, ExportQualityMode, IfcModel, PlaceholderExporter,
    RvtDocExporter, leave_out_base_quantities, record_unread_streams, recount_exported,
};
use crate::control::WalkerControl;
use crate::reader::UnreadStream;
use crate::walker::WalkerLimits;
use crate::{Result, RevitFile};

/// Which exporter builds the model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExportKind {
    /// [`RvtDocExporter`]: the export `rvt-ifc` runs with no switch.
    #[default]
    Document,
    /// [`DiagnosticRvtDocExporter`]: also the low-confidence schema-scan hits,
    /// as proxies with diagnostic provenance (`rvt-ifc --diagnostic-proxies`).
    /// The quality mode does not apply.
    DiagnosticProxies,
    /// [`PlaceholderExporter`]: the project alone, no storeys or elements
    /// (`rvt-ifc --placeholder`). No bodies are drawn from saved graphics.
    Placeholder,
}

/// Which bodies are drawn from the mesh Revit saved for display.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SavedMeshChoice {
    /// Doors and windows of Revit 2023 to 2027, the releases whose saved
    /// graphics are read ([`default_scope`]): `rvt-ifc` with no switch.
    #[default]
    Default,
    /// Every element whose body would be its record's bounding box
    /// (`rvt-ifc --saved-meshes`). Slower.
    All,
    /// None: doors and windows keep their record's box
    /// (`rvt-ifc --no-saved-meshes`).
    Off,
}

/// The `rvt-ifc` switches that change the model. [`Default`] is `rvt-ifc` with
/// no switch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExportOptions {
    /// Which exporter builds the model.
    pub kind: ExportKind,
    /// The quality mode's *content* policy (`--mode`). The mode's validation
    /// ([`ExportQualityMode::validate`]) is left to the caller, so the
    /// diagnostics can still be written when a stronger mode rejects the
    /// result.
    pub mode: ExportQualityMode,
    /// Which bodies are drawn from saved graphics.
    pub saved_meshes: SavedMeshChoice,
    /// Also write Revit's own parameter sets (`--revit-property-sets`).
    pub revit_property_sets: bool,
    /// Also place properties where Revit's exporter does
    /// (`--revit-set-placement`).
    pub revit_set_placement: bool,
    /// Leave out the base quantities (`--no-base-quantities`).
    pub no_base_quantities: bool,
    /// Export this project phase, by its name (exactly) or its ElementId,
    /// instead of the project's last (`--phase`,
    /// [`RevitFile::set_export_phase`]). An unknown phase is
    /// [`Error::InvalidInput`](crate::Error::InvalidInput) listing the file's
    /// phases.
    pub phase: Option<String>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            kind: ExportKind::Document,
            mode: ExportQualityMode::Scaffold,
            saved_meshes: SavedMeshChoice::Default,
            revit_property_sets: false,
            revit_set_placement: false,
            no_base_quantities: false,
            phase: None,
        }
    }
}

impl ExportOptions {
    /// Which exporter builds the model.
    #[must_use]
    pub fn with_kind(mut self, kind: ExportKind) -> Self {
        self.kind = kind;
        self
    }

    /// The quality mode's content policy.
    #[must_use]
    pub fn with_mode(mut self, mode: ExportQualityMode) -> Self {
        self.mode = mode;
        self
    }

    /// Which bodies are drawn from saved graphics.
    #[must_use]
    pub fn with_saved_meshes(mut self, saved_meshes: SavedMeshChoice) -> Self {
        self.saved_meshes = saved_meshes;
        self
    }

    /// Whether Revit's own parameter sets are written.
    #[must_use]
    pub fn with_revit_property_sets(mut self, revit_property_sets: bool) -> Self {
        self.revit_property_sets = revit_property_sets;
        self
    }

    /// Whether properties are placed where Revit's exporter places them.
    #[must_use]
    pub fn with_revit_set_placement(mut self, revit_set_placement: bool) -> Self {
        self.revit_set_placement = revit_set_placement;
        self
    }

    /// Whether the base quantities are left out.
    #[must_use]
    pub fn with_no_base_quantities(mut self, no_base_quantities: bool) -> Self {
        self.no_base_quantities = no_base_quantities;
        self
    }

    /// The phase to export, by name or ElementId; `None` is the project's
    /// last.
    #[must_use]
    pub fn with_phase(mut self, phase: Option<String>) -> Self {
        self.phase = phase;
        self
    }
}

/// A step after the exporter, which [`ExportNote`]s name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExportStep {
    /// Bodies drawn from the saved graphics.
    SavedMeshes,
    /// Revit's own parameter sets.
    RevitPropertySets,
    /// Properties placed where Revit's exporter places them.
    RevitSetPlacement,
    /// The base quantities left out.
    BaseQuantities,
}

/// One thing a step reports, that `rvt-ifc` prints on stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExportNote {
    /// The step it reports.
    pub step: ExportStep,
    /// Why the step did not run, when it did not: the saved graphics, the
    /// parameters or the placement could not be read, and the export went
    /// ahead without what they add. `None` when the step ran.
    pub skipped: Option<String>,
    /// The sentence `rvt-ifc` prints after `rvt-ifc: `.
    pub text: String,
}

/// The model of an export, its diagnostics and what the run noted.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExportRun {
    /// The model to write the IFC (or glTF) from.
    pub model: IfcModel,
    /// What was read, approximated and left out, as `rvt-ifc --diagnostics`
    /// writes it. It already counts the saved meshes, the property
    /// placement, the base quantities and the unread streams.
    pub diagnostics: ExportDiagnostics,
    /// Streams a resource limit kept from being read whole
    /// ([`RevitFile::unread_streams`]), after every step: the model is
    /// missing what they hold. The caller decides whether to write it.
    pub unread: Vec<UnreadStream>,
    /// What the steps reported, in step order.
    pub notes: Vec<ExportNote>,
}

/// The model `rvt-ifc` writes the IFC from, built as `rvt-ifc` builds it:
///
/// 1. the phase, when `options.phase` names one (before anything is read);
/// 2. the exporter `options.kind` names, under `limits`;
/// 3. the bodies drawn from saved graphics, for the Document and
///    DiagnosticProxies kinds;
/// 4. Revit's parameter sets and their placement, when asked;
/// 5. the base quantities left out, when asked;
/// 6. the unread streams recorded in the diagnostics, which steps 3 and 4 may
///    have added to.
///
/// The saved graphics, the parameter sets and the placement only add to the
/// export: when one cannot be read, the export goes ahead without it and the
/// note says so. A cancelled `control` ends the run with
/// [`Error::Cancelled`](crate::Error::Cancelled) and nothing is kept;
/// `control` also receives the walk's progress. The document exporter and the
/// steps after it use it; the diagnostic-proxies and placeholder exporters
/// take none, as before.
///
/// Choose the phase before the first export or read of a handle: reads of the
/// handle are kept, and a handle that has been read under one phase keeps
/// what it read. Giving the phase the handle already has is harmless.
///
/// # Errors
///
/// The errors of reading the file, as
/// [`RvtDocExporter::export_with_diagnostics`] lists them; a phase the file
/// does not have as [`Error::InvalidInput`](crate::Error::InvalidInput).
///
/// # Examples
///
/// ```
/// use rvt::RevitFile;
/// use rvt::control::WalkerControl;
/// use rvt::ifc::{ExportOptions, run_export};
/// use rvt::walker::WalkerLimits;
///
/// let mut rf = RevitFile::open("corpus/tier1/architectural-2024/architectural-2024.rvt")?;
/// let run = run_export(
///     &mut rf,
///     &ExportOptions::default(),
///     WalkerLimits::default(),
///     &WalkerControl::default(),
/// )?;
/// println!("{} entities", run.model.entities.len());
/// for note in &run.notes {
///     println!("{}", note.text);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_export(
    rf: &mut RevitFile,
    options: &ExportOptions,
    limits: WalkerLimits,
    control: &WalkerControl,
) -> Result<ExportRun> {
    if let Some(phase) = &options.phase {
        rf.set_export_phase(phase)?;
    }
    let result = match options.kind {
        ExportKind::Document => RvtDocExporter.export_with_diagnostics_mode_limits_and_control(
            rf,
            options.mode,
            limits,
            control,
        )?,
        ExportKind::DiagnosticProxies => {
            DiagnosticRvtDocExporter.export_with_diagnostics_and_limits(rf, limits)?
        }
        ExportKind::Placeholder => PlaceholderExporter.export_with_diagnostics(rf)?,
    };
    let mut model = result.model;
    let mut diagnostics = result.diagnostics;
    let mut notes = Vec::new();

    let scope = match (options.kind, options.saved_meshes) {
        (ExportKind::Placeholder, _) | (_, SavedMeshChoice::Off) => None,
        (_, SavedMeshChoice::All) => Some(SavedMeshScope::All),
        (_, SavedMeshChoice::Default) => default_scope(rf),
    };
    if let Some(scope) = scope {
        match attach_scoped_with_control(rf, &mut model, scope, control) {
            Ok(report) => {
                recount_exported(&mut diagnostics, &model);
                if report.candidates > 0 {
                    let what = if scope == SavedMeshScope::All {
                        "bounding-box bodies"
                    } else {
                        "door and window bounding-box bodies"
                    };
                    notes.push(ExportNote {
                        step: ExportStep::SavedMeshes,
                        skipped: None,
                        text: format!(
                            "{} of {} {what} drawn from their saved graphics ({} triangles); {} kept their box, their saved graphics not decoding whole",
                            report.replaced, report.candidates, report.triangles, report.partial
                        ),
                    });
                }
            }
            Err(error) => {
                control.check()?;
                notes.push(ExportNote {
                    step: ExportStep::SavedMeshes,
                    skipped: Some(error.to_string()),
                    text: format!("saved graphics not read, bodies unchanged: {error}"),
                });
            }
        }
    }

    if options.revit_property_sets {
        match super::revit_property_sets::attach(rf, &mut model) {
            Ok(report) => notes.push(ExportNote {
                step: ExportStep::RevitPropertySets,
                skipped: None,
                text: format!(
                    "Revit property sets on {} elements and {} types, and type sets on {} elements of types with no type object ({} properties, {} named parameters left out)",
                    report.elements,
                    report.types,
                    report.untyped_elements,
                    report.properties,
                    report.left_out
                ),
            }),
            Err(error) => {
                control.check()?;
                notes.push(ExportNote {
                    step: ExportStep::RevitPropertySets,
                    skipped: Some(format!("{error:#}")),
                    text: format!("Revit property sets not written: {error:#}"),
                });
            }
        }
    }

    if options.revit_set_placement {
        match super::revit_set_placement::attach(rf, &mut model) {
            Ok(report) => {
                recount_exported(&mut diagnostics, &model);
                notes.push(ExportNote {
                    step: ExportStep::RevitSetPlacement,
                    skipped: None,
                    text: format!(
                        "{} properties on {} elements written where Revit's exporter writes them ({} parameters of a kind not written left out)",
                        report.properties, report.elements, report.left_out
                    ),
                });
            }
            Err(error) => {
                control.check()?;
                notes.push(ExportNote {
                    step: ExportStep::RevitSetPlacement,
                    skipped: Some(format!("{error:#}")),
                    text: format!(
                        "properties not placed as Revit's exporter places them: {error:#}"
                    ),
                });
            }
        }
    }

    if options.no_base_quantities {
        let left_out = leave_out_base_quantities(&mut model);
        recount_exported(&mut diagnostics, &model);
        notes.push(ExportNote {
            step: ExportStep::BaseQuantities,
            skipped: None,
            text: format!("{left_out} base quantities left out"),
        });
    }

    // #1075: the steps above may have read more streams than the export did.
    record_unread_streams(&mut diagnostics, rf.unread_streams());
    let unread = rf.unread_streams().to_vec();
    Ok(ExportRun {
        model,
        diagnostics,
        unread,
        notes,
    })
}
