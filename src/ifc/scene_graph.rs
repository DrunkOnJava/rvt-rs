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

/// One end of a host relationship (M4-04). Carries enough to label
/// the row and to re-select the related element: `entity_index` is
/// the same index the scene graph puts on its nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedElement {
    /// Index into `model.entities`.
    pub entity_index: usize,
    pub name: String,
    pub ifc_type: String,
}

/// A labelled, pre-formatted panel row. `label` is the display
/// term the viewer prints in the key gutter; `value` is already
/// unit-carrying text (`"10.500 ft"`, `"90.0°"`) so the frontend
/// never re-derives a unit or a rounding rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelRow {
    pub label: String,
    pub value: String,
}

impl PanelRow {
    fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// One property inside a [`PanelPropertyGroup`]. `kind` names the
/// [`super::entities::PropertyValue`] flavour the value came from,
/// so the viewer can align measurements without parsing the
/// formatted text back apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelProperty {
    pub name: String,
    /// Formatted value carrying its unit where the value kind has
    /// one. Booleans read as words rather than `true` / `false`.
    pub value: String,
    /// `"text"`, `"integer"`, `"real"`, `"boolean"`, `"length"`,
    /// `"angle"`, `"area"`, `"volume"`, `"count"`, `"time"`, or
    /// `"mass"`.
    pub kind: String,
    /// `true` for the numeric kinds — the viewer right-aligns
    /// those into a tabular column.
    pub numeric: bool,
}

/// An element's property set as a titled group. The viewer
/// renders `name` as the group heading and one row per entry in
/// `properties`, instead of dumping the raw `property_set` JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelPropertyGroup {
    /// Property-set name — `"Pset_WallCommon"`, `"Pset_RevitType_Wall"`.
    pub name: String,
    pub properties: Vec<PanelProperty>,
}

/// The storey an element sits on, resolved from `storey_index` to
/// the name the model gives it. `index` is the same index
/// the scene graph puts on its `IFCBUILDINGSTOREY` nodes, so the
/// viewer can jump to that node the way #269 jumps to a host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelStorey {
    /// Index into `model.building_storeys`.
    pub index: usize,
    pub name: String,
    pub elevation_feet: f64,
    /// Pre-formatted elevation, e.g. `"0.000 ft"`.
    pub elevation_label: String,
}

/// The material an element is associated with, resolved from
/// `material_index` to its display name. `element_count`
/// is how many `BuildingElement`s in the model share it — what
/// the viewer's highlight affordance acts on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelMaterial {
    /// Index into `model.materials`.
    pub index: usize,
    pub name: String,
    pub element_count: usize,
}

