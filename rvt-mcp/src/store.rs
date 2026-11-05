//! The decoded models the server keeps.
//!
//! A model is identified by its file's canonical path, size and modification
//! time, and by the project phase it was opened at, when one was named: by the
//! phase's ElementId, so a phase named two ways is one model. `open_model`
//! decodes it once and returns an opaque `model_id` hashed from that identity;
//! every tool takes the `model_id` or the path. The server keeps the most
//! recently used few and drops the others, so an id can be unknown after an
//! eviction or a restart, and the tools then say so and ask for `open_model`
//! again.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::UNIX_EPOCH;

use rvt::RevitFile;
use rvt::control::WalkerControl;
use rvt::elem_table::RecordFraming;
use rvt::ifc::element_provenance::ElementProvenanceTable;
use rvt::ifc::export_decisions::ExportDecisionLog;
use rvt::ifc::step_writer::{Placement, element_global_ids, placements, storey_of};
use rvt::ifc::{
    ExportDiagnostics, ExportNote, ExportOptions, ExportRun, IfcModel,
    run_export_with_limits_and_control,
};
use rvt::reader::UnreadStream;
use sha2::{Digest, Sha256};

use crate::config::{Config, Failure};
use crate::schema::{ElemTableProvenance, FileProvenance};

/// `bytes` as lowercase hex, two digits a byte.
pub fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How a failed export ends the tool job: a cancelled request is cancelled.
fn export_failure(error: rvt::Error) -> Failure {
    match error {
        rvt::Error::Cancelled => Failure::Cancelled,
        other => Failure::Message(format!("the export could not read the file: {other}")),
    }
}

/// The decode every tool answers from and the exports are written from:
/// [`run_export_with_limits_and_control`] under the server's walker limits, at
/// the phase `options` names. A cancelled `control` ends the run with
/// [`Failure::Cancelled`] and nothing is kept.
pub fn run_decode(
    rf: &mut RevitFile,
    config: &Config,
    options: &ExportOptions,
    control: &WalkerControl,
) -> Result<ExportRun, Failure> {
    run_export_with_limits_and_control(rf, options, config.walker_limits, control)
        .map_err(export_failure)
}

/// What `rvt-ifc` prints about the run's steps, one sentence each.
pub fn note_texts(notes: &[ExportNote]) -> Vec<String> {
    notes.iter().map(ToString::to_string).collect()
}

/// A file as the server identifies it: where it is and which bytes it holds,
/// as far as its size and modification time say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub path: PathBuf,
    pub size: u64,
    modified_nanos: u128,
}

impl Identity {
    /// The identity of the file at `path`, which is made canonical.
    pub fn of(path: &Path) -> Result<Self, String> {
        let canonical = std::fs::canonicalize(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Self::stat(canonical)
    }

    fn stat(path: PathBuf) -> Result<Self, String> {
        let metadata = std::fs::metadata(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if !metadata.is_file() {
            return Err(format!(
                "{} is not a file; pass a .rvt, .rfa, .rte or .rft file",
                path.display()
            ));
        }
        let modified_nanos = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |elapsed| elapsed.as_nanos());
        Ok(Self {
            path,
            size: metadata.len(),
            modified_nanos,
        })
    }

    /// The opaque handle: a hash of the identity and the ElementId of the
    /// phase it was opened at, with no path in it.
    pub fn model_id(&self, phase: Option<i64>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.path.to_string_lossy().as_bytes());
        hasher.update([0u8]);
        hasher.update(self.size.to_le_bytes());
        hasher.update(self.modified_nanos.to_le_bytes());
        if let Some(phase) = phase {
            hasher.update([1u8]);
            hasher.update(phase.to_le_bytes());
        }
        let digest = hasher.finalize();
        format!("rvt-{}", &lower_hex(&digest)[..16])
    }

    /// `rvt`, `rfa`, `rte`, `rft` or `unknown`, from the file name extension.
    pub fn file_kind(&self) -> &'static str {
        let extension = self
            .path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        match extension.as_str() {
            "rvt" => "rvt",
            "rfa" => "rfa",
            "rte" => "rte",
            "rft" => "rft",
            _ => "unknown",
        }
    }

    /// Whether the file still has the size and modification time it had.
    pub fn unchanged(&self) -> bool {
        Self::stat(self.path.clone()).is_ok_and(|now| now == *self)
    }
}

