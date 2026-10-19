//! Revit 2024 Areas from their partition chain records (#763).
//!
//! An Area's chain record (RE-35) carries `OST_Areas` (-2003200) at
//! `+0x12` and no bounding-box marker at `+0x50`, so the element-record
//! scan, which finds records by that marker, never reads one. Its frame
//! holds RE-21's instance words: the container ElementId at `+0x32`, unset
//! (`ff` × 8) on every Area, and the placement kind at `+0x42`. After them
//! come two ascending lists of ElementIds, one of six (the Level, two ids of
//! the area scheme, two more and the Area's own) and one of two (the Level
//! and one more). Revit's export writes each placed Area as an `IfcSpace`
//! in its Level's storey: `Name` its number and `LongName` its name, which
//! RE-117's `ROOM_NUMBER` / `ROOM_NAME` entries hold as they do a room's.
//!
//! Measured on the BIM4LCA office (`research/bim4lca-measure.tsv`) against
//! Revit's IFC export, matched by Revit's GlobalId:
//!
//! - as built, 30 Areas, all placed, all 30 Revit's area spaces, with
//!   number, name and storey equal on 30 of 30;
//! - at building permit, 32 Areas. The 30 with the placed-instance kind are
//!   Revit's 30, with storey equal on 30 of 30. The other 2 carry
//!   `0xffff8020` and are not in Revit's export. Their copies in the latest
//!   partition (#800) give the number and name.
//!
//! The Level is the one recovered Level id the record names, as
//! [`crate::element_record_level_refs::unique_level_reference`] takes it from
//! an element record's list. Where an Area's record has copies in several
//! partitions, the latest partition's is read (RE-145). An Area in a
//! non-primary design option is left out, as an element record is (RE-40):
//! Revit's exporter collects spatial elements through the same design-option
//! filter as the rest (`ElementFilteringUtil.GetExportFilter` in revit-ifc,
//! CLEANROOM.md source 5), and skips an Area with no location.
//!
//! # Honesty
//!
//! - No geometry is read. Revit's export draws each area space as an
//!   `IfcExtrudedAreaSolid`. The Area's boundary is not decoded, so its
//!   space has no body.
//! - Revit 2024 only, the one release measured; elsewhere nothing is read
//!   (fail closed).

use crate::RevitFile;
use crate::partition_element_records::{
    CATEGORY_OFFSET, CONTAINER_NONE, CONTAINER_OFFSET, PLACEMENT_KIND_INSTANCE,
    PLACEMENT_KIND_OFFSET, file_bbox_marker, frame_design_option, partition_record_chain,
};
use crate::partition_room_parameters::{LatestCopy, partition_number};
use crate::walker::{DecodedElement, ElementProvenance, InstanceField};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where the Area record is measured.
pub const AREA_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Autodesk `BuiltInCategory.OST_Areas`.
pub const OST_AREAS: i64 = -2_003_200;

/// The `m_source` of an Area read here.
pub const AREA_RECORD_SOURCE: &str = "partition_area_record";

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// One Area record as its latest partition holds it.
struct AreaRecord {
    stream: String,
    offset: usize,
    bytes: Vec<u8>,
}

/// Two copies of a record agree when their bytes do, wherever they sit.
impl PartialEq for AreaRecord {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

/// Every declared Area's chain record, its latest partition's copy; an Area
/// whose copies in that partition differ is left out.
fn area_records(
    rf: &mut RevitFile,
    marker: &[u8; 8],
    declared: &BTreeSet<u32>,
) -> BTreeMap<u32, AreaRecord> {
    let mut found = LatestCopy::default();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let partition = partition_number(&stream);
        let buf = inflated.bytes();
        for span in partition_record_chain(buf, marker) {
            let Ok(id) = u32::try_from(span.element_id) else {
                continue;
            };
            if !declared.contains(&id)
                || read_u64(buf, span.start + CATEGORY_OFFSET).map(|c| c as i64) != Some(OST_AREAS)
            {
                continue;
            }
            found.offer(
                id,
                partition,
                AreaRecord {
                    stream: stream.clone(),
                    offset: span.start,
                    bytes: buf[span.start..span.end].to_vec(),
                },
            );
        }
    }
    found.into_values()
}

