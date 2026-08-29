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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walker::InstanceField;

    fn level_el(id: u32, name: &str) -> DecodedElement {
        DecodedElement {
            id: Some(id),
            class: "Level".into(),
            fields: vec![("m_name".into(), InstanceField::String(name.into()))],
            byte_range: 0..0,
        }
    }

    fn floor_on_level(level_id: u32) -> DecodedElement {
        DecodedElement {
            id: Some(9001),
            class: "Floor".into(),
            fields: vec![(
                "m_level_id".into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: level_id,
                },
            )],
            byte_range: 0..0,
        }
    }

    fn room_on_level(level_id: u32) -> DecodedElement {
        DecodedElement {
            id: None,
            class: "Room".into(),
            fields: vec![
                ("m_name".into(), InstanceField::String("Office".into())),
                (
                    "m_level_id".into(),
                    InstanceField::ElementId {
                        tag: 0,
                        id: level_id,
                    },
                ),
            ],
            byte_range: 0..0,
        }
    }

    #[test]
    fn bind_resolves_floor_and_room_when_level_ids_match() {
        let mut bind = LevelStoreyBind::new();
        bind.record_level(level_el(42, "Level 1").id, 0);
        bind.record_level(level_el(99, "Level 2").id, 1);
        assert_eq!(bind.storey_index_for(&floor_on_level(42)), Some(0));
        assert_eq!(bind.storey_index_for(&room_on_level(99)), Some(1));
    }

    #[test]
    fn bind_fail_closed_when_level_id_unknown() {
        let mut bind = LevelStoreyBind::new();
        bind.record_level(Some(42), 0);
        assert_eq!(bind.storey_index_for(&floor_on_level(7)), None);
    }

    #[test]
    fn bind_fail_closed_when_level_has_no_element_id() {
        let mut bind = LevelStoreyBind::new();
        bind.record_level(None, 0);
        assert!(bind.is_empty());
        assert_eq!(bind.storey_index_for(&floor_on_level(42)), None);
    }

    #[test]
    fn bind_fail_closed_when_floor_lacks_level_id() {
        let bind = LevelStoreyBind::new();
        let floor = DecodedElement {
            id: None,
            class: "Floor".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert_eq!(bind.storey_index_for(&floor), None);
    }

    #[test]
    fn level_id_from_decoded_ignores_non_spatial_classes() {
        let wall = DecodedElement {
            id: Some(1),
            class: "Wall".into(),
            fields: vec![(
                "m_level_id".into(),
                InstanceField::ElementId { tag: 0, id: 42 },
            )],
            byte_range: 0..0,
        };
        // Wall uses base_level_id on the typed Wall view; this helper
        // is Floor/Room-only for the #33 leftover path.
        assert_eq!(level_id_from_decoded(&wall), None);
    }
}
