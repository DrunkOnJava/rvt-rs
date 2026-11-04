//! Revit 2015 to 2027 Areas from their partition chain records (#763, #1194).
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
//! # The body (RE-317)
//!
//! Revit's export draws each area space as an `IfcExtrudedAreaSolid` of the
//! Area's boundary, from its Level up to the Level above, when both are
//! building stories. The boundary is not
//! in the Area. It is the region around the Area's point that the lines of
//! its area scheme on its Level enclose ([`crate::partition_area_boundaries`]),
//! and the point is two `f64`s at [`AREA_POINT_OFFSET`] in the payload of the
//! Area's data object (RE-153). On the BIM4LCA office all 30 Areas, as built
//! and at building permit, equal Revit's in area (within 0.1%) and in outline
//! (within 0.01 ft), and 29 of the 30 in the mesh's vertices
//! (`reports/element-framing/RE-317-area-boundaries.md`).
//!
//! The region of Revit's own plan topology comes first
//! ([`crate::native_area_regions`], RE-333): the loop Revit computed for the
//! Area, with each line moved by its area rule's offset and arcs drawn, checked
//! against the area Revit saves for it and against the Area's point. Where it
//! is not read or fails a check, the region is the lines' arrangement with its
//! joins (RE-317, RE-330). Neither is asked of a group of lines that cannot be
//! read as a whole (a non-primary design option, a stray height, copies that
//! disagree, a missing line). The topology draws the Areas whose lines meet
//! only once offset, or include an arc or a slanted line; where both give a
//! region they are expected to agree (RE-333 section 7 has the counts). Every
//! other check below applies to it as to the lines' region.
//!
//! # Honesty
//!
//! - An Area has a body only where its point lies in exactly one region of
//!   its scheme's lines on its Level, the lines it names bound that region,
//!   every line of the scheme on the Level is in the primary design option,
//!   no line that is not a straight line along the plan axes lies where the
//!   region does (RE-330), and every declared area boundary line of the file
//!   was read and placed on a Level. The lines its record names must be
//!   declared lines of that scheme on that Level whose copies agree. An Area
//!   whose region another placed Area of its scheme and Level shares has none,
//!   whatever stops the other from being drawn. Otherwise it has none, as
//!   before, and says why in [`AREA_BODY_REFUSAL_FIELD`].
//! - Lines that stop short of the line they join are carried on to it, and
//!   parallel lines that stop at the same place a wall's thickness apart are
//!   joined by a connector, where their records name each other and the gap is
//!   at most 1 ft (RE-330; the premise is inferred from one IFC2X3 export). A
//!   line that is not a straight line along an axis (its record holds its box
//!   and no more) is left out of the arrangement, and so is an Area whose
//!   region its box touches, or a line that names it bounds.
//! - A line in a non-primary design option makes its group unreadable: no area
//!   boundary line of the office or of Autodesk's 2024 samples has a design
//!   option, so how Revit treats one is not measured.
//! - The height is the distance to the Level above, where that Level and the
//!   Area's own are building stories and no other Level lies between them or at
//!   either elevation. Revit's exporter takes the Level's "story above" first
//!   (`LEVEL_UP_TO_LEVEL`), which is not read, and ignores the Area's own
//!   upper Level; a Level whose story above is not the next Level up is not
//!   measured. An Area on the top storey has none.
//! - The base offset of the Area is read as zero, which is what the office
//!   measures.
//! - An Area whose lines do not close, even with their joins, or meet only once
//!   moved by their area rule's offsets, has its point in no region of the
//!   lines' boxes; its region is Revit's plan topology's where that is read and
//!   passes its checks (RE-333), and none otherwise.
//! - Revit 2015 to 2027 are read (RE-324, RE-333); Revit 2025 to 2027 are
//!   measured against the same project's 2024 save, not against a Revit
//!   export. Earlier releases are not read (fail closed).
//!
//! # Revit 2015 to 2023 (RE-324)
//!
//! The 32-bit layout (RE-315) writes the same record with `u32` ElementIds:
//! the `BuiltInCategory` 14 bytes after the record's ElementId
//! ([`NARROW_CATEGORY_OFFSET`]), the design option, container and placement
//! words 12, 16 and 24 bytes after it, and the same two counted lists of
//! ElementIds, now `u32`. The record's partition walk is
//! [`crate::partition_element_records_2023::record_spans`]. Each boundary line
//! is a 32-bit element record, the Area's point is the same two `f64`s
//! [`NARROW_POINT_OFFSET`] bytes after the `01 00 00 00 · ElementId` header of
//! its element data, and the number and name are read as for a room. On
//! Autodesk's `rac_advanced` of 2015 to 2023 (25 Areas) and `rst_basic` of 2016
//! to 2023 (1) every field equals the same project's 2024 save
//! (`reports/element-framing/RE-324-areas-2015-2023.md`). Releases 2016 to 2023
//! leave out an Area in a non-primary design option, as 2024 does; 2015 reads
//! no option sets, so an Area that names any option is left out there.
//!
//! **An Area of Revit 2015 to 2023 is written only where its region and body
//! are proven.** The first rule, writing every placed Area as 2024 does, wrote
//! spaces that Revit's export lacks on a held-out project (RE-324). Revit's
//! exporter makes no space for an Area whose boundary yields no loop (a
//! redundant or not-enclosed Area, `SpatialElementExporter.CreateIFCSpace` in
//! revit-ifc, CLEANROOM.md source 5), and the lines of its scheme are all that
//! is read of an Area's boundary: a proven region is a boundary. The Areas left
//! out are counted by the reason their body has none in the export diagnostics
//! (`area_without_proven_body`).

use crate::RevitFile;
use crate::element_record_plan_profiles::{PLAN_PROFILE_SOURCE_FIELD, PlanProfile};
use crate::le_bytes;
use crate::native_area_regions::AreaRegion;
use crate::partition_area_boundaries::{AreaFace, Arrangement, BoundaryLine};
use crate::partition_design_options::DesignOptions;
use crate::partition_element_records::{
    CATEGORY_OFFSET, CLASS_TAG_OFFSET, CONTAINER_NONE, CONTAINER_OFFSET, PLACEMENT_KIND_INSTANCE,
    PLACEMENT_KIND_OFFSET, PartitionElementRecord, file_bbox_marker, frame_design_option,
    partition_record_chain, scan_category_records_multi,
};
use crate::partition_element_records_2023 as per32;
use crate::partition_level_records::PartitionLevel;
use crate::partition_room_parameters::{
    DATA_OBJECT_HEADER, LatestCopy, data_objects, partition_number,
};
use crate::walker::{DecodedElement, ElementProvenance, InstanceField};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where the Area record is measured: Revit 2015 to 2023 (RE-324),
/// 2024 (RE-191), and 2025 to 2027, whose records are 2024's after the
/// entries some carry before their body (RE-333): Autodesk's `rac_advanced`
/// gets the Areas of its 2024 save, and Snowdon Towers Architectural (2025)
/// the 101 of Revit's own export.
pub const AREA_SUPPORTED_REVIT_VERSIONS: &[u32] = &[
    2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027,
];

