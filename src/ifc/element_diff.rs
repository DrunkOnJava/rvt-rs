//! What changed between two exports of a model, element by element (#1843).
//!
//! Each element the IFC export writes is keyed by the GlobalId it writes, and
//! two exports (two saves of one project, read by the same build) are compared
//! key by key. An element is added, removed, changed in one of the fields of
//! [`ElementState`], or the same. `rvt-diff --elements` prints it.
//!
//! An element whose GlobalId an export writes for more than one element is
//! left out of the comparison, since two elements under one key cannot be told
//! apart, and is reported: [`ElementDiff::duplicate_global_ids`] lists the
//! GlobalIds and [`ElementDiff::set_aside`] counts the elements.
//!
//! The fields are what the export writes, not the Revit file's records:
//! - the element's class, name, storey, type, predefined type and material;
//! - where its body is placed, and a digest of the body itself;
//! - each property the export writes, by set and name.
//!
//! A change rvt-rs does not read is not seen: a parameter it leaves out, or a
//! body it draws as its record's box. A change in how rvt-rs reads a file
//! shows as a change in the file, so compare two saves with one build.
//!
//! Some properties of rvt-rs's own sets say how a value was read, not what it
//! is ([`LABEL_PROPERTIES`], and `BodySource` where the body is equal). A
//! difference in one of them alone leaves the element unchanged, and the
//! element is listed in [`ElementDiff::relabelled`], so the drift stays
//! visible without counting as a change in the model (#1864): two saves of
//! `rst_basic` bind the same Level to 12 beams by two rules.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;
use serde_json::Value;

use super::IfcModel;
use super::entities::{IfcEntity, PropertySet};
use super::step_writer::{Placement, element_global_ids, placements, storey_of, type_objects};

/// One element as an export writes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ElementState {
    /// The GlobalId the IFC export writes.
    pub global_id: String,
    /// Its `Tag`, the Revit ElementId.
    pub tag: Option<String>,
    /// The IFC entity, such as `IFCBEAM`.
    pub ifc_type: String,
    /// The element's name as the model holds it.
    pub name: String,
    /// The name of the storey the export puts it on.
    pub storey: Option<String>,
    /// The name of the type object the export writes for it.
    pub type_name: Option<String>,
    /// Its IFC `PredefinedType`.
    pub predefined_type: Option<String>,
    /// The name of its material.
    pub material: Option<String>,
    /// Where its body is placed, model feet.
    pub location_feet: Option<[f64; 3]>,
    /// The body's rotation about Z, radians.
    pub rotation_radians: Option<f64>,
    /// A digest of the body the export draws: the mapped geometry it uses, else
    /// its solid, else its extrusion. Two equal digests are two equal bodies.
    pub body: Option<String>,
    /// Each property the export writes for it, keyed `<set>.<property>`, as
    /// the value's IFC kind and the value (`{"kind": "Real", "value": 2.5}`),
    /// less those of rvt-rs's own sets that say where in the file it was read
    /// ([`FILE_LOCATION_PROPERTIES`]).
    pub properties: BTreeMap<String, Value>,
}

/// The properties of rvt-rs's own sets (`Rvt...`) that name where in the file
/// an element was read: the stream and the byte offsets. A later save stores
/// an element in another stream and at other offsets while the element stays
/// the same, so a diff of two saves listed every element as changed (on
/// Autodesk's samples, every element between the 2025 and 2026 saves).
pub const FILE_LOCATION_PROPERTIES: [&str; 5] = [
    "SourceStream",
    "PartitionStream",
    "PartitionOffset",
    "ByteStart",
    "ByteEnd",
];

/// The properties that say which rule read a value and not the value itself:
/// the value they label is compared on its own (`LevelElementId` and
/// `LevelBindResolved` for a Level bind), so a difference in them alone is a
/// label change ([`ElementChange::labels`]), not a change (#1864).
pub const LABEL_PROPERTIES: [&str; 1] = ["LevelBindSource"];

/// The property naming where a body came from. A label change where the two
/// bodies are equal, a change where they differ (#1864).
const BODY_SOURCE_PROPERTY: &str = "BodySource";

