//! Bounded traversal of saved graphics under the default
//! 3D context. This selects surface carriers, not regenerated Revit geometry.
//! Unsupported solid-affecting filters and external instances remain explicit.
use crate::native_parameters::ObjectGraph;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Row-major affine matrix, operating on column vectors in saved length units.
pub type GraphicsTransform = [[f64; 4]; 4];
const IDENTITY: GraphicsTransform = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0., 0., 0., 1.],
];
#[derive(Debug, Clone, Serialize)]
pub struct SelectedGraphics {
    pub object_index: usize,
    /// None denotes the supplied root graph; Some identifies a resolved symbol graph.
    pub source_owner_id: Option<u64>,
    pub world_transform: GraphicsTransform,
    pub path: Vec<usize>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TraversalDiagnostic {
    pub object_index: usize,
    pub message: String,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct GraphicsSelection {
    pub selected: Vec<SelectedGraphics>,
    pub diagnostics: Vec<TraversalDiagnostic>,
    pub excluded_non_surface_branches: usize,
    pub rejected_filters: usize,
    pub excluded_visibility_branches: usize,
    /// Observed converter-profile behavior, not universal Revit semantics.
    pub profile_observations: Vec<String>,
}
/// Select reachable Geometry/GPolyMesh carriers from object zero. Pointer edges
/// are validated against their serialized source offset and token; token-only
/// lookup is insufficient for deferred anonymous objects and backreferences.
pub fn select_graphics(graph: &ObjectGraph) -> GraphicsSelection {
    select_graphics_with_resolver(graph, &|_| None)
}
/// Resolve external symbol graphics without merging/rebasing graph-local tokens.
pub fn select_graphics_with_resolver<'a>(
    graph: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
) -> GraphicsSelection {
    select_graphics_with_resolver_at_detail(graph, resolver, 3)
}
/// Select a saved 3D representation at an explicit native detail level1..3.
/// FamilyComputeHelper's area getter requests1; default GLB export requests3.
pub fn select_graphics_with_resolver_at_detail<'a>(
    graph: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
    detail_level: i64,
) -> GraphicsSelection {
    let mut walk = Walker {
        graph,
        edge_indexes: HashMap::from([(graph as *const ObjectGraph as usize, edge_index(graph))]),
        resolver,
        detail_level,
        source_owner_id: None,
        ancestry: Vec::new(),
        result: GraphicsSelection::default(),
        visits: 0,
    };
    if !(1..=3).contains(&detail_level) {
        walk.fail(0, "detail level must be1,2,or3");
    } else if graph.objects.is_empty() {
        walk.fail(0, "empty graphics graph");
    } else {
        walk.visit(0, IDENTITY, &mut Vec::new());
    }
    walk.result
}
type EdgeIndex = HashMap<(usize, usize, u64), Option<usize>>;
fn edge_index(graph: &ObjectGraph) -> EdgeIndex {
    let mut index = HashMap::new();
    for (i, edge) in graph.edges.iter().enumerate() {
        let key = (
            edge.source_object_index,
            edge.pointer_offset,
            u64::from(edge.pointer_token),
        );
        index
            .entry(key)
            .and_modify(|v| *v = None)
            .or_insert(Some(i));
    }
    index
}
struct Walker<'a, 'r> {
    edge_indexes: HashMap<usize, EdgeIndex>,
    graph: &'a ObjectGraph,
    resolver: &'r dyn Fn(u64) -> Option<&'a ObjectGraph>,
    source_owner_id: Option<u64>,
    detail_level: i64,
    ancestry: Vec<(Option<u64>, usize)>,
    result: GraphicsSelection,
    visits: usize,
}
impl Walker<'_, '_> {
    fn fail(&mut self, index: usize, message: &str) {
        self.result.diagnostics.push(TraversalDiagnostic {
            object_index: index,
            message: message.into(),
        });
    }
    fn target(&self, source: usize, p: &Value) -> Result<usize, String> {
        let offset = p["offset"].as_u64().ok_or("missing pointer offset")? as usize;
        let token = p["pointer_token"].as_u64().ok_or("missing pointer token")?;
        let edge_slot = self.edge_indexes[&(self.graph as *const ObjectGraph as usize)]
            .get(&(source, offset, token))
            .ok_or("unresolved null/external graphics pointer")?;
        let edge_index = edge_slot.ok_or("ambiguous graphics edge")?;
        let e = &self.graph.edges[edge_index];
        let target = self
            .graph
            .objects
            .get(e.target_object_index)
            .ok_or("edge target out of bounds")?;
        if e.target_class_tag != target.class_tag
            || p["class_tag"]
                .as_u64()
                .is_some_and(|tag| tag != u64::from(target.class_tag))
        {
            return Err("graphics edge class mismatch".into());
        }
        Ok(e.target_object_index)
    }
    fn children(&self, index: usize) -> Result<Vec<usize>, String> {
        self.graph.objects[index].fields["m_subNodes"]
            .as_array()
            .ok_or("missing graphics children")?
            .iter()
            .map(|p| self.target(index, p))
            .collect()
    }
    fn visit(&mut self, index: usize, matrix: GraphicsTransform, path: &mut Vec<usize>) {
        self.visits += 1;
        if self.visits > 100_000 || path.len() >= 128 {
            self.fail(index, "graphics traversal resource limit");
            return;
        }
        if self.ancestry.contains(&(self.source_owner_id, index)) {
            self.fail(index, "graphics reference cycle");
            return;
        }
        path.push(index);
        self.ancestry.push((self.source_owner_id, index));
        let result = self.visit_inner(index, matrix, path);
        if let Err(message) = result {
            self.fail(index, &message)
        }
        path.pop();
        self.ancestry.pop();
    }
    fn visit_inner(
        &mut self,
        index: usize,
        matrix: GraphicsTransform,
        path: &mut Vec<usize>,
    ) -> Result<(), String> {
        let obj = &self.graph.objects[index];
        // GGroup native child traversal admits visibility0/2;1/3 are skipped
        // outside special draw mode7. The default export uses normal mode.
        if let Some(flags) = obj.fields["m_GInfo"]["m_flags"].as_u64() {
            if matches!(graphics_visibility(flags), 1 | 3) {
                self.result.excluded_visibility_branches += 1;
                return Ok(());
            }
        }
        match obj.class_name.as_str() {
            "Geometry" | "GPolyMesh" => self.result.selected.push(SelectedGraphics {
                object_index: index,
                source_owner_id: self.source_owner_id,
                world_transform: matrix,
                path: path.clone(),
            }),
            "GElement" | "GGroup" | "GFilter" => {
                let children = self.children(index)?;
                if obj.class_name == "GFilter" {
                    if children.is_empty() {
                        return Ok(());
                    }
                    // Curves are deliberately outside this surface-only selector.
                    if children
                        .iter()
                        .all(|&i| self.only_curves(i, &mut HashSet::new()))
                    {
                        self.result.excluded_non_surface_branches += 1;
                        return Ok(());
                    }
                    if obj.fields["m_bIsNestedDetailFamily"].as_bool() == Some(true) {
                        return Err("nested detail-family filter requires view semantics".into());
                    }
                    let conditions = obj.fields["m_oConditions"]
                        .as_array()
                        .ok_or("missing filter conditions")?;
                    let mut unknown = None;
                    for p in conditions {
                        let ci = self.target(index, p)?;
                        let field = &self.graph.objects[ci].fields;
                        if self.graph.objects[ci].class_name == "GConditionInt"
                            && field["m_param"].as_i64() == Some(7)
                            && field["m_value"].as_i64() == Some(2)
                            && field["m_comp"].as_i64() == Some(0)
                        {
                            let observation="integer parameter7/value2/comparison0 accepted in native runtime; semantic meaning unqualified".to_string();
                            if !self.result.profile_observations.contains(&observation) {
                                self.result.profile_observations.push(observation);
                            }
                        }

                        match if matches!(
                            self.graph.objects[ci].class_name.as_str(),
                            "GConditionDir" | "GConditionCut"
                        ) {
                            direction_condition(&self.graph.objects[ci].fields, matrix)
                        } else {
                            condition_at_detail(
                                &self.graph.objects[ci].class_name,
                                &self.graph.objects[ci].fields,
                                self.detail_level,
                            )
                        } {
                            Ok(false) => {
                                self.result.rejected_filters += 1;
                                return Ok(());
                            }
                            Ok(true) => {}
                            Err(e) => unknown = Some(e),
                        }
                    }
                    if let Some(e) = unknown {
                        return Err(e);
                    }
                }
                for child in children {
                    self.visit(child, matrix, path)
                }
            }
            "GComponentRef" => {
                // Component visibility depends on serialized symbol and view context.
                // Curve-only references cannot contribute to this surface export,
                // independently of that still-unqualified owner/view predicate.
                let info = self.target(index, &obj.fields["m_instanceInfo"])?;
                let info_object = &self.graph.objects[info];
                if info_object.class_name != "InstanceInfo" {
                    return Err("unsupported component instance-info class".into());
                }
                let fields = &info_object.fields;
                let _transform = read_transform(&fields["m_Trf"])?;
                if fields["m_GRepId"].as_u64() != Some(0)
                    || fields["m_cda"]["m_pDoc"]["pointer_token"].as_u64() != Some(1)
                    || !fields["m_cda"]["m_pDoc"]["class_tag"].is_null()
                {
                    return Err("unsupported component representation/document context".into());
                }
                let symbol = crate::native_metadata::identifier(&fields["m_symbolId"])
                    .ok()
                    .and_then(|id| u64::try_from(id).ok())
                    .filter(|id| *id > 0)
                    .ok_or("invalid component owner ID")?;
                let target = (self.resolver)(symbol).ok_or("missing current component graphics")?;
                if !target
                    .objects
                    .first()
                    .is_some_and(|o| current_graph_owner_matches(o, symbol))
                {
                    return Err("resolved component graphics owner mismatch".into());
                }
                let previous = self.graph;
                self.edge_indexes
                    .entry(target as *const ObjectGraph as usize)
                    .or_insert_with(|| edge_index(target));
                self.graph = target;
                let curves_only = self.only_curves(0, &mut HashSet::new());
                self.graph = previous;
                if curves_only {
                    self.result.excluded_non_surface_branches += 1;
                } else {
                    return Err(
                        "surface-bearing component requires qualified owner/view visibility".into(),
                    );
                }
            }
            "GInstance" => {
                if obj.fields["m_bHasScale"].as_bool() != Some(false) {
                    return Err("scaled instance mode is not qualified".into());
                }
                if obj.fields["m_resolveSymInView"].as_bool() != Some(false) {
                    return Err("view-resolved instance is unsupported".into());
                }
                if obj.fields["m_forbiddenTarget"]["m_targets"].as_u64() != Some(0) {
                    return Err("instance target restrictions are unsupported".into());
                }
                let info = self.target(index, &obj.fields["m_instanceInfo"])?;
                if self.graph.objects[info].class_name != "InstanceInfo" {
                    return Err("unsupported instance-info class".into());
                }
                let transform = read_transform(&self.graph.objects[info].fields["m_Trf"])?;
                let embedded_pointer = &obj.fields["m_oEmbeddedSymbolGRep"];
                if embedded_pointer["pointer_token"].as_u64() == Some(0) {
                    let info_fields = &self.graph.objects[info].fields;
                    if info_fields["m_GRepId"].as_u64() != Some(0)
                        || info_fields["m_cda"]["m_pDoc"]["pointer_token"].as_u64() != Some(1)
                        || !info_fields["m_cda"]["m_pDoc"]["class_tag"].is_null()
                    {
                        return Err(
                            "external symbol has unsupported representation/document context"
                                .into(),
                        );
                    }
                    let symbol = crate::native_metadata::identifier(&info_fields["m_symbolId"])
                        .ok()
                        .and_then(|id| u64::try_from(id).ok())
                        .filter(|id| *id > 0)
                        .ok_or("invalid external symbol id")?;
                    let target = (self.resolver)(symbol)
                        .ok_or_else(|| format!("missing current symbol graphics {symbol}"))?;
                    let root = target
                        .objects
                        .first()
                        .ok_or("empty current symbol graphics")?;
                    if !current_graph_owner_matches(root, symbol) {
                        return Err("resolved symbol graphics owner mismatch".into());
                    }
                    let previous_graph = self.graph;
                    let previous_owner = self.source_owner_id;
                    self.edge_indexes
                        .entry(target as *const ObjectGraph as usize)
                        .or_insert_with(|| edge_index(target));
                    self.graph = target;
                    self.source_owner_id = Some(symbol);
                    self.visit(0, multiply(matrix, transform), path);
                    self.graph = previous_graph;
                    self.source_owner_id = previous_owner;
                } else {
                    let embedded = self.target(index, embedded_pointer)?;
                    self.visit(embedded, multiply(matrix, transform), path);
                }
            }
            "GLine" | "GArc" | "GEllipse" | "GCurve" | "GNurbSpline" | "GPolyLine"
            | "GPolyline" | "GPoint" => self.result.excluded_non_surface_branches += 1,
            other => return Err(format!("unsupported reachable graphics class {other}")),
        }
        Ok(())
    }
    fn only_curves(&self, index: usize, seen: &mut HashSet<usize>) -> bool {
        if seen.len() >= 128 || !seen.insert(index) {
            return false;
        }
        let result = match self.graph.objects[index].class_name.as_str() {
            "GLine" | "GArc" | "GEllipse" | "GCurve" | "GNurbSpline" | "GPolyLine"
            | "GPolyline" | "GPoint" => true,
            "GElement" | "GGroup" | "GFilter" => self
                .children(index)
                .is_ok_and(|c| c.iter().all(|&i| self.only_curves(i, seen))),
            _ => false,
        };
        seen.remove(&index);
        result
    }
}
// The 2023 GElement schema carries its owning element ID in GInfo.tag;
// 2024+ additionally serializes m_elementId. A present dedicated field must
// agree; never silently fall back from a malformed or mismatched dedicated ID.
fn current_graph_owner_matches(root: &crate::native_parameters::GraphObject, id: u64) -> bool {
    root.class_name == "GElement"
        && match root.fields.get("m_elementId") {
            Some(value) => value.as_u64() == Some(id),
            None => root.fields["m_GInfo"]["m_tag"].as_u64() == Some(id),
        }
}
fn graphics_visibility(flags: u64) -> u8 {
    let bits = flags & 0x1e0;
    if bits == 0 {
        0
    } else if bits == 0xe0 {
        3
    } else if bits < 0xe0 {
        if flags & 0x160 == 0x20 { 1 } else { 2 }
    } else if flags & 0x1a0 == 0x100 {
        1
    } else {
        2
    }
}
fn read_transform(v: &Value) -> Result<GraphicsTransform, String> {
    let rows = v["m_3x3"]
        .as_array()
        .filter(|v| v.len() == 3)
        .ok_or("missing instance matrix")?;
    let origin = v["m_or"]
        .as_array()
        .filter(|v| v.len() == 3)
        .ok_or("missing instance translation")?;
    let mut m = IDENTITY;
    for i in 0..3 {
        let row = rows[i]
            .as_array()
            .filter(|v| v.len() == 3)
            .ok_or("invalid instance matrix row")?;
        for j in 0..3 {
            m[i][j] = row[j]
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or("invalid instance coefficient")?;
        }
        m[i][3] = origin[i]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or("invalid instance translation")?;
    }
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-12 {
        return Err("singular instance transform".into());
    }
    Ok(m)
}
fn multiply(a: GraphicsTransform, b: GraphicsTransform) -> GraphicsTransform {
    let mut result = [[0.; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    result
}
// Native default context is view type4. Direction predicates6/7/8 are
// false before negation in that context;10/11 use pi/2 +/-1e-10 thresholds.
fn direction_condition(v: &Value, matrix: GraphicsTransform) -> Result<bool, String> {
    let negate = v["m_negateDirCondition"]
        .as_bool()
        .ok_or("missing direction negation")?;
    let comparison = v["m_comp"].as_i64().ok_or("missing direction comparison")?;
    if matches!(comparison, 6..=8) {
        return Ok(negate);
    }
    let a = v["m_dir"]
        .as_array()
        .filter(|a| a.len() == 3)
        .ok_or("missing filter direction")?;
    let mut local = [0.; 3];
    for i in 0..3 {
        local[i] = a[i]
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or("invalid filter direction")?;
    }
    let mut world = [0.; 3];
    for i in 0..3 {
        world[i] = (0..3).map(|j| matrix[i][j] * local[j]).sum();
    }
    let length = world.iter().map(|v| v * v).sum::<f64>().sqrt();
    if length < 1e-12 {
        return Err("zero filter direction".into());
    }
    let actual = [-0.577350269189625, -0.577350269189627, 0.577350269189626];
    let answer = match comparison {
        10 | 11 => {
            let cosine = (0..3).map(|i| world[i] * actual[i]).sum::<f64>() / length;
            let angle = cosine.clamp(-1., 1.).acos();
            if comparison == 10 {
                angle < 1.5707963266948965
            } else {
                angle > 1.5707963268948966
            }
        }
        -1 | 0 | 3 => {
            let distance = (0..3)
                .map(|i| (world[i] - actual[i]).powi(2))
                .sum::<f64>()
                .sqrt();
            let equal = if distance < 1e-14 {
                true
            } else if distance > 1e-6 {
                false
            } else {
                return Err("direction equality inside unqualified native tolerance band".into());
            };
            if comparison == 0 { !equal } else { equal }
        }
        _ => return Err(format!("unsupported direction comparison {comparison}")),
    };
    Ok(answer != negate)
}
fn condition_at_detail(class: &str, v: &Value, detail_level: i64) -> Result<bool, String> {
    if !matches!(
        class,
        "GConditionInt"
            | "GConditionViewType"
            | "GConditionViewDetailLevel"
            | "GConditionVModeType"
    ) {
        return Err(format!("unsupported solid-affecting condition {class}"));
    }
    let kind = v["m_param"]
        .as_i64()
        .ok_or("missing integer condition parameter type")?;
    let saved = v["m_value"]
        .as_i64()
        .ok_or("missing integer condition value")?;
    if kind == 7 && saved == 2 && v["m_comp"].as_i64() == Some(0) {
        return Ok(true);
    }
    // The default geometry dispatch maps parameter type1 to the comparison
    // value 4. Runtime observations record this context.
    let actual = match kind {
        1 => 4,
        3 => detail_level,
        6 => 0,
        _ => {
            return Err(format!(
                "unsupported integer condition parameter type {kind}"
            ));
        }
    };
    match v["m_comp"].as_i64() {
        Some(-1 | 3) => Ok(saved == actual),
        Some(0) => Ok(saved != actual),
        Some(1) => Ok(saved > actual),
        Some(2) => Ok(saved >= actual),
        Some(4) => Ok(saved <= actual),
        Some(5) => Ok(saved < actual),
        _ => Err("unsupported integer condition comparison".into()),
    }
}
