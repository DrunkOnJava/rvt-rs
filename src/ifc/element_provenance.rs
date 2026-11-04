//! Where each decoded element came from, down to the bytes (#1325).
//!
//! [`ElementProvenanceTable`] is keyed by ElementId and holds, for every
//! element the production walk decoded, the carrier its record was read from
//! ([`ElementCarrier`]), the stream and the byte offset in that stream
//! inflated, where the ElementId itself is stored, the class tag the record
//! names and the class that tag resolves to in the file's own schema, the
//! element's `BuiltInCategory`, and the record the native readers hold for it
//! ([`NativeRecordRef`]). It rides in the export diagnostics
//! (`decoded.element_provenance`) when the export is asked for it
//! (`rvt-ifc --element-provenance`, `ExportOptions::with_element_provenance`,
//! the MCP server's decode, Python's `element_provenance=True`): it is large,
//! 6,202,130 bytes of compact JSON with the value sources of #1360 and #1368
//! against 38,640 bytes of diagnostics without it (about 161 times) on the
//! BIM4LCA office as built, and the CLI's pretty-printed sidecar grows from
//! 52,677 to 11,098,081 bytes (Measure run 41890994176), so it is off by
//! default. It is never written into the IFC.
//!
//! An offset indexes the stream as `RevitFile::inflated_partition` returns it,
//! which is what `rvt-dump` writes as `Partitions_NN.decomp` (#291). For the
//! schema walk it indexes `Global/Latest` inflated as the walk inflates it
//! (`inflate_stream_auto`); `rvt-dump` inflates that stream with its own
//! routine, and the two are not compared. The schema-walk carrier had no
//! element on any of the seven models the offsets test reads, so its offsets
//! are unmeasured.
//!
//! The offset is the frame the exporter selected for the element's record.
//! A body reader may take another copy of the element's data (the sketch-line
//! readers take the current one, `native_document::current_partitions`,
//! RE-216), so [`ElementProvenanceEntry::value_sources`] names, for each value
//! such a reader took, the copy it took it from (#1360): a [`ValueSource`] with
//! the reader, the [`CopyRule`] it chose the copy by, and the stream and
//! offset of the copy. A reader notes its choice on the handle where it makes
//! it, and only when the export is asked for this table
//! (`RevitFile::record_value_sources`); `attach_value_sources` joins those
//! notes with the final model's `BodySource`.
//! [`ElementProvenanceTable::value_source_counts`] counts, by reader, the
//! elements whose value source is in another partition than their frame.
//! Which readers note their choice is in `docs/export-diagnostics.md`; a
//! reader that does not leaves the element with no value source, which is not
//! a claim that it read the frame.
//!
//! The table is built from what the export already read: the walk the handle
//! memoised, the inflated partitions it holds and the index the native passes
//! made of their records. Nothing here changes what the export writes. An
//! element whose offset cannot be given is kept, with no offset, and counted
//! by carrier in [`ElementProvenanceTable::by_carrier`]; an element with no
//! ElementId (a Level read from partition strings, a material read by name) has
//! no key and is counted in [`ElementProvenanceTable::unkeyed_elements`].

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use super::IfcModel;
use super::entities::{IfcEntity, PropertyValue};
use crate::control::WalkerControl;
use crate::formats::SchemaClasses;
use crate::walker::{
    DEFAULT_MIN_ELEMENT_CONFIDENCE, DecodedElement, InstanceField, PRODUCTION_ELEMENT_MIN_SCORE,
    WalkerLimits, walk_shared,
};
use crate::{RevitFile, le_bytes, streams};

/// The most elements the table lists. Every element is counted in
/// [`ElementProvenanceTable::by_carrier`] whatever this is; past it the
/// highest ElementIds are left out of [`ElementProvenanceTable::elements`] and
/// counted in [`ElementProvenanceTable::omitted_entries`].
pub const MAX_ELEMENT_PROVENANCE_ENTRIES: usize = 250_000;

/// Where an element's record was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ElementCarrier {
    /// A partition element record frame that starts its partition record: the
    /// ElementId is the `u64` (`u32` on Revit 2008 to 2023) at the frame's
    /// first byte (RE-21, RE-35). The element's stream and offset are those
    /// of the frame, and `record_offset` equals `offset`.
    PartitionElementRecordFirstPrologue,
    /// A partition element record frame with no ElementId of its own (RE-30),
    /// which sits inside the partition record of its element: the ElementId is
    /// that record's (RE-35, `enclosing_record_id`), stored at
    /// `record_offset`, before `offset`.
    PartitionElementRecordSecondPrologue,
    /// An instance of the `Global/Latest` schema walk (RE-14): the ElementId
    /// is the instance's own id field, at no fixed place from `offset`.
    /// Unmeasured: no model of the offsets test has one.
    GlobalLatestSchemaWalk,
    /// A Revit 2023 ArcWall record in a partition (RE-14.3): the offset is
    /// that of the record's envelope.
    PartitionArcWall,
    /// An Area read from its partition chain record (#763, #1194), the copy
    /// the latest partition that holds it has: the ElementId is stored at
    /// `offset`, which is the record header.
    PartitionAreaRecord,
    /// A row read from partition data that has no frame of its own, such as a
    /// material from its material record: no stream or offset is given.
    PartitionDerived,
    /// A record of a kind this table does not place, or a frame whose stream
    /// could not be read again: no stream or offset is given.
    Other,
}