/// Bytes from the start of a Revit 2015 to 2023 Area record to its
/// `BuiltInCategory` (RE-324): the record's `u32` ElementId, `u32` size, the
/// tag of `ElementHeader`, a `u16` count and a `u16`.
pub const NARROW_CATEGORY_OFFSET: usize = 14;

/// Where a Revit 2015 to 2023 Area record keeps its design option, a `u32`,
/// `ff ff ff ff` for the main model: 12 bytes after the category (RE-315).
const NARROW_DESIGN_OPTION_OFFSET: usize = NARROW_CATEGORY_OFFSET + 12;

/// Where it keeps its container reference, a `u32`, `ff ff ff ff` unset: 16
/// bytes after the category (RE-315).
const NARROW_CONTAINER_OFFSET: usize = NARROW_CATEGORY_OFFSET + 16;

/// Where it keeps its placement kind, a `u32`: 24 bytes after the category
/// (RE-315). `0xffffef7f` on every Area measured, 2015 to 2018 included.
const NARROW_PLACEMENT_OFFSET: usize = NARROW_CATEGORY_OFFSET + 24;

/// Where the counted lists of ElementIds are looked for in a Revit 2015 to 2023
/// Area record: past the placement word.
const NARROW_LISTS_FROM: usize = NARROW_CATEGORY_OFFSET + 28;

/// The longest counted list read from a Revit 2015 to 2023 Area record.
const NARROW_LIST_MAX: usize = 64;

/// Bytes from the end of the `01 00 00 00 · ElementId` header of an Area's
/// element data on Revit 2015 to 2023 to its point, two `f64`s of plan feet
/// (RE-324). On Revit 2024 the point is [`AREA_POINT_OFFSET`] bytes into the
/// payload of the Area's data object.
pub const NARROW_POINT_OFFSET: usize = 35;

/// Autodesk `BuiltInCategory.OST_Areas`.
pub const OST_AREAS: i64 = -2_003_200;

/// The `m_source` of an Area read here.
pub const AREA_RECORD_SOURCE: &str = "partition_area_record";

/// Autodesk `BuiltInCategory.OST_AreaSchemes`: an area scheme.
pub const OST_AREA_SCHEMES: i64 = -2_003_201;

/// Autodesk `BuiltInCategory.OST_AreaSchemeLines`: an area boundary line.
pub const OST_AREA_SCHEME_LINES: i64 = -2_000_079;

/// Where the point of an Area is in the payload of its data object, two
/// `f64`s of plan feet (RE-317).
pub const AREA_POINT_OFFSET: usize = 107;

/// The source of an Area's plan profile and of its body (RE-317).
pub const AREA_BOUNDARY_SOURCE: &str = "partition_area_boundary_lines";

/// The field of an Area that holds the elevation of its body's base, feet.
pub const AREA_BODY_BASE_FIELD: &str = "m_area_body_base";

/// The field of an Area that holds its body's height, feet.
pub const AREA_BODY_HEIGHT_FIELD: &str = "m_area_body_height";

/// The field of an Area, true where its outline runs along a stretch of a line
/// carried on past the extent its record gives, or along a connector (RE-330).
pub const AREA_JOINED_FIELD: &str = "m_area_body_joined";

/// How far a boundary line's height may lie from its Level's, feet.
pub const AREA_LEVEL_TOLERANCE_FEET: f64 = 1e-3;

/// How near the box of a line that cannot be read must come to the box of an
/// Area's region for the Area to be refused, feet (RE-330).
pub const UNREAD_MARGIN_FEET: f64 = 0.01;

/// The least distance to the Level above that gives an Area a body, feet.
pub const AREA_MIN_HEIGHT_FEET: f64 = 1e-3;

/// The lowest ElementId read as an area boundary line in an Area's record.
pub const AREA_LINE_ID_FLOOR: u32 = 1000;

/// The field of an Area that holds why it has no body, where it has none.
pub const AREA_BODY_REFUSAL_FIELD: &str = "m_area_body_refusal";

/// An Area has no body because the Levels were not recovered.
pub const REFUSED_LEVELS: &str = "levels_not_recovered";
/// A declared area boundary line was not read, or cannot be placed on a Level:
/// no Area of the file is drawn, since the line may be in any group.
pub const REFUSED_UNACCOUNTED: &str = "boundary_line_not_read";
/// The file holds no readable area boundary line.
pub const REFUSED_NO_LINES: &str = "no_boundary_lines";
/// The Area's data object holds no single point.
pub const REFUSED_NO_POINT: &str = "point_not_read";
/// The Area's record does not name one Level and one area scheme.
pub const REFUSED_UNNAMED: &str = "level_or_scheme_not_named";
/// A line the Area's record names is not a declared line of its scheme on its
/// Level that was read: it has no Level or scheme, its copies disagree, or it is
/// in a non-primary design option.
pub const REFUSED_LINES: &str = "named_lines_not_in_scheme_and_level";
/// The Area's Level and the Level above it are not both building stories, or
/// another Level lies at the Area's.
pub const REFUSED_HEIGHT: &str = "next_storey_not_proven";
/// The lines of the Area's scheme on its Level cannot be read as an arrangement
/// of straight lines along the axes, one of them may be missing, or one is in a
/// non-primary design option; or a line that is not a straight line along an
/// axis (a slanted line, an arc, a point) lies where the Area's region does, or
/// is joined to a line that bounds it (RE-330).
pub const REFUSED_GROUP: &str = "boundary_lines_not_readable";
/// The Area's point lies in no single region of the lines.
pub const REFUSED_REGION: &str = "point_in_no_single_region";
/// The lines the Area's record names do not bound its region.
pub const REFUSED_OFF_REGION: &str = "named_lines_not_on_region";
/// Another placed Area of the same scheme and Level has the same region,
/// whether or not that Area is drawn.
pub const REFUSED_REDUNDANT: &str = "two_areas_one_region";

