//! Scene graph builder (VW1-05) — project → storey → element hierarchy.
//!
//! Produces a tree representation of an [`IfcModel`] suitable for
//! downstream viewers (WebGL, Three.js, plain JSON). The model
//! already carries the flat element list and the storey list; this
//! module nests them into a single [`SceneNode`] tree:
//!
//! ```text
//! SceneNode { ifc_type: "IFCPROJECT", name: "<project>", children: [
//!     SceneNode { ifc_type: "IFCBUILDINGSTOREY", name: "Ground Floor", children: [
//!         SceneNode { ifc_type: "IFCWALL", name: "Wall-1", children: [] },
//!         SceneNode { ifc_type: "IFCWALL", name: "Wall-2", children: [
//!             SceneNode { ifc_type: "IFCDOOR", name: "Front Door" },  // hosted
//!         ] },
//!     ] },
//!     SceneNode { ifc_type: "IFCBUILDINGSTOREY", name: "Second Floor", children: [...] },
//! ] }
//! ```
//!
//! Hosted elements (doors / windows whose `host_element_index`
//! points at a wall) are nested under their host rather than
//! alongside it — the tree matches how a 3D viewer should render
//! them visually (door "inside" wall).

use super::IfcModel;
use super::entities::IfcEntity;
use serde::{Deserialize, Serialize};

/// A single node in the rendered scene graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNode {
    /// Display name from the underlying entity (project name, storey
    /// name, element display name). Falls back to `ifc_type` when
    /// the source carries no name.
    pub name: String,
    /// IFC4 type string — `"IFCPROJECT"`, `"IFCBUILDINGSTOREY"`,
    /// `"IFCWALL"`, etc. Uppercase to match the STEP schema
    /// convention already used by [`IfcEntity`].
    pub ifc_type: String,
    /// Index into `model.entities` for `BuildingElement` leaf nodes.
    /// `None` for synthetic nodes (the root `IFCPROJECT` and each
    /// `IFCBUILDINGSTOREY`, which are derived from project metadata /
    /// storey list rather than an entity).
    pub entity_index: Option<usize>,
    /// Index into `model.building_storeys` for storey nodes. `None`
    /// elsewhere.
    pub storey_index: Option<usize>,
    /// Child nodes in render order.
    pub children: Vec<SceneNode>,
}

impl SceneNode {
    /// Count this node + every descendant. Handy for sanity-checks
    /// and for sizing viewer-side buffers.
    pub fn descendants_count(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(SceneNode::descendants_count)
            .sum::<usize>()
    }

    /// Find the first descendant whose name matches `name` exactly
    /// (depth-first, pre-order). Returns `None` when no match
    /// exists.
    pub fn find_by_name<'a>(&'a self, name: &str) -> Option<&'a SceneNode> {
        if self.name == name {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_name(name) {
                return Some(found);
            }
        }
        None
    }

    /// Flatten the tree to `(depth, &node)` pairs, pre-order. Viewers
    /// that render the scene as an indented list use this shape.
    pub fn flatten(&self) -> Vec<(usize, &SceneNode)> {
        let mut out = Vec::new();
        self.flatten_into(0, &mut out);
        out
    }

    fn flatten_into<'a>(&'a self, depth: usize, out: &mut Vec<(usize, &'a SceneNode)>) {
        out.push((depth, self));
        for child in &self.children {
            child.flatten_into(depth + 1, out);
        }
    }
}

/// Category-based visibility filter (VW1-09) for a scene graph.
/// Carries a hide-list of IFC type strings; any node whose
/// `ifc_type` matches (case-insensitive) is filtered out of the
/// tree, along with all of its descendants.
///
/// Viewers implement "layer toggles" by toggling IFC types in this
/// filter and re-rendering the returned tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryFilter {
    /// IFC types to hide. Matched case-insensitively against
    /// `SceneNode.ifc_type`. Use [`Self::hide`] to add a type,
    /// [`Self::show`] to remove.
    pub hidden: std::collections::BTreeSet<String>,
}

impl CategoryFilter {
    /// New empty filter — everything visible.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `ifc_type` as hidden. Case-insensitive — `"IFCWALL"` and
    /// `"ifcwall"` are equivalent.
    pub fn hide(&mut self, ifc_type: &str) {
        self.hidden.insert(ifc_type.to_ascii_uppercase());
    }

    /// Mark `ifc_type` as visible again (removes from hide-list).
    pub fn show(&mut self, ifc_type: &str) {
        self.hidden.remove(&ifc_type.to_ascii_uppercase());
    }