impl ElementCarrier {
    /// The carrier as the JSON diagnostics and the MCP server write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PartitionElementRecordFirstPrologue => "partition_element_record_first_prologue",
            Self::PartitionElementRecordSecondPrologue => {
                "partition_element_record_second_prologue"
            }
            Self::GlobalLatestSchemaWalk => "global_latest_schema_walk",
            Self::PartitionArcWall => "partition_arc_wall",
            Self::PartitionAreaRecord => "partition_area_record",
            Self::PartitionDerived => "partition_derived",
            Self::Other => "other",
        }
    }
}

/// Which of an element's native record copies a [`NativeRecordRef`] names
/// (RE-145, RE-216).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NativeRecordCopy {
    /// The element has one native record, in one partition.
    Only,
    /// Several partitions hold a copy and this is the current one: the
    /// partition its stored revision routes to where that holds a copy, else
    /// the latest that does (`native_document::current_partitions`).
    Current,
    /// Several partitions hold a copy and none could be named current (the
    /// routed partition was not read, or the element index does not route
    /// it): no stream or offset is given.
    Unresolved,
}

impl NativeRecordCopy {
    /// The copy as the JSON diagnostics and the MCP server write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Only => "only",
            Self::Current => "current",
            Self::Unresolved => "unresolved",
        }
    }
}

/// The native record (a record by member name, the reassembled group framing
/// of `native_document`) the native readers hold for an element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NativeRecordRef {
    /// In how many partitions the element has a native record.
    pub copies: u32,
    /// Which copy [`Self::stream`] and [`Self::offset`] name.
    pub copy: NativeRecordCopy,
    /// The partition of that copy, such as `Partitions/46`. It can differ
    /// from [`ElementProvenanceEntry::stream`], the frame the exporter
    /// selected. The record a reader decoded is a [`ValueSource`] with the
    /// rule [`CopyRule::Native`], where that reader notes its choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Where the record's header starts in the partition inflated, where the
    /// record's group is those bytes. The ElementId is the `u64` (`u32` on
    /// Revit 2023 and earlier) at this offset. Absent when the group was
    /// joined from segments that are not consecutive members of the stream,
    /// or inflated again, which leaves it no place in the stream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// The class tag the record's body opens with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_tag: Option<u16>,
    /// The class that tag names in the file's schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
}

/// How a reader chose the copy of an element's data it took a value from
/// (#1360).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CopyRule {
    /// The frame the exporter selected: the stream and offset of the element's
    /// entry. The value is read from it, or from it and its neighbours' frames.
    Selected,
    /// Every copy of the element's data in the partitions that hold one holds
    /// the same value, so the reader took the first, the lowest-numbered
    /// partition's. `copies` says how many copies held it.
    Agreeing,
    /// The copies disagree and the reader took the current partition's
    /// (`native_document::current_partitions`, RE-216).
    Current,
    /// A native record by member name, the current copy of it
    /// (`native_document`).
    Native,
    /// One frame or partition of the record, as the reader's scan kept it,
    /// without the record's other copies being compared: the last frame of a
    /// type symbol whose frames do not agree, the solids of a room, read in
    /// the room's own partition only, and the first frame of each sketch line
    /// of a roof by extrusion.
    Frame,
}

impl CopyRule {
    /// The rule as the JSON diagnostics and the MCP server write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Agreeing => "agreeing",
            Self::Current => "current",
            Self::Native => "native",
            Self::Frame => "frame",
        }
    }
}

/// The most records a [`ValueSource`] lists in [`ValueSource::parts`]. A
/// profile of more has the rest counted in [`ValueSource::parts_left_out`].
pub const MAX_SOURCE_PARTS: usize = 64;

/// One record a [`ValueSource`] is made of (#1368).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SourcePart {
    /// The ElementId of the record's element: a sketch line, a type symbol.
    pub element_id: u32,
    /// The partition the record is in, such as `Partitions/46`.
    pub stream: String,
    /// Where the record's frame starts in that partition inflated.
    pub offset: usize,
}

/// The copy of an element's data a reader took a value from (#1360).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ValueSource {
    /// What was read, named by the reader that read it: `bounded_line` (a
    /// wall's or beam's location line), `bounded_arc`, `sketch_curve`,
    /// `sketch_circle`, `sketch_spline`, `instance_transform`, a native
    /// reader's name, `selected_frame` for a body read from the frame, or
    /// `selected_frame_box` for a body whose box and placement are read from
    /// the frame and whose profile is read from another record, which no
    /// reader notes yet (its `ProfileSource` names the reader).
    pub reader: String,
    /// How the copy was chosen.
    pub rule: CopyRule,
    /// In how many partitions the element's data named it, so how many copies
    /// there were to choose from (two headers in one partition count once);
    /// for a source made of several [`Self::parts`] (a profile's sketch
    /// lines), how many records it is made of. A source of one part counts the
    /// partitions that held that record.
    pub copies: u32,
    /// The partition of the copy, such as `Partitions/46`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Where the copy starts in that partition inflated: the element-data
    /// header whose element id follows it, for the readers of element data;
    /// the record's header, for a native record; the frame, for the selected
    /// frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// For [`CopyRule::Agreeing`], the numbers of the partitions that hold a
    /// copy, every one of which holds the same value: the copy named is the
    /// lowest-numbered partition's, which says nothing about the frame's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub partitions: Vec<u32>,
    /// The records the value is made of, where it is made of several or of
    /// another element's (a profile's sketch lines, a type symbol's frame),
    /// at most [`MAX_SOURCE_PARTS`]. Such a source has no `stream` or `offset`
    /// of its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<SourcePart>,
    /// How many more records than [`MAX_SOURCE_PARTS`] the value is made of.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub parts_left_out: u32,
    /// Whether the value is held in another partition than the element's frame
    /// ([`ElementProvenanceEntry::stream`]): the copy named is in another, and,
    /// for [`CopyRule::Agreeing`], so is every copy that agrees with it; for a
    /// source made of records, none of them is in the frame's partition (a
    /// profile's sketch lines are also held there when the Sketch's curve
    /// list has a copy there).
    #[serde(default, skip_serializing_if = "is_false")]
    pub differs_from_frame: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