/// How the chain records of a file are walked (RE-324).
#[derive(Debug, Clone, Copy)]
enum Layout {
    /// Revit 2024 and later: `u64` ElementIds, the file's bbox marker.
    Wide([u8; 8]),
    /// Revit 2023 and earlier: `u32` ElementIds, the tag of `ElementHeader`.
    Narrow(u16),
}

/// A declared chain record: where it starts and ends, and its ElementId.
struct ChainSpan {
    start: usize,
    end: usize,
    id: u32,
    /// The bytes of the record's entry block (Revit 2027, RE-333), which the
    /// body follows; 0 where it has none.
    entries: usize,
}

/// Where a 2024 and later chain record says how many entries come before
/// its body: a `u16` after the class tag at `+0x0c` (RE-333).
const CHAIN_ENTRY_COUNT_OFFSET: usize = 0x0e;

/// Where the entries of a chain record start, and where its body starts when
/// it has none: the record's `BuiltInCategory` ([`CATEGORY_OFFSET`]).
const CHAIN_ENTRIES_FROM: usize = CATEGORY_OFFSET;

/// One entry before a Revit 2027 record's body: an `i64` (a
/// BuiltInParameter, negative), an `i64` that is -1, a `u16` and a `u32`.
const CHAIN_ENTRY_LEN: usize = 22;

/// The bytes of the entries before the body of the chain record at
/// `start..end`, or `None` where they do not fit or an entry's second word
/// is not -1. No 2024 file read here has any; files saved by Revit 2025 to
/// 2027 may (RE1 Architecture and Snowdon Towers on 2025, `rac_advanced` on
/// 2027), and the body after them is the one a record without them has
/// (RE-333).
fn chain_entries(buf: &[u8], start: usize, end: usize) -> Option<usize> {
    let count = usize::from(le_bytes::u16_at(buf, start + CHAIN_ENTRY_COUNT_OFFSET)?);
    let entries = count.checked_mul(CHAIN_ENTRY_LEN)?;
    if start + CHAIN_ENTRIES_FROM + entries + 8 > end {
        return None;
    }
    (0..count)
        .all(|k| {
            le_bytes::u64_at(buf, start + CHAIN_ENTRIES_FROM + k * CHAIN_ENTRY_LEN + 8)
                == Some(u64::MAX)
        })
        .then_some(entries)
}

impl Layout {
    fn of(rf: &mut RevitFile, revit_version: u32) -> Option<Self> {
        if per32::is_32bit_release(revit_version) {
            per32::record_marker(rf).map(|(_, tag)| Self::Narrow(tag))
        } else {
            file_bbox_marker(rf, revit_version).map(Self::Wide)
        }
    }

    fn category_offset(self) -> usize {
        match self {
            Self::Wide(_) => CATEGORY_OFFSET,
            Self::Narrow(_) => NARROW_CATEGORY_OFFSET,
        }
    }

    /// The declared records of one inflated partition's chain. A declared
    /// record of the wide layout whose entry block does not read is left out
    /// and counted in `dropped`: an Area among them is not seen.
    fn chain(self, buf: &[u8], declared: &BTreeSet<u32>, dropped: &mut usize) -> Vec<ChainSpan> {
        match self {
            Self::Wide(marker) => {
                let mut spans = Vec::new();
                for span in partition_record_chain(buf, &marker) {
                    let Ok(id) = u32::try_from(span.element_id) else {
                        continue;
                    };
                    if !declared.contains(&id) {
                        continue;
                    }
                    match chain_entries(buf, span.start, span.end) {
                        Some(entries) => spans.push(ChainSpan {
                            start: span.start,
                            end: span.end,
                            id,
                            entries,
                        }),
                        None => *dropped += 1,
                    }
                }
                spans
            }
            Self::Narrow(tag) => per32::record_spans(buf, tag, declared)
                .into_iter()
                .map(|span| ChainSpan {
                    start: span.start,
                    end: span.end,
                    id: span.element_id,
                    entries: 0,
                })
                .collect(),
        }
    }

    /// Whether design option sets are read on `revit_version`: on 2024 and
    /// later, and on 2016 to 2023 (RE-315), but not on 2015.
    fn reads_design_options(self, revit_version: u32) -> bool {
        match self {
            Self::Wide(_) => true,
            Self::Narrow(_) => per32::applies_export_filters(revit_version),
        }
    }
}

/// One Area record as its latest partition holds it.
struct AreaRecord {
    stream: String,
    offset: usize,
    bytes: Vec<u8>,
    layout: Layout,
    /// The counted lists of ElementIds a Revit 2015 to 2023 record holds, in
    /// order; empty on 2024.
    lists: Vec<Vec<u32>>,
}