    /// `true` when `ifc_type` is currently hidden.
    pub fn is_hidden(&self, ifc_type: &str) -> bool {
        self.hidden.contains(&ifc_type.to_ascii_uppercase())
    }

    /// Apply this filter to a scene graph — returns a new
    /// `SceneNode` tree with all matching subtrees pruned. The
    /// input is borrowed, not modified. An empty filter returns a
    /// clone of the full tree.
    ///
    /// A node is pruned when its own `ifc_type` is in the hide-
    /// list; child pruning then runs on the surviving descendants.
    /// Pruning the root node returns a stub `SceneNode` with no
    /// children (the root itself is preserved so viewers always
    /// have something to bind to).
    pub fn apply(&self, root: &SceneNode) -> SceneNode {
        if self.hidden.is_empty() {
            return root.clone();
        }
        SceneNode {
            name: root.name.clone(),
            ifc_type: root.ifc_type.clone(),
            entity_index: root.entity_index,
            storey_index: root.storey_index,
            children: root
                .children
                .iter()
                .filter(|child| !self.is_hidden(&child.ifc_type))
                .map(|child| self.apply(child))
                .collect(),
        }
    }
}

/// Element info panel payload (VW1-08). The shape a viewer's
/// "click to inspect" UI reads — a single JSON-ready struct
/// describing an element's identity, location, and property set.
///
/// Populate via [`element_info_panel`] given a scene node's
/// `entity_index` and the underlying model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfoPanel {
    pub name: String,
    pub ifc_type: String,
    pub type_guid: Option<String>,
    /// Storey display name when the element is contained in one.
    pub storey_name: Option<String>,
    /// Storey elevation in feet (native Revit unit). `None` when
    /// the element hasn't been bound to a storey.
    pub storey_elevation_feet: Option<f64>,
    /// World-space location in feet, if the element has one.
    pub location_feet: Option<[f64; 3]>,
    /// Yaw rotation in radians (about +Z), if set.
    pub rotation_radians: Option<f64>,
    /// Material display name resolved through the model's
    /// material list. `None` when the element has no material
    /// associated.
    pub material_name: Option<String>,
    /// Flat `(property_name, formatted_value)` pairs from the
    /// element's `Pset_*Common` property set. Empty when no
    /// property set is attached.
    pub properties: Vec<(String, String)>,
}

/// Build an element-info panel payload (VW1-08) for the entity at
/// `entity_index` in `model`. Returns `None` when the index is
/// out of range or the entity at that index isn't a
/// `BuildingElement` (project / type-object entities have no
/// viewer-side info panel).
pub fn element_info_panel(model: &IfcModel, entity_index: usize) -> Option<ElementInfoPanel> {
    let ent = model.entities.get(entity_index)?;
    let IfcEntity::BuildingElement {
        ifc_type,
        name,
        type_guid,
        storey_index,
        material_index,
        property_set,
        location_feet,
        rotation_radians,
        ..
    } = ent
    else {
        return None;
    };
    let (storey_name, storey_elevation_feet) = match storey_index {
        Some(i) => model
            .building_storeys
            .get(*i)
            .map(|s| (Some(s.name.clone()), Some(s.elevation_feet)))
            .unwrap_or((None, None)),
        None => (None, None),
    };
    let material_name = material_index.and_then(|i| model.materials.get(i).map(|m| m.name.clone()));
    let properties: Vec<(String, String)> = property_set
        .as_ref()
        .map(|pset| {
            pset.properties
                .iter()
                .map(|p| (p.name.clone(), format_property_value(&p.value)))
                .collect()
        })
        .unwrap_or_default();
    Some(ElementInfoPanel {
        name: name.clone(),
        ifc_type: ifc_type.clone(),
        type_guid: type_guid.clone(),
        storey_name,
        storey_elevation_feet,
        location_feet: *location_feet,
        rotation_radians: *rotation_radians,
        material_name,
        properties,
    })
}