impl ValueSource {
    pub(crate) fn new(
        reader: &str,
        rule: CopyRule,
        copies: u32,
        stream: Option<String>,
        offset: Option<usize>,
    ) -> Self {
        Self {
            reader: reader.to_string(),
            rule,
            copies,
            stream,
            offset,
            partitions: Vec::new(),
            parts: Vec::new(),
            parts_left_out: 0,
            differs_from_frame: false,
        }
    }

    /// A source made of the records `parts`, the first [`MAX_SOURCE_PARTS`]
    /// of them listed and the rest counted.
    pub(crate) fn from_parts(
        reader: &str,
        rule: CopyRule,
        copies: usize,
        mut parts: Vec<SourcePart>,
    ) -> Self {
        let left_out = parts.len().saturating_sub(MAX_SOURCE_PARTS);
        parts.truncate(MAX_SOURCE_PARTS);
        let mut source = Self::new(
            reader,
            rule,
            u32::try_from(copies).unwrap_or(u32::MAX),
            None,
            None,
        );
        source.parts = parts;
        source.parts_left_out = u32::try_from(left_out).unwrap_or(u32::MAX);
        source
    }

    /// Whether the source says where it is: a copy, or records.
    pub(crate) fn has_place(&self) -> bool {
        self.stream.is_some() || !self.parts.is_empty()
    }

    /// The partitions that hold an agreeing copy.
    pub(crate) fn with_partitions(mut self, partitions: Vec<u32>) -> Self {
        self.partitions = partitions;
        self
    }

    /// Whether the value is held in `stream`: the copy named is, or an
    /// agreeing copy listed in [`Self::partitions`] is.
    pub(crate) fn held_in(&self, stream: &str) -> bool {
        self.stream.as_deref() == Some(stream)
            || self.parts.iter().any(|part| part.stream == stream)
            || stream
                .strip_prefix("Partitions/")
                .and_then(|number| number.parse::<u32>().ok())
                .is_some_and(|number| self.partitions.contains(&number))
    }

    /// The source of a copy in partition `partition` at `offset`.
    pub(crate) fn in_partition(
        reader: &str,
        rule: CopyRule,
        copies: usize,
        partition: u32,
        offset: Option<usize>,
    ) -> Self {
        Self::new(
            reader,
            rule,
            u32::try_from(copies).unwrap_or(u32::MAX),
            Some(format!("Partitions/{partition}")),
            offset,
        )
    }
}

/// The copies a reader met while it scanned the partitions for the element
/// data of some ids (#1368): the first copy of each id that gave a value, and
/// the partitions that named it and that gave one. The reader notes its choice
/// once the scan has settled which ids it kept.
#[derive(Default)]
pub(crate) struct CopyTrail {
    first: BTreeMap<u32, (u32, usize)>,
    named: BTreeMap<u32, BTreeSet<u32>>,
    gave: BTreeMap<u32, BTreeSet<u32>>,
}

impl CopyTrail {
    /// The copy of `id` in `partition` was met.
    pub(crate) fn named(&mut self, id: u32, partition: u32) {
        self.named.entry(id).or_default().insert(partition);
    }

    /// The copy of `id` in `partition`, whose element-data header starts at
    /// `at`, gave a value.
    pub(crate) fn gave(&mut self, id: u32, partition: u32, at: usize) {
        self.first.entry(id).or_insert((partition, at));
        self.gave.entry(id).or_default().insert(partition);
    }

    /// Note, under each of `kept` that gave a value, the first copy that did,
    /// as `reader`'s choice among copies that agree.
    pub(crate) fn note(
        self,
        rf: &mut RevitFile,
        reader: &str,
        kept: impl IntoIterator<Item = u32>,
    ) {
        for id in kept {
            let Some(&(partition, at)) = self.first.get(&id) else {
                continue;
            };
            let held = self.named.get(&id).map_or(0, BTreeSet::len);
            let partitions = self.gave.get(&id).cloned().unwrap_or_default();
            let source =
                ValueSource::in_partition(reader, CopyRule::Agreeing, held, partition, Some(at))
                    .with_partitions(partitions.into_iter().collect());
            rf.note_value_source(id, source);
        }
    }
}

/// The value sources the readers noted on a handle, by ElementId.
pub type ValueSourceNotes = BTreeMap<u32, Vec<ValueSource>>;

/// How many elements a reader's value sources cover, and how many of those are
/// in another partition than the element's frame.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ValueSourceCount {
    /// Elements with a value source from the reader.
    pub elements: usize,
    /// Of those, elements whose value source is in another partition than
    /// their frame ([`ValueSource::differs_from_frame`]).
    pub differ_from_frame: usize,
    /// Of those, elements whose copy was chosen as the current partition's
    /// ([`CopyRule::Current`]): the copies disagreed.
    #[serde(default, skip_serializing_if = "is_zero_count")]
    pub current: usize,
}

