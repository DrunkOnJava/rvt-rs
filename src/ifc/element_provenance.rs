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
//! 7.4 MB of JSON against 53 KB of diagnostics on the BIM4LCA office as built
//! (Measure run 41831418684), so it is off by default. It is never written
//! into the IFC.
//!
//! An offset indexes the stream as `RevitFile::inflated_partition` returns it,
//! which is what `rvt-dump` writes as `Partitions_NN.decomp` (#291). For the
//! schema walk it indexes `Global/Latest` inflated as the walk inflates it
//! (`Global_Latest.decomp`).
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
    /// could not be read again: no offset is given.
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
    /// The partition of that copy, such as `Partitions/46`.
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
    /// Where the record or frame starts in that stream inflated.
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
/// offset given.
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
    /// The native records of the listed elements.
    #[serde(default)]
    pub native: NativeRecordCount,
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
}

/// The field of an element-record element that holds its `BuiltInCategory`.
const BUILTIN_CATEGORY_FIELD: &str = "m_builtinCategory";

/// Whether `version`'s partition records store the ElementId as a `u64`
/// (Revit 2024 and later) rather than a `u32`.
fn wide_record_ids(version: u32) -> bool {
    !crate::partition_element_records_2023::is_32bit_release(version)
}

fn field_integer(element: &DecodedElement, wanted: &str) -> Option<i64> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == wanted => Some(*value),
        _ => None,
    })
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
                entry.carrier = ElementCarrier::Other;
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
                    entry.carrier = ElementCarrier::Other;
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
            tag_by_name.entry(class.name.as_str()).or_insert(class.tag);
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