fn format_property_value(v: &super::entities::PropertyValue) -> String {
    use super::entities::PropertyValue;
    match v {
        PropertyValue::Text(s) => s.clone(),
        PropertyValue::Integer(i) => i.to_string(),
        PropertyValue::Real(r) => format!("{r:.3}"),
        PropertyValue::Boolean(b) => b.to_string(),
        PropertyValue::LengthFeet(f) => format!("{f:.3} ft"),
        PropertyValue::AngleRadians(r) => format!("{:.3}°", r.to_degrees()),
        PropertyValue::AreaSquareFeet(a) => format!("{a:.2} sqft"),
        PropertyValue::VolumeCubicFeet(c) => format!("{c:.2} cuft"),
        PropertyValue::CountValue(c) => c.to_string(),
        PropertyValue::TimeSeconds(t) => format!("{t:.1} s"),
        other => format!("{other:?}"),
    }
}

/// Collect all distinct `ifc_type` strings present in the scene
/// graph (VW1-09). Use as the source of truth for a viewer's
/// "layer" toggle UI.
pub fn distinct_ifc_types(root: &SceneNode) -> Vec<String> {
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    collect_types(root, &mut seen);
    seen.into_iter().collect()
}

fn collect_types(node: &SceneNode, out: &mut std::collections::BTreeSet<String>) {
    out.insert(node.ifc_type.clone());
    for child in &node.children {
        collect_types(child, out);
    }
}

/// Build a scene graph tree from an `IfcModel` (VW1-05). Walks the
/// flat `model.entities` list + `model.building_storeys` and nests
/// them into a three-level tree:
///
/// 1. Root `IFCPROJECT` (project name from model)
/// 2. One `IFCBUILDINGSTOREY` per `model.building_storeys` entry,
///    plus a synthetic `"Unassigned"` storey for entities whose
///    `storey_index` is `None` (rare — occurs when elements haven't
///    been resolved to a level yet).
/// 3. `BuildingElement` leaves, grouped by storey and further
///    nested by `host_element_index` (so doors/windows land under
///    their wall).
///
/// Non-BuildingElement entities (Project / BuildingElementType /
/// TypeObject) are skipped — they're metadata, not render-surface.
pub fn build_scene_graph(model: &IfcModel) -> SceneNode {
    // Root project node.
    let project_name = model
        .project_name
        .clone()
        .unwrap_or_else(|| "Project".into());

    // Map: storey_index -> Vec<entity_index>, plus a bucket for
    // "unassigned" entities.
    let mut per_storey: Vec<Vec<usize>> = vec![Vec::new(); model.building_storeys.len()];
    let mut unassigned: Vec<usize> = Vec::new();
    // Map from entity_index -> Vec<child entity_index> for
    // host_element_index relationships.
    let mut hosted_children: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    // Which entities are already hosted (so we don't emit them as
    // top-level storey children).
    let mut hosted_set: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();

    for (idx, ent) in model.entities.iter().enumerate() {
        if let IfcEntity::BuildingElement {
            storey_index,
            host_element_index,
            ..
        } = ent
        {
            if let Some(host) = host_element_index {
                hosted_children.entry(*host).or_default().push(idx);
                hosted_set.insert(idx);
                continue;
            }
            match storey_index {
                Some(si) if *si < per_storey.len() => per_storey[*si].push(idx),
                _ => unassigned.push(idx),
            }
        }
    }

    let mut storey_nodes: Vec<SceneNode> = Vec::with_capacity(model.building_storeys.len() + 1);
    for (si, storey) in model.building_storeys.iter().enumerate() {
        let mut children: Vec<SceneNode> = Vec::with_capacity(per_storey[si].len());
        for ent_idx in &per_storey[si] {
            children.push(build_element_node(model, *ent_idx, &hosted_children));
        }
        storey_nodes.push(SceneNode {
            name: storey.name.clone(),
            ifc_type: "IFCBUILDINGSTOREY".into(),
            entity_index: None,
            storey_index: Some(si),
            children,
        });
    }
    if !unassigned.is_empty() {
        let children: Vec<SceneNode> = unassigned
            .iter()
            .map(|i| build_element_node(model, *i, &hosted_children))
            .collect();
        storey_nodes.push(SceneNode {
            name: "Unassigned".into(),
            ifc_type: "IFCBUILDINGSTOREY".into(),
            entity_index: None,
            storey_index: None,
            children,
        });
    }

    SceneNode {
        name: project_name,
        ifc_type: "IFCPROJECT".into(),
        entity_index: None,
        storey_index: None,
        children: storey_nodes,
    }
}