/// Element info panel payload (VW1-08). The shape a viewer's
/// "click to inspect" UI reads — a single JSON-ready struct
/// describing an element's identity, location, and property set.
///
/// Every field is either resolved to display text or omitted:
/// index fields carry the name they point at, measurements carry
/// their unit, and an absent optional stays absent rather than
/// becoming an empty string the viewer renders as a dash.
/// `missing` names the fields that were absent, so the viewer can
/// cross-check them against the export diagnostics and say "not
/// recovered" only where that is the honest word.
///
/// Populate via [`element_info_panel`] given a scene node's
/// `entity_index` and the underlying model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfoPanel {
    pub name: String,
    pub ifc_type: String,
    pub type_guid: Option<String>,
    /// IFC4 `PredefinedType` token (`"COLUMN"`, `"STANDARD"`),
    /// when the category map assigned one.
    #[serde(default)]
    pub predefined_type: Option<String>,
    /// Storey display name when the element is contained in one.
    pub storey_name: Option<String>,
    /// Storey elevation in feet (native Revit unit). `None` when
    /// the element hasn't been bound to a storey.
    pub storey_elevation_feet: Option<f64>,
    /// The same storey, resolved to a jump target.
    #[serde(default)]
    pub storey: Option<PanelStorey>,
    /// World-space location in feet, if the element has one.
    pub location_feet: Option<[f64; 3]>,
    /// Yaw rotation in radians (about +Z), if set.
    pub rotation_radians: Option<f64>,
    /// Location and rotation as labelled, unit-carrying rows
    ///. Empty when the element has neither.
    #[serde(default)]
    pub placement_rows: Vec<PanelRow>,
    /// Extrusion extents as labelled rows in feet. Empty
    /// when the element carries no extrusion.
    #[serde(default)]
    pub extent_rows: Vec<PanelRow>,
    /// Material display name resolved through the model's
    /// material list. `None` when the element has no material
    /// associated.
    pub material_name: Option<String>,
    /// The same material, resolved to a highlight target.
    #[serde(default)]
    pub material: Option<PanelMaterial>,
    /// Flat `(property_name, formatted_value)` pairs from the
    /// element's `Pset_*Common` property set. Empty when no
    /// property set is attached.
    pub properties: Vec<(String, String)>,
    /// The same property set as a titled group with typed rows
    ///. `None` when no property set is attached.
    #[serde(default)]
    pub property_group: Option<PanelPropertyGroup>,
    /// The element this one is hosted by — the wall a door or
    /// window sits in (M4-04). `None` when nothing hosts it.
    #[serde(default)]
    pub host: Option<RelatedElement>,
    /// Elements hosted by this one, in entity order — the doors and
    /// windows a wall carries. Empty when it hosts nothing.
    #[serde(default)]
    pub hosted: Vec<RelatedElement>,
    /// Field names absent on this element, drawn from the fixed
    /// set `"storey"`, `"material"`, `"properties"`,
    /// `"placement"`, `"extents"`. The viewer reports one
    /// as "not recovered" only when the export diagnostics agree
    /// it is a known decode gap; otherwise it stays omitted.
    #[serde(default)]
    pub missing: Vec<String>,
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
        predefined_type,
        storey_index,
        material_index,
        property_set,
        location_feet,
        rotation_radians,
        extrusion,
        host_element_index,
        ..
    } = ent
    else {
        return None;
    };
    let storey = storey_index
        .and_then(|i| model.building_storeys.get(i).map(|s| (i, s)))
        .map(|(index, s)| PanelStorey {
            index,
            name: s.name.clone(),
            elevation_feet: s.elevation_feet,
            elevation_label: format_feet(s.elevation_feet),
        });
    let (storey_name, storey_elevation_feet) = match &storey {
        Some(s) => (Some(s.name.clone()), Some(s.elevation_feet)),
        None => (None, None),
    };
    let material = material_index
        .and_then(|i| model.materials.get(i).map(|m| (i, m)))
        .map(|(index, m)| PanelMaterial {
            index,
            name: m.name.clone(),
            element_count: count_elements_using_material(model, index),
        });
    let material_name = material.as_ref().map(|m| m.name.clone());
    let properties: Vec<(String, String)> = property_set
        .as_ref()
        .map(|pset| {
            pset.properties
                .iter()
                .map(|p| (p.name.clone(), format_property_value(&p.value)))
                .collect()
        })
        .unwrap_or_default();
    let property_group = property_set.as_ref().map(|pset| PanelPropertyGroup {
        name: pset.name.clone(),
        properties: pset
            .properties
            .iter()
            .map(|p| PanelProperty {
                name: p.name.clone(),
                value: format_property_value(&p.value),
                kind: property_value_kind(&p.value).into(),
                numeric: property_value_is_numeric(&p.value),
            })
            .collect(),
    });
    let placement_rows = placement_rows(location_feet.as_ref(), rotation_radians.as_ref());
    let extent_rows = extent_rows(extrusion.as_ref());
    let missing = missing_fields(
        storey.is_some(),
        material.is_some(),
        property_group
            .as_ref()
            .is_some_and(|g| !g.properties.is_empty()),
        !placement_rows.is_empty(),
        !extent_rows.is_empty(),
    );
    let host = host_element_index.and_then(|i| related_element(model, i));
    let hosted: Vec<RelatedElement> = model
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            matches!(
                e,
                IfcEntity::BuildingElement {
                    host_element_index: Some(h),
                    ..
                } if *h == entity_index
            )
        })
        .filter_map(|(i, _)| related_element(model, i))
        .collect();
    Some(ElementInfoPanel {
        name: name.clone(),
        ifc_type: ifc_type.clone(),
        type_guid: type_guid.clone(),
        predefined_type: predefined_type.clone(),
        storey_name,
        storey_elevation_feet,
        storey,
        location_feet: *location_feet,
        rotation_radians: *rotation_radians,
        placement_rows,
        extent_rows,
        material_name,
        material,
        properties,
        property_group,
        host,
        hosted,
        missing,
    })
}

/// How many `BuildingElement`s associate with `material_index` —
/// the population the viewer's material highlight lights up.
fn count_elements_using_material(model: &IfcModel, material_index: usize) -> usize {
    model
        .entities
        .iter()
        .filter(|e| {
            matches!(
                e,
                IfcEntity::BuildingElement {
                    material_index: Some(m),
                    ..
                } if *m == material_index
            )
        })
        .count()
}

/// Feet with the 3-decimal precision the viewer's storey labels
/// already use — enough to read a wall thickness, not so much
/// that a decoded float's noise floor shows through. A value that
/// rounds to zero prints as `0.000`, never `-0.000`: the sign on
/// a rounded-away negative is noise, and it reads as a real
/// measurement.
fn format_feet(v: f64) -> String {
    let rounded = if (v * 1000.0).round() == 0.0 { 0.0 } else { v };
    format!("{rounded:.3} ft")
}

/// Location (X / Y / Z) and rotation as labelled rows. Rotation
/// is reported in degrees because that is how Revit states it;
/// the radian original stays on `rotation_radians`.
fn placement_rows(
    location_feet: Option<&[f64; 3]>,
    rotation_radians: Option<&f64>,
) -> Vec<PanelRow> {
    let mut rows = Vec::new();
    if let Some(loc) = location_feet {
        rows.push(PanelRow::new("X", format_feet(loc[0])));
        rows.push(PanelRow::new("Y", format_feet(loc[1])));
        rows.push(PanelRow::new("Z", format_feet(loc[2])));
    }
    if let Some(rot) = rotation_radians {
        rows.push(PanelRow::new(
            "Rotation",
            format!("{:.1}°", rot.to_degrees()),
        ));
    }
    rows
}

