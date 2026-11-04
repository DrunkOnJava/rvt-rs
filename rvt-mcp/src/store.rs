//! The decoded models the server keeps.
//!
//! A model is identified by its file's canonical path, size and modification
//! time. `open_model` decodes it once and returns an opaque `model_id` hashed
//! from that identity; every tool takes the `model_id` or the path. The
//! server keeps the most recently used few and drops the others, so an id can
//! be unknown after an eviction or a restart, and the tools then say so and
//! ask for `open_model` again.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::UNIX_EPOCH;

use rvt::RevitFile;
use rvt::control::WalkerControl;
use rvt::elem_table::RecordFraming;
use rvt::ifc::step_writer::element_global_ids;
use rvt::ifc::{ExportDiagnostics, IfcModel};
use rvt::reader::UnreadStream;
use sha2::{Digest, Sha256};

use crate::config::{Config, Failure};
use crate::pipeline::{Pipeline, PipelineOptions, run_pipeline};
use crate::schema::{ElemTableProvenance, FileProvenance};

/// `bytes` as lowercase hex, two digits a byte.
pub fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
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

    /// The opaque handle: a hash of the identity, with no path in it.
    pub fn model_id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.path.to_string_lossy().as_bytes());
        hasher.update([0u8]);
        hasher.update(self.size.to_le_bytes());
        hasher.update(self.modified_nanos.to_le_bytes());
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
}

impl Computed {
    fn new(pipeline: Pipeline, file: FileProvenance) -> Self {
        let global_ids = element_global_ids(&pipeline.model);
        Self {
            model: pipeline.model,
            diagnostics: pipeline.diagnostics,
            unread: pipeline.unread,
            notes: pipeline.notes,
            global_ids,
            file,
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
    }
}

/// An open model: the file's handle, which keeps what the decode inflated,
/// and the default decode once it has run.
pub struct Entry {
    pub identity: Identity,
    pub id: String,
    file: Mutex<RevitFile>,
    default: Mutex<Option<Arc<Computed>>>,
}

impl Entry {
    fn new(identity: Identity, file: RevitFile) -> Self {
        let id = identity.model_id();
        Self {
            identity,
            id,
            file: Mutex::new(file),
            default: Mutex::new(None),
        }
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
        let pipeline = run_pipeline(&mut file, config, &PipelineOptions::default(), control)?;
        let provenance = file_provenance(&mut file);
        drop(file);
        let done = Arc::new(Computed::new(pipeline, provenance));
        *slot = Some(Arc::clone(&done));
        Ok(done)
    }

    /// Run `job` on the file's handle, for a decode with other switches.
    pub fn with_file<T>(&self, job: impl FnOnce(&mut RevitFile) -> T) -> T {
        let mut file = lock(&self.file);
        job(&mut file)
    }
}

/// The least recently used decoded models, most recent first.
pub struct Store {
    config: Config,
    entries: Mutex<Vec<Arc<Entry>>>,
}

impl Store {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The entry for `model`, a `model_id` or a path. A path that is not
    /// held is opened, which reads the file into memory.
    pub fn open(&self, model: &str) -> Result<Arc<Entry>, String> {
        let model = model.trim();
        if looks_like_id(model) {
            self.by_id(model)
        } else {
            self.by_path(model)
        }
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
            lock(&self.entries).retain(|held| held.id != entry.id);
            return Err(format!(
                "the file behind model_id {id} changed or was removed since open_model returned it. Call open_model again with the file path."
            ));
        }
        Ok(entry)
    }

    fn by_path(&self, path: &str) -> Result<Arc<Entry>, String> {
        if path.is_empty() {
            return Err("model is empty: pass a model_id from open_model, or the path of a .rvt, .rfa, .rte or .rft file".to_string());
        }
        let identity = Identity::of(Path::new(path))?;
        if let Some(entry) = self.touch(&identity) {
            return Ok(entry);
        }
        let file = RevitFile::open_with_limits(&identity.path, self.config.open_limits)
            .map_err(|error| error.to_string())?;
        let entry = Arc::new(Entry::new(identity, file));
        let mut entries = lock(&self.entries);
        // Another call may have opened it while this one read the file.
        if let Some(existing) = entries.iter().find(|held| held.identity == entry.identity) {
            return Ok(Arc::clone(existing));
        }
        // The same path with another size or time is the file's earlier self.
        entries.retain(|held| held.identity.path != entry.identity.path);
        entries.insert(0, Arc::clone(&entry));
        entries.truncate(self.config.max_open_models);
        Ok(entry)
    }

    /// The held entry of `identity`, made the most recent.
    fn touch(&self, identity: &Identity) -> Option<Arc<Entry>> {
        let mut entries = lock(&self.entries);
        let position = entries
            .iter()
            .position(|entry| entry.identity == *identity)?;
        let entry = entries.remove(position);
        entries.insert(0, Arc::clone(&entry));
        Some(entry)
    }
}
