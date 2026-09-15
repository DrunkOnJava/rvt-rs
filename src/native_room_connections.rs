//! Phase-qualified door adjacency from explicit host topology. No nearest-room guess.
use crate::{
    native_document::Record, native_equipment::Identity, native_lifecycle,
    native_metadata::identifier, native_spatial_boundaries, native_spatial_context,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub owner: Identity,
    pub field: String,
    pub body_sha256: String,
    pub stream: String,
    pub group_record_offset: usize,
}
#[derive(Debug, Serialize)]
pub struct Connection {
    pub door: Identity,
    pub phase_id: i64,
    pub status: &'static str,
    pub from_room: Option<Identity>,
    pub to_room: Option<Identity>,
    pub diagnostic: Option<String>,
    pub sources: Vec<Value>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_room_connection_parity: bool,
    pub semantics: &'static str,
    pub connections: Vec<Connection>,
    pub diagnostics: Vec<String>,
}
struct Door {
    source: Source,
    type_id: i64,
    eligible: bool,
    orientation: Value,
    from_to_flip: bool,
    hand_flip: bool,
}
struct Family {
    source: Source,
    eligible: bool,
}
struct Room {
    source: Source,
    height: f64,
}
#[derive(Default)]
pub struct InventoryBuilder {
    doors: BTreeMap<u64, Door>,
    families: BTreeMap<i64, Family>,
    types: BTreeMap<i64, (i64, Source)>,
    rooms: BTreeMap<u64, Room>,
    diagnostics: Vec<String>,
}
fn source(r: &Record, field: &str) -> Source {
    Source {
        owner: Identity {
            element_id: r.identity.element_id,
            unique_id: r.identity.unique_id.clone(),
        },
        field: field.into(),
        body_sha256: r.source.body_sha256.clone(),
        stream: r.source.stream.clone(),
        group_record_offset: r.source.group_record_offset,
    }
}
fn false_field(f: &Value, k: &str) -> bool {
    f[k].as_bool() == Some(false)
}
impl InventoryBuilder {
    pub fn ingest(&mut self, r: &Record) -> Result<()> {
        if r.channel != 102
            || !matches!(
                r.class_name.as_deref(),
                Some("FamilyInstance" | "FamilySymbol" | "Family" | "RoomElem")
            )
        {
            return Ok(());
        }
        if let Err(e) = self.project(r) {
            self.diagnostics
                .push(format!("{}: {e:#}", r.identity.element_id));
        }
        Ok(())
    }
    fn project(&mut self, r: &Record) -> Result<()> {
        let g = r
            .graph
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("connection graph absent"))?;
        let o = g
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty connection graph"))?;
        let f = &o.fields;
        match o.class_name.as_str() {
            "FamilyInstance" => {
                let p = &f["m_oFamInstSpec"];
                let edges: Vec<_> = g
                    .edges
                    .iter()
                    .filter(|e| {
                        e.source_object_index == 0
                            && Some(e.pointer_offset as u64) == p["offset"].as_u64()
                    })
                    .collect();
                if edges.len() != 1
                    || g.objects[edges[0].target_object_index].class_name != "FamInstDoor"
                {
                    return Ok(());
                }
                let orientation = serde_json::json!({"m_flippedFromToRoom":f["m_flippedFromToRoom"],"m_flippedX":f["m_flippedX"],"m_flippedY":f["m_flippedY"],"semantics":"raw_flags_not_additional_transform_operations"});
                for key in ["m_flippedFromToRoom", "m_flippedX", "m_flippedY"] {
                    ensure!(
                        f[key].as_bool().is_some(),
                        "door orientation flag absent: {key}"
                    );
                }
                let from_to_flip = f["m_flippedFromToRoom"].as_bool().unwrap();
                let hand_flip = f["m_flippedX"].as_bool().unwrap();
                let eligible = ["m_workPlaneFlipped", "m_instDormant", "m_useOffsetPos"]
                    .iter()
                    .all(|k| false_field(f, k))
                    && identifier(&f["m_designOptionId"])? == -1
                    && identifier(&f["m_superInstanceId"])? == -1;
                ensure!(
                    self.doors
                        .insert(
                            r.identity.element_id,
                            Door {
                                source: source(
                                    r,
                                    "m_oFamInstSpec; orientation flags; m_masterSymbolId"
                                ),
                                type_id: identifier(&f["m_masterSymbolId"])?,
                                eligible,
                                orientation,
                                from_to_flip,
                                hand_flip
                            }
                        )
                        .is_none(),
                    "duplicate door owner"
                );
            }
            "FamilySymbol" => {
                self.types.insert(
                    r.identity.element_id as i64,
                    (identifier(&f["m_familyId"])?, source(r, "m_familyId")),
                );
            }
            "Family" => {
                self.families.insert(
                    r.identity.element_id as i64,
                    Family {
                        source: source(r, "m_hasRoomCalculationPoint; m_categoryId"),
                        eligible: false_field(f, "m_hasRoomCalculationPoint")
                            && identifier(&f["m_categoryId"])? == -2000023,
                    },
                );
            }
            "RoomElem" => {
                ensure!(
                    false_field(f, "m_bIsLocationless")
                        && identifier(&f["m_areaSchemeId"])? == -1
                        && identifier(&f["m_areaSpaceElemId"])? == -1
                        && f["m_SpaceRoomLocationInfo"]["pointer_token"].as_u64() == Some(0),
                    "non-room/unplaced room subtype"
                );
                let h = f["m_height"]
                    .as_f64()
                    .ok_or_else(|| anyhow::anyhow!("room height absent"))?;
                ensure!(
                    h.is_finite()
                        && h > 0.
                        && f["m_lowerOffset"].as_f64() == Some(0.)
                        && identifier(&f["m_upperLevelId"])? == -1,
                    "room vertical extent unsupported"
                );
                self.rooms.insert(
                    r.identity.element_id,
                    Room {
                        source: source(r, "m_height; m_lowerOffset; room subtype"),
                        height: h,
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }
    pub fn finish(
        self,
        spatial: &native_spatial_context::Inventory,
        lifecycle: &native_lifecycle::Inventory,
        boundaries: &native_spatial_boundaries::Inventory,
    ) -> Result<Inventory> {
        let mut out = Inventory {
            format: "rvt-native-room-connections-v1",
            complete_room_connection_parity: false,
            semantics: "computed_from_explicit_host_phase_topology_and_saved_facing; not_stored_room_ids",
            connections: vec![],
            diagnostics: self.diagnostics.clone(),
        };
        for (id, door) in &self.doors {
            let Some(life) = lifecycle
                .elements
                .iter()
                .find(|e| e.identity.element_id == *id)
            else {
                out.diagnostics
                    .push(format!("door {id}: lifecycle unavailable"));
                continue;
            };
            for phase in &life.phase_states {
                let mut row = Connection {
                    door: door.source.owner.clone(),
                    phase_id: phase.phase_id,
                    status: "unresolved",
                    from_room: None,
                    to_room: None,
                    diagnostic: None,
                    sources: vec![
                        serde_json::to_value(&door.source)?,
                        serde_json::to_value(phase)?,
                        door.orientation.clone(),
                    ],
                };
                if phase.status == Some("Future") {
                    row.status = "computed_null_before_creation";
                } else {
                    match self.resolve(*id, door, phase, spatial, boundaries) {
                        Ok((from, to, sources)) => {
                            row.status = "computed_two_sided_host_adjacency";
                            row.from_room = Some(from);
                            row.to_room = Some(to);
                            row.sources.extend(sources);
                        }
                        Err(e) => row.diagnostic = Some(format!("{e:#}")),
                    }
                }
                out.connections.push(row);
            }
        }
        Ok(out)
    }
    fn resolve(
        &self,
        id: u64,
        door: &Door,
        phase: &native_lifecycle::PhaseState,
        spatial: &native_spatial_context::Inventory,
        boundaries: &native_spatial_boundaries::Inventory,
    ) -> Result<(Identity, Identity, Vec<Value>)> {
        ensure!(
            matches!(phase.status, Some("New" | "Existing")),
            "unsupported door phase state"
        );
        ensure!(
            door.eligible,
            "work-plane flipped, nested, offset or design-option door unqualified"
        );
        let (family_id, ts) = self
            .types
            .get(&door.type_id)
            .ok_or_else(|| anyhow::anyhow!("door type absent"))?;
        let family = self
            .families
            .get(family_id)
            .ok_or_else(|| anyhow::anyhow!("door family absent"))?;
        ensure!(
            family.eligible,
            "custom calculation points or non-door family unqualified"
        );
        let context = spatial
            .elements
            .iter()
            .find(|e| e.identity.element_id == id)
            .ok_or_else(|| anyhow::anyhow!("door context absent"))?;
        let host = context
            .references
            .get("host")
            .and_then(|r| r.target_identity.as_ref())
            .ok_or_else(|| anyhow::anyhow!("door host unresolved"))?;
        let level = context
            .references
            .get("level")
            .and_then(|r| r.raw_target_id)
            .ok_or_else(|| anyhow::anyhow!("door level unresolved"))?;
        let t = context
            .transform
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("door transform absent"))?;
        ensure!(
            t.basis_z
                .iter()
                .zip([0., 0., 1.])
                .all(|(a, b)| (*a - b).abs() < 1e-9)
                && t.basis_y[2].abs() < 1e-9,
            "nonvertical door"
        );
        let facing = room_facing(t.basis_y, door.hand_flip, door.from_to_flip);
        let mut from = vec![];
        let mut to = vec![];
        let mut sources = vec![
            serde_json::to_value(ts)?,
            serde_json::to_value(&family.source)?,
            serde_json::to_value(&t.source)?,
            serde_json::to_value(&context.references["host"])?,
        ];
        for room in &boundaries.rooms {
            if room.phase_id != phase.phase_id || room.level_id != level {
                continue;
            }
            let Some(boundary) = room
                .evaluated_boundaries
                .iter()
                .find(|b| b.boundary_location == "Center")
            else {
                continue;
            };
            for seg in boundary
                .loops
                .iter()
                .flatten()
                .filter(|s| s.element_id == host.element_id as i64 && s.link_element_id < 0)
            {
                let Some(side) = host_side(seg.start, seg.end, t.origin, facing) else {
                    continue;
                };
                let qualified = self.rooms.get(&room.owner.element_id).ok_or_else(|| {
                    anyhow::anyhow!("host-adjacent room vertical/subtype unavailable")
                })?;
                ensure!(
                    t.origin[2] >= seg.start[2] - 1e-8
                        && t.origin[2] < seg.start[2] + qualified.height - 1e-8,
                    "door outside supported room vertical extent"
                );
                if side > 0 {
                    to.push(room.owner.clone());
                } else {
                    from.push(room.owner.clone());
                }
                sources.push(serde_json::to_value(&seg.sources)?);
                sources.push(serde_json::to_value(&qualified.source)?);
            }
        }
        ensure!(
            from.len() == 1 && to.len() == 1 && from[0].element_id != to[0].element_id,
            "two unique opposite host-adjacent rooms required"
        );
        sources.push(serde_json::json!({"role":"phase_room_orientation","saved_transform_facing":t.basis_y,"hand_flip":door.hand_flip,"from_to_flip":door.from_to_flip,"effective_room_direction":facing,"rule":"saved basis Y corrected by m_flippedX XOR m_flippedFromToRoom; m_flippedY does not change default room direction in qualified wall-hosted doors"}));
        Ok((from.remove(0), to.remove(0), sources))
    }
}
fn room_facing(saved_facing: [f64; 3], hand_flip: bool, from_to_flip: bool) -> [f64; 3] {
    saved_facing.map(|v| if hand_flip ^ from_to_flip { -v } else { v })
}
fn host_side(a: [f64; 3], b: [f64; 3], p: [f64; 3], facing: [f64; 3]) -> Option<i8> {
    if a.iter()
        .chain(b.iter())
        .chain(p.iter())
        .chain(facing.iter())
        .any(|v| !v.is_finite())
    {
        return None;
    }
    if (facing.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() > 1e-8 {
        return None;
    }
    let d = [b[0] - a[0], b[1] - a[1]];
    let length = d[0].hypot(d[1]);
    if length < 1e-8 || (a[2] - b[2]).abs() > 1e-8 {
        return None;
    }
    let q = [p[0] - a[0], p[1] - a[1]];
    let along = (q[0] * d[0] + q[1] * d[1]) / length;
    if along <= 1e-8 || along >= length - 1e-8 || (q[0] * d[1] - q[1] * d[0]).abs() / length > 1e-8
    {
        return None;
    }
    let dot = (-d[1] * facing[0] + d[0] * facing[1]) / length;
    if (dot.abs() - 1.).abs() > 1e-8 {
        return None;
    }
    Some(if dot > 0. { 1 } else { -1 })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_facing_and_explicit_room_swap_remain_independent() {
        let a = [20., 0., 0.];
        let b = [20., 20., 0.];
        let origin = [20., 10., 0.];
        assert_eq!(
            host_side(a, b, origin, room_facing([-1., 0., 0.], false, false)),
            Some(1)
        );
        assert_eq!(
            host_side(a, b, origin, room_facing([1., 0., 0.], true, false)),
            Some(1)
        );
        assert_eq!(
            host_side(a, b, origin, room_facing([-1., 0., 0.], false, true)),
            Some(-1)
        );
        assert_eq!(
            host_side(a, b, origin, room_facing([1., 0., 0.], true, true)),
            Some(-1)
        );
    }
    #[test]
    fn direction_and_strict_host_membership() {
        assert_eq!(
            host_side([20., 0., 0.], [20., 20., 0.], [20., 10., 0.], [-1., 0., 0.]),
            Some(1)
        );
        assert_eq!(
            host_side([20., 20., 0.], [20., 0., 0.], [20., 10., 0.], [-1., 0., 0.]),
            Some(-1)
        );
        assert_eq!(
            host_side([20., 0., 0.], [20., 20., 0.], [19., 10., 0.], [-1., 0., 0.]),
            None
        );
        assert_eq!(
            host_side([20., 0., 0.], [20., 20., 0.], [20., 20., 0.], [-1., 0., 0.]),
            None
        );
    }
}