/// Extrusion extents as labelled rows in feet, plus the profile
/// shape when the element overrides the default rectangle.
fn extent_rows(extrusion: Option<&super::entities::Extrusion>) -> Vec<PanelRow> {
    let Some(ex) = extrusion else {
        return Vec::new();
    };
    let mut rows = vec![
        PanelRow::new("Width", format_feet(ex.width_feet)),
        PanelRow::new("Depth", format_feet(ex.depth_feet)),
        PanelRow::new("Height", format_feet(ex.height_feet)),
    ];
    rows.push(PanelRow::new(
        "Profile",
        profile_label(ex.profile_override.as_ref()),
    ));
    rows
}

/// Plain-language name for an extrusion profile.
fn profile_label(profile: Option<&super::entities::ProfileDef>) -> String {
    use super::entities::ProfileDef;
    match profile {
        None | Some(ProfileDef::Rectangle { .. }) => "Rectangle".into(),
        Some(ProfileDef::Circle { .. }) => "Circle".into(),
        Some(ProfileDef::IShape { .. }) => "I-shape".into(),
        Some(ProfileDef::TShape { .. }) => "T-shape".into(),
        Some(ProfileDef::LShape { .. }) => "L-shape".into(),
        Some(ProfileDef::UShape { .. }) => "U-shape".into(),
        Some(ProfileDef::RectangleHollow { .. }) => "Rectangular tube".into(),
        Some(ProfileDef::CircleHollow { .. }) => "Round tube".into(),
        Some(ProfileDef::ArbitraryClosed { .. }) => "Arbitrary closed".into(),
        Some(ProfileDef::ArbitraryWithVoids { .. }) => "Arbitrary with voids".into(),
    }
}

/// Which of the optional panel fields came back empty. Ordered so
/// the viewer's one-line note reads the same way every time.
fn missing_fields(
    has_storey: bool,
    has_material: bool,
    has_properties: bool,
    has_placement: bool,
    has_extents: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    if !has_storey {
        out.push("storey".to_string());
    }
    if !has_material {
        out.push("material".to_string());
    }
    if !has_properties {
        out.push("properties".to_string());
    }
    if !has_placement {
        out.push("placement".to_string());
    }
    if !has_extents {
        out.push("extents".to_string());
    }
    out
}

/// Describe the `BuildingElement` at `entity_index` as a host
/// relationship row. `None` for out-of-range or non-element indices.
fn related_element(model: &IfcModel, entity_index: usize) -> Option<RelatedElement> {
    match model.entities.get(entity_index)? {
        IfcEntity::BuildingElement { name, ifc_type, .. } => Some(RelatedElement {
            entity_index,
            name: name.clone(),
            ifc_type: ifc_type.clone(),
        }),
        _ => None,
    }
}

fn format_property_value(v: &super::entities::PropertyValue) -> String {
    use super::entities::PropertyValue;
    match v {
        PropertyValue::Text(s) => s.clone(),
        PropertyValue::Integer(i) => i.to_string(),
        PropertyValue::Real(r) => format!("{r:.3}"),
        // Words, not `true` / `false` — the panel is read by people,
        // and a Revit yes/no parameter is a yes/no answer.
        PropertyValue::Boolean(b) => if *b { "Yes" } else { "No" }.to_string(),
        PropertyValue::LengthFeet(f) => format!("{f:.3} ft"),
        PropertyValue::AngleRadians(r) => format!("{:.3}°", r.to_degrees()),
        PropertyValue::AreaSquareFeet(a) => format!("{a:.2} sq ft"),
        PropertyValue::VolumeCubicFeet(c) => format!("{c:.2} cu ft"),
        PropertyValue::CountValue(c) => c.to_string(),
        PropertyValue::TimeSeconds(t) => format!("{t:.1} s"),
        PropertyValue::MassPounds(m) => format!("{m:.2} lb"),
    }
}

/// Stable token naming a property's value flavour, so the viewer
/// can style a measurement differently from free text without
/// re-parsing the formatted string.
fn property_value_kind(v: &super::entities::PropertyValue) -> &'static str {
    use super::entities::PropertyValue;
    match v {
        PropertyValue::Text(_) => "text",
        PropertyValue::Integer(_) => "integer",
        PropertyValue::Real(_) => "real",
        PropertyValue::Boolean(_) => "boolean",
        PropertyValue::LengthFeet(_) => "length",
        PropertyValue::AngleRadians(_) => "angle",
        PropertyValue::AreaSquareFeet(_) => "area",
        PropertyValue::VolumeCubicFeet(_) => "volume",
        PropertyValue::CountValue(_) => "count",
        PropertyValue::TimeSeconds(_) => "time",
        PropertyValue::MassPounds(_) => "mass",
    }
}

/// `true` when the formatted value is a number the viewer should
/// right-align into a tabular column.
fn property_value_is_numeric(v: &super::entities::PropertyValue) -> bool {
    use super::entities::PropertyValue;
    !matches!(v, PropertyValue::Text(_) | PropertyValue::Boolean(_))
}