/// A `model_id` as [`Identity::model_id`] writes it.
fn looks_like_id(text: &str) -> bool {
    text.len() == 20
        && text.starts_with("rvt-")
        && text[4..]
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// What the default decode of a model gives, kept with it.
pub struct Computed {
    pub model: IfcModel,
    pub diagnostics: ExportDiagnostics,
    pub unread: Vec<UnreadStream>,
    pub notes: Vec<String>,
    /// The GlobalId the IFC export writes for each entity, by index.
    pub global_ids: Vec<Option<String>>,
    pub file: FileProvenance,
    /// Where each decoded element was read from (#1325). The decode asks for
    /// it and it is taken out of `diagnostics`, so the diagnostics the tools
    /// return and the exports' summaries stay what `rvt-ifc --diagnostics`
    /// writes without `--element-provenance`: the table is 4.5 MB of compact
    /// JSON on the BIM4LCA office.
    pub element_provenance: Option<ElementProvenanceTable>,
    /// Why each element, body, opening or outline was left out (#1326), taken
    /// out of `diagnostics` as the provenance table is: `explain_element`
    /// reads it.
    pub export_decisions: Option<ExportDecisionLog>,
    /// Where the IFC export puts each entity in the spatial structure, by
    /// index into the model's entities.
    pub placements: Vec<Option<Placement>>,
}

impl Computed {
    /// Whether the export writes the entity at `index` as an element: a piece
    /// folded into the body of the element that owns it is not.
    pub fn is_written(&self, index: usize) -> bool {
        !matches!(self.placements.get(index), Some(Some(Placement::Folded)))
    }

    /// The storey of the entity at `index` as the export puts it: the one it
    /// is contained in, that of its space or whole, or that of the host an
    /// opening cuts.
    pub fn storey(&self, index: usize) -> Option<usize> {
        storey_of(&self.placements, index)
    }

    fn new(mut run: ExportRun, mut file: FileProvenance) -> Self {
        let element_provenance = run.diagnostics.decoded.element_provenance.take();
        let export_decisions = run.diagnostics.export_decisions.take();
        file.omitted_entries = element_provenance
            .as_ref()
            .map(|table| table.omitted_entries);
        let global_ids = element_global_ids(&run.model);
        let placements = placements(&run.model);
        let notes = note_texts(&run.notes);
        Self {
            model: run.model,
            diagnostics: run.diagnostics,
            unread: run.unread,
            notes,
            global_ids,
            file,
            element_provenance,
            export_decisions,
            placements,
        }
    }
}

/// What the file says about itself, from the handle that decoded it.
fn file_provenance(rf: &mut RevitFile) -> FileProvenance {
    let (revit_release, revit_build) = match rf.basic_file_info() {
        Ok(info) => (Some(info.version), info.build),
        Err(_) => (None, None),
    };
    let schema_class_count = rf.schema().ok().map(|table| table.classes.len());
    let elem_table = rvt::elem_table::layout(rf).ok().map(|layout| {
        let (framing, owner_field_bytes) = match layout.framing {
            RecordFraming::Explicit { marker_len } => ("explicit", Some(marker_len)),
            RecordFraming::Implicit => ("implicit", None),
            _ => ("other", None),
        };
        ElemTableProvenance {
            record_stride: layout.stride,
            owner_field_offset: layout.marker_offset,
            framing: framing.to_string(),
            owner_field_bytes,
        }
    });
    FileProvenance {
        revit_release,
        revit_build,
        schema_class_count,
        elem_table,
        partition_count: Some(rf.partition_stream_names().len()),
        layout_release: rf.layout_release().ok(),
        omitted_entries: None,
    }
}

/// The project phase a model was opened at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedPhase {
    /// The phase as the first request named it, a name or an ElementId.
    pub spec: String,
    /// Its ElementId, which the model is keyed by.
    pub id: i64,
}

/// An open model: the file's handle, which keeps what the decode inflated,
/// and the default decode once it has run.
pub struct Entry {
    pub identity: Identity,
    /// The project phase the handle was opened at (`None`: the project's
    /// last). It is set before anything is read from the handle.
    pub phase: Option<OpenedPhase>,
    pub id: String,
    file: Mutex<RevitFile>,
    default: Mutex<Option<Arc<Computed>>>,
}