fn is_zero_count(value: &usize) -> bool {
    *value == 0
}

/// Where one element came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ElementProvenanceEntry {
    /// The kind of record the element was read from.
    pub carrier: ElementCarrier,
    /// The stream the element's record is in: a partition (`Partitions/46`)
    /// or `Global/Latest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Where the record or frame starts in that stream inflated. For a
    /// partition element record it is the frame the exporter selected among
    /// the element's frames: on Revit 2024 and later the one with the
    /// greatest vertical extent and, on a tie, the higher-numbered partition
    /// and then the later offset (`select_instance_records`);
    /// on Revit 2008 to 2023 the newest, the highest-numbered partition and
    /// then the latest offset, with no extent test
    /// (`select_newest_instance_records`, RE-107). A body reader may take
    /// another copy (the sketch-line readers take the current one, RE-216),
    /// so this and [`NativeRecordRef::stream`] can name different partitions:
    /// the copy a reader took is in [`Self::value_sources`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Where the ElementId is stored: the partition record header whose first
    /// bytes are the id. The frame's own start for a first-prologue frame, the
    /// enclosing partition record's start for a second-prologue one. Absent for
    /// the carriers that do not store the id in a header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_offset: Option<usize>,
    /// For a second-prologue frame, the ElementId of the partition record that
    /// encloses it, which is the element's (RE-35).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enclosing_record_id: Option<u32>,
    /// The class tag the record names: for an element record the `u16` at
    /// `+0x4a`, otherwise the tag of the class the walk decoded it as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_tag: Option<u16>,
    /// The class that tag resolves to in the file's own `Formats/Latest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// The element record's `BuiltInCategory` (a negative `OST_*` id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin_category: Option<i64>,
    /// The element's native record, where the native passes of the export
    /// indexed one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native: Option<NativeRecordRef>,
    /// The copy each reader took a value from, where it is not read straight
    /// from [`Self::stream`] and [`Self::offset`], one per reader (#1360).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_sources: Vec<ValueSource>,
}

impl ElementProvenanceEntry {
    fn new(carrier: ElementCarrier) -> Self {
        Self {
            carrier,
            stream: None,
            offset: None,
            record_offset: None,
            enclosing_record_id: None,
            class_tag: None,
            class_name: None,
            builtin_category: None,
            native: None,
            value_sources: Vec::new(),
        }
    }

    /// Whether the element's stream and offset are given, and, for a
    /// second-prologue frame, the partition record that holds its ElementId.
    pub fn is_located(&self) -> bool {
        self.stream.is_some()
            && self.offset.is_some()
            && (self.carrier != ElementCarrier::PartitionElementRecordSecondPrologue
                || self.record_offset.is_some())
    }
}

/// How many elements of one carrier have their offset given.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CarrierCount {
    /// Elements of this carrier.
    pub elements: usize,
    /// Of those, elements whose stream and offset are given
    /// ([`ElementProvenanceEntry::is_located`]).
    pub located: usize,
    /// Of those, elements without: [`Self::elements`] less [`Self::located`].
    pub unlocated: usize,
}

/// How many elements have a native record and how many of those have its
/// offset given, counted over every element with an ElementId, whatever the
/// cap on the listed entries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NativeRecordCount {
    /// Elements with a native record in the index.
    pub with_record: usize,
    /// Of those, records whose partition and offset are given.
    pub located: usize,
    /// Of those, records without an offset or without a partition.
    pub unlocated: usize,
    /// Elements with a native record in several partitions (RE-216).
    pub several_copies: usize,
    /// Of those, elements none of whose copies could be named current.
    pub copy_unresolved: usize,
}

/// The provenance of the elements of one export, by ElementId.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ElementProvenanceTable {
    /// The release whose record layout the file takes
    /// (`RevitFile::layout_release`), which says how wide the ElementId
    /// stored at a record offset is ([`Self::record_id_bytes`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_release: Option<u32>,
    /// How many `Partitions/NN` streams the file has.
    #[serde(default)]
    pub partition_count: usize,
    /// The width of the ElementId at a partition record's first bytes: 8 from
    /// Revit 2024, 4 on 2008 to 2023.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id_bytes: Option<u8>,
    /// Whether the export's native passes built the index of native records
    /// ([`NativeRecordRef`] is given only where they did).
    #[serde(default)]
    pub native_record_index: bool,
    /// Decoded elements with no ElementId, which have no key here.
    #[serde(default)]
    pub unkeyed_elements: usize,
    /// Elements listed in [`Self::elements`] and counted, by carrier
    /// ([`ElementCarrier::as_str`]); every element, whatever the cap.
    #[serde(default)]
    pub by_carrier: BTreeMap<String, CarrierCount>,
    /// The native records of every element with an ElementId, the omitted
    /// ones included.
    #[serde(default)]
    pub native: NativeRecordCount,
    /// The value sources of the listed elements, counted by reader (#1360).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub value_source_counts: BTreeMap<String, ValueSourceCount>,
    /// Elements left out of [`Self::elements`] by
    /// [`MAX_ELEMENT_PROVENANCE_ENTRIES`], the highest ElementIds.
    #[serde(default)]
    pub omitted_entries: usize,
    /// Each element's provenance, by ElementId.
    #[serde(default)]
    pub elements: BTreeMap<u32, ElementProvenanceEntry>,
}

impl ElementProvenanceTable {
    /// The provenance of the element with `element_id`.
    pub fn get(&self, element_id: u32) -> Option<&ElementProvenanceEntry> {
        self.elements.get(&element_id)
    }

