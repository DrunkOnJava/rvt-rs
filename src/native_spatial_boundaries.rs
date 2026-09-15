//! Saved room plan topology. Centerline carriers are not finish/core boundaries.
use crate::{native_document::Record, native_equipment::Identity, native_metadata::identifier};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
mod evaluation;
pub use evaluation::{Boundary, BoundarySegment, Opening};

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub owner: Identity,
    pub object_index: usize,
    pub field: String,
    pub body_sha256: String,
    pub stream: String,
    pub group_record_offset: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub struct SegmentKey {
    pub host_or_link_instance_id: i64,
    pub linked_element_id: i64,
    pub major_index: i64,
    pub sub_index: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub key: SegmentKey,
    pub owner: Option<Identity>,
    pub points: Vec<[f64; 2]>,
    pub parameter_range: [f64; 2],
    pub raw_area_terms: [f64; 2],
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct Circuit {
    pub id: i64,
    pub stored_area_square_feet: f64,
    pub directed_sides: Vec<SegmentKey>,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct Topology {
    pub owner: Identity,
    pub level_id: i64,
    pub listed_room_ids: Vec<i64>,
    pub segments: Vec<Segment>,
    pub circuits: Vec<Circuit>,
    pub components: Value,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct Room {
    pub owner: Identity,
    pub source_kind: String,
    pub zone_scheme_id: i64,
    pub area_scheme_id: i64,
    pub level_id: i64,
    pub phase_id: i64,
    pub upper_level_id: i64,
    pub lower_offset: f64,
    pub upper_offset: Option<f64>,
    pub height: Option<f64>,
    pub locationless: Option<bool>,
    pub cached_circuit_id: i64,
    pub topology_owner: Option<Identity>,
    pub circuit: Option<Circuit>,
    pub carrier_loops: Vec<Vec<Segment>>,
    pub status: String,
    pub source: Source,
    pub evaluated_boundaries: Vec<Boundary>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_boundary_parity: bool,
    pub semantics: &'static str,
    pub rooms: Vec<Room>,
    pub topologies: Vec<Topology>,
    pub diagnostics: Vec<String>,
    pub openings: Vec<Opening>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    identities: BTreeMap<u64, Identity>,
    rooms: Vec<Room>,
    topologies: Vec<Topology>,
    diagnostics: Vec<String>,
    evaluation: evaluation::Builder,
}
fn source(r: &Record, object_index: usize, field: &str) -> Source {
    Source {
        owner: Identity {
            element_id: r.identity.element_id,
            unique_id: r.identity.unique_id.clone(),
        },
        object_index,
        field: field.into(),
        body_sha256: r.source.body_sha256.clone(),
        stream: r.source.stream.clone(),
        group_record_offset: r.source.group_record_offset,
    }
}
fn number(v: &Value) -> Result<f64> {
    let x = v
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("topology number absent"))?;
    ensure!(x.is_finite(), "nonfinite topology number");
    Ok(x)
}
fn int(v: &Value) -> Result<i64> {
    v.as_i64()
        .ok_or_else(|| anyhow::anyhow!("topology integer absent"))
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| anyhow::anyhow!("topology array absent"))
}
fn key(v: &Value) -> Result<SegmentKey> {
    Ok(SegmentKey {
        host_or_link_instance_id: identifier(&v["m_item"]["m_linkInstOrHostId"])?,
        linked_element_id: identifier(&v["m_item"]["m_linkRef"])?,
        major_index: int(&v["m_majorIndex"])?,
        sub_index: int(&v["m_subIndex"])?,
    })
}
impl InventoryBuilder {
    pub fn ingest(&mut self, r: &Record) -> Result<()> {
        if r.channel != 102 {
            return Ok(());
        }
        self.identities
            .insert(r.identity.element_id, source(r, 0, "").owner);
        if let Err(e) = self.evaluation.ingest(r) {
            self.diagnostics
                .push(format!("boundary inputs {}: {e:#}", r.identity.element_id));
        }
        if !matches!(r.class_name.as_deref(), Some("RoomElem" | "LevelRoomPlan")) {
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
            .ok_or_else(|| anyhow::anyhow!("room topology graph unavailable"))?;
        let root = g
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty topology graph"))?;
        let f = &root.fields;
        if root.class_name == "RoomElem" {
            let (source_kind, zone_scheme_id, area_scheme_id) =
                crate::native_spatial_context::classify_room_space(&root.fields)?;
            self.rooms.push(Room {
                owner: source(r, 0, "").owner,
                source_kind,
                zone_scheme_id,
                area_scheme_id,
                level_id: identifier(&f["m_levelId"])?,
                phase_id: identifier(&f["m_phaseId"])?,
                lower_offset: number(&f["m_lowerOffset"])?,
                cached_circuit_id: int(&f["m_cachedCircuitId"]["m_id"])?,
                upper_level_id: identifier(&f["m_upperLevelId"])?,
                upper_offset: f.get("m_upperOffset").map(number).transpose()?,
                height: f.get("m_height").map(number).transpose()?,
                locationless: f.get("m_bIsLocationless").and_then(Value::as_bool),
                topology_owner: None,
                circuit: None,
                carrier_loops: Vec::new(),
                status: "unresolved_saved_circuit".into(),
                source: source(r, 0, "m_cachedCircuitId"),
                evaluated_boundaries: Vec::new(),
            });
            return Ok(());
        }
        let p = &f["m_planTopology"];
        if p["pointer_token"].as_u64() == Some(0) {
            return Ok(());
        }
        let edges: Vec<_> = g
            .edges
            .iter()
            .filter(|e| {
                e.source_object_index == 0 && Some(e.pointer_offset as u64) == p["offset"].as_u64()
            })
            .collect();
        ensure!(edges.len() == 1, "plan topology requires one owned edge");
        let i = edges[0].target_object_index;
        let o = &g.objects[i];
        ensure!(
            o.class_name == "PlanTopology",
            "unexpected plan topology class"
        );
        let f = &o.fields;
        ensure!(
            identifier(&f["m_levelRoomPlanId"])? == i64::try_from(r.identity.element_id)?,
            "plan topology owner mismatch"
        );
        let mut segments = Vec::new();
        for (n, v) in array(&f["m_segments"])?.iter().enumerate() {
            let z = &v["m_info"];
            let points = array(&z["m_points"])?
                .iter()
                .map(|p| {
                    ensure!(array(p)?.len() == 2, "topology point dimension");
                    Ok([number(&p[0])?, number(&p[1])?])
                })
                .collect::<Result<Vec<_>>>()?;
            segments.push(Segment {
                key: key(&v["m_key"])?,
                owner: None,
                points,
                parameter_range: [number(&z["m_param0"])?, number(&z["m_param1"])?],
                raw_area_terms: [number(&z["m_area"])?, number(&z["m_areaOnRight"])?],
                source: source(r, i, &format!("m_segments[{n}]")),
            });
        }
        let mut circuits = Vec::new();
        for (n, v) in array(&f["m_circuits"])?.iter().enumerate() {
            circuits.push(Circuit {
                id: int(&v["m_id"]["m_id"])?,
                stored_area_square_feet: number(&v["m_area"])?,
                directed_sides: array(&v["m_sides"])?
                    .iter()
                    .map(key)
                    .collect::<Result<_>>()?,
                source: source(r, i, &format!("m_circuits[{n}]")),
            });
        }
        self.topologies.push(Topology {
            owner: source(r, 0, "").owner,
            level_id: identifier(&f["m_levelId"])?,
            listed_room_ids: array(&f["m_rooms"])?
                .iter()
                .map(identifier)
                .collect::<Result<_>>()?,
            segments,
            circuits,
            components: f["m_components"].clone(),
            source: source(r, i, "PlanTopology"),
        });
        Ok(())
    }
    pub fn finish(mut self) -> Result<Inventory> {
        for t in &mut self.topologies {
            for s in &mut t.segments {
                s.owner = u64::try_from(s.key.host_or_link_instance_id)
                    .ok()
                    .and_then(|id| self.identities.get(&id).cloned());
            }
        }
        for r in &mut self.rooms {
            let ts: Vec<_> = self
                .topologies
                .iter()
                .filter(|t| {
                    t.level_id == r.level_id
                        && t.listed_room_ids.contains(&(r.owner.element_id as i64))
                })
                .collect();
            if ts.len() != 1 {
                r.status = "unresolved_or_ambiguous_room_topology_membership".into();
                continue;
            }
            let t = ts[0];
            r.topology_owner = Some(t.owner.clone());
            let cs: Vec<_> = t
                .circuits
                .iter()
                .filter(|c| c.id == r.cached_circuit_id)
                .collect();
            if cs.len() != 1 {
                r.status = "unresolved_or_ambiguous_cached_circuit".into();
                continue;
            }
            r.circuit = Some(cs[0].clone());
            match loops(t, cs[0]) {
                Ok(ls) => {
                    r.carrier_loops = ls;
                    r.status = "resolved_saved_carrier_topology_not_evaluated_boundary".into();
                }
                Err(e) => {
                    r.status = "unsupported_directed_carrier_topology".into();
                    self.diagnostics
                        .push(format!("room {}: {e:#}", r.owner.element_id));
                }
            }
            for mode in ["Center", "Finish", "CoreBoundary", "CoreCenter"] {
                match self.evaluation.evaluate(r, mode) {
                    Ok(b) => r.evaluated_boundaries.push(b),
                    Err(e) => self
                        .diagnostics
                        .push(format!("room {} {mode}: {e:#}", r.owner.element_id)),
                }
            }
        }
        let openings = self.evaluation.openings(&self.identities);
        Ok(Inventory {
            format: "rvt-native-spatial-boundaries/v1",
            complete_boundary_parity: false,
            semantics: "saved_carriers_and_explicitly_qualified_boundary_evaluations_internal_feet",
            rooms: self.rooms,
            topologies: self.topologies,
            diagnostics: self.diagnostics,
            openings,
        })
    }
}
fn directed(t: &Topology, c: &Circuit) -> Result<Vec<Segment>> {
    let mut out = Vec::new();
    for k in &c.directed_sides {
        let mut base = k.clone();
        let (reverse, sub_index) = decode_sub_index(base.sub_index)?;
        base.sub_index = sub_index;
        let found: Vec<_> = t.segments.iter().filter(|s| s.key == base).collect();
        ensure!(
            found.len() == 1,
            "directed side has no unique saved carrier"
        );
        let mut s = found[0].clone();
        ensure!(s.points.len() >= 2, "empty/point carrier");
        if reverse {
            s.points.reverse();
            s.parameter_range.reverse();
        }
        s.key = k.clone();
        out.push(s);
    }
    ensure!(!out.is_empty(), "empty circuit");
    for i in 0..out.len() {
        let a = out[i].points.last().unwrap();
        let b = out[(i + 1) % out.len()].points.first().unwrap();
        ensure!(
            (a[0] - b[0]).abs() < 1e-7 && (a[1] - b[1]).abs() < 1e-7,
            "carrier circuit is not closed in saved order"
        );
    }
    Ok(out)
}

fn decode_sub_index(raw: i64) -> Result<(bool, i64)> {
    let reverse = raw < 0;
    let index = i32::try_from(raw).context("directed segment sub-index range")?;
    // Native directed indices encode reverse as bitwise complement:
    // -1 reverses 0, -2 reverses 1, and so on.
    Ok((reverse, i64::from(if reverse { !index } else { index })))
}
fn loops(t: &Topology, c: &Circuit) -> Result<Vec<Vec<Segment>>> {
    let mut out = vec![directed(t, c)?];
    for component in array(&t.components)? {
        if int(&component["m_enclosingCircuit"]["m_id"])? == c.id {
            let outer = int(&component["m_outerCircuit"]["m_id"])?;
            let cs: Vec<_> = t.circuits.iter().filter(|x| x.id == outer).collect();
            ensure!(cs.len() == 1, "inner component outer circuit ambiguous");
            out.push(directed(t, cs[0])?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Topology {
        let source = Source {
            owner: Identity {
                element_id: 10,
                unique_id: "test".into(),
            },
            object_index: 1,
            field: "fixture".into(),
            body_sha256: "test".into(),
            stream: "test".into(),
            group_record_offset: 0,
        };
        let points = [[0., 0.], [4., 0.], [4., 3.], [0., 3.]];
        let segments: Vec<_> = (0..4)
            .map(|i| Segment {
                key: SegmentKey {
                    host_or_link_instance_id: i as i64 + 20,
                    linked_element_id: -1,
                    major_index: 0,
                    sub_index: 0,
                },
                owner: None,
                points: vec![points[i], points[(i + 1) % 4]],
                parameter_range: [0., 1.],
                raw_area_terms: [0., 0.],
                source: source.clone(),
            })
            .collect();
        let circuit = Circuit {
            id: 0,
            stored_area_square_feet: 8.,
            directed_sides: segments.iter().map(|s| s.key.clone()).collect(),
            source: source.clone(),
        };
        Topology {
            owner: source.owner.clone(),
            level_id: 1,
            listed_room_ids: vec![2],
            segments,
            circuits: vec![circuit],
            components: serde_json::json!([]),
            source,
        }
    }
    #[test]
    fn directed_order_and_stored_area_are_not_reinterpreted() {
        let t = fixture();
        let c = &t.circuits[0];
        assert_eq!(directed(&t, c).unwrap()[0].points, vec![[0., 0.], [4., 0.]]);
        assert_eq!(c.stored_area_square_feet, 8.); // The carrier polygon has area12.
        let mut reverse = c.clone();
        reverse.directed_sides.reverse();
        for k in &mut reverse.directed_sides {
            k.sub_index = -1;
        }
        assert_eq!(
            directed(&t, &reverse).unwrap()[0].points,
            vec![[0., 0.], [0., 3.]]
        );
    }
    #[test]
    fn ambiguous_missing_and_disconnected_carriers_refuse() {
        let mut t = fixture();
        t.segments.push(t.segments[0].clone());
        assert!(directed(&t, &t.circuits[0]).is_err());
        t.segments.pop();
        t.segments[0].points[1] = [5., 0.];
        assert!(directed(&t, &t.circuits[0]).is_err());
        t = fixture();
        t.circuits[0].directed_sides[0].sub_index = -2;
        assert!(directed(&t, &t.circuits[0]).is_err());
    }

    #[test]
    fn negative_sub_index_is_signed_complement_of_i32_index() {
        assert_eq!(decode_sub_index(1).unwrap(), (false, 1));
        assert_eq!(decode_sub_index(-1).unwrap(), (true, 0));
        assert_eq!(decode_sub_index(-2).unwrap(), (true, 1));
        assert!(decode_sub_index(i64::MIN).is_err());
    }

    #[test]
    fn synthetic_six_segment_circuit_closes_with_saved_area() {
        let raw: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/synthetic-signed-subindex-circuit.json"
        )))
        .unwrap();
        let source = Source {
            owner: Identity {
                element_id: 1000,
                unique_id: "fixture".into(),
            },
            object_index: 0,
            field: "fixture".into(),
            body_sha256: "fixture".into(),
            stream: "fixture".into(),
            group_record_offset: 0,
        };
        // The fixture uses synthetic coordinates and identifiers. Its points
        // are directed points, so restore base orientation before exercising
        // the decoder.
        let segments = raw["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let mut points: Vec<[f64; 2]> =
                    serde_json::from_value(v["points"].clone()).unwrap();
                if v["directed_key"]["sub_index"].as_i64().unwrap() < 0 {
                    points.reverse();
                }
                Segment {
                    key: serde_json::from_value(v["base_key"].clone()).unwrap(),
                    owner: None,
                    points,
                    parameter_range: [0., 1.],
                    raw_area_terms: [0., 0.],
                    source: source.clone(),
                }
            })
            .collect::<Vec<_>>();
        let directed_sides = raw["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| serde_json::from_value(v["directed_key"].clone()).unwrap())
            .collect::<Vec<SegmentKey>>();
        let circuit = Circuit {
            id: 3000,
            stored_area_square_feet: raw["saved_area_square_feet"].as_f64().unwrap(),
            directed_sides,
            source: source.clone(),
        };
        let topology = Topology {
            owner: source.owner.clone(),
            level_id: 2000,
            listed_room_ids: vec![1000],
            segments,
            circuits: vec![circuit],
            components: serde_json::json!([]),
            source,
        };
        let loop_segments = directed(&topology, &topology.circuits[0]).unwrap();
        assert_eq!(loop_segments.len(), 6);
        assert!(
            topology.circuits[0]
                .directed_sides
                .iter()
                .any(|k| k.sub_index == -2)
        );
        let area = loop_segments
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let a = s.points[0];
                let b = loop_segments[(i + 1) % loop_segments.len()].points[0];
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            .abs()
            / 2.;
        assert!((area - raw["signed_area_square_feet"].as_f64().unwrap()).abs() < 1e-9);
        assert!((area - topology.circuits[0].stored_area_square_feet).abs() < 1e-9);
    }
}
