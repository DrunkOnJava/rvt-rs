//! The export every front end runs: [`run_export`], with its limits
//! ([`run_export_with_limits`]) and its control
//! ([`run_export_with_limits_and_control`]).
//!
//! `rvt-ifc`, `rvt-gltf`, the Python package, the viewer's WebAssembly
//! bindings and `rvt-rs-mcp` all build their model from this one function, so the
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
use std::fmt;

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

/// One thing a step reports, which `rvt-ifc` prints on stderr as
/// `rvt-ifc: <note>` (the [`Display`](fmt::Display) of the note). The numbers
/// are those of the step's own report. A `...Skipped` note says the step could
/// not run, and why: the export went ahead without what the step adds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExportNote {
    /// Bodies drawn from the saved graphics of `candidates` boxed elements.
    #[non_exhaustive]
    SavedMeshes {
        /// Which bodies the step looked at.
        scope: SavedMeshScope,
        /// Elements whose body was the record's bounding box.
        candidates: usize,
        /// Of those, elements now drawn from their saved mesh.
        replaced: usize,
        /// Of those, elements keeping their box because part of their saved
        /// graphics did not decode.
        partial: usize,
        /// Triangles written in all.
        triangles: usize,
    },
    /// The saved graphics could not be read; the bodies are unchanged.
    #[non_exhaustive]
    SavedMeshesSkipped {
        /// The reason.
        error: String,
    },
    /// Revit's own parameter sets, written.
    #[non_exhaustive]
    RevitPropertySets {
        /// Elements given at least one property.
        elements: usize,
        /// Type objects' types given at least one property.
        types: usize,
        /// Elements given their type's sets, their type having no type
        /// object.
        untyped_elements: usize,
        /// Properties written, on elements and types.
        properties: usize,
        /// Named parameters left out.
        left_out: usize,
    },
    /// Revit's parameter sets could not be read; none were written.
    #[non_exhaustive]
    RevitPropertySetsSkipped {
        /// The reason.
        error: String,
    },
    /// Properties placed where Revit's exporter places them.
    #[non_exhaustive]
    RevitSetPlacement {
        /// Elements given at least one property.
        elements: usize,
        /// Properties written.
        properties: usize,
        /// Parameters of a kind not written, left out.
        left_out: usize,
    },
    /// The parameters could not be read; no property was placed.
    #[non_exhaustive]
    RevitSetPlacementSkipped {
        /// The reason.
        error: String,
    },
    /// The base quantities left out.
    #[non_exhaustive]
    BaseQuantities {
        /// How many quantities were left out.
        left_out: usize,
    },
}

impl ExportNote {
    /// Why the step did not run, for the `...Skipped` notes; `None` for a
    /// step that ran.
    pub fn skipped(&self) -> Option<&str> {
        match self {
            Self::SavedMeshesSkipped { error }
            | Self::RevitPropertySetsSkipped { error }
            | Self::RevitSetPlacementSkipped { error } => Some(error),
            _ => None,
        }
    }
}

impl fmt::Display for ExportNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SavedMeshes {
                scope,
                candidates,
                replaced,
                partial,
                triangles,
            } => {
                let what = if *scope == SavedMeshScope::All {
                    "bounding-box bodies"
                } else {
                    "door and window bounding-box bodies"
                };
                write!(
                    f,
                    "{replaced} of {candidates} {what} drawn from their saved graphics ({triangles} triangles); {partial} kept their box, their saved graphics not decoding whole"
                )
            }
            Self::SavedMeshesSkipped { error } => {
                write!(f, "saved graphics not read, bodies unchanged: {error}")
            }
            Self::RevitPropertySets {
                elements,
                types,
                untyped_elements,
                properties,
                left_out,
            } => write!(
                f,
                "Revit property sets on {elements} elements and {types} types, and type sets on {untyped_elements} elements of types with no type object ({properties} properties, {left_out} named parameters left out)"
            ),
            Self::RevitPropertySetsSkipped { error } => {
                write!(f, "Revit property sets not written: {error}")
            }
            Self::RevitSetPlacement {
                elements,
                properties,
                left_out,
            } => write!(
                f,
                "{properties} properties on {elements} elements written where Revit's exporter writes them ({left_out} parameters of a kind not written left out)"
            ),
            Self::RevitSetPlacementSkipped { error } => write!(
                f,
                "properties not placed as Revit's exporter places them: {error}"
            ),
            Self::BaseQuantities { left_out } => {
                write!(f, "{left_out} base quantities left out")
            }
        }
    }
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