    /// Whether `element_id` has no entry because the cap on the listed
    /// entries ([`MAX_ELEMENT_PROVENANCE_ENTRIES`]) left it out: the cap
    /// leaves out the highest ElementIds, so it is one past the last listed.
    pub fn is_omitted(&self, element_id: u32) -> bool {
        self.omitted_entries > 0
            && self
                .elements
                .keys()
                .next_back()
                .is_some_and(|last| element_id > *last)
    }
}

/// The field of an element-record element that holds its `BuiltInCategory`.
const BUILTIN_CATEGORY_FIELD: &str = "m_builtinCategory";

/// Whether `version`'s partition records store the ElementId as a `u64`
/// (Revit 2024 and later) rather than a `u32`.
fn wide_record_ids(version: u32) -> bool {
    !crate::partition_element_records_2023::is_32bit_release(version)
}

/// An entry whose record cannot be read again: carrier `Other`, and no stream
/// or offset to give.
fn unplace(entry: &mut ElementProvenanceEntry) {
    entry.carrier = ElementCarrier::Other;
    entry.stream = None;
    entry.offset = None;
    entry.record_offset = None;
    entry.enclosing_record_id = None;
}

fn field_integer(element: &DecodedElement, wanted: &str) -> Option<i64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
}

/// The stream an element's record was read from and the offset in it, where
/// the record kind is one that has them (#1326): the same places
/// [`ElementProvenanceTable`] gives. Not read again from the bytes.
pub(crate) fn placement(element: &DecodedElement) -> (Option<String>, Option<usize>) {
    let provenance = &element.provenance;
    match provenance.record_kind.as_deref() {
        Some("schema_instance") => (
            Some(streams::GLOBAL_LATEST.to_string()),
            provenance.source_offset,
        ),
        Some("partition_element_record" | "partition_arcwall") => {
            (provenance.source_stream.clone(), provenance.source_offset)
        }
        Some(kind) if kind == crate::partition_area_records::AREA_RECORD_SOURCE => {
            (provenance.source_stream.clone(), provenance.source_offset)
        }
        _ => (None, None),
    }
}

/// The entry for `element` before the partition frames are read again: its
/// carrier as far as its record kind says, its stream, offset and class.
fn entry_of(
    element: &DecodedElement,
    classes: Option<&SchemaClasses>,
    tag_by_name: &HashMap<&str, u16>,
) -> ElementProvenanceEntry {
    let provenance = &element.provenance;
    let mut entry = ElementProvenanceEntry::new(ElementCarrier::Other);
    let mut by_record_tag = false;
    match provenance.record_kind.as_deref() {
        Some("schema_instance") => {
            entry.carrier = ElementCarrier::GlobalLatestSchemaWalk;
            entry.stream = Some(streams::GLOBAL_LATEST.to_string());
            entry.offset = provenance.source_offset;
        }
        Some("partition_element_record") => {
            // First or second prologue is read from the frame's own bytes.
            entry.carrier = ElementCarrier::PartitionElementRecordFirstPrologue;
            entry.stream = provenance.source_stream.clone();
            entry.offset = provenance.source_offset;
            by_record_tag = true;
        }
        Some("partition_arcwall") => {
            entry.carrier = ElementCarrier::PartitionArcWall;
            entry.stream = provenance.source_stream.clone();
            entry.offset = provenance.source_offset;
        }
        Some(kind) if kind == crate::partition_area_records::AREA_RECORD_SOURCE => {
            entry.carrier = ElementCarrier::PartitionAreaRecord;
            entry.stream = provenance.source_stream.clone();
            entry.offset = provenance.source_offset;
            entry.record_offset = provenance.source_offset;
        }
        Some("partition_materials" | "partition_schema_mvp" | "partition_rect_opening_index") => {
            entry.carrier = ElementCarrier::PartitionDerived;
        }
        _ => {}
    }
    if by_record_tag {
        entry.builtin_category = field_integer(element, BUILTIN_CATEGORY_FIELD);
        entry.class_tag = field_integer(element, crate::partition_schema_mvp::CLASS_TAG_FIELD)
            .and_then(|tag| u16::try_from(tag).ok());
        entry.class_name = entry
            .class_tag
            .and_then(|tag| classes.and_then(|classes| classes.by_tag(tag)))
            .map(|class| class.name.clone());
    } else if let Some(&tag) = tag_by_name.get(element.class.as_str()) {
        entry.class_tag = Some(tag);
        entry.class_name = Some(element.class.clone());
    }
    entry
}

