//! High-level reader API. Opens a `.rvt` / `.rfa` / `.rte` / `.rft` file
//! and exposes its streams + parsed metadata.

use crate::{
    Error, Result,
    basic_file_info::BasicFileInfo,
    class_index, compression,
    part_atom::PartAtom,
    streams::{
        BASIC_FILE_INFO, CONTENTS, FORMATS_LATEST, GLOBAL_CONTENT_DOCUMENTS,
        GLOBAL_DOC_INCREMENT_TABLE, GLOBAL_ELEM_TABLE, GLOBAL_HISTORY, GLOBAL_LATEST,
        GLOBAL_PARTITION_TABLE, PART_ATOM, PROJECT_INFORMATION, REVIT_PREVIEW_4_0,
        TRANSMISSION_DATA,
    },
};
use cfb::CompoundFile;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
};

/// Default maximum file size accepted by [`RevitFile::open`].
///
/// 2 GiB is above any real-world Revit project we've observed
/// (typical: few MB–a few hundred MB; worksharing extreme: ~1 GiB)
/// and well below "pathological or hostile" territory. Callers with
/// specific larger-file needs should use [`RevitFile::open_with_limits`].
pub const DEFAULT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Default maximum stream size accepted by [`RevitFile::read_stream`].
///
/// 256 MiB per stream is comfortably above any observed stream. The
/// largest legitimate stream we've seen in the corpus is ~40 MiB
/// (Global/Latest on a large worksharing project). Hostile input
/// with a claimed huge stream size will be rejected before a
/// multi-GB allocation.
pub const DEFAULT_MAX_STREAM_BYTES: u64 = 256 * 1024 * 1024;

/// Limits applied when opening a Revit file. Protects against
/// pathological or hostile input that would otherwise force
/// unbounded memory allocation.
///
/// RVT is a file-upload target, and bounded resource
/// consumption is a DoS-safety requirement, not a nice-to-have.
#[derive(Debug, Clone, Copy)]
pub struct OpenLimits {
    /// Maximum file size accepted. File bytes are read into memory
    /// entirely on open (CFB requires random access), so this
    /// doubles as an upper bound on the initial allocation.
    pub max_file_bytes: u64,
    /// Maximum per-stream size accepted by [`RevitFile::read_stream`].
    /// Streams larger than this cause an error rather than a
    /// multi-GB alloc.
    pub max_stream_bytes: u64,
    /// Inflate limits applied to every `compression::inflate_at`
    /// call sourced from this `RevitFile`. Keeps bounded-decompression
    /// and open-limits consistent so a file opened under restrictive
    /// limits doesn't accidentally re-inflate under permissive ones.
    pub inflate_limits: compression::InflateLimits,
}

impl Default for OpenLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_stream_bytes: DEFAULT_MAX_STREAM_BYTES,
            inflate_limits: compression::InflateLimits::default(),
        }
    }
}

/// A stream a resource limit kept rvt-rs from reading whole (#1075). Every
/// reader that needed it went on without it, so what it holds is missing from
/// whatever the handle produced; [`RevitFile::unread_streams`] lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnreadStream {
    /// The stream, e.g. `Partitions/189`.
    pub name: String,
    /// Its stored size in bytes.
    pub stored_bytes: u64,
    /// Which limit stopped it.
    pub kind: UnreadKind,
    /// That limit in bytes.
    pub limit: u64,
}

/// One phase of a project: its ElementId and, where the file names it, its
/// name ([`RevitFile::project_phases`]).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct ProjectPhase {
    /// The phase's ElementId, which [`RevitFile::set_export_phase`] takes as
    /// well as its name.
    pub id: i64,
    /// The phase's name (`ProjectPhase.m_name`), if the file holds one.
    pub name: Option<String>,
}

/// Which resource limit stopped a stream ([`UnreadStream`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum UnreadKind {
    /// The stream is larger than [`OpenLimits::max_stream_bytes`] and was not
    /// read at all.
    StreamTooLarge,
    /// One of its gzip members would inflate past
    /// [`compression::InflateLimits::max_output_bytes`] and was left out.
    MemberTooLarge,
    /// Its members together would inflate past
    /// [`compression::InflateLimits::max_stream_total_bytes_for`]; those after
    /// the cap were left out.
    InflatedTotalTooLarge,
}

impl UnreadKind {
    /// Short machine-readable name, as the export diagnostics use it.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StreamTooLarge => "stream_too_large",
            Self::MemberTooLarge => "inflated_member_too_large",
            Self::InflatedTotalTooLarge => "inflated_total_too_large",
        }
    }
}

impl std::fmt::Display for UnreadStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = match self.kind {
            UnreadKind::StreamTooLarge => "is larger than the stream limit",
            UnreadKind::MemberTooLarge => {
                "has a compressed member that inflates past the inflate limit"
            }
            UnreadKind::InflatedTotalTooLarge => {
                "inflates past the limit on one stream's inflated total"
            }
        };
        write!(
            f,
            "stream {} ({} bytes stored) {what} of {} bytes, so what it holds is missing",
            self.name, self.stored_bytes, self.limit
        )
    }
}

