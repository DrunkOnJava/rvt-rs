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
    /// [`compression::InflateLimits::max_stream_total_bytes`]; those after
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
    /// Memoised data objects (RE-153) of each partition stream: the
    /// original symbols, railing types and MEP systems each scanned every
    /// partition for them.
    // Tracker: B80.
    data_objects: std::collections::HashMap<String, PartitionDataObjects>,
    /// Memoised native extraction preamble: each native consumer read
    /// the schema, the element index and the increments again.
    // Tracker: B82.
    native_preamble: Option<Arc<crate::native_document::Preamble>>,
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
            walk: None,
            arc_wall_scan: None,
            data_objects: std::collections::HashMap::new(),
            native_preamble: None,
            native_record_index: None,
            schema_classes_memo: None,
            layout_release_memo: None,
            material_names: std::collections::HashMap::new(),
            material_appearances: std::collections::HashMap::new(),
            category_style_materials: None,
            unread_streams: Vec::new(),
        })
    }

    /// The streams a resource limit kept from being read whole since this
    /// handle was opened, in the order they were met (#1075). Empty when
    /// nothing was cut; when not, the readers went on without them, so
    /// anything produced from this handle is missing what they hold.
    pub fn unread_streams(&self) -> &[UnreadStream] {
        &self.unread_streams
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
        let limits = self.limits.inflate_limits;
        let (inflated, shortfall) =
            compression::InflatedStream::from_stored_with_limits(name, &raw, limits);
        if shortfall.members_over_limit > 0 {
            self.note_unread(UnreadStream {
                name: name.to_string(),
                stored_bytes: raw.len() as u64,
                kind: UnreadKind::MemberTooLarge,
                limit: limits.max_output_bytes as u64,
            });
        }
        if shortfall.stopped_at_total {
            self.note_unread(UnreadStream {
                name: name.to_string(),
                stored_bytes: raw.len() as u64,
                kind: UnreadKind::InflatedTotalTooLarge,
                limit: limits.max_stream_total_bytes() as u64,
            });
        }
        let inflated = Arc::new(inflated);
        self.inflated
            .insert(name.to_string(), Arc::clone(&inflated));
        Ok(inflated)
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

    /// Sorted `Partitions/*` stream paths.
    pub fn partition_stream_names(&self) -> Vec<String> {
        self.stream_names()
            .into_iter()
            .filter(|name| name.starts_with("Partitions/"))
            .collect()
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

    /// Like [`Self::preview_png`], but never trims after `IEND` — returns
    /// every byte from PNG magic through end of the OLE stream. Useful for
    /// forensic inspection of trailing payload.
    pub fn preview_png_untrimmed(&mut self) -> Result<Vec<u8>> {
        let bytes = self.read_stream(REVIT_PREVIEW_4_0)?;
        extract_preview_png_untrimmed(&bytes)
    }

    /// Raw bytes of the `RevitPreview4.0` stream including Revit's
    /// custom wrapper. Use `preview_png` for just the PNG.
    pub fn preview_raw(&mut self) -> Result<Vec<u8>> {
        self.read_stream(REVIT_PREVIEW_4_0)
    }

    /// Probe `TransmissionData` (UTF-16LE vs opaque vs empty) with
    /// opportunistic UUID / path / XML-node extracts when UTF-16LE.
    ///
    /// Does **not** resolve linked models or decode Autodesk transmission
    /// field layouts — see [`crate::transmission_data`]. Empty extracts
    /// ≠ “no links”.
    pub fn transmission_data_probe(
        &mut self,
    ) -> Result<crate::transmission_data::TransmissionDataProbe> {
        crate::transmission_data::probe_transmission_data(self)
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

    /// Find the version-specific `Partitions/NN` stream name.
    pub fn partition_stream_name(&self) -> Option<String> {
        self.stream_names()
            .into_iter()
            .find(|n| n.starts_with("Partitions/"))
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