/// Read each partition element record's first bytes again to tell a frame that
/// starts its record from one inside another's, and find the record that holds
/// the ElementId of the second kind.
fn classify_frames(
    rf: &mut RevitFile,
    entries: &mut BTreeMap<u32, ElementProvenanceEntry>,
    pending: BTreeMap<String, Vec<u32>>,
    layout_release: Option<u32>,
) {
    use crate::partition_element_records as per;
    use crate::partition_element_records_2023 as per32;

    let Some(version) = layout_release else {
        for id in pending.values().flatten() {
            if let Some(entry) = entries.get_mut(id) {
                unplace(entry);
            }
        }
        return;
    };
    let wide = wide_record_ids(version);
    let marker = if wide {
        per::file_bbox_marker(rf, version)
    } else {
        None
    };
    let header_tag = if wide {
        None
    } else {
        per32::record_marker(rf).map(|(_, header_tag)| header_tag)
    };
    let declared: BTreeSet<u32> = if wide {
        BTreeSet::new()
    } else {
        crate::elem_table::parse_records(rf)
            .map(|records| crate::elem_table::declared_ids(&records))
            .unwrap_or_default()
    };
    for (stream, ids) in pending {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            for id in &ids {
                if let Some(entry) = entries.get_mut(id) {
                    unplace(entry);
                }
            }
            continue;
        };
        let buf = inflated.bytes();
        let mut chain: Option<Vec<per::PartitionRecordSpan>> = None;
        let mut spans: Option<Vec<per32::RecordSpan>> = None;
        for id in ids {
            let Some(entry) = entries.get_mut(&id) else {
                continue;
            };
            let Some(offset) = entry.offset else {
                continue;
            };
            let starts_record = if wide {
                le_bytes::u64_at(buf, offset) == Some(u64::from(id))
            } else {
                le_bytes::u32_at(buf, offset) == Some(id)
            };
            if starts_record {
                entry.carrier = ElementCarrier::PartitionElementRecordFirstPrologue;
                entry.record_offset = Some(offset);
                continue;
            }
            entry.carrier = ElementCarrier::PartitionElementRecordSecondPrologue;
            if wide {
                if let Some(marker) = marker {
                    let records =
                        chain.get_or_insert_with(|| per::partition_record_chain(buf, &marker));
                    if let Some(record) = per::enclosing_record(records.as_slice(), offset) {
                        if u32::try_from(record.element_id) == Ok(id) {
                            entry.record_offset = Some(record.start);
                            entry.enclosing_record_id = Some(id);
                        }
                    }
                }
            } else if let Some(header_tag) = header_tag {
                let records =
                    spans.get_or_insert_with(|| per32::record_spans(buf, header_tag, &declared));
                if let Some(record) = per32::enclosing_span(records.as_slice(), offset) {
                    if record.element_id == id {
                        entry.record_offset = Some(record.start);
                        entry.enclosing_record_id = Some(id);
                    }
                }
            }
        }
    }
}

/// A native record's class tag and the offset of its header in the partition
/// inflated, as the record index holds them.
type IndexedCopy = (Option<u16>, Option<usize>);

/// Give each listed element the native record the export's native passes
/// indexed for it, and say which copy it is where several partitions hold one.
fn attach_native_records(
    rf: &mut RevitFile,
    entries: &mut BTreeMap<u32, ElementProvenanceEntry>,
    classes: Option<&SchemaClasses>,
    table: &mut ElementProvenanceTable,
) {
    let Some(index) = rf.native_record_index() else {
        return;
    };
    table.native_record_index = true;
    // Each listed element's records, by partition.
    let mut copies: HashMap<u32, BTreeMap<u32, IndexedCopy>> = HashMap::new();
    for &(_, id, partition, tag, offset) in index.records() {
        let Ok(id) = u32::try_from(id) else {
            continue;
        };
        if !entries.contains_key(&id) {
            continue;
        }
        copies
            .entry(id)
            .or_default()
            .entry(partition)
            .or_insert((tag, offset));
    }
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (&id, held) in &copies {
        if held.len() > 1 {
            holders.insert(id, held.keys().copied().collect());
        }
    }
    let current = crate::native_document::current_partitions(rf, &holders);
    for (id, held) in copies {
        let Some(entry) = entries.get_mut(&id) else {
            continue;
        };
        let count = u32::try_from(held.len()).unwrap_or(u32::MAX);
        let (copy, chosen) = if held.len() == 1 {
            (
                NativeRecordCopy::Only,
                held.iter()
                    .next()
                    .map(|(partition, value)| (*partition, *value)),
            )
        } else {
            match current
                .get(&id)
                .and_then(|partition| held.get(partition).map(|value| (*partition, *value)))
            {
                Some(chosen) => (NativeRecordCopy::Current, Some(chosen)),
                None => (NativeRecordCopy::Unresolved, None),
            }
        };
        let class_tag = chosen.and_then(|(_, (tag, _))| tag);
        let reference = NativeRecordRef {
            copies: count,
            copy,
            stream: chosen.map(|(partition, _)| format!("Partitions/{partition}")),
            offset: chosen.and_then(|(_, (_, offset))| offset),
            class_tag,
            class_name: class_tag
                .and_then(|tag| classes.and_then(|classes| classes.by_tag(tag)))
                .map(|class| class.name.clone()),
        };
        table.native.with_record += 1;
        if reference.stream.is_some() && reference.offset.is_some() {
            table.native.located += 1;
        } else {
            table.native.unlocated += 1;
        }
        if held.len() > 1 {
            table.native.several_copies += 1;
        }
        if copy == NativeRecordCopy::Unresolved {
            table.native.copy_unresolved += 1;
        }
        entry.native = Some(reference);
    }
}

/// The reader name of the value source of a body read from the element's own
/// frame ([`CopyRule::Selected`]).
pub const SELECTED_FRAME_READER: &str = "selected_frame";

/// The reader name of the value source of a body whose box and placement are
/// read from the element's own frame and whose profile is read from another
/// record (sketch lines, or the type's section): the frame is named for the
/// box only, and the profile has no source yet.
pub const SELECTED_FRAME_BOX_READER: &str = "selected_frame_box";

/// The reader name of the value source of a Sketch's curve list (#1368).
const SKETCH_LIST_READER: &str = "sketch_list";