/// The one id of `level_ids` that `bytes` holds as a `u64` past the frame's
/// fixed head, or `None` for none or several.
fn named_level(bytes: &[u8], level_ids: &BTreeSet<u32>) -> Option<u32> {
    let mut named = BTreeSet::new();
    for at in CATEGORY_OFFSET..bytes.len().saturating_sub(7) {
        if let Some(id) = read_u64(bytes, at)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|id| level_ids.contains(id))
        {
            named.insert(id);
        }
    }
    let mut ids = named.into_iter();
    match (ids.next(), ids.next()) {
        (Some(id), None) => Some(id),
        _ => None,
    }
}

/// Every placed Area of a Revit 2024 file, as a `DecodedElement` of class
/// `Area` with its number, name and Level where they are read. Empty on
/// every other release.
pub fn areas_from_partition_chain_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    if !AREA_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Vec::new();
    }
    let Some(marker) = file_bbox_marker(rf, revit_version) else {
        return Vec::new();
    };
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Vec::new(),
    };
    let options = rf.design_options();
    let placed: BTreeMap<u32, AreaRecord> = area_records(rf, &marker, &declared)
        .into_iter()
        .filter(|(_, record)| {
            read_u64(&record.bytes, CONTAINER_OFFSET) == Some(CONTAINER_NONE)
                && read_u32(&record.bytes, PLACEMENT_KIND_OFFSET) == Some(PLACEMENT_KIND_INSTANCE)
                && !frame_design_option(&record.bytes, 0)
                    .is_some_and(|option| options.is_non_primary(option))
        })
        .collect();
    let ids: BTreeSet<u32> = placed.keys().copied().collect();
    let entries =
        crate::partition_room_parameters::scan_room_parameter_entries(rf, revit_version, &ids);
    placed
        .into_iter()
        .map(|(id, record)| {
            let level = named_level(&record.bytes, level_ids);
            let mut fields = vec![
                (
                    "m_builtinCategory".into(),
                    InstanceField::Integer {
                        value: OST_AREAS,
                        signed: true,
                        size: 8,
                    },
                ),
                (
                    "m_source_stream".into(),
                    InstanceField::String(record.stream.clone()),
                ),
                (
                    "m_source_offset".into(),
                    InstanceField::Integer {
                        value: record.offset as i64,
                        signed: false,
                        size: 8,
                    },
                ),
                (
                    "m_source".into(),
                    InstanceField::String(AREA_RECORD_SOURCE.into()),
                ),
                ("m_level_bound".into(), InstanceField::Bool(level.is_some())),
            ];
            if let Some(level) = level {
                fields.push((
                    crate::element_record_level_refs::LEVEL_REFERENCE_FIELD.into(),
                    InstanceField::ElementId { tag: 0, id: level },
                ));
            }
            if let Some((number, name)) = entries.get(&id) {
                fields.push(("m_name".into(), InstanceField::String(name.clone())));
                fields.push((
                    crate::partition_schema_mvp::ROOM_NUMBER_FIELD.into(),
                    InstanceField::String(number.clone()),
                ));
            }
            DecodedElement {
                id: Some(id),
                class: "Area".into(),
                fields,
                byte_range: record.offset..record.offset + record.bytes.len(),
                provenance: ElementProvenance::partition(
                    &record.stream,
                    record.offset,
                    AREA_RECORD_SOURCE,
                    "partition_area_records::areas_from_partition_chain_records",
                    0.8,
                    level.is_none().then_some("level_binding_unresolved"),
                ),
            }
        })
        .collect()
}
