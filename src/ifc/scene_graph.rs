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