/// The readers of a profile's sketch lines (#1368): the frames of the lines,
/// and the data of the lines whose outline is read from their own ends.
const SKETCH_LINE_READERS: &[&str] = &["sketch_lines", "sketch_line_data"];

/// The readers that source a profile's records for a box body, the ones that
/// take its place as `selected_frame_box` (#1368): the sketch lines' frames,
/// or the type symbol's frame. The Sketch's curve list or the lines' data
/// alone do not.
const PROFILE_FRAME_READERS: &[&str] = &[
    "sketch_lines",
    "type_section",
    "room_solid_faces",
    "type_hollow_section",
    "type_i_section",
    "pipe_connectors",
];

/// The readers whose value a body with this `BodySource` is drawn from (#1360):
/// a value source is listed for an element only when its reader is one of
/// them, so a line a reader took for a wall that was drawn as its box is not
/// claimed as the wall's body. A `BodySource` no row names has no value source
/// yet.
fn body_readers(body: &str) -> &'static [&'static str] {
    use super::export_content as ec;
    match body {
        ec::RECORD_BBOX_BODY_SOURCE
        | crate::element_record_wall_joins::WALL_BODY_JOIN_TRIMMED
        | crate::element_record_column_cuts::COLUMN_BODY_JOIN_CUT
        | crate::element_record_beam_cuts::BEAM_BODY_COLUMN_CUT => &[SELECTED_FRAME_READER],
        ec::WALL_CENTRELINE_BODY_SOURCE
        | ec::WALL_TAPERED_BODY_SOURCE
        | ec::WALL_LAYER_JOIN_BODY_SOURCE
        | ec::BEAM_AXIS_BODY_SOURCE
        | ec::BEAM_BOX_BODY_SOURCE => &["bounded_line"],
        ec::WALL_ARC_BODY_SOURCE => &["bounded_arc"],
        ec::INSTANCE_TURNED_BODY_SOURCE | ec::INSTANCE_ORIENTED_BODY_SOURCE => {
            &["instance_transform"]
        }
        super::saved_meshes::SAVED_MESH_BODY_SOURCE => &["saved_graphics"],
        ec::SPIRAL_RUN_BODY_SOURCE => &["native_run"],
        ec::STAIR_RUN_BODY_SOURCE => &[
            "stair_run_lines",
            "stair_run_ends",
            "stair_run_type",
            "stair_run_base_cuts",
            "native_run",
            "native_run_type",
        ],
        ec::COLUMN_HOLLOW_SECTION_BODY_SOURCE => &[SELECTED_FRAME_READER, "type_hollow_section"],
        ec::COLUMN_I_SECTION_BODY_SOURCE => &[
            SELECTED_FRAME_READER,
            "instance_transform",
            "type_i_section",
        ],
        ec::PIPE_AXIS_BODY_SOURCE => &[SELECTED_FRAME_READER, "pipe_connectors"],
        ec::ROOF_SLOPE_BODY_SOURCE => &[
            SELECTED_FRAME_READER,
            "sketch_lines",
            "sketch_list",
            "sketch_line_data",
            "roof_slope_edges",
            "roof_line_data",
            "roof_fascia_depth",
            "roof_rafter_cut",
            "roof_type_layers",
        ],
        ec::ROOF_EXTRUSION_BODY_SOURCE => &[
            SELECTED_FRAME_READER,
            "sketch_lines",
            "sketch_list",
            "roof_line_data",
            "roof_type_layers",
        ],
        ec::LANDING_TREAD_BODY_SOURCE => &["native_landing"],
        ec::LANDING_SLAB_BODY_SOURCE => &["landing_lines", "native_landing", "native_landing_type"],
        _ => &[],
    }
}

/// The readers whose value a profile with this `ProfileSource` is drawn from
/// (#1368): the sketch lines and the Sketch's curve list for a profile solved
/// from sketch lines, the type symbol's frame for a type's section.
fn profile_readers(profile: Option<&str>) -> &'static [&'static str] {
    match profile {
        Some(crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE) => {
            &["sketch_lines", "sketch_list", "sketch_line_data"]
        }
        Some(crate::partition_schema_mvp::TYPE_PROFILE_SOURCE) => &["type_section"],
        Some(source) if crate::partition_room_boundaries::is_room_outline_source(source) => {
            &["room_solid_faces"]
        }
        _ => &[],
    }
}