impl Entry {
    fn new(identity: Identity, phase: Option<OpenedPhase>, file: RevitFile) -> Self {
        let id = identity.model_id(phase.as_ref().map(|phase| phase.id));
        Self {
            identity,
            phase,
            id,
            file: Mutex::new(file),
            default: Mutex::new(None),
        }
    }

    fn is(&self, identity: &Identity, phase: Option<i64>) -> bool {
        self.identity == *identity && self.phase.as_ref().map(|phase| phase.id) == phase
    }

    /// The phase to name in a decode's `ExportOptions`.
    pub fn phase_spec(&self) -> Option<String> {
        self.phase.as_ref().map(|phase| phase.spec.clone())
    }

    /// The decode `rvt-ifc` runs with no switch, run once and kept.
    pub fn computed(
        &self,
        config: &Config,
        control: &WalkerControl,
    ) -> Result<Arc<Computed>, Failure> {
        let mut slot = lock(&self.default);
        if let Some(done) = slot.as_ref() {
            return Ok(Arc::clone(done));
        }
        let mut file = lock(&self.file);
        let options = ExportOptions::default()
            .with_phase(self.phase_spec())
            .with_element_provenance(true)
            .with_export_decisions(true);
        let run = run_decode(&mut file, config, &options, control)?;
        let provenance = file_provenance(&mut file);
        drop(file);
        let done = Arc::new(Computed::new(run, provenance));
        *slot = Some(Arc::clone(&done));
        Ok(done)
    }

    /// Run `job` on the file's handle, for a decode with other switches.
    pub fn with_file<T>(&self, job: impl FnOnce(&mut RevitFile) -> T) -> T {
        let mut file = lock(&self.file);
        job(&mut file)
    }
}

/// How many phase resolutions the store remembers.
const PHASE_MEMORY: usize = 64;

/// A phase as one request spelled it, in the file of one identity.
type PhaseKey = (Identity, String);

/// The least recently used decoded models, most recent first.
pub struct Store {
    config: Config,
    entries: Mutex<Vec<Arc<Entry>>>,
    /// What a phase named in a request turned out to be in a file: its
    /// ElementId, or why the file has no such phase. A phase the file lacks
    /// would otherwise read the whole file again each time it was asked for.
    /// Newest first.
    phases: Mutex<Vec<(PhaseKey, Result<i64, String>)>>,
}