/// Row of a schedule table (VW1-15) — one per `BuildingElement`
/// in an IfcModel. Fields are pre-formatted strings so the
/// viewer can render the table without additional
/// unit-conversion logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleRow {
    pub entity_index: usize,
    pub ifc_type: String,
    pub name: String,
    pub storey: String,
    pub material: String,
    /// The IFC element's GUID if present; empty when the source
    /// didn't carry one.
    pub guid: String,
}

/// One IFC type's share of a [`Schedule`]. The viewer
/// renders the schedule as these groups rather than a bare total,
/// and uses `entity_indices` to highlight the whole type in the
/// 3-D scene at once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleTypeGroup {
    pub ifc_type: String,
    pub count: usize,
    /// Every `model.entities` index in this group, in row order.
    pub entity_indices: Vec<usize>,
    /// Distinct storey names the group spans, in row order, with
    /// unbound elements left out. Empty when none are bound.
    pub storeys: Vec<String>,
}

/// Schedule-view payload (VW1-15). Pre-ordered by (storey
/// elevation, name) for stable output — viewers that want a
/// different sort should resort the returned vec.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    pub rows: Vec<ScheduleRow>,
    /// Per-IFC-type breakdown of `rows`, ordered by descending
    /// count then type name. Derived from `rows`, so a
    /// caller that resorts or filters should rebuild it with
    /// [`Schedule::from_rows`].
    #[serde(default)]
    pub groups: Vec<ScheduleTypeGroup>,
}

impl Schedule {
    /// Build a schedule from rows, deriving the per-type groups.
    pub fn from_rows(rows: Vec<ScheduleRow>) -> Self {
        let groups = group_rows_by_ifc_type(&rows);
        Self { rows, groups }
    }

    /// Row count.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// `true` when no rows.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Filter to rows matching the given IFC type (case-
    /// insensitive). Returns a new `Schedule`.
    pub fn filter_by_ifc_type(&self, ifc_type: &str) -> Schedule {
        let target = ifc_type.to_ascii_uppercase();
        Schedule::from_rows(
            self.rows
                .iter()
                .filter(|r| r.ifc_type.to_ascii_uppercase() == target)
                .cloned()
                .collect(),
        )
    }

    /// Render as CSV (VW1-15). Header row + one row per
    /// `ScheduleRow` with fields in the same order as the
    /// struct: entity_index, ifc_type, name, storey, material,
    /// guid. Values with commas or quotes are double-quote
    /// wrapped + inner quotes doubled, per RFC 4180, and a value a
    /// spreadsheet would read as a formula is prefixed with `'`.
    /// For spreadsheet schedules with placement, size, host and
    /// room columns use [`super::schedule_csv`].
    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        out.push_str("entity_index,ifc_type,name,storey,material,guid\n");
        for r in &self.rows {
            out.push_str(&format!("{},", r.entity_index));
            out.push_str(&csv_field(&r.ifc_type));
            out.push(',');
            out.push_str(&csv_field(&r.name));
            out.push(',');
            out.push_str(&csv_field(&r.storey));
            out.push(',');
            out.push_str(&csv_field(&r.material));
            out.push(',');
            out.push_str(&csv_field(&r.guid));
            out.push('\n');
        }
        out
    }
}

fn csv_field(s: &str) -> String {
    // Names come from an untrusted file: neutralise formula triggers
    // (OWASP CSV injection) before RFC 4180 quoting.
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

/// Build a tabular schedule of every `BuildingElement` in `model`
/// (VW1-15). Non-BuildingElement entities are skipped. Rows are
/// ordered by `(storey_elevation_feet, name)` so the output is
/// stable across runs.
pub fn build_schedule(model: &IfcModel) -> Schedule {
    let mut rows: Vec<(f64, ScheduleRow)> = Vec::new();
    for (idx, ent) in model.entities.iter().enumerate() {
        if let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            storey_index,
            material_index,
            ..
        } = ent
        {
            let (storey, elev) = storey_index
                .and_then(|i| model.building_storeys.get(i))
                .map(|s| (s.name.clone(), s.elevation_feet))
                .unwrap_or_else(|| ("".into(), f64::INFINITY));
            let material = material_index
                .and_then(|i| model.materials.get(i))
                .map(|m| m.name.clone())
                .unwrap_or_default();
            rows.push((
                elev,
                ScheduleRow {
                    entity_index: idx,
                    ifc_type: ifc_type.clone(),
                    name: name.clone(),
                    storey,
                    material,
                    guid: type_guid.clone().unwrap_or_default(),
                },
            ));
        }
    }
    rows.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.name.cmp(&b.1.name))
    });
    Schedule::from_rows(rows.into_iter().map(|(_, r)| r).collect())
}