/// Two copies of a record agree when their bytes do, wherever they sit.
impl PartialEq for AreaRecord {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

/// What the chain records of a file hold that an Area's body depends on.
struct ChainCensus {
    /// Every declared Area's chain record, its latest partition's copy; an
    /// Area whose copies in that partition differ is left out.
    areas: BTreeMap<u32, AreaRecord>,
    /// The ElementIds of the declared area schemes.
    scheme_ids: BTreeSet<u32>,
    /// The ElementIds of every declared area boundary line.
    line_ids: BTreeSet<u32>,
    /// The declared chain records of the wide layout whose entry block did not
    /// read, so that whatever they are was not seen (RE-333).
    dropped: usize,
}

/// The ascending counted lists of at least two declared ElementIds that a
/// Revit 2015 to 2023 record holds from `from` on: a `u32` count and as many
/// `u32` ElementIds (RE-324). Every Area measured holds two, of six and of two.
fn counted_lists(bytes: &[u8], from: usize, declared: &BTreeSet<u32>) -> Vec<Vec<u32>> {
    let mut out = Vec::new();
    let mut at = from;
    while at + 12 <= bytes.len() {
        let count = le_bytes::u32_at(bytes, at).unwrap_or(0) as usize;
        let list = (2..=NARROW_LIST_MAX)
            .contains(&count)
            .then(|| bytes.get(at + 4..at + 4 + 4 * count))
            .flatten()
            .map(|body| {
                body.chunks_exact(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect::<Vec<u32>>()
            })
            .filter(|ids| {
                ids.iter().all(|id| declared.contains(id)) && ids.windows(2).all(|w| w[0] < w[1])
            });
        match list {
            Some(ids) => {
                at += 4 + 4 * ids.len();
                out.push(ids);
            }
            None => at += 1,
        }
    }
    out
}

/// The chain records of the Areas, the area schemes and the area boundary
/// lines of a file.
fn area_records(rf: &mut RevitFile, layout: Layout, declared: &BTreeSet<u32>) -> ChainCensus {
    let mut found = LatestCopy::default();
    let mut schemes = BTreeSet::new();
    let mut line_ids = BTreeSet::new();
    let mut dropped = 0usize;
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let partition = partition_number(&stream);
        let buf = inflated.bytes();
        for span in layout.chain(buf, declared, &mut dropped) {
            let id = span.id;
            let category =
                le_bytes::u64_at(buf, span.start + span.entries + layout.category_offset())
                    .map(|c| c as i64);
            if category == Some(OST_AREA_SCHEMES) {
                schemes.insert(id);
                continue;
            }
            if category == Some(OST_AREA_SCHEME_LINES) {
                line_ids.insert(id);
                continue;
            }
            if category != Some(OST_AREAS) {
                continue;
            }
            // The record as Revit 2024 to 2026 write it: a 2027 record's
            // entries are left out, so its fields are where theirs are. Only
            // the wide layout has any; a narrow record is taken whole, and a
            // span that does not lie in the partition is not read.
            let bytes = match layout {
                Layout::Wide(_) => {
                    let body = span.start + CHAIN_ENTRIES_FROM;
                    match (
                        buf.get(span.start..body),
                        buf.get(body + span.entries..span.end),
                    ) {
                        (Some(head), Some(tail)) => [head, tail].concat(),
                        _ => continue,
                    }
                }
                Layout::Narrow(_) => match buf.get(span.start..span.end) {
                    Some(record) => record.to_vec(),
                    None => continue,
                },
            };
            let lists = match layout {
                Layout::Wide(_) => Vec::new(),
                Layout::Narrow(_) => counted_lists(&bytes, NARROW_LISTS_FROM, declared),
            };
            found.offer(
                id,
                partition,
                AreaRecord {
                    stream: stream.clone(),
                    offset: span.start,
                    bytes,
                    layout,
                    lists,
                },
            );
        }
    }
    ChainCensus {
        areas: found.into_values(),
        scheme_ids: schemes,
        line_ids,
        dropped,
    }
}

impl AreaRecord {
    /// Whether the container slot is unset.
    fn container_unset(&self) -> bool {
        match self.layout {
            Layout::Wide(_) => {
                le_bytes::u64_at(&self.bytes, CONTAINER_OFFSET) == Some(CONTAINER_NONE)
            }
            Layout::Narrow(_) => {
                le_bytes::u32_at(&self.bytes, NARROW_CONTAINER_OFFSET) == Some(u32::MAX)
            }
        }
    }

    /// Whether the placement word is RE-21's placed-instance kind.
    fn placed_kind(&self) -> bool {
        let at = match self.layout {
            Layout::Wide(_) => PLACEMENT_KIND_OFFSET,
            Layout::Narrow(_) => NARROW_PLACEMENT_OFFSET,
        };
        le_bytes::u32_at(&self.bytes, at) == Some(PLACEMENT_KIND_INSTANCE)
    }

    /// The design option the Area belongs to, `None` for the main model.
    fn design_option(&self) -> Option<u32> {
        match self.layout {
            Layout::Wide(_) => frame_design_option(&self.bytes, 0),
            Layout::Narrow(_) => le_bytes::u32_at(&self.bytes, NARROW_DESIGN_OPTION_OFFSET)
                .filter(|option| *option != u32::MAX),
        }
    }

    /// Whether the Area is in the primary design option, or in none. Where no
    /// option sets are read (Revit 2015), an Area that names an option is not.
    fn in_primary_option(&self, options: &DesignOptions, options_read: bool) -> bool {
        match self.design_option() {
            None => true,
            Some(option) => options_read && !options.is_non_primary(option),
        }
    }

    /// Every id of `wanted` from `floor` up that the record names: as a `u64`
    /// past the frame's fixed head on 2024, in its counted lists on 2015 to
    /// 2023.
    fn named_ids(&self, wanted: &BTreeSet<u32>, floor: u32) -> BTreeSet<u32> {
        let mut named = BTreeSet::new();
        match self.layout {
            Layout::Wide(_) => {
                for at in CATEGORY_OFFSET..self.bytes.len().saturating_sub(7) {
                    if let Some(id) = le_bytes::u64_at(&self.bytes, at)
                        .and_then(|v| u32::try_from(v).ok())
                        .filter(|id| *id >= floor && wanted.contains(id))
                    {
                        named.insert(id);
                    }
                }
            }
            Layout::Narrow(_) => {
                for id in self.lists.iter().flatten() {
                    if *id >= floor && wanted.contains(id) {
                        named.insert(*id);
                    }
                }
            }
        }
        named
    }

    /// The one id of `wanted` the record names, or `None` for none or several.
    fn named_one(&self, wanted: &BTreeSet<u32>) -> Option<u32> {
        let mut ids = self.named_ids(wanted, 0).into_iter();
        match (ids.next(), ids.next()) {
            (Some(id), None) => Some(id),
            _ => None,
        }
    }
}

/// One area boundary line as its record holds it.
#[derive(Debug, Clone, PartialEq)]
struct LineRecord {
    from: (f64, f64),
    to: (f64, f64),
    height: f64,
    key: (u32, u32),
    /// The ElementIds its reference list names: its Level, its scheme, the
    /// lines it is joined to (RE-330). Two copies of a line that differ here
    /// make it a conflicted one, as two that differ in box do: the joins read
    /// this list.
    names: BTreeSet<u32>,
}

/// The area boundary lines of a file, and what could not be read of them.
#[derive(Default)]
struct BoundaryLines {
    by_id: BTreeMap<u32, LineRecord>,
    /// Lines whose copies in several partitions disagree.
    conflicted: BTreeSet<u32>,
    /// The height of every line that is in no group it can be named in, and
    /// of both copies of a line whose copies disagree, feet: each could lie in
    /// any group on its Level.
    stray_heights: Vec<f64>,
    /// The ids of each scheme's lines on each Level, in the order met.
    groups: BTreeMap<(u32, u32), Vec<u32>>,
    /// Every line id the scan returned a record for, whatever became of it.
    seen: BTreeSet<u32>,
    /// A line whose height is not a number: it can be placed on no Level.
    unplaceable: bool,
    /// The groups that hold a line in a non-primary design option. Revit's
    /// handling of such a line is not measured (no area boundary line of the
    /// office or of Autodesk's 2024 samples has a design option), so the group
    /// is not read.
    option_groups: BTreeSet<(u32, u32)>,
}

/// What an Area's body is: its region, the base and the height.
struct AreaBody {
    face: AreaFace,
    base_feet: f64,
    height_feet: f64,
    joined: bool,
}

/// What is known of the file around its Areas.
struct Surroundings<'a> {
    declared: &'a BTreeSet<u32>,
    level_ids: &'a BTreeSet<u32>,
    scheme_ids: &'a BTreeSet<u32>,
    levels: &'a [PartitionLevel],
    /// Whether the file's design option sets are read: a line or an Area that
    /// names an option cannot be placed in the primary one where they are not
    /// (Revit 2015).
    options_read: bool,
}

