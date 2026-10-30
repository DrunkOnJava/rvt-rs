//! Strict classifier for the observed empty-trim saved-graphics records.
//!
//! Saved channel-103 cutout evidence contains Plane-backed `Face` records
//! with no trim loop.  They are retained by the graphics reader as an
//! unbounded-face diagnostic today.  This module only identifies the exact
//! observed record shape; it does not discard the face or assign general
//! meaning to its flags.  A match is not a general free-plane classification:
//! callers must retain the raw graph record and its provenance.

use crate::native_metadata::identifier;
use crate::native_parameters::ObjectGraph;
use crate::native_saved_mesh::pointer;
use anyhow::Result;
use serde_json::Value;

const OBSERVED_GINFO_FLAGS: u64 = 0x8204;
const OBSERVED_FACE_FLAGS_V9: u64 = 6;

/// Return true only for the observed channel-103 empty-trim Plane face.
///
/// The predicate is deliberately conservative and returns false for every
/// malformed, incomplete, differently flagged, looped, filled, or
/// actively/linked-edge-referenced face.  The caller must preserve the raw face and graph
/// provenance even when this returns true.
pub fn is_observed_empty_trim_face(graph: &ObjectGraph, face_index: usize) -> bool {
    let Some(face) = graph.objects.get(face_index) else {
        return false;
    };
    if face.class_name != "Face" {
        return false;
    }
    let Some(fields) = face.fields.as_object() else {
        return false;
    };
    if fields
        .get("m_GInfo")
        .and_then(Value::as_object)
        .and_then(|v| v.get("m_flags"))
        .and_then(Value::as_u64)
        != Some(OBSERVED_GINFO_FLAGS)
    {
        return false;
    }
    if fields.get("m_faceFlags_v9").and_then(Value::as_u64) != Some(OBSERVED_FACE_FLAGS_V9)
        || fields
            .get("m_faceRegions")
            .and_then(Value::as_array)
            .map(Vec::len)
            != Some(0)
        || !is_minus_one_id(fields.get("m_renderStyleId"))
        || !is_null_pointer(fields.get("m_pFirstLoop"))
        || !is_null_pointer(fields.get("m_pGFilling"))
        || !is_null_pointer(fields.get("m_oBackgroundFilling"))
    {
        return false;
    }
    if !matches!(has_active_edge_face_reference(graph, face_index), Ok(false)) {
        return false;
    }
    let Some(surface_value) = fields.get("m_pSurf") else {
        return false;
    };
    let Ok(Some(surface_index)) = pointer(graph, face_index, surface_value) else {
        return false;
    };
    let Some(surface) = graph.objects.get(surface_index) else {
        return false;
    };
    surface.class_name == "Plane" && ordered_finite_envelope(&surface.fields)
}

fn is_null_pointer(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_object)
        .and_then(|v| v.get("pointer_token"))
        .and_then(Value::as_u64)
        == Some(0)
}

fn is_minus_one_id(value: Option<&Value>) -> bool {
    value.is_some_and(|v| identifier(v).ok() == Some(-1))
}

fn has_active_edge_face_reference(graph: &ObjectGraph, face_index: usize) -> Result<bool> {
    for (source_index, object) in graph.objects.iter().enumerate() {
        if object.class_name != "Edge" {
            continue;
        }
        let Some(values) = object.fields.get("m_pFace") else {
            continue;
        };
        let values = values
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Edge.m_pFace is not an array"))?;
        if values.len() != 2 {
            return Err(anyhow::anyhow!(
                "Edge.m_pFace must contain two face pointers"
            ));
        }
        for value in values {
            if pointer(graph, source_index, value)? == Some(face_index) {
                // The observed records are retired Edge objects: both
                // adjacency arrays exist, have two sides, and contain only
                // null pointers. Any linked adjacency makes the face active
                // (or malformed), so fail closed.
                for field in ["m_next", "m_prev"] {
                    let adjacency = object
                        .fields
                        .get(field)
                        .ok_or_else(|| anyhow::anyhow!("retired Edge missing {field}"))?;
                    let entries = adjacency
                        .as_array()
                        .ok_or_else(|| anyhow::anyhow!("retired Edge {field} is not an array"))?;
                    if entries.len() != 2
                        || entries.iter().any(|entry| !is_null_pointer(Some(entry)))
                    {
                        return Ok(true);
                    }
                }
                // Retired edges must not be reachable from another Edge or
                // EdgeLoop through either adjacency field.
                for (link_source, link_object) in graph.objects.iter().enumerate() {
                    if !matches!(
                        link_object.class_name.as_str(),
                        "Edge" | "EdgeLoop" | "EdgeLoopWithChainEnvelopes"
                    ) {
                        continue;
                    }
                    for field in ["m_next", "m_prev"] {
                        let Some(adjacency) = link_object.fields.get(field) else {
                            continue;
                        };
                        let entries: Vec<&Value> = if let Some(array) = adjacency.as_array() {
                            array.iter().collect()
                        } else {
                            vec![adjacency]
                        };
                        for entry in entries {
                            if pointer(graph, link_source, entry)? == Some(source_index) {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}

fn ordered_finite_envelope(fields: &Value) -> bool {
    let Some(corners) = fields
        .get("m_Envelope")
        .and_then(Value::as_object)
        .and_then(|v| v.get("m_corners"))
        .and_then(Value::as_array)
    else {
        return false;
    };
    if corners.len() != 2 {
        return false;
    }
    let Some(first) = corners[0].as_array() else {
        return false;
    };
    let Some(second) = corners[1].as_array() else {
        return false;
    };
    if first.len() != 2 || second.len() != 2 {
        return false;
    }
    let Some(a) = first.iter().map(Value::as_f64).collect::<Option<Vec<_>>>() else {
        return false;
    };
    let Some(b) = second.iter().map(Value::as_f64).collect::<Option<Vec<_>>>() else {
        return false;
    };
    a.iter().chain(b.iter()).all(|value| value.is_finite()) && b[0] > a[0] && b[1] > a[1]
}
