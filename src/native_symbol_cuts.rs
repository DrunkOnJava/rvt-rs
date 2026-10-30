//! The outline a door or window cuts in its host, from the native record of
//! the family symbol that draws it (RE-215).
//!
//! A `FamilySymbol` record's `m_pCutLoops` points at one `CurveLoop`, whose
//! `m_curves` are the `GLine`s of the outline the symbol cuts in its host,
//! in the symbol's XZ plane. A `FamilyInstance` names its type in
//! `m_masterSymbolId`, and the symbol that draws it in its `InstanceInfo`
//! object's `m_symbolId`. The two differ where the instance's own parameters
//! change the family's geometry: Core Interior's windows are of type 17335,
//! whose loop is 3.3136 ft square, and are drawn by symbol 19275, whose loop
//! is 4.9213 by 8.8583 ft. Revit's exporter sizes a door or window from that
//! symbol (revit-ifc's `ExporterIFCUtils.GetOriginalSymbol`, CLEANROOM.md
//! source 5). The loop's extent along X and along Z is the `OverallWidth`
//! and `OverallHeight` Revit's export writes on every door and window
//! measured with a cut loop: Core Interior's 132 doors and 6 windows, RE1
//! Architecture's 4 doors, and the door and 3 windows of each of the R2025
//! models 12 to 17 (`reports/element-framing/RE-215-symbol-cut-loops.md`).

use crate::native_json::{element_id, finite_triple};
use crate::vector_math::distance;
use crate::{RevitFile, native_document, native_saved_mesh::pointer};
use std::collections::{BTreeMap, BTreeSet};

/// The farthest apart, in feet, two ends of the loop's lines may be and
/// still join, and the most the loop may stray from its plane.
const JOIN_TOLERANCE_FEET: f64 = 1e-6;

/// The box of the outline a family symbol cuts in its host, in the symbol's
/// own coordinates, feet: its width runs along X and its height along Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutLoop {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl CutLoop {
    /// The loop's extent along the symbol's X axis, feet.
    pub fn width_feet(&self) -> f64 {
        self.max[0] - self.min[0]
    }

    /// The loop's extent along the symbol's Z axis, feet.
    pub fn height_feet(&self) -> f64 {
        self.max[2] - self.min[2]
    }
}

/// The cut loop of a `FamilySymbol` graph: one loop, every curve a `GLine`,
/// each line's start the previous line's end, closed, flat in the symbol's
/// XZ plane and with some width and height. Anything else is `None`.
fn cut_loop(graph: &crate::native_parameters::ObjectGraph) -> Option<CutLoop> {
    let root = graph.objects.first()?;
    let [loop_pointer] = root.fields.get("m_pCutLoops")?.as_array()?.as_slice() else {
        return None;
    };
    let index = pointer(graph, 0, loop_pointer).ok()??;
    let curve_loop = graph.objects.get(index)?;
    if curve_loop.class_name != "CurveLoop"
        || curve_loop
            .fields
            .get("m_open")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
    {
        return None;
    }
    let mut ends = Vec::new();
    for curve in curve_loop.fields.get("m_curves")?.as_array()? {
        let line = graph.objects.get(pointer(graph, index, curve).ok()??)?;
        if line.class_name != "GLine" {
            return None;
        }
        let origin = line.fields.get("m_origin").and_then(finite_triple)?;
        let direction = line.fields.get("m_dirVec").and_then(finite_triple)?;
        let [start, end] = point2(line.fields.get("m_endParams"))?;
        let at = |t: f64| [0, 1, 2].map(|i| origin[i] + direction[i] * t);
        ends.push((at(start), at(end)));
    }
    if ends.len() < 3 {
        return None;
    }
    let joined = (0..ends.len())
        .all(|i| distance(ends[i].1, ends[(i + 1) % ends.len()].0) <= JOIN_TOLERANCE_FEET);
    if !joined {
        return None;
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for (p, _) in &ends {
        for ((low, high), v) in min.iter_mut().zip(max.iter_mut()).zip(p) {
            *low = low.min(*v);
            *high = high.max(*v);
        }
    }
    let cut = CutLoop { min, max };
    (max[1] - min[1] <= JOIN_TOLERANCE_FEET && cut.width_feet() > 0.0 && cut.height_feet() > 0.0)
        .then_some(cut)
}

fn point2(value: Option<&serde_json::Value>) -> Option<[f64; 2]> {
    let items = value?.as_array()?;
    let [a, b] = items.as_slice() else {
        return None;
    };
    let p = [a.as_f64()?, b.as_f64()?];
    p.iter().all(|v| v.is_finite()).then_some(p)
}

/// For each of `instances` (family instances' ElementIds), the cut loop of
/// the symbol that draws it (its `InstanceInfo.m_symbolId`). An instance
/// that names no symbol, or whose symbol has no loop [`CutLoop`] reads, is
/// left out. One pass reads the instances and every family symbol, since
/// which symbols they name is known only once they are read. Errs where the
/// native record path does not read the file's release.
pub fn instance_cut_loops(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, CutLoop>> {
    Ok(instance_cut_loops_and_uncut(rf, instances)?.0)
}

/// [`instance_cut_loops`], and the instances in `instances` whose symbol
/// cuts no outline at all: its `m_pCutLoops` is an empty list (#810). An
/// instance whose symbol's loop is read but not supported (an arc, several
/// loops, a pointer that does not resolve) is in neither.
pub fn instance_cut_loops_and_uncut(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<(BTreeMap<u64, CutLoop>, BTreeSet<u64>)> {
    let (loops, uncut, _, _, _) = instance_cuts_and_symbols(rf, instances)?;
    Ok((loops, uncut))
}

/// The extents of the solid a family symbol cuts where it cuts no loop
/// (`m_complexCutGeoms`, RE-225), in the symbol's own coordinates, feet:
/// along X (the width), Y (across the host) and Z (the height).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutSolidSize {
    /// Extent along the symbol's X axis.
    pub x_feet: f64,
    /// Extent along its Y axis, across the host.
    pub y_feet: f64,
    /// Extent along its Z axis.
    pub z_feet: f64,
}

/// The extents of `vertices`, or `None` when any is not finite or the
/// solid is flat along an axis.
fn cut_solid_size(vertices: &[[f64; 3]]) -> Option<CutSolidSize> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for vertex in vertices {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(vertex[axis]);
            hi[axis] = hi[axis].max(vertex[axis]);
        }
    }
    let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
    size.iter()
        .all(|extent| extent.is_finite() && *extent > 0.0)
        .then_some(CutSolidSize {
            x_feet: size[0],
            y_feet: size[1],
            z_feet: size[2],
        })
}