/// One field of an element that differs between the two exports.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FieldChange {
    /// The field of [`ElementState`], or `property:<set>.<property>`.
    pub field: String,
    /// Its value in the first export, `null` where it had none.
    pub left: Value,
    /// Its value in the second export, `null` where it has none.
    pub right: Value,
}

/// An element both exports write, with the fields that differ.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ElementChange {
    /// The GlobalId both exports write.
    pub global_id: String,
    /// Its `Tag` in the second export.
    pub tag: Option<String>,
    /// Its IFC entity in the second export.
    pub ifc_type: String,
    /// Its name in the second export.
    pub name: String,
    /// The fields that differ, in [`ElementState`]'s order and then the
    /// properties by key. Empty for an element in [`ElementDiff::relabelled`].
    pub changes: Vec<FieldChange>,
    /// The label properties that differ ([`LABEL_PROPERTIES`], and
    /// `BodySource` where the body is equal), which are not changes.
    pub labels: Vec<FieldChange>,
}

/// The GlobalIds each export writes for more than one element, sorted. A
/// GlobalId written twice names no one element, so an element with one is
/// not compared ([`ElementDiff::set_aside`]).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DuplicateGlobalIds {
    /// Written more than once by the first export.
    pub left: Vec<String>,
    /// Written more than once by the second export.
    pub right: Vec<String>,
}

/// How many elements of each export [`diff_elements`] did not compare.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SetAside {
    /// Elements of the first export whose GlobalId either export writes twice.
    pub left: usize,
    /// Elements of the second export whose GlobalId either export writes twice.
    pub right: usize,
}

/// Two exports compared element by element.
///
/// Every element of each export is counted once:
/// `left_elements = removed + changed + unchanged + set_aside.left` and
/// `right_elements = added + changed + unchanged + set_aside.right`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ElementDiff {
    /// The elements the first export writes.
    pub left_elements: usize,
    /// The elements the second export writes.
    pub right_elements: usize,
    /// In the second export only, by GlobalId.
    pub added: Vec<ElementState>,
    /// In the first export only, by GlobalId.
    pub removed: Vec<ElementState>,
    /// In both, with a field that differs, by GlobalId.
    pub changed: Vec<ElementChange>,
    /// In both, with every field equal but labels.
    pub unchanged: usize,
    /// Of the unchanged, those whose label properties differ, by GlobalId: the
    /// rule that read a value changed and the value did not.
    pub relabelled: Vec<ElementChange>,
    /// The GlobalIds an export writes twice. Their elements are in none of
    /// the four lists above.
    pub duplicate_global_ids: DuplicateGlobalIds,
    /// The elements left out of the comparison for that.
    pub set_aside: SetAside,
}

/// Every element `model`'s IFC export writes, in the model's order. A piece
/// the writer folds into its owner's body is part of the owner, not an
/// element of its own.
pub fn element_states(model: &IfcModel) -> Vec<ElementState> {
    let global_ids = element_global_ids(model);
    let placements = placements(model);
    let types = type_objects(model);
    let mut type_names: HashMap<usize, &str> = HashMap::new();
    for object in types.all() {
        for &element in &object.elements {
            type_names.entry(element).or_insert(object.name.as_str());
        }
    }
    let mut sets: HashMap<usize, Vec<&PropertySet>> = HashMap::new();
    for entity in &model.entities {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            sets.entry(*element).or_default().push(set);
        }
    }
    let mut out = Vec::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            predefined_type,
            material_index,
            property_set,
            location_feet,
            rotation_radians,
            extrusion,
            solid_shape,
            representation_map_index,
            ..
        } = entity
        else {
            continue;
        };
        if matches!(placements.get(index), Some(Some(Placement::Folded))) {
            continue;
        }
        let Some(Some(global_id)) = global_ids.get(index) else {
            continue;
        };
        let mut properties = BTreeMap::new();
        let own = property_set.iter();
        let attached = sets.get(&index).into_iter().flatten().copied();
        for set in own.chain(attached) {
            let own_set = set.name.starts_with("Rvt");
            for property in &set.properties {
                if own_set && FILE_LOCATION_PROPERTIES.contains(&property.name.as_str()) {
                    continue;
                }
                properties.insert(
                    format!("{}.{}", set.name, property.name),
                    to_value(&property.value),
                );
            }
        }
        let map = representation_map_index.and_then(|at| model.representation_maps.get(at));
        // The writer draws the mapped item, else the solid, else the extrusion
        // (`step_writer`). A body kept beside a higher one is not drawn, and two
        // saves that differ only in it differ in nothing the IFC writes (#1864).
        let drawn_solid = solid_shape.as_ref().filter(|_| map.is_none());
        let drawn_extrusion = extrusion
            .as_ref()
            .filter(|_| map.is_none() && solid_shape.is_none());
        let body = (extrusion.is_some() || solid_shape.is_some() || map.is_some())
            .then(|| digest(&(drawn_extrusion, drawn_solid, map)));
        out.push(ElementState {
            global_id: global_id.clone(),
            tag: type_guid.clone(),
            ifc_type: ifc_type.clone(),
            name: name.clone(),
            storey: storey_of(&placements, index)
                .and_then(|storey| model.building_storeys.get(storey))
                .map(|storey| storey.name.clone()),
            type_name: type_names.get(&index).map(|name| (*name).to_string()),
            predefined_type: predefined_type.clone(),
            material: material_index
                .and_then(|at| model.materials.get(at))
                .map(|material| material.name.clone()),
            location_feet: *location_feet,
            rotation_radians: *rotation_radians,
            body,
            properties,
        });
    }
    out
}