/// Opened Revit file. Holds the CFB handle + cached stream bytes.
pub struct RevitFile {
    cfb: CompoundFile<Cursor<Vec<u8>>>,
    /// Limits to apply on subsequent reads. Copied from the
    /// `OpenLimits` passed at construction; defaults to
    /// `OpenLimits::default()` for back-compat `open`/`open_bytes`.
    limits: OpenLimits,
    /// Memoised inflate of multi-member streams, keyed by stream path.
    /// Every `Partitions/*` consumer (element records, arc walls, plan
    /// loops, level records, IFC export overrides, the partition
    /// scanner) used to read and inflate the same streams again, and a
    /// geometry export runs a dozen of those passes over ~180 MiB of
    /// inflated bytes. The streams are immutable for the lifetime of
    /// the handle — `RevitFile` never writes — so one inflate serves
    /// all of them.
    inflated: std::collections::HashMap<String, Arc<compression::InflatedStream>>,
    /// Memoised `Partitions/NN` string-record extraction. The UTF-16
    /// decode over the whole partition is the single most expensive
    /// step after inflate, and unit recovery, storey names and the
    /// partition MVP each asked for it independently.
    partition_strings: Option<Arc<Vec<crate::object_graph::StringRecord>>>,
    /// Memoised RE-35 second-prologue ElementIds. Every category sweep and
    /// the unattributed count read the same result, so the record chains
    /// are walked once per file.
    second_prologue_ids: Option<Arc<crate::partition_element_records::SecondPrologueIds>>,
    /// Memoised RE-38 family and type names.
    element_names: Option<Arc<crate::partition_names::ElementNames>>,
    /// Memoised design option sets and their primary options (RE-40).
    design_options: Option<Arc<crate::partition_design_options::DesignOptions>>,
    /// Memoised phase order and the elements not in the export phase (#328).
    phase_filter: Option<Arc<crate::native_phases::PhaseFilter>>,
    /// The ElementId of the phase [`RevitFile::set_export_phase`] chose.
    chosen_export_phase: Option<i64>,
    /// The Areas of a Revit 2015 to 2023 file the reader left unwritten, by
    /// why (RE-324), set when the Areas are read.
    unwritten_areas: BTreeMap<String, Vec<crate::partition_element_records::LeftOutElement>>,
    /// What became of each frame at the markers of a Revit 2008 to 2023 file
    /// (#1389), set when its element records are recovered.
    element_record_stages: Option<crate::partition_element_records_2023::RecordStages>,
    /// Whether the recovery lists the frames that stop, for the log of
    /// decisions ([`Self::record_frame_decisions`], #1389).
    frame_decisions: bool,
    /// Memoised production walk: the elements
    /// [`crate::walker::iter_elements_with_control`] yields for a minimum
    /// score and walker limits. An export walks once to write and again for
    /// its diagnostics' class counts; the second reuses the first.
    // Tracker: B71.
    walk: Option<CachedWalk>,
    /// Memoised partition ArcWall scan for a release and walker limits: the
    /// production walk, the partition MVP and the IFC export (twice) each ran
    /// the same scan over every partition.
    arc_wall_scan: Option<CachedArcWallScan>,
    /// Where an export in progress reports each `Partitions/*` stream it
    /// inflates, and how many there are ([`Self::report_partition_inflates_to`]).
    /// Held as `AssertUnwindSafe` so the handle keeps the unwind-safety it
    /// has without one: a control wraps a caller's closure, which is neither.
    partition_progress: Option<(
        std::panic::AssertUnwindSafe<crate::control::WalkerControl>,
        u64,
    )>,
    /// Memoised data objects (RE-153) of each partition stream: the
    /// original symbols, railing types and MEP systems each scanned every
    /// partition for them.
    // Tracker: B80.
    data_objects: std::collections::HashMap<String, PartitionDataObjects>,
    /// Memoised offsets of an element-data header in a partition, by stream
    /// and header: a dozen element-data readers each searched every partition
    /// for the release's header.
    header_hits: std::collections::HashMap<(String, Vec<u8>), Arc<Vec<usize>>>,
    /// Memoised native extraction preamble: each native consumer read
    /// the schema, the element index and the increments again.
    // Tracker: B82.
    native_preamble: Option<Arc<crate::native_document::Preamble>>,
    /// Memoised parameter-definition pass of native extraction, with the
    /// options it was read under (#1237).
    native_definitions: Option<(
        crate::native_document::DefinitionKey,
        Arc<crate::native_document::DefinitionPass>,
    )>,
    /// Memoised index of every native record's partitions (#1032): each
    /// native pass walked every partition again to find its records.
    native_record_index: Option<Arc<crate::native_document::RecordIndex>>,
    /// Memoised `Formats/Latest` classes: a dozen partition readers each
    /// read and parsed the schema again for a few class tags.
    schema_classes_memo: Option<Arc<crate::formats::SchemaClasses>>,
    layout_release_memo: Option<u32>,
    /// Memoised material names (RE-58, RE-203), by release and the hash of
    /// the declared ids they were read for: half a dozen material steps of
    /// an export each asked for them.
    material_names: std::collections::HashMap<(u32, u64), Arc<BTreeMap<u32, String>>>,
    /// Memoised material appearances, keyed as `material_names` is:
    /// the material, wall and layer steps of an export each ask for them.
    material_appearances: std::collections::HashMap<
        (u32, u64),
        Arc<BTreeMap<u32, crate::partition_materials::MaterialAppearance>>,
    >,
    /// Memoised materials each category's graphics styles name (RE-206).
    category_style_materials: Option<Arc<BTreeMap<i64, BTreeSet<u32>>>>,
    /// Streams a resource limit kept from being read whole (#1075).
    unread_streams: Vec<UnreadStream>,
    /// Doors and windows written as such though their type's record was not
    /// read (#1247).
    unread_type_doors_windows: BTreeSet<u32>,
    /// Memoised steps of the walls' native records that name a cutting element
    /// (`research::native_wall_cuts::wall_cut_steps`), by wall, and the walls
    /// whose record was looked for, found or not: the openings the walls
    /// carry and the walls drawn from their saved mesh (RE-340) each asked for
    /// the same records.
    pub(crate) wall_cut_steps_memo: BTreeMap<u64, Vec<crate::native_wall_cuts::WallCutStep>>,
    pub(crate) wall_cut_steps_asked: BTreeSet<u64>,
    /// Family instances written as spaces that were left out because their
    /// family draws no model form (RE-312, #1262), by class.
    bodiless_family_spaces: BTreeMap<String, Vec<crate::partition_element_records::LeftOutElement>>,
    /// How many family instances written as spaces were not checked for a 3D
    /// body, and why, where the check could not run (RE-312).
    bodiless_family_space_error: Option<(usize, String)>,
    /// The copy of an element's data each reader took a value from, by
    /// ElementId, noted where the reader chose it (#1360). `None` unless the
    /// export asked for the provenance table: the readers then note nothing.
    value_sources: Option<crate::ifc::element_provenance::ValueSourceNotes>,
}

/// The limits it was opened with and how many streams it has inflated so
/// far; the container's bytes and the caches are left out.
impl std::fmt::Debug for RevitFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RevitFile")
            .field("limits", &self.limits)
            .field("inflated_streams", &self.inflated.len())
            .finish_non_exhaustive()
    }
}

/// A partition stream's data objects, each at its offset (RE-153).
#[cfg(feature = "unstable-research")]
pub type PartitionDataObjects = Arc<Vec<(usize, crate::partition_room_parameters::DataObject)>>;
#[cfg(not(feature = "unstable-research"))]
pub(crate) type PartitionDataObjects =
    Arc<Vec<(usize, crate::partition_room_parameters::DataObject)>>;

/// A memoised walk: its minimum score, its limits and its elements.
type CachedWalk = (
    i64,
    crate::walker::WalkerLimits,
    Arc<Vec<crate::walker::DecodedElement>>,
);

/// A memoised ArcWall scan: its release, its limits and its result.
type CachedArcWallScan = (
    u32,
    crate::walker::WalkerLimits,
    Arc<crate::partition_arc_walls::PartitionArcWallScan>,
);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Summary {
    pub file_size: u64,
    pub streams: Vec<String>,
    pub version: u32,
    pub build: Option<String>,
    pub original_path: Option<String>,
    pub guid: Option<String>,
    pub locale: Option<String>,
    pub partition_stream: Option<String>,
    pub partatom: Option<PartAtom>,
    pub class_name_count: usize,
    pub class_name_sample: Vec<String>,
}