/// The element records of the area boundary lines of a file: the 32-bit
/// records on Revit 2015 to 2023 (RE-324), the 2024 records otherwise. `None`
/// where the scan fails.
fn scan_line_records(
    rf: &mut RevitFile,
    revit_version: u32,
    declared: &BTreeSet<u32>,
) -> Option<Vec<PartitionElementRecord>> {
    if per32::is_32bit_release(revit_version) {
        Some(
            per32::scan_records(rf, revit_version)
                .into_iter()
                .filter(|record| record.builtin_category == OST_AREA_SCHEME_LINES)
                .collect(),
        )
    } else {
        scan_category_records_multi(rf, revit_version, &[OST_AREA_SCHEME_LINES], declared).ok()
    }
}

/// The one Level and the one area scheme a line's reference list names.
fn line_key(
    references: &[u64],
    level_ids: &BTreeSet<u32>,
    scheme_ids: &BTreeSet<u32>,
) -> Option<(u32, u32)> {
    let named: BTreeSet<u32> = references
        .iter()
        .filter_map(|reference| u32::try_from(*reference).ok())
        .collect();
    let mut levels = named.intersection(level_ids);
    let mut schemes = named.intersection(scheme_ids);
    match (levels.next(), levels.next(), schemes.next(), schemes.next()) {
        (Some(&level), None, Some(&scheme), None) => Some((level, scheme)),
        _ => None,
    }
}

/// Every area boundary line of the file, from its element record. A line in
/// a non-primary design option is left out, as an element is (RE-40).
fn boundary_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    surroundings: &Surroundings<'_>,
    options: &DesignOptions,
) -> BoundaryLines {
    let mut out = BoundaryLines::default();
    let Some(records) = scan_line_records(rf, revit_version, surroundings.declared) else {
        return out;
    };
    for record in records {
        out.seen.insert(record.element_id);
        let [x0, y0, z0, x1, y1, z1] = record.bbox_feet;
        let finite = record.bbox_feet.iter().all(|v| v.is_finite());
        let key = line_key(
            &record.references,
            surroundings.level_ids,
            surroundings.scheme_ids,
        );
        if record
            .design_option
            .is_some_and(|option| !surroundings.options_read || options.is_non_primary(option))
        {
            if let Some(key) = key {
                out.option_groups.insert(key);
            } else if z0.is_finite() {
                out.stray_heights.push(z0);
            } else {
                out.unplaceable = true;
            }
            continue;
        }
        let usable = finite
            && (z1 - z0).abs() <= AREA_LEVEL_TOLERANCE_FEET
            && record.is_placed_instance()
            && !record.is_container_member();
        let (true, Some(key)) = (usable, key) else {
            if z0.is_finite() {
                out.stray_heights.push(z0);
            } else {
                out.unplaceable = true;
            }
            continue;
        };
        let line = LineRecord {
            from: (x0, y0),
            to: (x1, y1),
            height: z0,
            key,
            names: record
                .references
                .iter()
                .filter_map(|reference| u32::try_from(*reference).ok())
                .collect(),
        };
        match out.by_id.get(&record.element_id) {
            None => {
                out.by_id.insert(record.element_id, line);
                out.groups.entry(key).or_default().push(record.element_id);
            }
            Some(held) if *held == line => {}
            Some(held) => {
                out.stray_heights.push(held.height);
                out.stray_heights.push(line.height);
                out.conflicted.insert(record.element_id);
            }
        }
    }
    out
}

/// The point of each Area (RE-317, RE-324).
fn area_points(
    rf: &mut RevitFile,
    revit_version: u32,
    placed: &BTreeMap<u32, AreaRecord>,
    level_ids: &BTreeSet<u32>,
) -> BTreeMap<u32, (f64, f64)> {
    if per32::is_32bit_release(revit_version) {
        area_points_narrow(rf, placed, level_ids)
    } else {
        area_points_wide(rf, placed)
    }
}