fn build_element_node(
    model: &IfcModel,
    entity_idx: usize,
    hosted_children: &std::collections::BTreeMap<usize, Vec<usize>>,
) -> SceneNode {
    let (name, ifc_type) = match model.entities.get(entity_idx) {
        Some(IfcEntity::BuildingElement { name, ifc_type, .. }) => (name.clone(), ifc_type.clone()),
        _ => ("?".into(), "IFCBUILDINGELEMENTPROXY".into()),
    };
    let children: Vec<SceneNode> = hosted_children
        .get(&entity_idx)
        .into_iter()
        .flat_map(|v| v.iter())
        .map(|child_idx| build_element_node(model, *child_idx, hosted_children))
        .collect();
    SceneNode {
        name,
        ifc_type,
        entity_index: Some(entity_idx),
        storey_index: None,
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::super::Storey;
    use super::super::entities::IfcEntity;
    use super::*;

    fn mk_element(
        name: &str,
        ifc_type: &str,
        storey: Option<usize>,
        host: Option<usize>,
    ) -> IfcEntity {
        IfcEntity::BuildingElement {
            ifc_type: ifc_type.into(),
            name: name.into(),
            type_guid: None,
            storey_index: storey,
            material_index: None,
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: host,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }
    }

    #[test]
    fn scene_graph_root_is_project() {
        let model = IfcModel {
            project_name: Some("Test Project".into()),
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        assert_eq!(scene.name, "Test Project");
        assert_eq!(scene.ifc_type, "IFCPROJECT");
        assert!(scene.entity_index.is_none());
    }

    #[test]
    fn scene_graph_falls_back_to_default_project_name() {
        let model = IfcModel::default();
        let scene = build_scene_graph(&model);
        assert_eq!(scene.name, "Project");
    }

    #[test]
    fn scene_graph_one_storey_two_walls() {
        let model = IfcModel {
            project_name: Some("Single Storey".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall-1", "IFCWALL", Some(0), None),
                mk_element("Wall-2", "IFCWALL", Some(0), None),
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        assert_eq!(scene.children.len(), 1); // one storey
        let storey = &scene.children[0];
        assert_eq!(storey.name, "Ground");
        assert_eq!(storey.children.len(), 2);
        assert_eq!(storey.children[0].name, "Wall-1");
        assert_eq!(storey.children[1].name, "Wall-2");
    }

    #[test]
    fn scene_graph_nests_hosted_elements() {
        let model = IfcModel {
            project_name: Some("Hosted".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall-1", "IFCWALL", Some(0), None), // idx 0
                mk_element("Front Door", "IFCDOOR", Some(0), Some(0)), // hosted by Wall-1
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        let storey = &scene.children[0];
        // Only Wall-1 appears at top level; Door is nested.
        assert_eq!(storey.children.len(), 1);
        assert_eq!(storey.children[0].name, "Wall-1");
        assert_eq!(storey.children[0].children.len(), 1);
        assert_eq!(storey.children[0].children[0].name, "Front Door");
        assert_eq!(storey.children[0].children[0].ifc_type, "IFCDOOR");
    }

    #[test]
    fn scene_graph_unassigned_elements_go_to_synthetic_storey() {
        let model = IfcModel {
            project_name: Some("Unassigned".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall-1", "IFCWALL", Some(0), None),
                mk_element("Floating Column", "IFCCOLUMN", None, None),
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        assert_eq!(scene.children.len(), 2); // real storey + "Unassigned"
        let unassigned = &scene.children[1];
        assert_eq!(unassigned.name, "Unassigned");
        assert_eq!(unassigned.children.len(), 1);
        assert_eq!(unassigned.children[0].name, "Floating Column");
    }

    #[test]
    fn descendants_count_includes_root() {
        // 1 project + 1 storey + 1 wall + 1 door = 4
        let model = IfcModel {
            project_name: Some("Count".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall-1", "IFCWALL", Some(0), None),
                mk_element("Door-1", "IFCDOOR", Some(0), Some(0)),
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        assert_eq!(scene.descendants_count(), 4);
    }

    #[test]
    fn find_by_name_locates_nested_door() {
        let model = IfcModel {
            project_name: Some("Find".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall-1", "IFCWALL", Some(0), None),
                mk_element("Target Door", "IFCDOOR", Some(0), Some(0)),
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        let found = scene.find_by_name("Target Door").unwrap();
        assert_eq!(found.ifc_type, "IFCDOOR");
    }

    #[test]
    fn find_by_name_returns_none_on_miss() {
        let model = IfcModel::default();
        let scene = build_scene_graph(&model);
        assert!(scene.find_by_name("nonexistent").is_none());
    }

    #[test]
    fn flatten_yields_depth_annotated_preorder() {
        let model = IfcModel {
            project_name: Some("Flatten".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall-1", "IFCWALL", Some(0), None),
                mk_element("Door-1", "IFCDOOR", Some(0), Some(0)),
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        let flat = scene.flatten();
        // Expect: (0, Project), (1, Storey), (2, Wall), (3, Door)
        assert_eq!(flat.len(), 4);
        assert_eq!(flat[0].0, 0);
        assert_eq!(flat[0].1.ifc_type, "IFCPROJECT");
        assert_eq!(flat[1].0, 1);
        assert_eq!(flat[2].0, 2);
        assert_eq!(flat[3].0, 3);
        assert_eq!(flat[3].1.name, "Door-1");
    }

    #[test]
    fn scene_graph_two_storeys_split_elements() {
        let model = IfcModel {
            project_name: Some("Two Storey".into()),
            building_storeys: vec![
                Storey {
                    name: "Ground".into(),
                    elevation_feet: 0.0,
                },
                Storey {
                    name: "Second".into(),
                    elevation_feet: 10.0,
                },
            ],
            entities: vec![
                mk_element("W-Ground", "IFCWALL", Some(0), None),
                mk_element("W-Second", "IFCWALL", Some(1), None),
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        assert_eq!(scene.children.len(), 2);
        assert_eq!(scene.children[0].children[0].name, "W-Ground");
        assert_eq!(scene.children[1].children[0].name, "W-Second");
    }

    // ---- VW1-09: CategoryFilter tests ----

    fn sample_scene() -> SceneNode {
        let model = IfcModel {
            project_name: Some("Scene".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall-1", "IFCWALL", Some(0), None),
                mk_element("Door-1", "IFCDOOR", Some(0), Some(0)),
                mk_element("Slab-1", "IFCSLAB", Some(0), None),
                mk_element("Column-1", "IFCCOLUMN", Some(0), None),
            ],
            ..Default::default()
        };
        build_scene_graph(&model)
    }

    #[test]
    fn category_filter_empty_returns_clone() {
        let scene = sample_scene();
        let filter = CategoryFilter::new();
        let filtered = filter.apply(&scene);
        assert_eq!(filtered, scene);
    }

    #[test]
    fn category_filter_hides_matching_ifc_type() {
        let scene = sample_scene();
        let mut filter = CategoryFilter::new();
        filter.hide("IFCWALL");
        let filtered = filter.apply(&scene);
        // Wall-1 (with hosted Door-1) is pruned. Slab + Column survive.
        let storey = &filtered.children[0];
        let names: Vec<&str> = storey.children.iter().map(|n| n.name.as_str()).collect();
        assert!(!names.contains(&"Wall-1"));
        assert!(!names.contains(&"Door-1")); // Door was hosted — gone with wall.
        assert!(names.contains(&"Slab-1"));
        assert!(names.contains(&"Column-1"));
    }

    #[test]
    fn category_filter_is_case_insensitive() {
        let scene = sample_scene();
        let mut filter = CategoryFilter::new();
        filter.hide("ifcslab");
        let filtered = filter.apply(&scene);
        let storey = &filtered.children[0];
        let names: Vec<&str> = storey.children.iter().map(|n| n.name.as_str()).collect();
        assert!(!names.contains(&"Slab-1"));
    }

    #[test]
    fn category_filter_show_removes_from_hide_list() {
        let mut filter = CategoryFilter::new();
        filter.hide("IFCWALL");
        assert!(filter.is_hidden("IFCWALL"));
        filter.show("IFCWALL");
        assert!(!filter.is_hidden("IFCWALL"));
    }

    #[test]
    fn category_filter_hides_multiple_types() {
        let scene = sample_scene();
        let mut filter = CategoryFilter::new();
        filter.hide("IFCWALL");
        filter.hide("IFCSLAB");
        let filtered = filter.apply(&scene);
        let storey = &filtered.children[0];
        let names: Vec<&str> = storey.children.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["Column-1"]);
    }

    #[test]
    fn distinct_ifc_types_enumerates_tree() {
        let scene = sample_scene();
        let types = distinct_ifc_types(&scene);
        assert!(types.contains(&"IFCPROJECT".to_string()));
        assert!(types.contains(&"IFCBUILDINGSTOREY".to_string()));
        assert!(types.contains(&"IFCWALL".to_string()));
        assert!(types.contains(&"IFCDOOR".to_string()));
        assert!(types.contains(&"IFCSLAB".to_string()));
        assert!(types.contains(&"IFCCOLUMN".to_string()));
    }

    #[test]
    fn distinct_ifc_types_dedupes() {
        let scene = sample_scene();
        let types = distinct_ifc_types(&scene);
        // All 2 walls would have been "IFCWALL" but our fixture only
        // has 1. Dedup still reports 1 entry per unique type.
        let wall_count = types.iter().filter(|t| *t == "IFCWALL").count();
        assert_eq!(wall_count, 1);
    }

    #[test]
    fn category_filter_serializable() {
        let mut filter = CategoryFilter::new();
        filter.hide("IFCWALL");
        filter.hide("IFCCOLUMN");
        let json = serde_json::to_string(&filter).unwrap();
        let back: CategoryFilter = serde_json::from_str(&json).unwrap();
        assert!(back.is_hidden("IFCWALL"));
        assert!(back.is_hidden("IFCCOLUMN"));
        assert!(!back.is_hidden("IFCSLAB"));
    }

    // ---- VW1-08: element info panel tests ----

    #[test]
    fn info_panel_returns_none_for_out_of_range_index() {
        let model = IfcModel::default();
        assert!(element_info_panel(&model, 0).is_none());
    }

    #[test]
    fn info_panel_returns_none_for_non_building_entity() {
        let model = IfcModel {
            entities: vec![IfcEntity::Project {
                name: Some("P".into()),
                description: None,
                long_name: None,
            }],
            ..Default::default()
        };
        assert!(element_info_panel(&model, 0).is_none());
    }

    #[test]
    fn info_panel_surfaces_name_and_type() {
        let model = IfcModel {
            entities: vec![mk_element("Wall-1", "IFCWALL", Some(0), None)],
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        assert_eq!(panel.name, "Wall-1");
        assert_eq!(panel.ifc_type, "IFCWALL");
        assert_eq!(panel.storey_name.as_deref(), Some("Ground"));
        assert_eq!(panel.storey_elevation_feet, Some(0.0));
    }

    #[test]
    fn info_panel_resolves_material_through_model_list() {
        let model = IfcModel {
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IFCWALL".into(),
                name: "Wall".into(),
                type_guid: None,
                storey_index: None,
                material_index: Some(0),
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            }],
            materials: vec![super::super::MaterialInfo {
                name: "Concrete".into(),
                color_packed: None,
                transparency: None,
            }],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        assert_eq!(panel.material_name.as_deref(), Some("Concrete"));
    }

    #[test]
    fn info_panel_formats_property_values() {
        use super::super::entities::{Property, PropertySet, PropertyValue};
        let pset = PropertySet {
            name: "Pset_WallCommon".into(),
            properties: vec![
                Property {
                    name: "Height".into(),
                    value: PropertyValue::LengthFeet(10.5),
                },
                Property {
                    name: "IsExternal".into(),
                    value: PropertyValue::Boolean(true),
                },
            ],
        };
        let model = IfcModel {
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IFCWALL".into(),
                name: "Wall".into(),
                type_guid: None,
                storey_index: None,
                material_index: None,
                property_set: Some(pset),
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            }],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        assert_eq!(panel.properties.len(), 2);
        let height = panel
            .properties
            .iter()
            .find(|(n, _)| n == "Height")
            .unwrap();
        assert!(height.1.contains("10.500 ft"));
        let ext = panel
            .properties
            .iter()
            .find(|(n, _)| n == "IsExternal")
            .unwrap();
        assert_eq!(ext.1, "true");
    }

    #[test]
    fn info_panel_is_serde_roundtrippable() {
        let model = IfcModel {
            entities: vec![mk_element("W", "IFCWALL", None, None)],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        let json = serde_json::to_string(&panel).unwrap();
        let back: ElementInfoPanel = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "W");
    }

    #[test]
    fn scene_graph_skips_non_building_entities() {
        // Project / BuildingElementType / TypeObject should not
        // appear in the render tree.
        let model = IfcModel {
            project_name: Some("Skip".into()),
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("Wall", "IFCWALL", Some(0), None),
                IfcEntity::Project {
                    name: Some("Inner Project".into()),
                    description: None,
                    long_name: None,
                },
            ],
            ..Default::default()
        };
        let scene = build_scene_graph(&model);
        // Only the wall appears — Project is metadata, not render-surface.
        assert_eq!(scene.children[0].children.len(), 1);
        assert_eq!(scene.children[0].children[0].name, "Wall");
    }
}