impl RevitFile {
    /// Open a Revit file from disk, under the default [`OpenLimits`].
    ///
    /// # Errors
    ///
    /// [`Error::Io`] when the file cannot be read (its message names the
    /// path), [`Error::NotACfbFile`] when it does not start with the OLE2 /
    /// MS-CFB magic bytes (`D0 CF 11 E0 A1 B1 1A E1`), [`Error::TooLarge`]
    /// when it is larger than [`OpenLimits::max_file_bytes`], and
    /// [`Error::Cfb`] when its container does not open.
    ///
    /// # Examples
    ///
    /// ```
    /// use rvt::RevitFile;
    ///
    /// let mut rf = RevitFile::open("corpus/tier1/architectural-2024/architectural-2024.rvt")?;
    /// let summary = rf.summarize_strict()?;
    /// println!("Revit {}", summary.version);
    /// # Ok::<(), rvt::Error>(())
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_limits(path, OpenLimits::default())
    }

    /// Open a Revit file from disk with explicit resource limits.
    ///
    /// Stats the file before reading; refuses if file size exceeds
    /// `limits.max_file_bytes` to prevent multi-GB allocations from
    /// a hostile input. Back-compat [`Self::open`] calls this with
    /// `OpenLimits::default()` (2 GiB file, 256 MiB per stream, 256
    /// MiB per inflate).
    ///
    /// ```
    /// use rvt::reader::{OpenLimits, RevitFile};
    ///
    /// // Only accept files up to 100 MB.
    /// let limits = OpenLimits {
    ///     max_file_bytes: 100 * 1024 * 1024,
    ///     ..OpenLimits::default()
    /// };
    /// let path = "corpus/tier1/architectural-2024/architectural-2024.rvt";
    /// let rf = RevitFile::open_with_limits(path, limits)?;
    /// assert!(!rf.stream_names().is_empty());
    ///
    /// // A file over the limit is refused before it is read.
    /// let tiny = OpenLimits { max_file_bytes: 1024, ..OpenLimits::default() };
    /// assert!(matches!(
    ///     RevitFile::open_with_limits(path, tiny),
    ///     Err(rvt::Error::TooLarge { .. })
    /// ));
    /// # Ok::<(), rvt::Error>(())
    /// ```
    pub fn open_with_limits(path: impl AsRef<Path>, limits: OpenLimits) -> Result<Self> {
        let path = path.as_ref();
        // Name the file in I/O errors: a batch run over a folder otherwise
        // reports a bare "No such file or directory". The `ErrorKind` is kept
        // so callers (and the Python bindings) can still match on it.
        let with_path = |e: std::io::Error| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", path.display()),
            ))
        };
        let metadata = std::fs::metadata(path).map_err(with_path)?;
        if metadata.is_dir() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::IsADirectory,
                format!(
                    "{}: is a directory, not a .rvt / .rfa / .rte / .rft file",
                    path.display()
                ),
            )));
        }
        if metadata.len() > limits.max_file_bytes {
            return Err(Error::TooLarge {
                what: "the file".into(),
                size: metadata.len(),
                limit: limits.max_file_bytes,
            });
        }
        let mut f = File::open(path).map_err(with_path)?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        f.read_to_end(&mut bytes).map_err(with_path)?;
        Self::open_bytes_with_limits(bytes, limits).map_err(|e| e.in_file(path))
    }

    /// Open a Revit file from an in-memory byte buffer.
    ///
    /// Useful for callers that have the file bytes already (e.g. streamed
    /// over the network). Equivalent to `open` after a `read_to_end`.
    ///
    /// # Errors
    ///
    /// [`Error::NotACfbFile`] when the bytes do not start with the OLE2 /
    /// MS-CFB magic bytes, [`Error::TooLarge`] when they exceed
    /// [`OpenLimits::max_file_bytes`], and [`Error::Cfb`] when the container
    /// does not open.
    ///
    /// # Examples
    ///
    /// ```
    /// use rvt::RevitFile;
    /// // Four bytes that are definitely not a valid CFB file.
    /// let result = RevitFile::open_bytes(b"nope".to_vec());
    /// assert!(matches!(result, Err(rvt::Error::NotACfbFile)));
    /// ```
    pub fn open_bytes(bytes: Vec<u8>) -> Result<Self> {
        Self::open_bytes_with_limits(bytes, OpenLimits::default())
    }

    /// Open-bytes variant with explicit limits. The byte count check
    /// has already been done by the caller if they came through
    /// [`Self::open_with_limits`]; here it's repeated for in-memory
    /// paths that skip disk stat.
    ///
    /// ```
    /// use rvt::reader::{OpenLimits, RevitFile};
    ///
    /// let bytes = std::fs::read("corpus/tier1/architectural-2024/architectural-2024.rvt")?;
    /// let rf = RevitFile::open_bytes_with_limits(bytes.clone(), OpenLimits::default())?;
    /// assert!(!rf.stream_names().is_empty());
    ///
    /// // Bytes over the limit are refused before they are parsed.
    /// let tiny = OpenLimits { max_file_bytes: 1024, ..OpenLimits::default() };
    /// assert!(matches!(
    ///     RevitFile::open_bytes_with_limits(bytes, tiny),
    ///     Err(rvt::Error::TooLarge { .. })
    /// ));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn open_bytes_with_limits(bytes: Vec<u8>, limits: OpenLimits) -> Result<Self> {
        if (bytes.len() as u64) > limits.max_file_bytes {
            return Err(Error::TooLarge {
                what: "the in-memory buffer".into(),
                size: bytes.len() as u64,
                limit: limits.max_file_bytes,
            });
        }
        if bytes.len() < 8 || bytes[..8] != [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
            return Err(Error::NotACfbFile);
        }
        let cfb = CompoundFile::open(Cursor::new(bytes)).map_err(|e| Error::Cfb {
            op: "open",
            path: None,
            source: e,
        })?;
        Ok(Self {
            cfb,
            limits,
            inflated: std::collections::HashMap::new(),
            partition_strings: None,
            second_prologue_ids: None,
            element_names: None,
            design_options: None,
            phase_filter: None,
            chosen_export_phase: None,
            unwritten_areas: BTreeMap::new(),
            element_record_stages: None,
            frame_decisions: false,
            walk: None,
            arc_wall_scan: None,
            partition_progress: None,
            data_objects: std::collections::HashMap::new(),
            header_hits: std::collections::HashMap::new(),
            native_preamble: None,
            native_definitions: None,
            native_record_index: None,
            schema_classes_memo: None,
            layout_release_memo: None,
            material_names: std::collections::HashMap::new(),
            material_appearances: std::collections::HashMap::new(),
            category_style_materials: None,
            unread_streams: Vec::new(),
            unread_type_doors_windows: BTreeSet::new(),
            wall_cut_steps_memo: BTreeMap::new(),
            wall_cut_steps_asked: BTreeSet::new(),
            bodiless_family_spaces: BTreeMap::new(),
            bodiless_family_space_error: None,
            value_sources: None,
        })
    }

    /// The streams a resource limit kept from being read whole since this
    /// handle was opened, in the order they were met (#1075). Empty when
    /// nothing was cut; when not, the readers went on without them, so
    /// anything produced from this handle is missing what they hold.
    pub fn unread_streams(&self) -> &[UnreadStream] {
        &self.unread_streams
    }

    /// The doors and windows this handle's export wrote as doors and
    /// windows though their type's record was not read (#1247), by
    /// ElementId: whether each draws geometry, or is its opening alone in
    /// Revit's export as a cased opening is, is not known.
    pub(crate) fn unread_type_doors_windows(&self) -> &BTreeSet<u32> {
        &self.unread_type_doors_windows
    }

    pub(crate) fn note_unread_type_doors_windows(&mut self, ids: impl IntoIterator<Item = u32>) {
        self.unread_type_doors_windows.extend(ids);
    }

    /// What became of each frame at the markers of this Revit 2008 to 2023
    /// file in the recovery of its element records (#1389); `None` where that
    /// recovery has not run, or the file takes another layout.
    pub(crate) fn element_record_stages(
        &self,
    ) -> Option<&crate::partition_element_records_2023::RecordStages> {
        self.element_record_stages.as_ref()
    }

    pub(crate) fn note_element_record_stages(
        &mut self,
        stages: crate::partition_element_records_2023::RecordStages,
    ) {
        self.element_record_stages = Some(stages);
    }

    /// Note that the walk dropped the element the record recovery handed under
    /// `id`, because an element with that ElementId was already in it (#1389).
    pub(crate) fn note_id_already_held(&mut self, id: u32) {
        if let Some(stages) = self.element_record_stages.as_mut() {
            stages.note_held(id);
        }
    }

    /// Start or stop listing, as the recovery of a Revit 2008 to 2023 file's
    /// element records goes, each frame that stops (#1389), which the log of
    /// decisions holds. The counts need no list and are kept either way.
    /// Starting drops the memoised production walk, since a walk made without
    /// the list has none to give.
    pub(crate) fn record_frame_decisions(&mut self, on: bool) {
        if on && !self.frame_decisions {
            self.walk = None;
        }
        self.frame_decisions = on;
    }

    /// Whether the recovery lists the frames that stop
    /// ([`Self::record_frame_decisions`]).
    pub(crate) fn recording_frame_decisions(&self) -> bool {
        self.frame_decisions
    }

    /// The family instances written as spaces that this handle's export left
    /// out because their family draws no model form (RE-312), by class, each
    /// with the frame it was read from (#1326).
    pub(crate) fn bodiless_family_spaces(
        &self,
    ) -> &BTreeMap<String, Vec<crate::partition_element_records::LeftOutElement>> {
        &self.bodiless_family_spaces
    }

    pub(crate) fn note_bodiless_family_spaces(
        &mut self,
        classes: BTreeMap<String, Vec<crate::partition_element_records::LeftOutElement>>,
    ) {
        self.bodiless_family_spaces = classes;
    }

    /// Forget what an earlier production walk noted for its diagnostics: the
    /// Areas left unwritten, the family spaces left out for want of a body,
    /// the error that stopped that check, and the doors and windows written
    /// though their type was not read. The partition recovery notes them, and
    /// only when it finds some, so each walk starts from none: the notes
    /// follow the memoised walk that produced them.
    pub(crate) fn reset_export_notes(&mut self) {
        self.element_record_stages = None;
        self.unwritten_areas.clear();
        self.bodiless_family_spaces.clear();
        self.bodiless_family_space_error = None;
        self.unread_type_doors_windows.clear();
        if let Some(noted) = self.value_sources.as_mut() {
            noted.clear();
        }
    }

    /// Whether the readers note the copy they took each value from
    /// ([`Self::record_value_sources`]).
    pub(crate) fn recording_value_sources(&self) -> bool {
        self.value_sources.is_some()
    }

    /// Start or stop noting the copy each reader takes a value from (#1360),
    /// which the provenance table lists. Starting drops the memoised
    /// production walk, since a walk made while the readers noted nothing has
    /// no notes to give; stopping drops the notes. A reader that is not asked
    /// notes nothing, so the default export does no extra work.
    pub(crate) fn record_value_sources(&mut self, on: bool) {
        if on == self.value_sources.is_some() {
            return;
        }
        if on {
            self.value_sources = Some(BTreeMap::new());
            self.walk = None;
        } else {
            self.value_sources = None;
        }
    }

    /// Note that a reader took a value of the element `element_id` from
    /// `source`, in place of an earlier note of the same reader's. Does
    /// nothing unless [`Self::record_value_sources`] is on.
    pub(crate) fn note_value_source(
        &mut self,
        element_id: u32,
        source: crate::ifc::element_provenance::ValueSource,
    ) {
        let Some(noted) = self.value_sources.as_mut() else {
            return;
        };
        let sources = noted.entry(element_id).or_default();
        match sources.iter_mut().find(|held| held.reader == source.reader) {
            Some(held) => *held = source,
            None => sources.push(source),
        }
    }

    /// Note for `owner`, under `as_reader`, the value source that `from` has
    /// under `reader`, where the record was read once for `from` and `owner`
    /// uses it: a type's parameters, read for the instances of the type
    /// (#1368). The record is `from`'s, so it is listed in `parts` with
    /// `from`'s id. Does nothing unless [`Self::record_value_sources`] is on
    /// and `from` has one.
    pub(crate) fn attribute_value_source(
        &mut self,
        from: u32,
        reader: &str,
        owner: u32,
        as_reader: &str,
    ) {
        let source = self
            .value_sources
            .as_ref()
            .and_then(|noted| noted.get(&from))
            .and_then(|sources| sources.iter().find(|held| held.reader == reader))
            .cloned();
        if let Some(mut source) = source {
            source.reader = as_reader.to_string();
            // The record is the other element's: name it in `parts`.
            if let (true, Some(stream), Some(offset)) = (
                source.parts.is_empty(),
                source.stream.take(),
                source.offset.take(),
            ) {
                source
                    .parts
                    .push(crate::ifc::element_provenance::SourcePart {
                        element_id: from,
                        stream,
                        offset,
                    });
            }
            self.note_value_source(owner, source);
        }
    }

    /// Take back the note of `reader` for `element_id`, where the value was
    /// not used after all (a record selected, then found not to give the
    /// value). Does nothing unless [`Self::record_value_sources`] is on.
    pub(crate) fn forget_value_source(&mut self, element_id: u32, reader: &str) {
        if let Some(sources) = self
            .value_sources
            .as_mut()
            .and_then(|noted| noted.get_mut(&element_id))
        {
            sources.retain(|held| held.reader != reader);
        }
    }

    /// The notes of [`Self::note_value_source`], by ElementId; `None` unless
    /// [`Self::record_value_sources`] is on.
    pub(crate) fn value_sources(
        &self,
    ) -> Option<&crate::ifc::element_provenance::ValueSourceNotes> {
        self.value_sources.as_ref()
    }

    /// How many family instances written as spaces were not checked for a 3D
    /// body, and why (RE-312).
    pub(crate) fn bodiless_family_space_error(&self) -> Option<&(usize, String)> {
        self.bodiless_family_space_error.as_ref()
    }

    pub(crate) fn note_bodiless_family_space_error(&mut self, checked: usize, error: String) {
        self.bodiless_family_space_error = Some((checked, error));
    }

    fn note_unread(&mut self, unread: UnreadStream) {
        if !self
            .unread_streams
            .iter()
            .any(|known| known.name == unread.name && known.kind == unread.kind)
        {
            self.unread_streams.push(unread);
        }
    }

    /// Record `err` against stream `name` when it is an inflate stopped by a
    /// limit, for readers that inflate a stream themselves.
    pub(crate) fn note_inflate_error(&mut self, name: &str, stored_bytes: usize, err: &Error) {
        if matches!(err, Error::DecompressLimitExceeded(_)) {
            self.note_unread(UnreadStream {
                name: name.to_string(),
                stored_bytes: stored_bytes as u64,
                kind: UnreadKind::MemberTooLarge,
                limit: self.limits.inflate_limits.max_output_bytes as u64,
            });
        }
    }

    /// Resource limits this file was opened under. Use to match
    /// the limits when calling bounded-inflate on extracted stream
    /// bytes.
    pub fn limits(&self) -> OpenLimits {
        self.limits
    }

    /// Read and inflate every gzip member of `name`, memoised.
    ///
    /// Repeat calls return the same shared buffer instead of reading
    /// the CFB stream and inflating it again. Use this for any
    /// multi-member stream a pipeline touches more than once —
    /// `Partitions/*` above all, where a geometry export otherwise
    /// inflates the same ~180 MiB a dozen times over.
    pub fn inflated_partition(&mut self, name: &str) -> Result<Arc<compression::InflatedStream>> {
        if let Some(cached) = self.inflated.get(name) {
            return Ok(Arc::clone(cached));
        }
        let raw = self.read_stream(name)?;
        let stored_len = raw.len();
        let limits = self.limits.inflate_limits;
        // The stored bytes are stripped where they are and dropped once the
        // members are inflated, so the stream is held once beside its
        // inflated form, not twice (#1075).
        let (inflated, shortfall) = {
            let prepared = compression::prepare_stream_owned(name, raw);
            compression::InflatedStream::from_prepared_with_limits(&prepared, stored_len, limits)
        };
        if shortfall.members_over_limit > 0 {
            self.note_unread(UnreadStream {
                name: name.to_string(),
                stored_bytes: stored_len as u64,
                kind: UnreadKind::MemberTooLarge,
                limit: limits.max_output_bytes as u64,
            });
        }
        if shortfall.stopped_at_total {
            self.note_unread(UnreadStream {
                name: name.to_string(),
                stored_bytes: stored_len as u64,
                kind: UnreadKind::InflatedTotalTooLarge,
                limit: limits.max_stream_total_bytes_for(stored_len) as u64,
            });
        }
        let inflated = Arc::new(inflated);
        self.inflated
            .insert(name.to_string(), Arc::clone(&inflated));
        if let Some((control, total)) = &self.partition_progress {
            let done = self
                .inflated
                .keys()
                .filter(|key| key.starts_with("Partitions/"))
                .count() as u64;
            control.report(
                crate::control::Stage::PartitionScan,
                done.min(*total),
                Some(*total),
            );
        }
        Ok(inflated)
    }

    /// From now until [`Self::stop_reporting_partition_inflates`], reports to
    /// `control` each time a `Partitions/*` stream is inflated for the first
    /// time, as [`crate::control::Stage::PartitionScan`] with the number
    /// inflated so far out of the number of partition streams. Whichever pass
    /// reads the partitions first inflates them all, in turn, and on a project
    /// that is most of an open's time, in passes that have no control of their
    /// own.
    pub(crate) fn report_partition_inflates_to(&mut self, control: &crate::control::WalkerControl) {
        let total = self.partition_stream_names().len() as u64;
        self.partition_progress = Some((std::panic::AssertUnwindSafe(control.clone()), total));
    }

    pub(crate) fn stop_reporting_partition_inflates(&mut self) {
        self.partition_progress = None;
    }

    /// The data objects (RE-153) of partition stream `name`, each at its
    /// offset in the stream's inflated bytes ([`Self::inflated_partition`]),
    /// memoised.
    // Tracker: B80.
    #[cfg(feature = "unstable-research")]
    pub fn partition_data_objects(&mut self, name: &str) -> Result<PartitionDataObjects> {
        self.memoised_partition_data_objects(name)
    }

    /// `partition_data_objects` for the crate's own readers, with or without
    /// `unstable-research`.
    pub(crate) fn memoised_partition_data_objects(
        &mut self,
        name: &str,
    ) -> Result<PartitionDataObjects> {
        if let Some(cached) = self.data_objects.get(name) {
            return Ok(Arc::clone(cached));
        }
        let inflated = self.inflated_partition(name)?;
        let objects = Arc::new(crate::partition_room_parameters::data_objects(
            inflated.bytes(),
        ));
        self.data_objects
            .insert(name.to_string(), Arc::clone(&objects));
        Ok(objects)
    }

    /// Stored stream `name` as the inflate and the native readers take it, its
    /// checksum pages stripped where the stream has them
    /// ([`compression::prepare_stream_for_inflate`]): one buffer, stripped in
    /// place, not the stored bytes and a stripped copy of them (#1075).
    pub(crate) fn read_prepared_stream(&mut self, name: &str) -> Result<Vec<u8>> {
        Ok(compression::prepare_stream_owned(
            name,
            self.read_stream(name)?,
        ))
    }

    /// Every offset of `header` in partition `name`'s inflated bytes, in
    /// order, memoised.
    pub(crate) fn header_hits(&mut self, name: &str, header: &[u8]) -> Result<Arc<Vec<usize>>> {
        let key = (name.to_string(), header.to_vec());
        if let Some(cached) = self.header_hits.get(&key) {
            return Ok(Arc::clone(cached));
        }
        let inflated = self.inflated_partition(name)?;
        let hits =
            Arc::new(memchr::memmem::find_iter(inflated.bytes(), header).collect::<Vec<_>>());
        self.header_hits.insert(key, Arc::clone(&hits));
        Ok(hits)
    }

    /// What every native extraction reads before the partitions
    /// (`crate::native_document::Preamble`), memoised.
    // Tracker: B82.
    pub(crate) fn native_preamble(
        &mut self,
    ) -> anyhow::Result<Arc<crate::native_document::Preamble>> {
        if let Some(cached) = &self.native_preamble {
            return Ok(Arc::clone(cached));
        }
        let preamble = Arc::new(crate::native_document::read_preamble(self)?);
        self.native_preamble = Some(Arc::clone(&preamble));
        Ok(preamble)
    }

    /// The parameter-definition pass read under `key`, if one has been.
    pub(crate) fn native_definitions(
        &self,
        key: &crate::native_document::DefinitionKey,
    ) -> Option<Arc<crate::native_document::DefinitionPass>> {
        self.native_definitions
            .as_ref()
            .filter(|(held, _)| held == key)
            .map(|(_, pass)| Arc::clone(pass))
    }

    /// Drop the kept parameter-definition pass, whose registry an MEP model
    /// makes tens of MiB: the export does once it has made its last
    /// extraction, so a file read once keeps nothing.
    pub(crate) fn forget_native_definitions(&mut self) {
        self.native_definitions = None;
    }

    /// Keep a parameter-definition pass for later extractions under `key`.
    pub(crate) fn remember_native_definitions(
        &mut self,
        key: crate::native_document::DefinitionKey,
        pass: Arc<crate::native_document::DefinitionPass>,
    ) {
        self.native_definitions = Some((key, pass));
    }

    /// Where every native record is (`crate::native_document::RecordIndex`),
    /// once a native pass has recorded it (#1032).
    pub(crate) fn native_record_index(&self) -> Option<Arc<crate::native_document::RecordIndex>> {
        self.native_record_index.clone()
    }

    /// Keep the record index a native pass recorded (#1032).
    pub(crate) fn remember_native_record_index(
        &mut self,
        index: crate::native_document::RecordIndex,
    ) {
        self.native_record_index = Some(Arc::new(index));
    }

    /// ElementIds `research::partition_element_records::assign_second_prologue_ids`
    /// reads for second-prologue frames from their enclosing partition
    /// records, by stream and frame offset, memoised (RE-35).
    #[cfg(feature = "unstable-research")]
    pub fn second_prologue_ids(
        &mut self,
    ) -> Arc<crate::partition_element_records::SecondPrologueIds> {
        self.memoised_second_prologue_ids()
    }

    /// `second_prologue_ids` for the crate's own readers, with or without
    /// `unstable-research`.
    pub(crate) fn memoised_second_prologue_ids(
        &mut self,
    ) -> Arc<crate::partition_element_records::SecondPrologueIds> {
        if let Some(cached) = &self.second_prologue_ids {
            return Arc::clone(cached);
        }
        let computed =
            Arc::new(crate::partition_element_records::compute_second_prologue_ids(self));
        self.second_prologue_ids = Some(Arc::clone(&computed));
        computed
    }

    /// The memoised material names for `key` (RE-203).
    pub(crate) fn cached_material_names(
        &self,
        key: (u32, u64),
    ) -> Option<Arc<BTreeMap<u32, String>>> {
        self.material_names.get(&key).cloned()
    }

    /// Memoise the material names for `key` (RE-203).
    pub(crate) fn cache_material_names(
        &mut self,
        key: (u32, u64),
        names: Arc<BTreeMap<u32, String>>,
    ) {
        self.material_names.insert(key, names);
    }

    /// The memoised material appearances for `key`.
    pub(crate) fn cached_material_appearances(
        &self,
        key: (u32, u64),
    ) -> Option<Arc<BTreeMap<u32, crate::partition_materials::MaterialAppearance>>> {
        self.material_appearances.get(&key).cloned()
    }

    /// Memoise the material appearances for `key`.
    pub(crate) fn cache_material_appearances(
        &mut self,
        key: (u32, u64),
        appearances: Arc<BTreeMap<u32, crate::partition_materials::MaterialAppearance>>,
    ) {
        self.material_appearances.insert(key, appearances);
    }

    /// The memoised materials each category's graphics styles name (RE-206).
    pub(crate) fn cached_category_style_materials(
        &self,
    ) -> Option<Arc<BTreeMap<i64, BTreeSet<u32>>>> {
        self.category_style_materials.clone()
    }

    /// Memoise the materials each category's graphics styles name (RE-206).
    pub(crate) fn cache_category_style_materials(
        &mut self,
        materials: Arc<BTreeMap<i64, BTreeSet<u32>>>,
    ) {
        self.category_style_materials = Some(materials);
    }

    /// Family and type names from the partition name entries, with each
    /// named type's family candidates, memoised (RE-38). Empty where the
    /// release is not proven or a stream does not parse.
    #[cfg(feature = "unstable-research")]
    pub fn element_names(&mut self) -> Arc<crate::partition_names::ElementNames> {
        self.memoised_element_names()
    }

    /// `element_names` for the crate's own readers, with or without
    /// `unstable-research`.
    pub(crate) fn memoised_element_names(&mut self) -> Arc<crate::partition_names::ElementNames> {
        if let Some(cached) = &self.element_names {
            return Arc::clone(cached);
        }
        let computed =
            Arc::new(crate::partition_names::compute_element_names(self).unwrap_or_default());
        self.element_names = Some(Arc::clone(&computed));
        computed
    }

    /// The partition ArcWall scan of `revit_version` under `limits`, run on
    /// the first request and shared after (the scan reads only the file, the
    /// release and the limits).
    pub(crate) fn memoised_arc_wall_scan(
        &mut self,
        revit_version: u32,
        limits: crate::walker::WalkerLimits,
    ) -> Arc<crate::partition_arc_walls::PartitionArcWallScan> {
        if let Some((version, held, scan)) = &self.arc_wall_scan {
            if *version == revit_version && *held == limits {
                return Arc::clone(scan);
            }
        }
        let computed = Arc::new(crate::partition_arc_walls::compute_partition_arc_walls(
            self,
            revit_version,
            limits,
        ));
        self.arc_wall_scan = Some((revit_version, limits, Arc::clone(&computed)));
        computed
    }

    /// The elements a production walk with `min_score` and `limits` yielded
    /// on this file, when one ran.
    // Tracker: B71.
    pub(crate) fn cached_walk(
        &self,
        min_score: i64,
        limits: crate::walker::WalkerLimits,
    ) -> Option<Arc<Vec<crate::walker::DecodedElement>>> {
        self.walk
            .as_ref()
            .filter(|(score, held, _)| *score == min_score && *held == limits)
            .map(|(_, _, elements)| Arc::clone(elements))
    }

    /// Keep a production walk's elements for later walks with the same
    /// `min_score` and `limits`.
    // Tracker: B71.
    pub(crate) fn store_walk(
        &mut self,
        min_score: i64,
        limits: crate::walker::WalkerLimits,
        elements: Arc<Vec<crate::walker::DecodedElement>>,
    ) {
        self.walk = Some((min_score, limits, elements));
    }

    /// Design option sets and their primary options, memoised (RE-40).
    /// Empty where the release is not proven or a stream does not parse.
    #[cfg(feature = "unstable-research")]
    pub fn design_options(&mut self) -> Arc<crate::partition_design_options::DesignOptions> {
        self.memoised_design_options()
    }

    /// `design_options` for the crate's own readers, with or without
    /// `unstable-research`.
    pub(crate) fn memoised_design_options(
        &mut self,
    ) -> Arc<crate::partition_design_options::DesignOptions> {
        if let Some(cached) = &self.design_options {
            return Arc::clone(cached);
        }
        let computed = Arc::new(
            crate::partition_design_options::compute_design_options(self).unwrap_or_default(),
        );
        self.design_options = Some(Arc::clone(&computed));
        computed
    }

    /// The project's phase order and the elements Revit's export leaves
    /// out by phase, memoised (#328, RE-184). Empty where the native record
    /// path does not read the release or the file lists no phases, and
    /// empty with its `read_error` set where it reads the release and the
    /// read failed.
    #[cfg(feature = "unstable-research")]
    pub fn phase_filter(&mut self) -> Arc<crate::native_phases::PhaseFilter> {
        self.memoised_phase_filter()
    }

    /// `phase_filter` for the crate's own readers, with or without
    /// `unstable-research`.
    pub(crate) fn memoised_phase_filter(&mut self) -> Arc<crate::native_phases::PhaseFilter> {
        if let Some(cached) = &self.phase_filter {
            return Arc::clone(cached);
        }
        let computed = Arc::new(
            crate::native_phases::compute_phase_filter(self).unwrap_or_else(|error| {
                crate::native_phases::PhaseFilter {
                    read_error: Some(format!("{error:#}")),
                    ..Default::default()
                }
            }),
        );
        self.phase_filter = Some(Arc::clone(&computed));
        computed
    }

    /// The project's phases in order, the last being the one an export shows
    /// unless [`Self::set_export_phase`] chooses another. Empty where the
    /// export applies no phase: a family, a release whose phases are not read
    /// (Revit 2015 and earlier), or a file whose phases could not be read.
    /// Reads nothing the export does not.
    pub fn project_phases(&mut self) -> Vec<ProjectPhase> {
        if let Ok(release) = self.layout_release() {
            if crate::partition_element_records_2023::is_32bit_release(release)
                && !crate::partition_element_records_2023::applies_export_filters(release)
            {
                return Vec::new();
            }
        }
        let filter = self.memoised_phase_filter();
        if filter.read_error.is_some() {
            return Vec::new();
        }
        filter
            .order
            .iter()
            .map(|id| ProjectPhase {
                id: *id,
                name: filter.names.get(id).cloned(),
            })
            .collect()
    }

    /// The phase whose rooms an export with no phase named writes in place of
    /// the project's last phase's, which holds none (#1359, Griffin,
    /// 2026-11-05): the latest phase that holds a room. `None` where the last
    /// phase holds a room, where no phase does, where a phase is named
    /// ([`Self::set_export_phase`], the last included, is exact), and where
    /// the export applies no phase ([`Self::project_phases`] is empty). Every
    /// other element still follows the last phase.
    pub fn room_phase_fallback(&mut self) -> Option<ProjectPhase> {
        if let Ok(release) = self.layout_release() {
            if crate::partition_element_records_2023::is_32bit_release(release)
                && !crate::partition_element_records_2023::applies_export_filters(release)
            {
                return None;
            }
        }
        let filter = self.memoised_phase_filter();
        if filter.read_error.is_some() {
            return None;
        }
        let id = filter.room_fallback_in_effect()?;
        Some(ProjectPhase {
            id,
            name: filter.names.get(&id).cloned(),
        })
    }

    /// Choose the phase the export shows, by the phase's name (exactly) or
    /// its ElementId, instead of the project's last (RE-326). Revit's IFC
    /// export shows one phase and leaves out an element that does not exist
    /// in it and a room whose own phase is not it; this changes both. It fails,
    /// naming the phases the file lists, when none matches, and for a file
    /// whose phases are not read or whose release applies no export phase.
    /// [`Self::export_phase`] then reports the ElementId of the phase chosen,
    /// which `phase` may have named by its name or by that id.
    ///
    /// It may be called at any time. Choosing a phase other than the one in
    /// effect drops the memoised production walk, the one read that holds
    /// what the phase leaves out, so the next export reads again at the new
    /// phase; choosing the one in effect keeps it. [`Self::clear_export_phase`]
    /// goes back to the default.
    ///
    /// A phase named is exact: naming the project's last phase writes no room
    /// where the default export falls back to an earlier phase's rooms
    /// ([`Self::room_phase_fallback`]), and so drops the walk made under the
    /// default.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] when the phase is not one of the file's, or
    /// the file's phases cannot be chosen from.
    pub fn set_export_phase(&mut self, phase: &str) -> Result<()> {
        if let Ok(release) = self.layout_release() {
            if crate::partition_element_records_2023::is_32bit_release(release)
                && !crate::partition_element_records_2023::applies_export_filters(release)
            {
                return Err(Error::InvalidInput(format!(
                    "the export phase is applied to Revit 2016 and later, and this file is \
                     Revit {release}: it writes every element and room whatever its phase"
                )));
            }
        }
        let filter = self.memoised_phase_filter();
        if let Some(error) = &filter.read_error {
            return Err(Error::InvalidInput(format!(
                "the project's phases could not be read ({error}), so the export phase \
                 cannot be chosen"
            )));
        }
        if filter.order.is_empty() {
            return Err(Error::InvalidInput(
                "this file lists no project phases (a family, or a release whose phases are \
                 not read), so no export phase can be chosen"
                    .into(),
            ));
        }
        let Some(id) = filter.find_phase(phase) else {
            return Err(Error::InvalidInput(format!(
                "the file has no phase named or numbered {phase:?}; its phases are {}",
                filter.phases_listing()
            )));
        };
        // A named phase is exact: where the room fallback was in effect, the
        // walk made under it holds rooms this phase leaves out, even when it
        // is the last (#1359).
        if filter.export != Some(id) || filter.room_export_phase() != Some(id) {
            self.walk = None;
        }
        self.phase_filter = Some(Arc::new(filter.with_export_phase(id)));
        self.chosen_export_phase = Some(id);
        Ok(())
    }

    /// Back to the default export phase, the project's last with the rooms'
    /// fallback ([`Self::room_phase_fallback`]), as it is before
    /// [`Self::set_export_phase`] chooses one: [`Self::export_phase`] is `None`
    /// again. As a choice of another phase does, going back from one drops the
    /// memoised production walk where the default writes anything else; on a
    /// handle that has chosen none this changes nothing.
    pub fn clear_export_phase(&mut self) {
        if self.chosen_export_phase.take().is_none() {
            return;
        }
        let filter = self.memoised_phase_filter();
        let default = filter.with_default_export_phase();
        if default.export != filter.export
            || default.room_export_phase() != filter.room_export_phase()
        {
            self.walk = None;
        }
        self.phase_filter = Some(Arc::new(default));
    }

    /// The ElementId of the phase [`Self::set_export_phase`] chose, whether it
    /// was given by name or by that id; `None` while the last phase is in
    /// effect, as it is until a phase is chosen.
    pub fn export_phase(&self) -> Option<i64> {
        self.chosen_export_phase
    }

    /// Record the Areas of a Revit 2015 to 2023 file that were left unwritten,
    /// by reason (RE-324), each with the record it was read from (#1326).
    pub(crate) fn set_unwritten_areas(
        &mut self,
        by_reason: BTreeMap<String, Vec<crate::partition_element_records::LeftOutElement>>,
    ) {
        self.unwritten_areas = by_reason;
    }

    /// The Areas of a Revit 2015 to 2023 file left unwritten because their
    /// region and body are not proven, by the reason their body has none
    /// (RE-324). Empty until the Areas are read.
    pub(crate) fn unwritten_areas(
        &self,
    ) -> &BTreeMap<String, Vec<crate::partition_element_records::LeftOutElement>> {
        &self.unwritten_areas
    }

    /// The `Partitions/*` stream paths in the order Revit numbers them:
    /// by the number after `Partitions/` (`Partitions/9` before
    /// `Partitions/46`), then by name. A higher number is a later partition,
    /// so a reader that keeps the first or the last copy of a record gets the
    /// oldest or the newest.
    pub fn partition_stream_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .stream_names()
            .into_iter()
            .filter(|name| name.starts_with("Partitions/"))
            .collect();
        names.sort_by(|a, b| {
            let number = crate::partition_room_parameters::partition_number;
            (number(a), a).cmp(&(number(b), b))
        });
        names
    }

    /// String records extracted from the `Partitions/NN` stream, memoised.
    ///
    /// See [`crate::object_graph::string_records_from_partitions`] for
    /// what the extraction does; this is the shared-result entry point
    /// that avoids re-running the UTF-16 decode over the whole
    /// partition for every consumer.
    pub fn partition_string_records(
        &mut self,
    ) -> Result<Arc<Vec<crate::object_graph::StringRecord>>> {
        if let Some(cached) = &self.partition_strings {
            return Ok(Arc::clone(cached));
        }
        let records = Arc::new(crate::object_graph::extract_partition_string_records(self)?);
        self.partition_strings = Some(Arc::clone(&records));
        Ok(records)
    }

    /// List all OLE stream paths (sorted). Paths are always returned
    /// with forward-slash separators regardless of host OS — on
    /// Windows, `Path::display()` emits backslashes, but CFB stream
    /// paths are logically `/`-separated.
    pub fn stream_names(&self) -> Vec<String> {
        let mut streams: Vec<_> = self
            .cfb
            .walk()
            .filter(|e| e.is_stream())
            .map(|e| {
                e.path()
                    .display()
                    .to_string()
                    .replace('\\', "/")
                    .trim_start_matches('/')
                    .to_string()
            })
            .collect();
        streams.sort();
        streams
    }

    /// Read a named stream's raw bytes, capped at the file's
    /// configured `max_stream_bytes`.
    ///
    /// For streams larger than the limit, returns an error rather
    /// than allocating a potentially multi-GB `Vec`. Use
    /// [`Self::read_stream_with_limit`] to override per-call.
    ///
    /// A stream refused for its size is recorded in
    /// [`Self::unread_streams`], since the readers that ask for it go on
    /// without it.
    pub fn read_stream(&mut self, name: &str) -> Result<Vec<u8>> {
        let read = self.read_stream_with_limit(name, self.limits.max_stream_bytes);
        if let Err(Error::TooLarge { size, limit, .. }) = &read {
            let unread = UnreadStream {
                name: name.trim_start_matches('/').to_string(),
                stored_bytes: self.stream_size(name).unwrap_or(*size),
                kind: UnreadKind::StreamTooLarge,
                limit: *limit,
            };
            self.note_unread(unread);
        }
        read
    }

    /// Read a named stream's raw bytes, capped at an explicit
    /// byte limit.
    ///
    /// `max_bytes` is the ceiling on output size. A stream whose
    /// declared size (or read position) exceeds this returns
    /// [`Error::TooLarge`].
    pub fn read_stream_with_limit(&mut self, name: &str, max_bytes: u64) -> Result<Vec<u8>> {
        let path = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{name}")
        };
        let mut stream = self
            .cfb
            .open_stream(&path)
            .map_err(|_| Error::StreamNotFound(name.to_string()))?;
        // Stream size is known up-front from the CFB directory entry.
        // Reject before reading.
        let stream_size = stream.len();
        if stream_size > max_bytes {
            return Err(Error::TooLarge {
                what: format!("stream '{name}'"),
                size: stream_size,
                limit: max_bytes,
            });
        }
        let cap = (stream_size as usize).min(max_bytes as usize);
        let mut out = Vec::with_capacity(cap);
        // Read in bounded chunks so we can catch the case where a
        // stream's directory-entry size is a lie (malformed CFB).
        let mut buf = [0u8; 8192];
        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if (out.len() as u64) + (n as u64) > max_bytes {
                return Err(Error::TooLarge {
                    what: format!("stream '{name}'"),
                    size: out.len() as u64 + n as u64,
                    limit: max_bytes,
                });
            }
            out.extend_from_slice(&buf[..n]);
        }
        Ok(out)
    }

    /// Parse `BasicFileInfo`.
    pub fn basic_file_info(&mut self) -> Result<BasicFileInfo> {
        let bytes = self.read_stream(BASIC_FILE_INFO)?;
        BasicFileInfo::from_bytes(&bytes)
    }

    /// Parse `PartAtom` XML.
    pub fn part_atom(&mut self) -> Result<PartAtom> {
        let bytes = self.read_stream(PART_ATOM)?;
        PartAtom::from_bytes(&bytes)
    }

    /// Parse the project-level Atom entry from the `ProjectInformation`
    /// stream (see [`crate::project_information`]). `Ok(None)` when the file
    /// has no such stream — families never do.
    pub fn project_information(&mut self) -> Result<Option<PartAtom>> {
        if !self.stream_names().iter().any(|n| n == PROJECT_INFORMATION) {
            return Ok(None);
        }
        let bytes = self.read_stream(PROJECT_INFORMATION)?;
        crate::project_information::parse(&bytes, self.limits.inflate_limits.max_output_bytes)
            .map(Some)
    }

    /// The document's Atom entry, whichever stream carries it: `PartAtom`
    /// on families and templates, `ProjectInformation` on project files.
    /// `Ok(None)` when neither stream is present.
    pub fn document_atom(&mut self) -> Result<Option<PartAtom>> {
        if self.stream_names().iter().any(|n| n == PART_ATOM) {
            return self.part_atom().map(Some);
        }
        self.project_information()
    }

    /// Extract the PNG thumbnail from `RevitPreview4.0`.
    ///
    /// The raw stream has a ~300-byte Revit-specific header (magic
    /// `62 19 22 05` — the same header magic seen at the start of the
    /// `Contents` stream). The PNG payload begins at the first occurrence
    /// of the standard PNG magic bytes.
    ///
    /// When an `IEND` chunk is present, the returned buffer ends at the
    /// trailing CRC of that chunk — trailing junk after a well-formed PNG
    /// is trimmed. If no `IEND` is found, bytes from PNG magic through
    /// end-of-stream are returned (fail-open for truncated previews).
    pub fn preview_png(&mut self) -> Result<Vec<u8>> {
        let bytes = self.read_stream(REVIT_PREVIEW_4_0)?;
        extract_preview_png(&bytes)
    }

    /// Decompress `Formats/Latest` and extract the class/schema inventory.
    pub fn class_names(&mut self) -> Result<BTreeSet<String>> {
        let bytes = self.read_stream(FORMATS_LATEST)?;
        // Formats/Latest has GZIP magic at offset 0 (no custom header).
        // Wave 2 gate deliberately does **not** strip page trailers here —
        // naive strip regresses opportunistic `class_names` on redistributable
        // corpora (#151 judge: narrow). `inflate_stream_at` stays path-aware
        // for API uniformity but is a no-op strip for Formats.
        let decompressed = compression::inflate_stream_at(FORMATS_LATEST, &bytes, 0)?;
        class_index::extract_class_names(&decompressed)
    }

    /// Decompress `Formats/Latest` and parse it into a full schema table —
    /// classes + fields + C++ type signatures. This is the structured
    /// version of `class_names()`.
    pub fn schema(&mut self) -> Result<crate::formats::SchemaTable> {
        let bytes = self.read_stream(FORMATS_LATEST)?;
        let decompressed = compression::inflate_stream_at(FORMATS_LATEST, &bytes, 0)?;
        crate::formats::parse_schema(&decompressed)
    }

    /// [`Self::schema_classes`], memoised for the crate's own readers.
    pub(crate) fn memoised_schema_classes(&mut self) -> Result<Arc<crate::formats::SchemaClasses>> {
        if let Some(cached) = &self.schema_classes_memo {
            return Ok(Arc::clone(cached));
        }
        let classes = Arc::new(self.schema_classes()?);
        self.schema_classes_memo = Some(Arc::clone(&classes));
        Ok(classes)
    }

    /// The release whose layout the file's records take, which the
    /// partition readers are keyed on: the release of the file's schema
    /// (`research::partition_element_records::schema_release`) when it is
    /// older than the release that saved the file, otherwise the saving
    /// release (`BasicFileInfo`'s version), which metadata and reports
    /// keep (#421). Memoised.
    pub fn layout_release(&mut self) -> Result<u32> {
        if let Some(release) = self.layout_release_memo {
            return Ok(release);
        }
        let saved = self.basic_file_info()?.version;
        let release = match crate::partition_element_records::schema_release(self) {
            Some(schema) if schema < saved => schema,
            _ => saved,
        };
        self.layout_release_memo = Some(release);
        Ok(release)
    }

    /// Every class of the embedded schema with its serialization tag, read by
    /// the schema grammar ([`crate::formats::schema_classes`], #154).
    pub fn schema_classes(&mut self) -> Result<crate::formats::SchemaClasses> {
        let bytes = self.read_stream(FORMATS_LATEST)?;
        let decompressed = compression::inflate_stream_at(FORMATS_LATEST, &bytes, 0)?;
        Ok(crate::formats::schema_classes(&decompressed))
    }

    /// The first `Partitions/NN` stream, in [`Self::partition_stream_names`]'
    /// order: the lowest-numbered.
    pub fn partition_stream_name(&self) -> Option<String> {
        self.partition_stream_names().into_iter().next()
    }

    /// Produce a one-shot summary of everything we can parse.
    ///
    /// Historically this method was mixed — strict on BasicFileInfo
    /// (propagated errors) but lossy on PartAtom / class-names
    /// (swallowed failures via `.ok()` / `.unwrap_or_default()`).
    /// Prefer [`RevitFile::summarize_strict`] or
    /// [`RevitFile::summarize_lossy`] — this wrapper calls through
    /// to the lossy path and discards the diagnostics for backwards
    /// compatibility. It's kept on the stable surface but new
    /// callers should pick the strictness level they actually want.
    #[deprecated(
        since = "0.1.2",
        note = "Use `summarize_strict` for errors-on-any-failure or `summarize_lossy` for Decoded<Summary> with accumulated diagnostics."
    )]
    pub fn summarize(&mut self) -> Result<Summary> {
        self.summarize_lossy().map(|d| d.value)
    }

    /// Strict variant of [`RevitFile::summarize`] — returns
    /// `Err` on any parse failure of a stream it reads, including the
    /// class-name inventory. A missing PartAtom is not a failure (project
    /// files have none) and leaves `partatom` empty. Use when downstream code
    /// can't tolerate a partially-populated `Summary`.
    ///
    /// # Errors
    ///
    /// [`Error::StreamNotFound`] when `BasicFileInfo` or `Formats/Latest` is
    /// missing, [`Error::Cfb`] or [`Error::Io`] when either cannot be read,
    /// [`Error::TooLarge`] when either is larger than
    /// [`OpenLimits::max_stream_bytes`],
    /// [`Error::BasicFileInfo`] or [`Error::PartAtom`] when either does not
    /// parse, and [`Error::Decompress`] or [`Error::DecompressLimitExceeded`]
    /// when `Formats/Latest` does not inflate or inflates past its cap.
    // Tracker: API-04.
    pub fn summarize_strict(&mut self) -> Result<Summary> {
        let streams = self.stream_names();
        let bfi = self.basic_file_info()?;
        // PartAtom: absence is a soft concept — not every file has
        // one. Missing PartAtom is NOT an error here; a failed
        // parse of an existing PartAtom IS.
        let partatom = match self.part_atom() {
            Ok(p) => Some(p),
            Err(Error::StreamNotFound(_)) => None,
            Err(e) => return Err(e),
        };
        let partition_stream = self.partition_stream_name();
        let class_names = self.class_names()?;
        let class_name_count = class_names.len();
        let class_name_sample: Vec<String> = class_names.into_iter().take(30).collect();

        let file_size: u64 = streams.iter().filter_map(|n| self.stream_size(n)).sum();

        Ok(Summary {
            file_size,
            streams,
            version: bfi.version,
            build: bfi.build,
            original_path: bfi.original_path,
            guid: bfi.guid,
            locale: bfi.locale,
            partition_stream,
            partatom,
            class_name_count,
            class_name_sample,
        })
    }

    /// Lossy variant — returns a [`crate::parse_mode::Decoded<Summary>`]
    /// that accumulates partial-parse issues into its
    /// `diagnostics` field instead of aborting. Fails only when
    /// BasicFileInfo itself is unreadable (without it there's
    /// nothing to summarise). Every other failure becomes a
    /// `Diagnostic::fail_stream` entry and the corresponding
    /// `Summary` field uses its default.
    ///
    /// Use when you're summarising a batch of files and want a
    /// best-effort report even for the ones with partial parses.
    // Tracker: API-05.
    pub fn summarize_lossy(&mut self) -> Result<crate::parse_mode::Decoded<Summary>> {
        use crate::parse_mode::{Decoded, Diagnostics};

        let streams = self.stream_names();
        // BasicFileInfo is load-bearing — without it we can't
        // populate any versioned identity. Propagate its error.
        let bfi = self.basic_file_info()?;

        let mut diagnostics = Diagnostics::default();

        let partatom = match self.part_atom() {
            Ok(p) => Some(p),
            Err(Error::StreamNotFound(_)) => None,
            Err(_) => {
                diagnostics.fail_stream("PartAtom");
                None
            }
        };

        let partition_stream = self.partition_stream_name();
        if partition_stream.is_none() {
            diagnostics.fail_stream("Partitions/*");
        }

        let class_names = match self.class_names() {
            Ok(n) => n,
            Err(_) => {
                diagnostics.fail_stream(FORMATS_LATEST);
                Default::default()
            }
        };
        let class_name_count = class_names.len();
        let class_name_sample: Vec<String> = class_names.into_iter().take(30).collect();

        let file_size: u64 = streams.iter().filter_map(|n| self.stream_size(n)).sum();

        let summary = Summary {
            file_size,
            streams,
            version: bfi.version,
            build: bfi.build,
            original_path: bfi.original_path,
            guid: bfi.guid,
            locale: bfi.locale,
            partition_stream,
            partatom,
            class_name_count,
            class_name_sample,
        };

        Ok(if diagnostics.is_empty() {
            Decoded::complete(summary)
        } else {
            Decoded::partial(summary, diagnostics)
        })
    }

    /// Get size of a named stream (returns `None` if missing).
    pub fn stream_size(&self, name: &str) -> Option<u64> {
        let path = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{name}")
        };
        self.cfb.entry(&path).ok().map(|e| e.len())
    }

    /// Check the common/invariant streams are all present. Useful for triage:
    /// if any of these is missing, the file is either corrupt or not a Revit file
    /// despite having a valid CFB container.
    pub fn has_revit_signature(&self) -> bool {
        self.missing_required_streams().is_empty()
    }

    /// Diagnostic form of `has_revit_signature` — returns the list of
    /// required streams that are missing, or an empty vec if all are
    /// present. Much more useful than the bool when triaging: "why does
    /// this 2016 file work on Linux but not Windows?" gets a concrete
    /// answer ("missing Global/DocumentIncrementTable") instead of "yes
    /// or no".
    pub fn missing_required_streams(&self) -> Vec<&'static str> {
        let names: BTreeSet<String> = self.stream_names().into_iter().collect();
        let required = [
            BASIC_FILE_INFO,
            CONTENTS,
            FORMATS_LATEST,
            GLOBAL_CONTENT_DOCUMENTS,
            GLOBAL_DOC_INCREMENT_TABLE,
            GLOBAL_ELEM_TABLE,
            GLOBAL_HISTORY,
            GLOBAL_LATEST,
            GLOBAL_PARTITION_TABLE,
            PART_ATOM,
            REVIT_PREVIEW_4_0,
            TRANSMISSION_DATA,
        ];
        required
            .iter()
            .copied()
            .filter(|r| !names.contains(*r))
            .collect()
    }
}