/// `left` and `right` compared by GlobalId. Each list of the result is in
/// GlobalId order.
///
/// An element whose GlobalId either export writes for more than one element
/// is not compared: two elements under one key cannot be told apart, and
/// keeping the later one would hide a change in the other. Those GlobalIds
/// are listed in [`ElementDiff::duplicate_global_ids`] and their elements
/// counted in [`ElementDiff::set_aside`].
pub fn diff_elements(left: &[ElementState], right: &[ElementState]) -> ElementDiff {
    fn repeated(states: &[ElementState]) -> BTreeSet<&str> {
        let mut seen = BTreeSet::new();
        let mut twice = BTreeSet::new();
        for state in states {
            if !seen.insert(state.global_id.as_str()) {
                twice.insert(state.global_id.as_str());
            }
        }
        twice
    }
    let (left_twice, right_twice) = (repeated(left), repeated(right));
    let set_aside_ids: BTreeSet<&str> = left_twice.union(&right_twice).copied().collect();
    let by_id = |states: &[ElementState]| -> (BTreeMap<String, ElementState>, usize) {
        let mut compared = BTreeMap::new();
        let mut aside = 0;
        for state in states {
            if set_aside_ids.contains(state.global_id.as_str()) {
                aside += 1;
            } else {
                compared.insert(state.global_id.clone(), state.clone());
            }
        }
        (compared, aside)
    };
    let ((left_by, left_aside), (right_by, right_aside)) = (by_id(left), by_id(right));
    let owned =
        |ids: &BTreeSet<&str>| -> Vec<String> { ids.iter().map(|id| (*id).to_string()).collect() };
    let mut diff = ElementDiff {
        left_elements: left.len(),
        right_elements: right.len(),
        duplicate_global_ids: DuplicateGlobalIds {
            left: owned(&left_twice),
            right: owned(&right_twice),
        },
        set_aside: SetAside {
            left: left_aside,
            right: right_aside,
        },
        ..ElementDiff::default()
    };
    for (id, before) in &left_by {
        match right_by.get(id) {
            None => diff.removed.push(before.clone()),
            Some(after) => {
                let (changes, labels) = field_changes(before, after);
                let change = ElementChange {
                    global_id: id.clone(),
                    tag: after.tag.clone(),
                    ifc_type: after.ifc_type.clone(),
                    name: after.name.clone(),
                    changes,
                    labels,
                };
                if !change.changes.is_empty() {
                    diff.changed.push(change);
                } else {
                    diff.unchanged += 1;
                    if !change.labels.is_empty() {
                        diff.relabelled.push(change);
                    }
                }
            }
        }
    }
    diff.added = right_by
        .into_iter()
        .filter(|(id, _)| !left_by.contains_key(id))
        .map(|(_, state)| state)
        .collect();
    diff
}