/// The point of each Area on Revit 2015 to 2023, from its element data
/// (RE-324): the two `f64`s [`NARROW_POINT_OFFSET`] bytes past the
/// `01 00 00 00 · ElementId` header, where the `u32` that follows the header is
/// the Area's own Level. Every copy in the record's partition must give the same
/// point.
fn area_points_narrow(
    rf: &mut RevitFile,
    placed: &BTreeMap<u32, AreaRecord>,
    level_ids: &BTreeSet<u32>,
) -> BTreeMap<u32, (f64, f64)> {
    let mut by_stream: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    for (&id, record) in placed {
        by_stream
            .entry(record.stream.as_str())
            .or_default()
            .insert(id);
    }
    let mut found: BTreeMap<u32, Option<(f64, f64)>> = BTreeMap::new();
    for (stream, ids) in by_stream {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let Some(id) = le_bytes::u32_at(buf, hit + 4).filter(|id| ids.contains(id)) else {
                continue;
            };
            let Some(record) = placed.get(&id) else {
                continue;
            };
            let data = hit + 8;
            let start = data + NARROW_POINT_OFFSET;
            let (Some(level), Some(x), Some(y)) = (
                le_bytes::u32_at(buf, data),
                le_bytes::f64_at(buf, start),
                le_bytes::f64_at(buf, start + 8),
            ) else {
                continue;
            };
            if record.named_one(level_ids) != Some(level) || !x.is_finite() || !y.is_finite() {
                continue;
            }
            let point = Some((x, y));
            match found.get(&id) {
                Some(held) if *held != point => {
                    found.insert(id, None);
                }
                Some(_) => {}
                None => {
                    found.insert(id, point);
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, point)| Some((id, point?)))
        .collect()
}

/// The point of each Area on Revit 2024, from the payload of its data object:
/// where the object's class is the record's and every copy in the record's
/// partition gives the same point.
fn area_points_wide(
    rf: &mut RevitFile,
    placed: &BTreeMap<u32, AreaRecord>,
) -> BTreeMap<u32, (f64, f64)> {
    let mut by_stream: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    for (&id, record) in placed {
        by_stream
            .entry(record.stream.as_str())
            .or_default()
            .insert(id);
    }
    let mut found: BTreeMap<u32, Option<(f64, f64)>> = BTreeMap::new();
    for (stream, ids) in by_stream {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (at, object) in data_objects(buf) {
            let Some(record) = placed.get(&object.element_id) else {
                continue;
            };
            if !ids.contains(&object.element_id)
                || le_bytes::u16_at(&record.bytes, CLASS_TAG_OFFSET).map(u32::from)
                    != Some(object.class)
            {
                continue;
            }
            let start = at + DATA_OBJECT_HEADER + AREA_POINT_OFFSET;
            if start + 16 > object.end {
                continue;
            }
            let point = match (
                le_bytes::f64_at(buf, start),
                le_bytes::f64_at(buf, start + 8),
            ) {
                (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Some((x, y)),
                _ => None,
            };
            match found.get(&object.element_id) {
                Some(held) if *held != point => {
                    found.insert(object.element_id, None);
                }
                Some(_) => {}
                None => {
                    found.insert(object.element_id, point);
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, point)| Some((id, point?)))
        .collect()
}

/// The elevation of the Level `level` and the distance up to the Level above
/// it, feet, where both are building stories and no other Level lies at either
/// elevation or between them (RE-317). Revit's exporter takes an Area's height
/// from its Level's "story above" where that is a building story, and else
/// from the next building story (`LevelUtil.CalculateDistanceToNextLevel` in
/// revit-ifc); the "story above" is not read, so a Level above that is not
/// known to be the next building story proves nothing.
fn level_and_storey_height(levels: &[PartitionLevel], level: u32) -> Option<(f64, f64)> {
    let base = levels
        .iter()
        .find(|candidate| candidate.element_id == level)?;
    if base.building_story != Some(true) || !base.elevation_feet.is_finite() {
        return None;
    }
    let mut next: Option<&PartitionLevel> = None;
    for other in levels {
        if other.element_id == level {
            continue;
        }
        if !other.elevation_feet.is_finite() {
            return None;
        }
        let gap = other.elevation_feet - base.elevation_feet;
        if gap.abs() <= AREA_MIN_HEIGHT_FEET {
            return None;
        }
        if gap < 0.0 {
            continue;
        }
        match next {
            Some(held)
                if (other.elevation_feet - held.elevation_feet).abs() <= AREA_MIN_HEIGHT_FEET =>
            {
                return None;
            }
            Some(held) if held.elevation_feet < other.elevation_feet => {}
            _ => next = Some(other),
        }
    }
    let next = next?;
    (next.building_story == Some(true)).then_some((
        base.elevation_feet,
        next.elevation_feet - base.elevation_feet,
    ))
}

/// The box `[min_x, min_y, max_x, max_y]` of the segment from `from` to `to`.
fn segment_box(from: (f64, f64), to: (f64, f64)) -> [f64; 4] {
    [
        from.0.min(to.0),
        from.1.min(to.1),
        from.0.max(to.0),
        from.1.max(to.1),
    ]
}

/// The arrangement of the lines of one scheme on one Level that can be read, and
/// the boxes of those that cannot (RE-330).
struct GroupArrangement {
    arrangement: Arrangement,
    /// The box of each line left out of the arrangement because it is not a
    /// straight line along an axis (a slanted line, an arc or a point: its record
    /// holds only its box), and of each line that names one, which may be joined
    /// to it.
    blocked: Vec<[f64; 4]>,
}

impl GroupArrangement {
    /// Whether a line left out of the arrangement may lie in, or bound, `face`:
    /// its box is within [`UNREAD_MARGIN_FEET`] of the box of the face's outline.
    /// A line outside that box cannot change the region.
    fn blocks(&self, face: &AreaFace) -> bool {
        let bounds = face.outer.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)],
        );
        self.blocked.iter().any(|line| {
            line[0] <= bounds[2] + UNREAD_MARGIN_FEET
                && line[2] >= bounds[0] - UNREAD_MARGIN_FEET
                && line[1] <= bounds[3] + UNREAD_MARGIN_FEET
                && line[3] >= bounds[1] - UNREAD_MARGIN_FEET
        })
    }
}

/// What the lines of one scheme on one Level give (RE-330, RE-333).
enum Group {
    /// The lines cannot be read as a whole: one is in a non-primary design
    /// option, at a stray height, has copies that disagree, or is missing. No
    /// region of any kind is drawn from the group, Revit's plan topology's
    /// included.
    Unreadable,
    /// The group is sound but its lines cannot be arranged: none is a straight
    /// line along an axis, or they overlap or are too many. Revit's plan
    /// topology may still give an Area a region.
    Unarranged,
    /// The arrangement of the lines that can be read.
    Arranged(GroupArrangement),
}

/// The arrangement of the lines of one scheme on one Level (RE-330). A line that
/// is not a straight line along an axis is not read, and an Area whose region it
/// may change is refused ([`GroupArrangement::blocks`]).
fn group_arrangement(lines: &BoundaryLines, key: (u32, u32), base_feet: f64) -> Group {
    if lines
        .stray_heights
        .iter()
        .any(|height| (height - base_feet).abs() <= AREA_LEVEL_TOLERANCE_FEET)
    {
        return Group::Unreadable;
    }
    if lines.option_groups.contains(&key) {
        return Group::Unreadable;
    }
    let Some(ids) = lines.groups.get(&key) else {
        return Group::Unreadable;
    };
    let mut readable = Vec::with_capacity(ids.len());
    let mut joined: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut unread: BTreeSet<u32> = BTreeSet::new();
    let mut blocked = Vec::new();
    for id in ids {
        let Some(line) = lines.by_id.get(id) else {
            return Group::Unreadable;
        };
        if lines.conflicted.contains(id)
            || (line.height - base_feet).abs() > AREA_LEVEL_TOLERANCE_FEET
        {
            return Group::Unreadable;
        }
        let boundary = BoundaryLine {
            id: *id,
            from: line.from,
            to: line.to,
        };
        if Arrangement::reads(&boundary) {
            joined.insert(*id, line.names.clone());
            readable.push(boundary);
        } else {
            unread.insert(*id);
            blocked.push(segment_box(line.from, line.to));
        }
    }
    if readable.is_empty() {
        return Group::Unarranged;
    }
    for boundary in &readable {
        let names_unread = joined
            .get(&boundary.id)
            .is_some_and(|names| names.iter().any(|name| unread.contains(name)));
        if names_unread {
            blocked.push(segment_box(boundary.from, boundary.to));
        }
    }
    match Arrangement::new(&readable, &joined) {
        Some(arrangement) => Group::Arranged(GroupArrangement {
            arrangement,
            blocked,
        }),
        None => Group::Unarranged,
    }
}

/// The Areas of a file that have a body, and why the others have none.
#[derive(Default)]
struct AreaBodies {
    drawn: BTreeMap<u32, AreaBody>,
    refused: BTreeMap<u32, &'static str>,
}

/// An area scheme on a Level.
type GroupKey = (u32, u32);

/// A scheme on a Level and the corners of a region in it, by their bits.
type RegionKey = (GroupKey, Vec<(u64, u64)>);

/// Where an Area's point lies among the lines of its scheme on its Level.
struct Located {
    key: GroupKey,
    /// The region, or why there is none: the lines cannot be read, or the point
    /// is in no single region of them.
    face: Result<AreaFace, &'static str>,
}

/// The elevation of the Level `level`, feet.
fn level_elevation(levels: &[PartitionLevel], level: u32) -> Option<f64> {
    let elevation = levels
        .iter()
        .find(|candidate| candidate.element_id == level)?
        .elevation_feet;
    elevation.is_finite().then_some(elevation)
}

/// The region of one Area, whatever else stops it from being drawn. Revit's
/// plan topology's region, `topology`, comes first where it was read and passed
/// its checks (the loop's area against the area Revit saves for it, the Area's
/// point in the loop and in no hole, RE-333); otherwise the region of the lines'
/// arrangement with its joins (RE-317, RE-330): the Area's point among the lines
/// of its scheme on its Level. A group of lines that cannot be read as a whole
/// (a non-primary design option, a stray height, copies that disagree, a missing
/// line) gives no region of either kind. Where both give none, the plan
/// topology's reason is the Area's, being the more specific. `Err` where it has
/// no point and no topology, or its Level or scheme is not named, or its Level's
/// elevation is not known.
fn locate(
    record: &AreaRecord,
    point: Option<(f64, f64)>,
    topology: Option<Result<AreaRegion, &'static str>>,
    lines: &BoundaryLines,
    surroundings: &Surroundings<'_>,
    arrangements: &mut BTreeMap<GroupKey, Group>,
) -> Result<Located, &'static str> {
    // An Area whose point is not read can still have the plan topology's
    // region, which is checked against the point the topology's Area saves.
    if point.is_none() && !matches!(topology, Some(Ok(_))) {
        return Err(REFUSED_NO_POINT);
    }
    let (Some(level), Some(scheme)) = (
        record.named_one(surroundings.level_ids),
        record.named_one(surroundings.scheme_ids),
    ) else {
        return Err(REFUSED_UNNAMED);
    };
    let key = (level, scheme);
    let base_feet = level_elevation(surroundings.levels, level).ok_or(REFUSED_HEIGHT)?;
    let group = arrangements
        .entry(key)
        .or_insert_with(|| group_arrangement(lines, key, base_feet));
    if matches!(group, Group::Unreadable) {
        return Ok(Located {
            key,
            face: Err(REFUSED_GROUP),
        });
    }
    let by_lines = match (&*group, point) {
        (Group::Arranged(group), Some(point)) => match group.arrangement.face_at(point) {
            None => Err(REFUSED_REGION),
            Some(face) if group.blocks(&face) => Err(REFUSED_GROUP),
            Some(face) => Ok(face),
        },
        (Group::Arranged(_), None) => Err(REFUSED_NO_POINT),
        (_, _) => Err(REFUSED_GROUP),
    };
    let face = match topology {
        Some(Ok(region)) => Ok(AreaFace {
            outer: region.outer,
            holes: region.holes,
            line_ids: region.line_ids,
            joined: false,
        }),
        Some(Err(reason)) => by_lines.map_err(|_| reason),
        None => by_lines,
    };
    Ok(Located { key, face })
}