/// An instance's cut loop, the instances whose symbol cuts none, the
/// symbol that draws each instance, the element hosting it, and the extents
/// of the solid an instance's symbol cuts where it cuts no loop, as
/// [`instance_cuts_and_symbols`] reads them.
pub type InstanceCuts = (
    BTreeMap<u64, CutLoop>,
    BTreeSet<u64>,
    BTreeMap<u64, u64>,
    BTreeMap<u64, u64>,
    BTreeMap<u64, CutSolidSize>,
);

/// [`instance_cut_loops_and_uncut`], and the symbol that draws each of
/// `instances` (its `InstanceInfo.m_symbolId`), read in the same pass:
/// the type where the instance's own parameters change nothing, else the
/// symbol its parameters make; the element hosting it, the `m_hostId`
/// its current record names (RE-245); and, where the symbol cuts no loop,
/// the extents of the solid it cuts (RE-225).
pub fn instance_cuts_and_symbols(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<InstanceCuts> {
    if instances.is_empty() {
        return Ok((
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
        ));
    }
    let mut solids: BTreeMap<u64, CutSolidSize> = BTreeMap::new();
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    let mut hosts: BTreeMap<u64, u64> = BTreeMap::new();
    let mut loops: BTreeMap<u64, CutLoop> = BTreeMap::new();
    let mut uncut_symbols: BTreeSet<u64> = BTreeSet::new();
    let options = native_document::Options {
        selected_ids: instances.clone(),
        selected_classes: BTreeSet::from(["FamilySymbol".to_string()]),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if record.class_name.as_deref() == Some("FamilySymbol") {
            if let Some(cut) = cut_loop(graph) {
                loops.insert(id, cut);
            } else if graph
                .objects
                .first()
                .and_then(|root| root.fields.get("m_pCutLoops"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                uncut_symbols.insert(id);
                if let Some(size) = cut_solid_vertices(graph).and_then(|v| cut_solid_size(&v)) {
                    solids.insert(id, size);
                }
            }
        } else if instances.contains(&id) {
            if let Some(symbol) = graph
                .objects
                .iter()
                .find(|object| object.class_name == "InstanceInfo")
                .and_then(|object| object.fields.get("m_symbolId"))
                .and_then(element_id)
            {
                drawn_by.insert(id, symbol);
            }
            if let Some(host) = graph
                .objects
                .iter()
                .find_map(|object| object.fields.get("m_hostId").and_then(element_id))
            {
                hosts.insert(id, host);
            }
        }
        Ok(())
    })?;
    let uncut = drawn_by
        .iter()
        .filter(|(_, symbol)| uncut_symbols.contains(symbol))
        .map(|(instance, _)| *instance)
        .collect();
    let loops = drawn_by
        .iter()
        .filter_map(|(instance, symbol)| Some((*instance, *loops.get(symbol)?)))
        .collect();
    let solids = drawn_by
        .iter()
        .filter_map(|(instance, symbol)| Some((*instance, *solids.get(symbol)?)))
        .collect();
    Ok((loops, uncut, drawn_by, hosts, solids))
}

/// A family instance hosted on a face of another element through a
/// sketch plane, and the solid the symbol that draws it cuts (#824).
#[derive(Debug, Clone, PartialEq)]
pub struct HostedCut {
    /// The instance's ElementId.
    pub instance: u64,
    /// The element whose face its sketch plane lies on.
    pub host: u64,
    /// The vertices of the cut solid's bounded faces, in the symbol's
    /// coordinates, feet.
    pub vertices: Vec<[f64; 3]>,
}

/// The vertices of the bounded faces of the solid a `FamilySymbol` graph
/// cuts where it cuts no loop: the `Geometry` objects its `m_complexCutGeoms`
/// names (RE-225). A face with no boundary (an unbounded plane) adds
/// nothing; one that does not mesh leaves the symbol with none.
fn cut_solid_vertices(graph: &crate::native_parameters::ObjectGraph) -> Option<Vec<[f64; 3]>> {
    let root = graph.objects.first()?;
    if !root.fields.get("m_pCutLoops")?.as_array()?.is_empty() {
        return None;
    }
    let mut vertices = Vec::new();
    for solid in root.fields.get("m_complexCutGeoms")?.as_array()? {
        let index = pointer(graph, 0, solid).ok()??;
        let geometry = graph.objects.get(index)?;
        if geometry.class_name != "Geometry" {
            return None;
        }
        for face in geometry.fields.get("m_pFaces")?.as_array()? {
            let face = pointer(graph, index, face).ok()??;
            if let Some(mesh) = crate::native_saved_mesh::face(graph, face).ok()? {
                vertices.extend(mesh.vertices);
            }
        }
    }
    (!vertices.is_empty()).then_some(vertices)
}

/// For each of `instances` (family instances' ElementIds) whose `m_hostId`
/// is a `SketchPlane` lying on a face of another element (the plane's
/// `GeomOnPlaneRef.m_geomRef.m_elemId`), and whose symbol (its
/// `InstanceInfo.m_symbolId`) cuts a solid rather than a loop, that element
/// and the solid's vertices (#824): the BIM4LCA office's four roof hatches,
/// on roof 2991742, each cutting a 600 mm square through it. Two passes:
/// the instances, then the planes and symbols they name.
pub fn face_hosted_cuts(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<Vec<HostedCut>> {
    if instances.is_empty() {
        return Ok(Vec::new());
    }
    let mut named: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: instances.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let plane = graph
            .objects
            .first()
            .and_then(|root| root.fields.get("m_hostId"))
            .and_then(element_id);
        let symbol = graph
            .objects
            .iter()
            .find(|object| object.class_name == "InstanceInfo")
            .and_then(|object| object.fields.get("m_symbolId"))
            .and_then(element_id);
        if let (Some(plane), Some(symbol)) = (plane, symbol) {
            named.insert(record.identity.element_id, (plane, symbol));
        }
        Ok(())
    })?;
    if named.is_empty() {
        return Ok(Vec::new());
    }
    let mut hosts: BTreeMap<u64, u64> = BTreeMap::new();
    let mut solids: BTreeMap<u64, Vec<[f64; 3]>> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: named
            .values()
            .flat_map(|(plane, symbol)| [*plane, *symbol])
            .collect(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match record.class_name.as_deref() {
            Some("SketchPlane") => {
                if let Some(host) = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "GeomOnPlaneRef")
                    .and_then(|object| object.fields.pointer("/m_geomRef/m_elemId"))
                    .and_then(element_id)
                {
                    hosts.insert(id, host);
                }
            }
            Some("FamilySymbol") => {
                if let Some(vertices) = cut_solid_vertices(graph) {
                    solids.insert(id, vertices);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(named
        .into_iter()
        .filter_map(|(instance, (plane, symbol))| {
            Some(HostedCut {
                instance,
                host: *hosts.get(&plane)?,
                vertices: solids.get(&symbol)?.clone(),
            })
        })
        .collect())
}