/// The fields of `left` and `right` that differ, and apart from them the label
/// properties that differ.
fn field_changes(
    left: &ElementState,
    right: &ElementState,
) -> (Vec<FieldChange>, Vec<FieldChange>) {
    let mut out = Vec::new();
    let mut compare = |field: &str, a: Value, b: Value| {
        if a != b {
            out.push(FieldChange {
                field: field.to_string(),
                left: a,
                right: b,
            });
        }
    };
    compare("tag", to_value(&left.tag), to_value(&right.tag));
    compare(
        "ifc_type",
        to_value(&left.ifc_type),
        to_value(&right.ifc_type),
    );
    compare("name", to_value(&left.name), to_value(&right.name));
    compare("storey", to_value(&left.storey), to_value(&right.storey));
    compare(
        "type_name",
        to_value(&left.type_name),
        to_value(&right.type_name),
    );
    compare(
        "predefined_type",
        to_value(&left.predefined_type),
        to_value(&right.predefined_type),
    );
    compare(
        "material",
        to_value(&left.material),
        to_value(&right.material),
    );
    compare(
        "location_feet",
        to_value(&left.location_feet),
        to_value(&right.location_feet),
    );
    compare(
        "rotation_radians",
        to_value(&left.rotation_radians),
        to_value(&right.rotation_radians),
    );
    compare("body", to_value(&left.body), to_value(&right.body));
    let keys: BTreeSet<&String> = left
        .properties
        .keys()
        .chain(right.properties.keys())
        .collect();
    let same_body = left.body == right.body;
    let is_label = |key: &str| {
        let property = key.rsplit_once('.').map_or(key, |(_, name)| name);
        LABEL_PROPERTIES.contains(&property) || (same_body && property == BODY_SOURCE_PROPERTY)
    };
    let mut labels = Vec::new();
    for key in keys {
        let field = format!("property:{key}");
        let a = left.properties.get(key).cloned().unwrap_or(Value::Null);
        let b = right.properties.get(key).cloned().unwrap_or(Value::Null);
        if is_label(key) {
            if a != b {
                labels.push(FieldChange {
                    field,
                    left: a,
                    right: b,
                });
            }
        } else {
            compare(&field, a, b);
        }
    }
    (out, labels)
}