/// The body of one Area: its region, where the lines its record names are all
/// declared lines of its scheme on its Level, are not conflicted and bound that
/// region, from its Level up to the Level above (RE-317); or why it has none.
fn draw_area(
    record: &AreaRecord,
    located: &Located,
    lines: &BoundaryLines,
    known_lines: &BTreeSet<u32>,
    surroundings: &Surroundings<'_>,
) -> Result<AreaBody, &'static str> {
    let named_lines = record.named_ids(known_lines, AREA_LINE_ID_FLOOR);
    let named_in_group = named_lines.iter().all(|line| {
        !lines.conflicted.contains(line)
            && lines
                .by_id
                .get(line)
                .is_some_and(|held| held.key == located.key)
    });
    if named_lines.is_empty() || !named_in_group {
        return Err(REFUSED_LINES);
    }
    let (base_feet, height_feet) =
        level_and_storey_height(surroundings.levels, located.key.0).ok_or(REFUSED_HEIGHT)?;
    let face = located.face.as_ref().map_err(|reason| *reason)?;
    if !named_lines.is_subset(&face.line_ids) {
        return Err(REFUSED_OFF_REGION);
    }
    Ok(AreaBody {
        face: face.clone(),
        base_feet,
        height_feet,
        joined: face.joined,
    })
}