impl Store {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            entries: Mutex::new(Vec::new()),
            phases: Mutex::new(Vec::new()),
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The entry for `model`, a `model_id` or a path, at `phase` (a phase's
    /// name or ElementId; `None` is the project's last). A path that is not
    /// held is opened, which reads the file into memory. A `model_id` already
    /// names the phase it was opened at, so a different `phase` with one is
    /// refused.
    pub fn open(&self, model: &str, phase: Option<&str>) -> Result<Arc<Entry>, String> {
        let model = model.trim();
        if !looks_like_id(model) {
            return self.by_path(model, phase);
        }
        let entry = self.by_id(model)?;
        let Some(spec) = phase else {
            return Ok(entry);
        };
        let (named, _) = self.resolve_phase(&entry.identity, spec)?;
        if entry.phase.as_ref().map(|phase| phase.id) == Some(named) {
            return Ok(entry);
        }
        Err(format!(
            "phase {spec:?} cannot be combined with model_id {model}: an id names the phase its model was opened at ({}). Pass the file path with phase, or call open_model with the path and phase.",
            entry
                .phase
                .as_ref()
                .map_or("the project's last".to_string(), |at| format!(
                    "{:?}",
                    at.spec
                ))
        ))
    }

    fn by_id(&self, id: &str) -> Result<Arc<Entry>, String> {
        let found = {
            let mut entries = lock(&self.entries);
            let position = entries.iter().position(|entry| entry.id == id);
            position.map(|position| {
                let entry = entries.remove(position);
                entries.insert(0, Arc::clone(&entry));
                entry
            })
        };
        let Some(entry) = found else {
            return Err(format!(
                "model_id {id} is not open: this server did not issue it, it was evicted (at most {} models are kept), or the server restarted. Call open_model again with the file path.",
                self.config.max_open_models
            ));
        };
        if !entry.identity.unchanged() {
            // Every phase of the changed file is as stale as this one.
            lock(&self.entries).retain(|held| held.identity != entry.identity);
            lock(&self.phases).retain(|((identity, _), _)| *identity != entry.identity);
            return Err(format!(
                "the file behind model_id {id} changed or was removed since open_model returned it. Call open_model again with the file path."
            ));
        }
        Ok(entry)
    }

    fn open_file(&self, identity: &Identity) -> Result<RevitFile, String> {
        RevitFile::open_with_limits(&identity.path, self.config.open_limits)
            .map_err(|error| error.to_string())
    }

    /// The ElementId of the phase `spec` names in the file of `identity`, and,
    /// when this call had to read the file to find out, the handle it read,
    /// already at that phase. A phase the file lacks is an error that lists the
    /// file's phases; the answer, either way, is remembered.
    fn resolve_phase(
        &self,
        identity: &Identity,
        spec: &str,
    ) -> Result<(i64, Option<RevitFile>), String> {
        if let Some(known) = self.remembered(identity, spec) {
            return known.map(|id| (id, None));
        }
        let mut file = self.open_file(identity)?;
        let resolved = file
            .set_export_phase(spec)
            .map_err(|error| error.to_string())
            .and_then(|()| {
                file.export_phase()
                    .ok_or_else(|| "the export phase was not recorded".to_string())
            });
        self.remember(identity, spec, resolved.clone());
        resolved.map(|id| (id, Some(file)))
    }

    fn remembered(&self, identity: &Identity, spec: &str) -> Option<Result<i64, String>> {
        lock(&self.phases)
            .iter()
            .find(|((held, named), _)| held == identity && named == spec)
            .map(|(_, resolved)| resolved.clone())
    }

    fn remember(&self, identity: &Identity, spec: &str, resolved: Result<i64, String>) {
        let mut phases = lock(&self.phases);
        phases.retain(|((held, named), _)| !(held == identity && named == spec));
        phases.insert(0, ((identity.clone(), spec.to_string()), resolved));
        phases.truncate(PHASE_MEMORY);
    }

    fn by_path(&self, path: &str, spec: Option<&str>) -> Result<Arc<Entry>, String> {
        if path.is_empty() {
            return Err("model is empty: pass a model_id from open_model, or the path of a .rvt, .rfa, .rte or .rft file".to_string());
        }
        let identity = Identity::of(Path::new(path))?;
        // Before anything is read from a handle: an unknown phase is refused
        // here, listing the file's phases, and never takes a held slot. Every
        // decode of the entry also names the phase in its `ExportOptions`.
        let (phase, mut opened) = match spec {
            None => (None, None),
            Some(spec) => {
                let (id, opened) = self.resolve_phase(&identity, spec)?;
                let phase = OpenedPhase {
                    spec: spec.to_string(),
                    id,
                };
                (Some(phase), opened)
            }
        };
        let phase_id = phase.as_ref().map(|phase| phase.id);
        if let Some(entry) = self.touch(&identity, phase_id) {
            return Ok(entry);
        }
        let file = match opened.take() {
            Some(file) => file,
            None => {
                let mut file = self.open_file(&identity)?;
                if let Some(phase) = &phase {
                    file.set_export_phase(&phase.spec)
                        .map_err(|error| error.to_string())?;
                }
                file
            }
        };
        let entry = Arc::new(Entry::new(identity, phase, file));
        let mut entries = lock(&self.entries);
        // Another call may have opened it while this one read the file.
        if let Some(existing) = entries
            .iter()
            .find(|held| held.is(&entry.identity, phase_id))
        {
            return Ok(Arc::clone(existing));
        }
        // The same path with another size or time is the file's earlier self,
        // at every phase.
        entries.retain(|held| {
            held.identity.path != entry.identity.path || held.identity == entry.identity
        });
        entries.insert(0, Arc::clone(&entry));
        entries.truncate(self.config.max_open_models);
        Ok(entry)
    }

    /// The held entry of `identity` at the phase with ElementId `phase`, made
    /// the most recent.
    fn touch(&self, identity: &Identity, phase: Option<i64>) -> Option<Arc<Entry>> {
        let mut entries = lock(&self.entries);
        let position = entries.iter().position(|entry| entry.is(identity, phase))?;
        let entry = entries.remove(position);
        entries.insert(0, Arc::clone(&entry));
        Some(entry)
    }
}