/// `value` as JSON; `null` where it cannot be, which no field here is.
fn to_value(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// The first 16 bytes of the SHA-256 of `value`'s JSON, in hex.
fn digest(value: &impl Serialize) -> String {
    use sha2::{Digest, Sha256};
    let json = serde_json::to_vec(value).unwrap_or_default();
    Sha256::digest(&json)[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// Pure comparison of two lists, so its failure modes are tested here and not
// on a file: a GlobalId written twice must never hide a change, and every
// element of each side must land in exactly one bucket.
#[cfg(test)]
mod tests {
    use super::*;

    fn element(global_id: &str, name: &str) -> ElementState {
        ElementState {
            global_id: global_id.to_string(),
            tag: None,
            ifc_type: "IFCWALL".to_string(),
            name: name.to_string(),
            storey: None,
            type_name: None,
            predefined_type: None,
            material: None,
            location_feet: None,
            rotation_radians: None,
            body: None,
            properties: BTreeMap::new(),
        }
    }

    fn balances(diff: &ElementDiff) {
        let in_both = diff.changed.len() + diff.unchanged;
        assert_eq!(
            diff.left_elements,
            diff.removed.len() + in_both + diff.set_aside.left
        );
        assert_eq!(
            diff.right_elements,
            diff.added.len() + in_both + diff.set_aside.right
        );
    }

    #[test]
    fn no_repeated_id_adds_removes_and_changes_as_before() {
        let left = [element("A", "a"), element("B", "b"), element("C", "c")];
        let right = [element("B", "b"), element("C", "c2"), element("D", "d")];
        let diff = diff_elements(&left, &right);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.unchanged, 1);
        assert_eq!(diff.set_aside, SetAside::default());
        assert_eq!(diff.duplicate_global_ids, DuplicateGlobalIds::default());
        balances(&diff);
    }

    #[test]
    fn an_id_written_twice_on_the_left_hides_no_change() {
        // Left holds X as A and as B, right holds X as B2. Comparing the later
        // copy alone says "B against B2" and never shows A.
        let left = [element("X", "A"), element("X", "B"), element("Y", "y")];
        let right = [element("X", "B2"), element("Y", "y")];
        let diff = diff_elements(&left, &right);
        assert_eq!(diff.duplicate_global_ids.left, ["X"]);
        assert!(diff.duplicate_global_ids.right.is_empty());
        assert_eq!((diff.set_aside.left, diff.set_aside.right), (2, 1));
        assert!(diff.added.is_empty() && diff.removed.is_empty() && diff.changed.is_empty());
        assert_eq!(diff.unchanged, 1);
        balances(&diff);
    }

    #[test]
    fn an_id_written_twice_on_the_right_hides_no_change() {
        let left = [element("X", "A"), element("Y", "y")];
        let right = [element("X", "B"), element("X", "C"), element("Y", "y")];
        let diff = diff_elements(&left, &right);
        assert!(diff.duplicate_global_ids.left.is_empty());
        assert_eq!(diff.duplicate_global_ids.right, ["X"]);
        assert_eq!((diff.set_aside.left, diff.set_aside.right), (1, 2));
        assert!(diff.added.is_empty() && diff.removed.is_empty() && diff.changed.is_empty());
        balances(&diff);
    }

    #[test]
    fn an_id_written_twice_by_both_is_listed_for_each() {
        let left = [element("X", "A"), element("X", "B")];
        let right = [element("X", "A"), element("X", "B"), element("Z", "z")];
        let diff = diff_elements(&left, &right);
        assert_eq!(diff.duplicate_global_ids.left, ["X"]);
        assert_eq!(diff.duplicate_global_ids.right, ["X"]);
        assert_eq!((diff.set_aside.left, diff.set_aside.right), (2, 2));
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.unchanged, 0);
        balances(&diff);
    }

    fn with(state: &ElementState, key: &str, value: &str, body: &str) -> ElementState {
        let mut state = state.clone();
        state.properties.insert(key.to_string(), Value::from(value));
        state.body = Some(body.to_string());
        state
    }

    #[test]
    fn a_level_bind_rule_alone_is_a_label_not_a_change() {
        // #1864: the same Level, bound by two rules on two saves.
        let key = "RvtElementRecord.LevelBindSource";
        let left = [with(&element("A", "a"), key, "reference_list", "b1")];
        let right = [with(&element("A", "a"), key, "reference_level", "b1")];
        let diff = diff_elements(&left, &right);
        assert!(diff.changed.is_empty());
        assert_eq!(diff.unchanged, 1);
        assert_eq!(diff.relabelled.len(), 1);
        assert_eq!(
            diff.relabelled[0].labels[0].field,
            format!("property:{key}")
        );
        balances(&diff);
    }

    #[test]
    fn a_body_source_is_a_change_where_the_body_differs() {
        // A label where the body is equal; with another body it says which
        // reader drew the new one, and the element has changed.
        let key = "RvtElementRecord.BodySource";
        let left = [with(&element("A", "a"), key, "sketch", "b1")];
        let same = [with(&element("A", "a"), key, "saved_mesh", "b1")];
        let moved = [with(&element("A", "a"), key, "saved_mesh", "b2")];
        let relabelled = diff_elements(&left, &same);
        assert!(relabelled.changed.is_empty() && relabelled.relabelled.len() == 1);
        let changed = diff_elements(&left, &moved);
        assert_eq!(changed.changed.len(), 1);
        let fields: Vec<&str> = changed.changed[0]
            .changes
            .iter()
            .map(|change| change.field.as_str())
            .collect();
        assert_eq!(fields, ["body", "property:RvtElementRecord.BodySource"]);
        assert!(changed.relabelled.is_empty());
        balances(&changed);
    }

    #[test]
    fn three_copies_of_an_id_count_as_three_set_aside() {
        let left = [element("X", "A"), element("X", "B"), element("X", "C")];
        let diff = diff_elements(&left, &[]);
        assert_eq!(diff.duplicate_global_ids.left, ["X"]);
        assert_eq!(diff.set_aside.left, 3);
        assert!(diff.removed.is_empty());
        balances(&diff);
    }
}