/// Give each listed element the value sources the readers noted on `rf` while
/// the export ran with `RevitFile::record_value_sources` on, and the frame
/// itself as the source of a body read from it, each only where the final
/// `model`'s `BodySource` or `ProfileSource` for the element is drawn from that
/// reader ([`body_readers`], [`profile_readers`]); then count them by reader
/// (#1360). Call it once the model is final: the saved graphics and the family
/// extrusions replace bodies after the table is built.
pub(crate) fn attach_value_sources(
    table: &mut ElementProvenanceTable,
    rf: &RevitFile,
    model: &IfcModel,
) {
    let noted = rf.value_sources();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            type_guid,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        let Some(id) = type_guid.as_deref().and_then(|tag| tag.parse::<u32>().ok()) else {
            continue;
        };
        let mut body = None;
        let mut profile_source = None;
        let mut profile_resolved = false;
        for property in property_set.iter().flat_map(|set| &set.properties) {
            match (property.name.as_str(), &property.value) {
                ("BodySource", PropertyValue::Text(text)) => body = Some(text.as_str()),
                ("ProfileSource", PropertyValue::Text(text)) => {
                    profile_source = Some(text.as_str())
                }
                ("ProfileResolved", PropertyValue::Boolean(true)) => profile_resolved = true,
                _ => {}
            }
        }
        let Some(body) = body else {
            continue;
        };
        let body_list = body_readers(body);
        let profile_list: &[&str] = if profile_resolved
            && (body_list.contains(&SELECTED_FRAME_READER) || profile_source == Some(body))
        {
            profile_readers(profile_source)
        } else {
            &[]
        };
        let Some(entry) = table.elements.get_mut(&id) else {
            continue;
        };
        for source in noted.and_then(|noted| noted.get(&id)).into_iter().flatten() {
            let reader = source.reader.as_str();
            if !(body_list.contains(&reader) || profile_list.contains(&reader))
                || entry
                    .value_sources
                    .iter()
                    .any(|held| held.reader == source.reader)
            {
                continue;
            }
            let mut source = source.clone();
            source.differs_from_frame = match &entry.stream {
                Some(frame) if source.has_place() => !source.held_in(frame),
                _ => false,
            };
            entry.value_sources.push(source);
        }
        // A profile's sketch lines are in the partitions of the lines; the
        // profile is held in the frame's partition too when the Sketch's curve
        // list, which names those lines, has a copy there.
        let list_in_frame = entry.stream.as_deref().is_some_and(|frame| {
            entry
                .value_sources
                .iter()
                .any(|held| held.reader == SKETCH_LIST_READER && held.held_in(frame))
        });
        if list_in_frame {
            for held in entry
                .value_sources
                .iter_mut()
                .filter(|held| SKETCH_LINE_READERS.contains(&held.reader.as_str()))
            {
                held.differs_from_frame = false;
            }
        }
        // A box whose profile was read from sketch lines or the type's section
        // takes only its box and placement from the frame, unless a reader
        // noted the profile's records.
        let profile_elsewhere = profile_resolved && profile_source != Some(body);
        if body_list.contains(&SELECTED_FRAME_READER)
            && !entry
                .value_sources
                .iter()
                .any(|source| source.rule == CopyRule::Selected)
        {
            let profile_sourced = entry
                .value_sources
                .iter()
                .any(|source| PROFILE_FRAME_READERS.contains(&source.reader.as_str()));
            let reader = if profile_elsewhere && !profile_sourced {
                SELECTED_FRAME_BOX_READER
            } else {
                SELECTED_FRAME_READER
            };
            let source = ValueSource::new(
                reader,
                CopyRule::Selected,
                1,
                entry.stream.clone(),
                entry.offset,
            );
            entry.value_sources.push(source);
        }
    }
    table.value_source_counts.clear();
    for entry in table.elements.values() {
        for source in &entry.value_sources {
            let count = table
                .value_source_counts
                .entry(source.reader.clone())
                .or_default();
            count.elements += 1;
            count.differ_from_frame += usize::from(source.differs_from_frame);
            count.current += usize::from(source.rule == CopyRule::Current);
        }
    }
}

/// The provenance of every element the production walk of `rf` decoded, under
/// `limits`. The walk is the one the export made, which the handle memoised;
/// when it was not made, it is made here. A walk that fails gives an empty
/// table.
pub fn build(rf: &mut RevitFile, limits: WalkerLimits) -> ElementProvenanceTable {
    let mut table = ElementProvenanceTable::default();
    let Ok(elements) = walk_shared(
        rf,
        PRODUCTION_ELEMENT_MIN_SCORE,
        limits,
        &WalkerControl::default(),
    ) else {
        return table;
    };
    table.layout_release = rf.layout_release().ok();
    table.partition_count = rf.partition_stream_names().len();
    table.record_id_bytes = table
        .layout_release
        .map(|version| if wide_record_ids(version) { 8 } else { 4 });
    let classes = rf.memoised_schema_classes().ok();
    let classes = classes.as_deref();
    let mut tag_by_name: HashMap<&str, u16> = HashMap::new();
    if let Some(classes) = classes {
        for class in &classes.classes {
            // The last class of a name wins, as in the walk's own lookup by
            // name.
            tag_by_name.insert(class.name.as_str(), class.tag);
        }
    }

    let mut entries: BTreeMap<u32, ElementProvenanceEntry> = BTreeMap::new();
    let mut pending: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for element in elements.iter() {
        if !element.meets_confidence(DEFAULT_MIN_ELEMENT_CONFIDENCE) {
            continue;
        }
        let Some(id) = element.id else {
            table.unkeyed_elements += 1;
            continue;
        };
        if entries.contains_key(&id) {
            continue;
        }
        let entry = entry_of(element, classes, &tag_by_name);
        if entry.carrier == ElementCarrier::PartitionElementRecordFirstPrologue {
            if let (Some(stream), Some(_)) = (&entry.stream, entry.offset) {
                pending.entry(stream.clone()).or_default().push(id);
            }
        }
        entries.insert(id, entry);
    }
    classify_frames(rf, &mut entries, pending, table.layout_release);
    attach_native_records(rf, &mut entries, classes, &mut table);

    for entry in entries.values() {
        let count = table
            .by_carrier
            .entry(entry.carrier.as_str().to_string())
            .or_default();
        count.elements += 1;
        if entry.is_located() {
            count.located += 1;
        } else {
            count.unlocated += 1;
        }
    }
    let first_omitted = entries.keys().nth(MAX_ELEMENT_PROVENANCE_ENTRIES).copied();
    if let Some(first_omitted) = first_omitted {
        table.omitted_entries = entries.split_off(&first_omitted).len();
    }
    table.elements = entries;
    table
}
