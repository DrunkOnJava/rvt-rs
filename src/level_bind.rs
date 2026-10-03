//! Fail-closed Level ElementId → building-storey index binding.
//!
//! Floors and Rooms carry `m_level_id` / `m_host_level_id` ElementId
//! references in the schema-driven typed view. Partition MVP Levels
//! today usually lack ElementIds, so this map stays empty on current
//! corpora and Floors/Rooms remain storey-unassigned — that is
//! intentional (fail closed), not a silent invent.
//!
//! RE-20 (magnetar Einhoven / Core Interior): Level ElementId recovery
//! remains **INSUFFICIENT** — `Level` is absent from Formats schema on
//! those files; proximity / LevelAssociationCell-shaped scans do not
//! yield unique elev→id maps.
//!
//! When both sides carry ElementIds that match, [`LevelStoreyBind::storey_index_for`]
//! returns the storey index; otherwise `None`.

use crate::elements::floor::Floor;
use crate::elements::zones::Zone;
use crate::walker::DecodedElement;
use std::collections::BTreeMap;

/// Map from Level ElementId → index into `building_storeys`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelStoreyBind {
    by_level_id: BTreeMap<u32, usize>,
}

impl LevelStoreyBind {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a Level ElementId → storey index. No-op when `level_id`
    /// is `None` (fail closed — never invent an id).
    pub fn record_level(&mut self, level_id: Option<u32>, storey_index: usize) {
        if let Some(id) = level_id {
            // First writer wins — duplicate Level ids keep the earlier storey.
            self.by_level_id.entry(id).or_insert(storey_index);
        }
    }

    pub fn len(&self) -> usize {
        self.by_level_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_level_id.is_empty()
    }

    /// Resolve a level ElementId to a storey index. `None` when the
    /// id was never recorded (unknown / unbound).
    pub fn storey_index_for_level_id(&self, level_id: u32) -> Option<usize> {
        self.by_level_id.get(&level_id).copied()
    }

    /// Resolve Floor / Room (and aliases) via their typed `level_id`.
    /// Returns `None` when the element has no level_id or the id is
    /// not in the bind map.
    pub fn storey_index_for(&self, decoded: &DecodedElement) -> Option<usize> {
        let level_id = level_id_from_decoded(decoded)?;
        self.storey_index_for_level_id(level_id)
    }
}

/// Extract a host-level ElementId from Floor / Room / Area / Space.
pub fn level_id_from_decoded(decoded: &DecodedElement) -> Option<u32> {
    match decoded.class.as_str() {
        "Floor" => Floor::from_decoded(decoded).level_id,
        "Room" | "Area" | "Space" => Zone::from_decoded(decoded).level_id,
        _ => None,
    }
}
