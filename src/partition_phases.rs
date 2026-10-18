//! Phase names, and the phase an element was created in (spike 257).
//!
//! A Revit phase stores its name after its ElementId in one fixed frame:
//!
//! ```text
//! 01 00 00 00 · u64 ElementId · 56 × ff · 00 00 00 · u32 n · UTF-16 × n
//! ```
//!
//! The same frame names other elements too (levels, grids, views), so only
//! two kinds of hit are phases:
//! - the frame sits in an object of the file's own `Phase` class: its `u16`
//!   class tag is [`PHASE_CLASS_TAG_BEFORE_ID`] bytes before the ElementId;
//! - the frame is the element's own serialised data (the element-data header
//!   before the ElementId) and the element has an `OST_Phases` record.
//!
//! An element's reference list (`+0x88`) names its Phase Created. Measured on
//! the nine Autodesk Revit 2024 samples against Revit's own IFC export
//! (`Phasing.Phase Created`): the list names exactly one phase for 38,555
//! elements and that phase is Revit's 38,555 times, with none wrong and none
//! naming two; DACH's 20 `Phase 1` elements and 2,586 `Phase 2` elements are
//! both told apart. A list naming no phase, or two (a demolished element would
//! name its Phase Demolished too, and no sample has one), gives no phase.
//!
//! Only Revit 2024 is measured.

use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// The schema class of a phase object, whose tag frames its name.
pub const PHASE_CLASS: &str = "ProjectPhase";

/// `BuiltInCategory.OST_Phases`, Autodesk's published API constant.
pub const OST_PHASES: i64 = -2_000_112;

/// How far before a phase object's ElementId its class tag sits: the tag,
/// 32 zero bytes and `01 00 00 00`.
pub const PHASE_CLASS_TAG_BEFORE_ID: usize = 2 + 32 + 4;

/// The run of `ff` between a phase's ElementId and its name.
const FILL: [u8; 56] = [0xff; 56];

/// Releases the frame is measured on.
pub const PHASE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Every phase-name frame in `buf`: `(ElementId, name)`, for hits whose class
/// tag is `phase_tag`, or that are the element data of an id in
/// `phase_records`.
pub fn find_phase_names(
    buf: &[u8],
    phase_tag: Option<u16>,
    phase_records: &BTreeSet<u32>,
    element_data_header: &[u8; 10],
) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    for fill in memchr::memmem::find_iter(buf, &FILL) {
        let (Some(id_at), Some(rest)) = (fill.checked_sub(8), buf.get(fill + 56..fill + 63)) else {
            continue;
        };
        if rest[..3] != [0, 0, 0] || id_at < 4 || buf[id_at - 4..id_at] != [1, 0, 0, 0] {
            continue;
        }
        let Some(id) = read_u64(buf, id_at).and_then(|v| u32::try_from(v).ok()) else {
            continue;
        };
        let tagged = phase_tag.is_some_and(|tag| {
            id_at
                .checked_sub(PHASE_CLASS_TAG_BEFORE_ID)
                .and_then(|at| buf.get(at..at + 2))
                == Some(&tag.to_le_bytes()[..])
        });
        let own_data = phase_records.contains(&id)
            && id_at >= 10
            && buf[id_at - 10..id_at] == element_data_header[..];
        if !tagged && !own_data {
            continue;
        }
        let units = u32::from_le_bytes([rest[3], rest[4], rest[5], rest[6]]) as usize;
        if !(1..=crate::partition_names::NAME_MAX_UNITS).contains(&units) {
            continue;
        }
        let Some(bytes) = buf.get(fill + 63..fill + 63 + 2 * units) else {
            continue;
        };
        let code_units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let Ok(name) = String::from_utf16(&code_units) else {
            continue;
        };
        if name.trim().is_empty()
            || name
                .chars()
                .any(|c| c.is_control() || matches!(c, '\u{fffd}' | '\u{fffe}' | '\u{ffff}'))
        {
            continue;
        }
        out.push((id, name));
    }
    out
}

/// The file's phases by ElementId. An id whose frames disagree is dropped.
/// Empty on a release the frame is not measured on.
pub fn phase_names(rf: &mut RevitFile) -> Result<BTreeMap<u32, String>> {
    let version = rf.basic_file_info()?.version;
    let Some(header) = crate::partition_names::element_data_header(version)
        .filter(|_| PHASE_SUPPORTED_REVIT_VERSIONS.contains(&version))
    else {
        return Ok(BTreeMap::new());
    };
    let phase_tag = rf.schema_classes().ok().and_then(|classes| {
        classes
            .classes
            .iter()
            .find(|class| class.name == PHASE_CLASS)
            .map(|class| class.tag)
    });
    let declared = crate::elem_table::parse_records(rf)
        .map(|records| crate::elem_table::declared_ids(&records))
        .unwrap_or_default();
    let phase_records: BTreeSet<u32> =
        crate::partition_type_records::scan_type_records(rf, version, OST_PHASES, &declared)
            .unwrap_or_default()
            .iter()
            .map(|record| record.element_id)
            .collect();
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in find_phase_names(inflated.bytes(), phase_tag, &phase_records, &header) {
            let slot = found.entry(id).or_insert_with(|| Some(name.clone()));
            if slot.as_deref() != Some(name.as_str()) {
                *slot = None;
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, name)| name.map(|n| (id, n)))
        .collect())
}

/// The one phase a reference list names, or `None` when it names none or
/// several.
pub fn phase_created<'a>(
    references: &[u64],
    phases: &'a BTreeMap<u32, String>,
) -> Option<(u32, &'a str)> {
    let named: BTreeSet<u32> = references
        .iter()
        .filter_map(|&slot| u32::try_from(slot).ok())
        .filter(|id| phases.contains_key(id))
        .collect();
    let mut it = named.into_iter();
    match (it.next(), it.next()) {
        (Some(id), None) => Some((id, phases[&id].as_str())),
        _ => None,
    }
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}