/// The body of each of `placed` that has one, and the reason for each that has
/// none (RE-317). An Area of a file with a declared area boundary line that
/// was not read, or cannot be placed on a Level, has none, since that line may
/// belong to any group. An Area whose region another placed Area of its scheme
/// and Level shares has none, whatever stops the other from being drawn:
/// Revit's second, redundant Area has no area of its own.
fn area_bodies(
    rf: &mut RevitFile,
    revit_version: u32,
    placed: &BTreeMap<u32, AreaRecord>,
    declared_lines: &BTreeSet<u32>,
    surroundings: &Surroundings<'_>,
    options: &DesignOptions,
) -> AreaBodies {
    let mut out = AreaBodies::default();
    if placed.is_empty() {
        return out;
    }
    let refuse_all = |out: &mut AreaBodies, reason: &'static str| {
        for &id in placed.keys() {
            out.refused.insert(id, reason);
        }
    };
    if surroundings.levels.is_empty() {
        refuse_all(&mut out, REFUSED_LEVELS);
        return out;
    }
    let lines = boundary_lines(rf, revit_version, surroundings, options);
    if lines.unplaceable || declared_lines.iter().any(|id| !lines.seen.contains(id)) {
        refuse_all(&mut out, REFUSED_UNACCOUNTED);
        return out;
    }
    if lines.by_id.is_empty() {
        refuse_all(&mut out, REFUSED_NO_LINES);
        return out;
    }
    let points = area_points(rf, revit_version, placed, surroundings.level_ids);
    let ids: BTreeSet<u64> = placed.keys().map(|&id| u64::from(id)).collect();
    let mut topology = crate::native_area_regions::area_regions(rf, &ids);
    let mut arrangements: BTreeMap<GroupKey, Group> = BTreeMap::new();
    let mut located: BTreeMap<u32, Result<Located, &'static str>> = BTreeMap::new();
    for (&id, record) in placed {
        let point = points.get(&id).copied();
        let region = topology.remove(&u64::from(id));
        located.insert(
            id,
            locate(
                record,
                point,
                region,
                &lines,
                surroundings,
                &mut arrangements,
            ),
        );
    }
    // Each Area's region is located; the groups' arrangements are not needed
    // again, so they go before the regions are compared.
    drop(arrangements);
    // The lines an Area's record may name: those the chain declares, and those
    // the scan returned without a chain record of their own.
    let known_lines: BTreeSet<u32> = declared_lines
        .iter()
        .copied()
        .chain(lines.by_id.keys().copied())
        .collect();
    let mut sharing: BTreeMap<RegionKey, Vec<u32>> = BTreeMap::new();
    for (&id, place) in &located {
        let Ok(place) = place else {
            continue;
        };
        let Ok(face) = &place.face else {
            continue;
        };
        let outline = face
            .outer
            .iter()
            .map(|p| (p.0.to_bits(), p.1.to_bits()))
            .collect();
        sharing.entry((place.key, outline)).or_default().push(id);
    }
    let redundant: BTreeSet<u32> = sharing
        .values()
        .filter(|ids| ids.len() > 1)
        .flatten()
        .copied()
        .collect();
    for (&id, record) in placed {
        let result = match located.get(&id) {
            Some(Ok(place)) => draw_area(record, place, &lines, &known_lines, surroundings),
            Some(Err(reason)) => Err(*reason),
            None => Err(REFUSED_NO_POINT),
        };
        match result {
            Ok(_) if redundant.contains(&id) => {
                out.refused.insert(id, REFUSED_REDUNDANT);
            }
            Ok(body) => {
                out.drawn.insert(id, body);
            }
            Err(reason) => {
                out.refused.insert(id, reason);
            }
        }
    }
    out
}

/// Every placed Area of a Revit 2015 to 2027 file, as a `DecodedElement` of
/// class `Area` with its number, name and Level where they are read, and its
/// boundary, base and height where they are proven (RE-317, RE-324). On 2015
/// to 2023 only the Areas whose body is proven; the others are counted by
/// reason on the file (`RevitFile::unwritten_areas`). Empty on every other
/// release.
pub fn areas_from_partition_chain_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
    levels: &[PartitionLevel],
) -> Vec<DecodedElement> {
    if !AREA_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Vec::new();
    }
    let Some(layout) = Layout::of(rf, revit_version) else {
        return Vec::new();
    };
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Vec::new(),
    };
    let options = rf.memoised_design_options();
    let options_read = layout.reads_design_options(revit_version);
    let census = area_records(rf, layout, &declared);
    rf.set_dropped_chain_entries(census.dropped);
    let mut placed: BTreeMap<u32, AreaRecord> = census
        .areas
        .into_iter()
        .filter(|(_, record)| {
            record.container_unset()
                && record.placed_kind()
                && record.in_primary_option(&options, options_read)
        })
        .collect();
    if placed.is_empty() {
        return Vec::new();
    }
    let surroundings = Surroundings {
        declared: &declared,
        level_ids,
        scheme_ids: &census.scheme_ids,
        levels,
        options_read,
    };
    let mut bodies = area_bodies(
        rf,
        revit_version,
        &placed,
        &census.line_ids,
        &surroundings,
        &options,
    );
    if matches!(layout, Layout::Narrow(_)) {
        // On Revit 2015 to 2023 an Area is written only where its region and
        // body are proven (RE-324): a wider rule wrote spaces Revit's export
        // lacks on a held-out project. Revit's exporter makes no space for an
        // Area whose boundary yields no loop, and the Area's lines are all
        // that is read of its boundary. Every placed Area is drawn or refused
        // with a reason, so the refusals are the Areas left out.
        let mut unwritten =
            BTreeMap::<String, Vec<crate::partition_element_records::LeftOutElement>>::new();
        for (id, reason) in &bodies.refused {
            let record = placed.get(id);
            unwritten.entry((*reason).to_string()).or_default().push(
                crate::partition_element_records::LeftOutElement {
                    element_id: Some(*id),
                    class: "Area".to_string(),
                    stream: record.map(|record| record.stream.clone()),
                    offset: record.map(|record| record.offset),
                    detail: Some((*reason).to_string()),
                },
            );
        }
        placed.retain(|id, _| bodies.drawn.contains_key(id));
        rf.set_unwritten_areas(unwritten);
        if placed.is_empty() {
            return Vec::new();
        }
    }
    let ids: BTreeSet<u32> = placed.keys().copied().collect();
    let entries =
        crate::partition_room_parameters::scan_room_parameter_entries(rf, revit_version, &ids);
    placed
        .into_iter()
        .map(|(id, record)| {
            let level = record.named_one(level_ids);
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
            if let Some(reason) = bodies.refused.get(&id) {
                fields.push((
                    AREA_BODY_REFUSAL_FIELD.into(),
                    InstanceField::String((*reason).into()),
                ));
            }
            if let Some(body) = bodies.drawn.remove(&id) {
                let profile = PlanProfile {
                    outer_xy: body.face.outer,
                    inner_xy: body.face.holes,
                    segment_ids: body.face.line_ids.into_iter().collect(),
                    pieces: Vec::new(),
                    void_openings: Vec::new(),
                };
                for (name, mut value) in profile.fields() {
                    if name == PLAN_PROFILE_SOURCE_FIELD {
                        value = InstanceField::String(AREA_BOUNDARY_SOURCE.into());
                    }
                    fields.push((name, value));
                }
                fields.push((
                    AREA_BODY_BASE_FIELD.into(),
                    InstanceField::Float {
                        value: body.base_feet,
                        size: 8,
                    },
                ));
                fields.push((
                    AREA_BODY_HEIGHT_FIELD.into(),
                    InstanceField::Float {
                        value: body.height_feet,
                        size: 8,
                    },
                ));
                if body.joined {
                    fields.push((AREA_JOINED_FIELD.into(), InstanceField::Bool(true)));
                }
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