const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// PNG chunk type `IEND` (zero-length terminal chunk).
const PNG_IEND_TYPE: [u8; 4] = [0x49, 0x45, 0x4E, 0x44];

/// Locate PNG magic inside a `RevitPreview4.0` stream and return bytes from
/// that offset through end-of-input (no IEND trim).
fn extract_preview_png_untrimmed(stream: &[u8]) -> Result<Vec<u8>> {
    let pos = stream
        .windows(8)
        .position(|w| w == PNG_MAGIC)
        .ok_or_else(|| Error::StreamNotFound("PNG magic inside RevitPreview4.0".into()))?;
    Ok(stream[pos..].to_vec())
}

/// Locate PNG magic and, when present, trim after the IEND chunk CRC.
fn extract_preview_png(stream: &[u8]) -> Result<Vec<u8>> {
    let mut png = extract_preview_png_untrimmed(stream)?;
    if let Some(end) = png_iend_exclusive_end(&png) {
        png.truncate(end);
    }
    Ok(png)
}

/// Return the exclusive end offset of a well-formed IEND chunk inside `png`,
/// or `None` if no IEND type marker with room for its CRC is found.
///
/// IEND wire layout: `[u32 BE length=0][IEND][u32 CRC]` (12 bytes). We scan
/// for the type bytes and require a 4-byte length field of zero immediately
/// before them plus four CRC bytes after.
fn png_iend_exclusive_end(png: &[u8]) -> Option<usize> {
    // Need at least 4 (len) + 4 (type) + 4 (crc) = 12 bytes for a hit.
    if png.len() < 12 {
        return None;
    }
    for i in 0..=png.len().saturating_sub(8) {
        if png[i..i + 4] != PNG_IEND_TYPE {
            continue;
        }
        // Type must be preceded by a 4-byte big-endian length of zero.
        if i < 4 {
            continue;
        }
        let len_start = i - 4;
        if png[len_start..i] != [0, 0, 0, 0] {
            continue;
        }
        let crc_end = i + 4 + 4; // type + CRC
        if crc_end > png.len() {
            return None;
        }
        return Some(crc_end);
    }
    None
}
