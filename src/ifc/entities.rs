//! IFC entity types targeted by the exporter.
//!
//! Intentionally minimal — only the entity categories the mapping table in
//! `mod.rs` references. Expand as Layer 4c progresses and more Revit
//! classes become decodable.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum IfcEntity {
    Project {
        name: Option<String>,
        description: Option<String>,
        long_name: Option<String>,
    },
    BuildingElementType {
        ifc_type: String,
        name: String,
        description: Option<String>,
    },
    BuildingElement {
        ifc_type: String,
        name: String,
        type_guid: Option<String>,
        /// Index into `IfcModel.building_storeys` — which storey
        /// contains this element. `None` means the writer should
        /// default it to the first storey (common when the element's
        /// `level_id` wasn't resolved yet).
        #[serde(default)]
        storey_index: Option<usize>,
        /// Index into `IfcModel.materials` — which material this
        /// element associates with. `None` means the element has no
        /// material, so no IfcRelAssociatesMaterial gets emitted for
        /// it (IFC4 treats material as optional for every concrete
        /// type).
        #[serde(default)]
        material_index: Option<usize>,
        /// Property-set name + list of (name, value) pairs to emit
        /// as IfcPropertySet → IfcPropertySingleValue → element via
        /// IfcRelDefinesByProperties. Empty = no property set. The
        /// property-set name should follow Revit / IFC convention:
        /// usually "Pset_RevitType_{ClassName}" to match what the
        /// Autodesk exporter produces.
        #[serde(default)]
        property_set: Option<PropertySet>,
        /// Element origin in feet, expressed in the project's
        /// coordinate system. When `Some`, the writer emits a
        /// unique IFCCARTESIANPOINT + IFCAXIS2PLACEMENT3D for this
        /// element (ft → m conversion at emit time). When `None`,
        /// the element uses the shared identity placement — fine
        /// until geometry lands and positions start mattering.
        #[serde(default)]
        location_feet: Option<[f64; 3]>,
        /// Element rotation about the Z (up) axis, in radians. Only
        /// consulted when `location_feet` is `Some` — the placement
        /// with a non-default X-axis direction needs a unique
        /// IFCDIRECTION.
        #[serde(default)]
        rotation_radians: Option<f64>,
        /// Optional rectangular extrusion geometry. When `Some`, the
        /// writer emits the IfcExtrudedAreaSolid chain and wires the
        /// element's Representation slot to it. When `None`, the
        /// element stays geometry-free (Representation = $).
        #[serde(default)]
        extrusion: Option<Extrusion>,
        /// Index into `IfcModel.entities` naming a host BuildingElement
        /// (typically the wall that contains this door/window). When
        /// set, the writer emits an IfcOpeningElement (same shape as
        /// this element's extrusion) + IfcRelVoidsElement (host →
        /// opening) + IfcRelFillsElement (opening → this element).
        /// The host must already be in `entities` before this element
        /// and must itself be a BuildingElement with an extrusion
        /// (otherwise the void subtracts from nothing).
        #[serde(default)]
        host_element_index: Option<usize>,
    },
    TypeObject {
        name: String,
        shape_representations: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub source: ClassificationSource,
    pub edition: Option<String>,
    pub items: Vec<ClassificationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassificationSource {
    OmniClass,
    Uniformat,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationItem {
    pub code: String,
    pub name: Option<String>,
}

/// Minimal rectangular-extrusion geometry descriptor for a
/// BuildingElement. The writer turns this into an
/// `IfcRectangleProfileDef` + `IfcExtrudedAreaSolid` +
/// `IfcShapeRepresentation` + `IfcProductDefinitionShape` chain
/// and points the element's Representation slot at the chain.
///
/// All values in feet; the writer converts to metres at emit
/// boundary (ft × 0.3048). The profile is centred on the element
/// origin and the extrusion runs +Z.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extrusion {
    /// Profile width (local X, in feet). For a wall = length along
    /// its location line. For a slab = plan dimension in X.
    pub width_feet: f64,
    /// Profile depth (local Y, in feet). For a wall = thickness.
    /// For a slab = plan dimension in Y.
    pub depth_feet: f64,
    /// Extrusion height (local Z, in feet). For a wall = height;
    /// for a slab = slab thickness.
    pub height_feet: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitAssignment {
    /// e.g. "autodesk.unit.unit:millimeters-1.0.1"
    pub forge_identifier: String,
    /// IFC base unit name, e.g. "MILLI" + "METRE"
    pub ifc_mapping: Option<String>,
}

/// A named collection of typed property values attached to a
/// BuildingElement. Emits as IfcPropertySet → IfcPropertySingleValue
/// → IfcRelDefinesByProperties in the STEP writer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySet {
    /// Property-set name. Convention: "Pset_WallCommon",
    /// "Pset_DoorCommon", or "Pset_RevitType_{ClassName}".
    pub name: String,
    pub properties: Vec<Property>,
}

/// A single property inside a [`PropertySet`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: PropertyValue,
}

/// IFC4 IfcValue subtypes we surface from Revit decoded fields.
/// Maps directly to the `NominalValue` slot of IfcPropertySingleValue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum PropertyValue {
    /// Free-form text — `IfcText`.
    Text(String),
    /// Integer count — `IfcInteger`.
    Integer(i64),
    /// Floating-point real — `IfcReal`.
    Real(f64),
    /// Boolean — `IfcBoolean`.
    Boolean(bool),
    /// Length measurement in feet (writer converts to metres).
    /// Maps to `IfcLengthMeasure` with project length unit.
    LengthFeet(f64),
    /// Angle in radians. Maps to `IfcPlaneAngleMeasure`.
    AngleRadians(f64),
}

impl PropertyValue {
    /// Emit the STEP-level IfcValue constructor for this value.
    /// Used by the writer; exposed for testing.
    pub fn to_step(&self) -> String {
        match self {
            PropertyValue::Text(s) => format!("IFCTEXT('{}')", escape_step_string(s)),
            PropertyValue::Integer(n) => format!("IFCINTEGER({n})"),
            PropertyValue::Real(v) => format!("IFCREAL({v:.6})"),
            PropertyValue::Boolean(b) => {
                format!("IFCBOOLEAN(.{})", if *b { "T" } else { "F" })
            }
            PropertyValue::LengthFeet(ft) => {
                // Convert to metres at emit time (project length unit).
                let metres = ft * 0.3048;
                format!("IFCLENGTHMEASURE({metres:.6})")
            }
            PropertyValue::AngleRadians(r) => format!("IFCPLANEANGLEMEASURE({r:.6})"),
        }
    }
}

/// Minimal STEP string escape — apostrophes doubled, backslashes
/// doubled. Non-ASCII escape isn't needed here because the writer's
/// full `escape()` is used for BuildingElement names; property
/// values are typically numeric or short ASCII. Keeping the escape
/// local to entities.rs avoids a cross-module dependency.
fn escape_step_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}