/// The model `rvt-ifc` writes the IFC from, built as `rvt-ifc` builds it, under
/// the default walker limits and with no control. See
/// [`run_export_with_limits_and_control`] for the steps.
///
/// # Errors
///
/// As [`run_export_with_limits_and_control`].
///
/// # Examples
///
/// ```
/// use rvt::RevitFile;
/// use rvt::ifc::{ExportOptions, run_export};
///
/// let mut rf = RevitFile::open("corpus/tier1/architectural-2024/architectural-2024.rvt")?;
/// let run = run_export(&mut rf, &ExportOptions::default())?;
/// println!("{} entities", run.model.entities.len());
/// for note in &run.notes {
///     println!("{note}");
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_export(rf: &mut RevitFile, options: &ExportOptions) -> Result<ExportRun> {
    run_export_with_limits(rf, options, WalkerLimits::default())
}

/// [`run_export`] under the resource limits of `limits`, with no control.
///
/// # Errors
///
/// As [`run_export_with_limits_and_control`].
pub fn run_export_with_limits(
    rf: &mut RevitFile,
    options: &ExportOptions,
    limits: WalkerLimits,
) -> Result<ExportRun> {
    run_export_with_limits_and_control(rf, options, limits, &WalkerControl::default())
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
/// A handle may be exported at one phase and then at another: choosing a
/// different phase drops the memoised walk that holds what the phase leaves
/// out ([`RevitFile::set_export_phase`]), so each export is the one a handle
/// opened for that phase gives, and giving the phase the handle already has
/// keeps what it read. `options.phase` of `None` leaves the handle's phase as
/// it is; [`RevitFile::clear_export_phase`] goes back to the project's last.
///
/// # Errors
///
/// The errors of reading the file, as
/// [`RvtDocExporter::export_with_diagnostics`] lists them; a phase the file
/// does not have as [`Error::InvalidInput`](crate::Error::InvalidInput);
/// [`Error::Cancelled`](crate::Error::Cancelled) once `control` is cancelled.
pub fn run_export_with_limits_and_control(
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
                    notes.push(ExportNote::SavedMeshes {
                        scope,
                        candidates: report.candidates,
                        replaced: report.replaced,
                        partial: report.partial,
                        triangles: report.triangles,
                    });
                }
            }
            Err(error) => {
                control.check()?;
                notes.push(ExportNote::SavedMeshesSkipped {
                    error: error.to_string(),
                });
            }
        }
    }

    if options.revit_property_sets {
        match super::revit_property_sets::attach(rf, &mut model) {
            Ok(report) => notes.push(ExportNote::RevitPropertySets {
                elements: report.elements,
                types: report.types,
                untyped_elements: report.untyped_elements,
                properties: report.properties,
                left_out: report.left_out,
            }),
            Err(error) => {
                control.check()?;
                notes.push(ExportNote::RevitPropertySetsSkipped {
                    error: format!("{error:#}"),
                });
            }
        }
    }

    if options.revit_set_placement {
        match super::revit_set_placement::attach(rf, &mut model) {
            Ok(report) => {
                recount_exported(&mut diagnostics, &model);
                notes.push(ExportNote::RevitSetPlacement {
                    elements: report.elements,
                    properties: report.properties,
                    left_out: report.left_out,
                });
            }
            Err(error) => {
                control.check()?;
                notes.push(ExportNote::RevitSetPlacementSkipped {
                    error: format!("{error:#}"),
                });
            }
        }
    }

    if options.no_base_quantities {
        let left_out = leave_out_base_quantities(&mut model);
        recount_exported(&mut diagnostics, &model);
        notes.push(ExportNote::BaseQuantities { left_out });
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
