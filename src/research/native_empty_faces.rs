//! Strict classifier for the observed empty-trim saved-graphics records.
//!
//! Saved channel-103 cutout evidence contains Plane-backed `Face` records
//! with no trim loop.  They are retained by the graphics reader as an
//! unbounded-face diagnostic today.  This module only identifies the exact
//! observed record shape; it does not discard the face or assign general
//! meaning to its flags.  A match is not a general free-plane classification:
//! callers must retain the raw graph record and its provenance.

use crate::graph_value::Fv;
use crate::native_metadata::identifier;
use crate::native_parameters::ObjectGraph;
use crate::native_saved_mesh::pointer;
use std::collections::{BTreeMap, BTreeSet};

const OBSERVED_GINFO_FLAGS: u64 = 0x8204;
const OBSERVED_FACE_FLAGS_V9: u64 = 6;

/// Return true only for the observed channel-103 empty-trim Plane face.
///
/// The predicate is deliberately conservative and returns false for every
/// malformed, incomplete, differently flagged, looped, filled, or
/// actively/linked-edge-referenced face.  The caller must preserve the raw face and graph
/// provenance even when this returns true.
///
/// It walks `graph` once for the face it is asked about; a caller asking
/// about many faces of one graph builds an [`EmptyTrimIndex`] once (#1833).
pub fn is_observed_empty_trim_face(graph: &ObjectGraph, face_index: usize) -> bool {
    observed_shape(graph, face_index)
        && !EmptyTrimIndex::new(graph).refused(face_index)
        && plane_surface(graph, face_index)
}

/// What the edges of a graph say against each of its faces, from one walk:
/// the per-face walk of the graph, done once for each face and again for
/// each edge naming it, cost a graph of many such faces its objects times
/// its faces (#1833). Each answer is the one that walk gave: its order only
/// decided which of two refusals it met first.
#[derive(Debug)]
pub struct EmptyTrimIndex {
    /// An `Edge` whose `m_pFace` is not two face pointers, or names one this
    /// graph cannot resolve: the walk refused every face for it.
    malformed: bool,
    /// A link record's adjacency this graph cannot resolve: the walk refused
    /// every face an edge names for it.
    link_error: bool,
    /// Each face an `Edge` names, and whether one naming it is anything but a
    /// retired edge no link record reaches.
    named: BTreeMap<usize, bool>,
}

impl EmptyTrimIndex {
    /// The one walk: the link records' adjacencies, then each `Edge`'s faces.
    pub fn new(graph: &ObjectGraph) -> Self {
        let mut linked = BTreeSet::new();
        let mut link_error = false;
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
                let entries: &[Fv] = match adjacency.as_array() {
                    Some(items) => items,
                    None => std::slice::from_ref(adjacency),
                };
                for entry in entries {
                    match pointer(graph, link_source, entry) {
                        Ok(Some(target)) => {
                            linked.insert(target);
                        }
                        Ok(None) => {}
                        Err(_) => link_error = true,
                    }
                }
            }
        }
        let mut malformed = false;
        let mut named: BTreeMap<usize, bool> = BTreeMap::new();
        for (source_index, object) in graph.objects.iter().enumerate() {
            if object.class_name != "Edge" {
                continue;
            }
            let Some(values) = object.fields.get("m_pFace") else {
                continue;
            };
            let Some(values) = values.as_array().filter(|values| values.len() == 2) else {
                malformed = true;
                continue;
            };
            // The observed records are retired Edge objects: both adjacency
            // arrays exist, have two sides, and contain only null pointers,
            // and no link record reaches the edge. Anything else makes the
            // face active (or malformed), so fail closed.
            let retired = ["m_next", "m_prev"].into_iter().all(|field| {
                object
                    .fields
                    .get(field)
                    .and_then(Fv::as_array)
                    .is_some_and(|entries| {
                        entries.len() == 2
                            && entries.iter().all(|entry| is_null_pointer(Some(entry)))
                    })
            }) && !linked.contains(&source_index);
            for value in values {
                match pointer(graph, source_index, value) {
                    Ok(Some(target)) => *named.entry(target).or_insert(false) |= !retired,
                    Ok(None) => {}
                    Err(_) => malformed = true,
                }
            }
        }
        Self {
            malformed,
            link_error,
            named,
        }
    }

    /// Whether the edges refuse `face_index`.
    fn refused(&self, face_index: usize) -> bool {
        self.malformed
            || self
                .named
                .get(&face_index)
                .is_some_and(|active| self.link_error || *active)
    }

    /// [`is_observed_empty_trim_face`] for a face of the graph this indexes.
    pub fn is_observed_empty_trim_face(&self, graph: &ObjectGraph, face_index: usize) -> bool {
        observed_shape(graph, face_index)
            && !self.refused(face_index)
            && plane_surface(graph, face_index)
    }
}

/// The face's own record is the observed shape: its flags, no regions, no
/// render style, no loop and no fillings.
fn observed_shape(graph: &ObjectGraph, face_index: usize) -> bool {
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
        .and_then(Fv::as_object)
        .and_then(|v| v.get("m_flags"))
        .and_then(Fv::as_u64)
        != Some(OBSERVED_GINFO_FLAGS)
    {
        return false;
    }
    fields.get("m_faceFlags_v9").and_then(Fv::as_u64) == Some(OBSERVED_FACE_FLAGS_V9)
        && fields
            .get("m_faceRegions")
            .and_then(Fv::as_array)
            .map(|items| items.len())
            == Some(0)
        && is_minus_one_id(fields.get("m_renderStyleId"))
        && is_null_pointer(fields.get("m_pFirstLoop"))
        && is_null_pointer(fields.get("m_pGFilling"))
        && is_null_pointer(fields.get("m_oBackgroundFilling"))
}

/// The face's surface is a Plane with an ordered finite envelope.
fn plane_surface(graph: &ObjectGraph, face_index: usize) -> bool {
    let Some(face) = graph.objects.get(face_index) else {
        return false;
    };
    let Some(fields) = face.fields.as_object() else {
        return false;
    };
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

fn is_null_pointer(value: Option<&Fv>) -> bool {
    value
        .and_then(Fv::as_object)
        .and_then(|v| v.get("pointer_token"))
        .and_then(Fv::as_u64)
        == Some(0)
}

fn is_minus_one_id(value: Option<&Fv>) -> bool {
    value.is_some_and(|v| identifier(v).ok() == Some(-1))
}

fn ordered_finite_envelope(fields: &Fv) -> bool {
    let Some(corners) = fields
        .get("m_Envelope")
        .and_then(Fv::as_object)
        .and_then(|v| v.get("m_corners"))
        .and_then(Fv::as_array)
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
    let Some(a) = first.iter().map(Fv::as_f64).collect::<Option<Vec<_>>>() else {
        return false;
    };
    let Some(b) = second.iter().map(Fv::as_f64).collect::<Option<Vec<_>>>() else {
        return false;
    };
    a.iter().chain(b.iter()).all(|value| value.is_finite()) && b[0] > a[0] && b[1] > a[1]
}
