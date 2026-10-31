//! Level parameters of an element, from its per-element association record
//! (RE-172).
//!
//! Every element has one record in the inflated `Partitions/*` streams that
//! opens `d3 02 01 00 00 00 · u32 ElementId · u32 0 · u32 LevelId · u32 0 ·
//! ff ff ff ff × 6`. The `LevelId` is the element's `Level` (walls: `Base
//! Constraint`; columns, railings, stairs and roofs: `Base Level`). It is
//! `0xffffffff` for elements with no level in that field (beams, stairs, some
//! MEP terminals): those get nothing. A wall's `Top Constraint` is a second
//! `0 · LevelId · 0 · ff..` group 100 to 320 bytes after the element id; it is
//! read only where that group holds a recovered Level's id.
//!
//! Both are emitted only when the id is a Level recovered by
//! [`crate::partition_level_records`], so an unknown id yields nothing (fail
//! closed). Measured on Autodesk's public Revit 2024 samples against Revit's
//! own IFC export; see `reports/element-framing/RE-172-level-parameters.md`.

use crate::RevitFile;
use crate::le_bytes;
use std::collections::BTreeMap;

/// Field carrying an element's level parameter (its Level name).
pub const LEVEL_PARAMETER_FIELD: &str = "m_level_parameter";
/// Field carrying a wall's Top Constraint (a Level name).
pub const TOP_LEVEL_PARAMETER_FIELD: &str = "m_top_level_parameter";

/// Bytes opening an association record.
const MARK: [u8; 6] = [0xd3, 0x02, 0x01, 0x00, 0x00, 0x00];
/// Where the top-constraint group may start, from the ElementId.
const TOP_WINDOW: std::ops::Range<usize> = 100..320;

/// An element's recovered level names.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelParameters {
    /// Name of the Level in the record's level field.
    pub level: Option<String>,
    /// Name of the wall top Level, where the group was located.
    pub top: Option<String>,
    /// A stair's Base and Top Level names (RE-174).
    pub stair: Option<(String, String)>,
}

/// Where a stair's Base Level id sits from its ElementId, by record variant;
/// the Top Level id is [`STAIR_TOP_AFTER`] bytes later (RE-174).
const STAIR_BASE_OFFSETS: [usize; 4] = [115, 121, 129, 137];
const STAIR_TOP_AFTER: usize = 32;

/// The Base and Top Level of a stair record: the one variant offset at which
/// two different recovered Levels sit [`STAIR_TOP_AFTER`] bytes apart.
fn stair_levels(buf: &[u8], id: usize, names: &BTreeMap<u32, String>) -> Option<(String, String)> {
    let mut hits = STAIR_BASE_OFFSETS.iter().filter_map(|&k| {
        let base = names.get(&le_bytes::u32_at(buf, id + k)?)?;
        let top = names.get(&le_bytes::u32_at(buf, id + k + STAIR_TOP_AFTER)?)?;
        (base != top).then(|| (base.clone(), top.clone()))
    });
    let found = hits.next()?;
    hits.next().is_none().then_some(found)
}

/// Recovered Level names and elevations (feet) by id.
pub type LevelNames = (BTreeMap<u32, String>, BTreeMap<u32, f64>);

/// Level parameters by ElementId, and the recovered Levels by id.
/// Revit 2024 only: the record is proven there.
pub fn level_parameters(
    rf: &mut RevitFile,
    revit_version: u32,
) -> (BTreeMap<u32, LevelParameters>, LevelNames) {
    let mut out = BTreeMap::new();
    if revit_version != 2024 {
        return (out, Default::default());
    }
    let levels = crate::partition_level_records::recover_partition_levels(rf, revit_version)
        .unwrap_or_default();
    let elevations: BTreeMap<u32, f64> = levels
        .iter()
        .map(|level| (level.element_id, level.elevation_feet))
        .collect();
    let names: BTreeMap<u32, String> = levels
        .into_iter()
        .map(|level| (level.element_id, level.name))
        .collect();
    if names.is_empty() {
        return (out, (names, elevations));
    }
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let starts: Vec<usize> = memchr::memmem::find_iter(buf, &MARK).collect();
        for (i, &at) in starts.iter().enumerate() {
            let id = at + MARK.len();
            let (Some(element), Some(0), Some(level), Some(tail)) = (
                le_bytes::u32_at(buf, id),
                le_bytes::u32_at(buf, id + 4),
                le_bytes::u32_at(buf, id + 8),
                le_bytes::u32_at(buf, id + 12),
            ) else {
                continue;
            };
            let Some(name) = names.get(&level).filter(|_| tail == 0) else {
                // No level in the field: a stair keeps its Base and Top
                // Level elsewhere in the record.
                if level == u32::MAX {
                    if let Some(stair) = stair_levels(buf, id, &names) {
                        out.entry(element).or_insert(LevelParameters {
                            level: Some(stair.0.clone()),
                            top: None,
                            stair: Some(stair),
                        });
                    }
                }
                continue;
            };
            // The group must lie inside this record, before the next one.
            let end = starts.get(i + 1).copied().unwrap_or(buf.len());
            let top = TOP_WINDOW.clone().find_map(|o| {
                let at = id + o;
                if at + 8 > end {
                    return None;
                }
                let g = [
                    le_bytes::u32_at(buf, at - 4)?,
                    le_bytes::u32_at(buf, at)?,
                    le_bytes::u32_at(buf, at + 4)?,
                    le_bytes::u32_at(buf, at + 8)?,
                ];
                (g[0] == 0 && g[2] == 0 && g[3] >= 0xffff_fff0)
                    .then(|| names.get(&g[1]).cloned())
                    .flatten()
            });
            // The first record of an id wins.
            out.entry(element).or_insert(LevelParameters {
                level: Some(name.clone()),
                top,
                stair: None,
            });
        }
    }
    (out, (names, elevations))
}
