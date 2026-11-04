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
    /// Draw a family instance from its family's own extrusions where they
    /// reproduce its saved mesh (`--family-extrusions`): implies the saved
    /// graphics of every boxed element.
    pub family_extrusions: bool,
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
    /// Also list where each decoded element was read from in the diagnostics
    /// (`decoded.element_provenance`, `rvt-ifc --element-provenance`, #1325).
    /// Off by default: the table is large (on the BIM4LCA office as built
    /// 6,202,130 bytes of compact JSON against 38,640 bytes of diagnostics
    /// without it, about 161 times) and never goes into the IFC. While it is
    /// on, the readers also note the copy they take each value from (#1360).
    pub element_provenance: bool,
    /// Also list each fail-closed decision per element in the diagnostics
    /// (`export_decisions`, `rvt-ifc --export-decisions`, #1326): why an
    /// element, body, opening or outline was left out, bounded per reason. Off
    /// by default; the diagnostics are then the bytes they were. It is a
    /// switch of its own, not part of `element_provenance`, because the log
    /// is bounded (at most 5,000 entries a reason) where the table grows with
    /// the model (161 times the diagnostics on the BIM4LCA office), and it
    /// needs no table in the diagnostics: where an entry's evidence is an
    /// element the walk decoded, its stream and offset are read from a table
    /// built for the purpose and dropped.
    pub export_decisions: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            kind: ExportKind::Document,
            mode: ExportQualityMode::Scaffold,
            saved_meshes: SavedMeshChoice::Default,
            family_extrusions: false,
            revit_property_sets: false,
            revit_set_placement: false,
            no_base_quantities: false,
            phase: None,
            element_provenance: false,
            export_decisions: false,
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

    /// Whether a family instance is drawn from its family's own extrusions
    /// where they reproduce its saved mesh. Off by default; implies the saved
    /// graphics of every boxed element.
    #[must_use]
    pub fn with_family_extrusions(mut self, family_extrusions: bool) -> Self {
        self.family_extrusions = family_extrusions;
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

    /// Whether the diagnostics list where each decoded element was read from
    /// ([`ElementProvenanceTable`](super::element_provenance::ElementProvenanceTable)).
    /// Off by default; the diagnostics are then the bytes they were before
    /// the option existed.
    #[must_use]
    pub fn with_element_provenance(mut self, element_provenance: bool) -> Self {
        self.element_provenance = element_provenance;
        self
    }

    /// Whether the diagnostics list each fail-closed decision per element
    /// ([`ExportDecisionLog`](super::export_decisions::ExportDecisionLog)). Off
    /// by default; independent of
    /// [`with_element_provenance`](Self::with_element_provenance).
    #[must_use]
    pub fn with_export_decisions(mut self, export_decisions: bool) -> Self {
        self.export_decisions = export_decisions;
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
    /// Bodies drawn from their family's own extrusions.
    #[non_exhaustive]
    FamilyExtrusions {
        /// Instances drawn from a saved mesh, alone in their Tag.
        candidates: usize,
        /// Of those, instances now drawn from their family's extrusions.
        replaced: usize,
        /// The families of the candidates whose document was found.
        families: usize,
        /// Of those, families all of whose extrusions rebuilt.
        families_rebuilt: usize,
        /// Candidates whose family was not found or not rebuilt.
        not_rebuilt: usize,
        /// Candidates with no instance transform that reads.
        unplaced: usize,
        /// Candidates whose mesh is too large to compare.
        unchecked: usize,
        /// Candidates whose mesh the extrusions do not reproduce, by volume.
        volume_differs: usize,
        /// By a vertex of the mesh off the extrusions' surface.
        vertices_differ: usize,
        /// By a sampled point of the extrusions' surface off the mesh.
        surface_differs: usize,
        /// Reproduced, but left with their mesh for an arc over 0.12 ft in
        /// radius.
        arcs: usize,
        /// Reproduced, but their profile cannot be written.
        unwritable: usize,
        /// Of the replaced, instances with an arc in a profile.
        with_arcs: usize,
        /// Instances left alone for sharing a Tag with another entity.
        ambiguous: usize,
    },
    /// The family extrusions could not be read; the bodies are unchanged.
    #[non_exhaustive]
    FamilyExtrusionsSkipped {
        /// The reason.
        error: String,
    },
}

impl ExportNote {
    /// Why the step did not run, for the `...Skipped` notes; `None` for a
    /// step that ran.
    pub fn skipped(&self) -> Option<&str> {
        match self {
            Self::SavedMeshesSkipped { error }
            | Self::FamilyExtrusionsSkipped { error }
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
            Self::FamilyExtrusions {
                candidates,
                replaced,
                families,
                families_rebuilt,
                not_rebuilt,
                unplaced,
                unchecked,
                volume_differs,
                vertices_differ,
                surface_differs,
                arcs,
                unwritable,
                with_arcs,
                ambiguous,
            } => write!(
                f,
                "{replaced} of {candidates} saved-mesh bodies drawn from their family's extrusions ({families} families found, {families_rebuilt} rebuilt; {not_rebuilt} had no rebuilt family, {unplaced} no instance transform, {unchecked} a mesh too large to compare; the extrusions did not reproduce the mesh of {volume_differs} by volume, {vertices_differ} by vertices and {surface_differs} by surface; {arcs} reproduced but left with the mesh for an arc over 0.12 ft in radius, {unwritable} reproduced but not writable; {with_arcs} of the drawn ones have an arc in a profile; {ambiguous} left alone for sharing a Tag)"
            ),
            Self::FamilyExtrusionsSkipped { error } => {
                write!(f, "family extrusions not read, bodies unchanged: {error}")
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
/// 3. the element provenance table in the diagnostics, when asked, from the
///    walk step 2 made;
/// 4. the bodies drawn from saved graphics, for the Document and
///    DiagnosticProxies kinds;
/// 5. Revit's parameter sets and their placement, when asked;
/// 6. the base quantities left out, when asked;
/// 7. the log of per-element decisions in the diagnostics, when asked, from the
///    decisions step 2 made and the final model;
/// 8. the unread streams recorded in the diagnostics, which steps 4 and 5 may
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
    // #1360: the readers note the copy they take each value from only when the
    // table that lists them is asked for.
    rf.record_value_sources(options.element_provenance && options.kind != ExportKind::Placeholder);
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
    let decisions = result.decisions;
    let mut notes = Vec::new();

    // #1325: built from the walk the exporter just made, which the handle
    // memoised, before the steps below touch the handle again. The log of
    // decisions (#1326) reads the same table for the place of an element's
    // record, so where it is asked for without the table in the diagnostics
    // the table is built here and kept apart, then dropped.
    let mut log_provenance = None;
    if options.kind != ExportKind::Placeholder {
        if options.element_provenance {
            control.check()?;
            diagnostics.decoded.element_provenance =
                Some(super::element_provenance::build(rf, limits));
        } else if options.export_decisions {
            control.check()?;
            log_provenance = Some(super::element_provenance::build(rf, limits));
        }
    }

    let scope = match (options.kind, options.saved_meshes) {
        (ExportKind::Placeholder, _) | (_, SavedMeshChoice::Off) => None,
        (_, SavedMeshChoice::All) => Some(SavedMeshScope::All),
        (_, SavedMeshChoice::Default) if options.family_extrusions => Some(SavedMeshScope::All),
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

    if options.family_extrusions && options.kind != ExportKind::Placeholder {
        match super::family_extrusions::attach_with_control(rf, &mut model, control) {
            Ok(report) => {
                recount_exported(&mut diagnostics, &model);
                notes.push(ExportNote::FamilyExtrusions {
                    candidates: report.candidates,
                    replaced: report.replaced,
                    families: report.families,
                    families_rebuilt: report.families_rebuilt,
                    not_rebuilt: report.not_rebuilt,
                    unplaced: report.unplaced,
                    unchecked: report.unchecked,
                    volume_differs: report.volume_differs,
                    vertices_differ: report.vertices_differ,
                    surface_differs: report.surface_differs,
                    arcs: report.arcs,
                    unwritable: report.unwritable,
                    with_arcs: report.with_arcs,
                    ambiguous: report.ambiguous,
                });
            }
            Err(error) => {
                control.check()?;
                notes.push(ExportNote::FamilyExtrusionsSkipped {
                    error: format!("{error:#}"),
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

    // #1360: the model is final, so the bodies the saved graphics and the
    // family extrusions replaced name their own sources.
    if let Some(table) = diagnostics.decoded.element_provenance.as_mut() {
        super::element_provenance::attach_value_sources(table, rf, &model);
    }

    // #1326: the decisions the `skipped` counts were made from, and those
    // behind the final model's box bodies and outlines.
    if options.export_decisions && options.kind != ExportKind::Placeholder {
        control.check()?;
        let mut all = decisions;
        all.extend(super::model_decisions(&model));
        let table = diagnostics
            .decoded
            .element_provenance
            .as_ref()
            .or(log_provenance.as_ref());
        let log = super::export_decisions::ExportDecisionLog::build(all, table);
        diagnostics.export_decisions = Some(log);
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