/// Collapse schedule rows into one group per IFC type, biggest
/// group first so the viewer's breakdown leads with what the
/// model is mostly made of. Ties break on type name so the
/// ordering is stable across runs.
fn group_rows_by_ifc_type(rows: &[ScheduleRow]) -> Vec<ScheduleTypeGroup> {
    let mut order: Vec<String> = Vec::new();
    let mut by_type: std::collections::HashMap<String, ScheduleTypeGroup> =
        std::collections::HashMap::new();
    for row in rows {
        let group = by_type.entry(row.ifc_type.clone()).or_insert_with(|| {
            order.push(row.ifc_type.clone());
            ScheduleTypeGroup {
                ifc_type: row.ifc_type.clone(),
                count: 0,
                entity_indices: Vec::new(),
                storeys: Vec::new(),
            }
        });
        group.count += 1;
        group.entity_indices.push(row.entity_index);
        if !row.storey.is_empty() && !group.storeys.iter().any(|s| s == &row.storey) {
            group.storeys.push(row.storey.clone());
        }
    }
    let mut groups: Vec<ScheduleTypeGroup> = order
        .into_iter()
        .filter_map(|t| by_type.remove(&t))
        .collect();
    groups.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.ifc_type.cmp(&b.ifc_type))
    });
    groups
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
            predefined_type: None,
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
                predefined_type: None,
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
                predefined_type: None,
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
        // Booleans read as words — the panel is for people.
        assert_eq!(ext.1, "Yes");
    }

    // ---- Resolved panel fields: index -> name, units, gaps ----

    /// Every optional an info panel can resolve, in one place, so a
    /// test names only the fields it cares about.
    #[derive(Default)]
    struct Rich {
        ifc_type: &'static str,
        name: &'static str,
        storey: Option<usize>,
        material: Option<usize>,
        pset: Option<super::super::entities::PropertySet>,
        location: Option<[f64; 3]>,
        rotation: Option<f64>,
        extrusion: Option<super::super::entities::Extrusion>,
    }

    fn mk_rich_element(spec: Rich) -> IfcEntity {
        IfcEntity::BuildingElement {
            ifc_type: spec.ifc_type.into(),
            name: spec.name.into(),
            type_guid: None,
            predefined_type: Some("STANDARD".into()),
            storey_index: spec.storey,
            material_index: spec.material,
            property_set: spec.pset,
            location_feet: spec.location,
            rotation_radians: spec.rotation,
            extrusion: spec.extrusion,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }
    }

    #[test]
    fn info_panel_resolves_storey_index_to_a_jump_target() {
        let model = IfcModel {
            entities: vec![mk_element("Col-1", "IFCCOLUMN", Some(1), None)],
            building_storeys: vec![
                Storey {
                    name: "Level 1".into(),
                    elevation_feet: 0.0,
                },
                Storey {
                    name: "Level 2".into(),
                    elevation_feet: 12.5,
                },
            ],
            ..Default::default()
        };
        let storey = element_info_panel(&model, 0).unwrap().storey.unwrap();
        assert_eq!(storey.index, 1);
        assert_eq!(storey.name, "Level 2");
        assert_eq!(storey.elevation_feet, 12.5);
        assert_eq!(storey.elevation_label, "12.500 ft");
    }

    #[test]
    fn info_panel_omits_storey_when_the_index_is_unbound() {
        let model = IfcModel {
            entities: vec![mk_element("Col-1", "IFCCOLUMN", None, None)],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        assert!(panel.storey.is_none());
        assert!(panel.missing.iter().any(|m| m == "storey"));
    }

    #[test]
    fn info_panel_resolves_material_index_with_its_population() {
        let model = IfcModel {
            entities: vec![
                mk_rich_element(Rich {
                    ifc_type: "IFCWALL",
                    name: "W1",
                    material: Some(0),
                    ..Default::default()
                }),
                mk_rich_element(Rich {
                    ifc_type: "IFCWALL",
                    name: "W2",
                    material: Some(0),
                    ..Default::default()
                }),
                mk_rich_element(Rich {
                    ifc_type: "IFCSLAB",
                    name: "S1",
                    material: Some(1),
                    ..Default::default()
                }),
            ],
            materials: vec![
                super::super::MaterialInfo {
                    name: "Concrete".into(),
                    color_packed: None,
                    transparency: None,
                },
                super::super::MaterialInfo {
                    name: "Glass".into(),
                    color_packed: None,
                    transparency: None,
                },
            ],
            ..Default::default()
        };
        let material = element_info_panel(&model, 0).unwrap().material.unwrap();
        assert_eq!(material.index, 0);
        assert_eq!(material.name, "Concrete");
        // Both walls share it; the slab's Glass does not count.
        assert_eq!(material.element_count, 2);
    }

    #[test]
    fn info_panel_groups_properties_with_kinds_and_units() {
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
                    value: PropertyValue::Boolean(false),
                },
                Property {
                    name: "Reference".into(),
                    value: PropertyValue::Text("Interior - 5 1/2".into()),
                },
                Property {
                    name: "GrossArea".into(),
                    value: PropertyValue::AreaSquareFeet(84.25),
                },
            ],
        };
        let model = IfcModel {
            entities: vec![mk_rich_element(Rich {
                ifc_type: "IFCWALL",
                name: "W1",
                pset: Some(pset),
                ..Default::default()
            })],
            ..Default::default()
        };
        let group = element_info_panel(&model, 0)
            .unwrap()
            .property_group
            .unwrap();
        assert_eq!(group.name, "Pset_WallCommon");
        assert_eq!(group.properties.len(), 4);

        let height = &group.properties[0];
        assert_eq!(height.name, "Height");
        assert_eq!(height.value, "10.500 ft");
        assert_eq!(height.kind, "length");
        assert!(height.numeric);

        let is_external = &group.properties[1];
        assert_eq!(is_external.value, "No");
        assert_eq!(is_external.kind, "boolean");
        assert!(!is_external.numeric);

        let reference = &group.properties[2];
        assert_eq!(reference.value, "Interior - 5 1/2");
        assert_eq!(reference.kind, "text");
        assert!(!reference.numeric);

        let area = &group.properties[3];
        assert_eq!(area.value, "84.25 sq ft");
        assert_eq!(area.kind, "area");
    }

    #[test]
    fn info_panel_labels_location_and_rotation_in_feet_and_degrees() {
        let model = IfcModel {
            entities: vec![mk_rich_element(Rich {
                ifc_type: "IFCCOLUMN",
                name: "C1",
                location: Some([12.5, -4.0, 0.0]),
                rotation: Some(std::f64::consts::FRAC_PI_2),
                ..Default::default()
            })],
            ..Default::default()
        };
        let rows = element_info_panel(&model, 0).unwrap().placement_rows;
        assert_eq!(
            rows.iter()
                .map(|r| (r.label.as_str(), r.value.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("X", "12.500 ft"),
                ("Y", "-4.000 ft"),
                ("Z", "0.000 ft"),
                ("Rotation", "90.0°"),
            ]
        );
    }

    #[test]
    fn info_panel_prints_a_rounded_away_negative_as_plain_zero() {
        let model = IfcModel {
            entities: vec![mk_rich_element(Rich {
                ifc_type: "IFCDOOR",
                name: "D1",
                location: Some([1.0, -0.0, -1e-9]),
                ..Default::default()
            })],
            ..Default::default()
        };
        let rows = element_info_panel(&model, 0).unwrap().placement_rows;
        assert_eq!(rows[1].value, "0.000 ft");
        assert_eq!(rows[2].value, "0.000 ft");
    }

    #[test]
    fn info_panel_labels_extrusion_extents_in_feet() {
        use super::super::entities::Extrusion;
        let model = IfcModel {
            entities: vec![mk_rich_element(Rich {
                ifc_type: "IFCWALL",
                name: "W1",
                extrusion: Some(Extrusion::rectangle(8.0, 0.5, 10.0)),
                ..Default::default()
            })],
            ..Default::default()
        };
        let rows = element_info_panel(&model, 0).unwrap().extent_rows;
        assert_eq!(
            rows.iter()
                .map(|r| (r.label.as_str(), r.value.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Width", "8.000 ft"),
                ("Depth", "0.500 ft"),
                ("Height", "10.000 ft"),
                ("Profile", "Rectangle"),
            ]
        );
    }

    #[test]
    fn info_panel_names_a_non_rectangular_profile() {
        use super::super::entities::Extrusion;
        let model = IfcModel {
            entities: vec![mk_rich_element(Rich {
                ifc_type: "IFCCOLUMN",
                name: "C1",
                extrusion: Some(Extrusion::circle(0.75, 10.0)),
                ..Default::default()
            })],
            ..Default::default()
        };
        let rows = element_info_panel(&model, 0).unwrap().extent_rows;
        assert_eq!(rows.last().unwrap().value, "Circle");
    }

    #[test]
    fn info_panel_reports_every_absent_optional_as_missing() {
        let model = IfcModel {
            entities: vec![mk_element("W1", "IFCWALL", None, None)],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        assert_eq!(
            panel.missing,
            vec!["storey", "material", "properties", "placement", "extents"]
        );
        assert!(panel.placement_rows.is_empty());
        assert!(panel.extent_rows.is_empty());
        assert!(panel.property_group.is_none());
    }

    #[test]
    fn info_panel_reports_nothing_missing_when_every_field_resolves() {
        use super::super::entities::{Extrusion, Property, PropertySet, PropertyValue};
        let model = IfcModel {
            entities: vec![mk_rich_element(Rich {
                ifc_type: "IFCWALL",
                name: "W1",
                storey: Some(0),
                material: Some(0),
                pset: Some(PropertySet {
                    name: "Pset_WallCommon".into(),
                    properties: vec![Property {
                        name: "IsExternal".into(),
                        value: PropertyValue::Boolean(true),
                    }],
                }),
                location: Some([0.0, 0.0, 0.0]),
                rotation: Some(0.0),
                extrusion: Some(Extrusion::rectangle(8.0, 0.5, 10.0)),
            })],
            building_storeys: vec![Storey {
                name: "Level 1".into(),
                elevation_feet: 0.0,
            }],
            materials: vec![super::super::MaterialInfo {
                name: "Concrete".into(),
                color_packed: None,
                transparency: None,
            }],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        assert!(panel.missing.is_empty());
        assert_eq!(panel.predefined_type.as_deref(), Some("STANDARD"));
    }

    #[test]
    fn info_panel_new_fields_survive_serde_as_snake_case() {
        use super::super::entities::Extrusion;
        let model = IfcModel {
            entities: vec![mk_rich_element(Rich {
                ifc_type: "IFCWALL",
                name: "W1",
                storey: Some(0),
                location: Some([1.0, 2.0, 3.0]),
                extrusion: Some(Extrusion::rectangle(1.0, 2.0, 3.0)),
                ..Default::default()
            })],
            building_storeys: vec![Storey {
                name: "Level 1".into(),
                elevation_feet: 0.0,
            }],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 0).unwrap();
        let json = serde_json::to_string(&panel).unwrap();
        for key in [
            "\"storey\"",
            "\"elevation_label\"",
            "\"placement_rows\"",
            "\"extent_rows\"",
            "\"missing\"",
            "\"predefined_type\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        let back: ElementInfoPanel = serde_json::from_str(&json).unwrap();
        assert_eq!(back.storey.unwrap().name, "Level 1");
        assert_eq!(back.placement_rows.len(), 3);
    }

    #[test]
    fn info_panel_reports_host_and_hosted_openings() {
        let model = IfcModel {
            entities: vec![
                mk_element("Wall-80747", "IFCWALL", Some(0), None),
                mk_element("Door-80750", "IFCDOOR", Some(0), Some(0)),
                mk_element("Window-80751", "IFCWINDOW", Some(0), Some(0)),
                mk_element("Wall-80900", "IFCWALL", Some(0), None),
            ],
            building_storeys: vec![Storey {
                name: "Level 1".into(),
                elevation_feet: 0.0,
            }],
            ..Default::default()
        };

        let door = element_info_panel(&model, 1).unwrap();
        assert_eq!(
            door.host,
            Some(RelatedElement {
                entity_index: 0,
                name: "Wall-80747".into(),
                ifc_type: "IFCWALL".into(),
            })
        );
        assert!(door.hosted.is_empty());

        let wall = element_info_panel(&model, 0).unwrap();
        assert!(wall.host.is_none());
        assert_eq!(
            wall.hosted
                .iter()
                .map(|r| (r.entity_index, r.ifc_type.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "IFCDOOR"), (2, "IFCWINDOW")]
        );

        // A wall that hosts nothing reports neither side.
        let bare = element_info_panel(&model, 3).unwrap();
        assert!(bare.host.is_none());
        assert!(bare.hosted.is_empty());
    }

    #[test]
    fn info_panel_host_fields_survive_serde_as_snake_case() {
        let model = IfcModel {
            entities: vec![
                mk_element("Wall-1", "IFCWALL", None, None),
                mk_element("Door-1", "IFCDOOR", None, Some(0)),
            ],
            ..Default::default()
        };
        let panel = element_info_panel(&model, 1).unwrap();
        let json = serde_json::to_string(&panel).unwrap();
        assert!(json.contains("\"host\""));
        assert!(json.contains("\"entity_index\""));
        assert!(json.contains("\"ifc_type\""));
        let back: ElementInfoPanel = serde_json::from_str(&json).unwrap();
        assert_eq!(back.host.unwrap().name, "Wall-1");
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

    // ---- VW1-15: Schedule tests ----

    #[test]
    fn empty_model_produces_empty_schedule() {
        let s = build_schedule(&IfcModel::default());
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn schedule_has_one_row_per_building_element() {
        let model = IfcModel {
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("W1", "IFCWALL", Some(0), None),
                mk_element("S1", "IFCSLAB", Some(0), None),
                mk_element("C1", "IFCCOLUMN", Some(0), None),
            ],
            ..Default::default()
        };
        let s = build_schedule(&model);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn schedule_skips_non_building_entities() {
        let model = IfcModel {
            entities: vec![
                mk_element("W1", "IFCWALL", None, None),
                IfcEntity::Project {
                    name: Some("P".into()),
                    description: None,
                    long_name: None,
                },
            ],
            ..Default::default()
        };
        assert_eq!(build_schedule(&model).len(), 1);
    }

    #[test]
    fn schedule_rows_are_sorted_by_storey_then_name() {
        let model = IfcModel {
            building_storeys: vec![
                Storey {
                    name: "2nd".into(),
                    elevation_feet: 10.0,
                },
                Storey {
                    name: "Ground".into(),
                    elevation_feet: 0.0,
                },
            ],
            entities: vec![
                mk_element("Z-upstairs", "IFCWALL", Some(0), None),
                mk_element("A-ground", "IFCWALL", Some(1), None),
                mk_element("B-ground", "IFCWALL", Some(1), None),
            ],
            ..Default::default()
        };
        let s = build_schedule(&model);
        // Ground storey (elev 0) first, then 2nd (elev 10).
        assert_eq!(s.rows[0].name, "A-ground");
        assert_eq!(s.rows[1].name, "B-ground");
        assert_eq!(s.rows[2].name, "Z-upstairs");
    }

    #[test]
    fn schedule_resolves_material_name() {
        let model = IfcModel {
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IFCWALL".into(),
                name: "W1".into(),
                type_guid: None,
                predefined_type: None,
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
        let s = build_schedule(&model);
        assert_eq!(s.rows[0].material, "Concrete");
    }

    #[test]
    fn schedule_filter_by_ifc_type_is_case_insensitive() {
        let model = IfcModel {
            entities: vec![
                mk_element("W1", "IFCWALL", None, None),
                mk_element("S1", "IFCSLAB", None, None),
                mk_element("W2", "IFCWALL", None, None),
            ],
            ..Default::default()
        };
        let full = build_schedule(&model);
        let walls = full.filter_by_ifc_type("ifcwall");
        assert_eq!(walls.len(), 2);
    }

    #[test]
    fn schedule_csv_has_header_plus_rows() {
        let model = IfcModel {
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![mk_element("W1", "IFCWALL", Some(0), None)],
            ..Default::default()
        };
        let csv = build_schedule(&model).to_csv();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "entity_index,ifc_type,name,storey,material,guid");
        assert!(lines[1].contains("IFCWALL"));
        assert!(lines[1].contains("W1"));
        assert!(lines[1].contains("Ground"));
    }

    #[test]
    fn schedule_csv_escapes_commas_in_names() {
        let model = IfcModel {
            entities: vec![mk_element("Wall, Fancy", "IFCWALL", None, None)],
            ..Default::default()
        };
        let csv = build_schedule(&model).to_csv();
        assert!(csv.contains("\"Wall, Fancy\""));
    }

    #[test]
    fn schedule_csv_escapes_double_quotes() {
        let model = IfcModel {
            entities: vec![mk_element("He said \"hi\"", "IFCWALL", None, None)],
            ..Default::default()
        };
        let csv = build_schedule(&model).to_csv();
        // Quotes inside a quoted field are doubled per RFC 4180.
        assert!(csv.contains("\"He said \"\"hi\"\"\""));
    }

    // ---- Schedule grouped by IFC type ----

    #[test]
    fn schedule_groups_by_ifc_type_biggest_first() {
        let model = IfcModel {
            building_storeys: vec![Storey {
                name: "Level 1".into(),
                elevation_feet: 0.0,
            }],
            entities: vec![
                mk_element("W1", "IFCWALL", Some(0), None),
                mk_element("D1", "IFCDOOR", Some(0), None),
                mk_element("W2", "IFCWALL", Some(0), None),
                mk_element("W3", "IFCWALL", Some(0), None),
                mk_element("S1", "IFCSLAB", Some(0), None),
            ],
            ..Default::default()
        };
        let s = build_schedule(&model);
        assert_eq!(
            s.groups
                .iter()
                .map(|g| (g.ifc_type.as_str(), g.count))
                .collect::<Vec<_>>(),
            // 3 walls lead; IFCDOOR precedes IFCSLAB on the name tie.
            vec![("IFCWALL", 3), ("IFCDOOR", 1), ("IFCSLAB", 1)]
        );
        assert_eq!(s.groups.iter().map(|g| g.count).sum::<usize>(), s.len());
    }

    #[test]
    fn schedule_groups_carry_every_entity_index_for_scene_highlight() {
        let model = IfcModel {
            entities: vec![
                mk_element("W1", "IFCWALL", None, None),
                mk_element("S1", "IFCSLAB", None, None),
                mk_element("W2", "IFCWALL", None, None),
            ],
            ..Default::default()
        };
        let s = build_schedule(&model);
        let walls = s.groups.iter().find(|g| g.ifc_type == "IFCWALL").unwrap();
        let mut indices = walls.entity_indices.clone();
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 2]);
    }

    #[test]
    fn schedule_groups_list_the_storeys_a_type_spans() {
        let model = IfcModel {
            building_storeys: vec![
                Storey {
                    name: "Level 1".into(),
                    elevation_feet: 0.0,
                },
                Storey {
                    name: "Level 2".into(),
                    elevation_feet: 12.0,
                },
            ],
            entities: vec![
                mk_element("W1", "IFCWALL", Some(0), None),
                mk_element("W2", "IFCWALL", Some(1), None),
                mk_element("W3", "IFCWALL", None, None),
            ],
            ..Default::default()
        };
        let s = build_schedule(&model);
        let walls = &s.groups[0];
        assert_eq!(walls.count, 3);
        // Unbound elements contribute no storey name rather than "".
        assert_eq!(walls.storeys, vec!["Level 1", "Level 2"]);
    }

    #[test]
    fn schedule_filter_rebuilds_its_groups() {
        let model = IfcModel {
            entities: vec![
                mk_element("W1", "IFCWALL", None, None),
                mk_element("S1", "IFCSLAB", None, None),
                mk_element("W2", "IFCWALL", None, None),
            ],
            ..Default::default()
        };
        let walls = build_schedule(&model).filter_by_ifc_type("ifcwall");
        assert_eq!(walls.groups.len(), 1);
        assert_eq!(walls.groups[0].ifc_type, "IFCWALL");
        assert_eq!(walls.groups[0].count, 2);
    }

    #[test]
    fn empty_schedule_has_no_groups() {
        assert!(build_schedule(&IfcModel::default()).groups.is_empty());
    }

    #[test]
    fn schedule_serde_roundtrips() {
        let model = IfcModel {
            entities: vec![mk_element("W1", "IFCWALL", None, None)],
            ..Default::default()
        };
        let s = build_schedule(&model);
        let json = serde_json::to_string(&s).unwrap();
        let back: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
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
