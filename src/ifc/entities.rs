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
        /// Index into `IfcModel.material_layer_sets` — the layered
        /// assembly (gypsum / insulation / sheathing / …) that
        /// makes up this element (IFC-28). Used for walls, slabs,
        /// roofs, and ceilings that have a real multi-layer
        /// composition. When set, the writer emits
        /// `IfcMaterialLayerSet` + `IfcMaterialLayerSetUsage` + an
        /// `IfcRelAssociatesMaterial` pointing at the layer-set
        /// usage — *instead of* the single-material path driven by
        /// `material_index`. When both are set, the layer set
        /// wins (single-material falls back to the first layer).
        #[serde(default)]
        material_layer_set_index: Option<usize>,
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

/// One layer of a compound building-element material assembly
/// (IFC-28). `thickness_feet` is the physical thickness (writer
/// converts to metres at emit time). `material_index` points into
/// `IfcModel.materials` — the material filling this layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLayer {
    pub material_index: usize,
    pub thickness_feet: f64,
    /// Optional per-layer name. When `Some`, emitted as the
    /// `IfcMaterialLayer.Name` attribute. Revit's convention is
    /// layer names like "Finish - Face Layer", "Structure", "Air
    /// Gap", "Insulation" — useful for downstream BIM tools that
    /// surface them in schedules.
    #[serde(default)]
    pub name: Option<String>,
}

/// An ordered set of [`MaterialLayer`]s representing the compound
/// composition of a wall / floor / roof / ceiling (IFC-28). Maps
/// to IFC4 `IfcMaterialLayerSet` + (via [`BuildingElement::material_layer_set_index`])
/// `IfcMaterialLayerSetUsage`.
///
/// `name` is the set-level label ("Generic - 6\" Wall", "Ext - CMU").
/// Revit's exterior wall types often carry 3-5 layers; interior
/// partitions are usually 2-3. The ordering matters: IFC4
/// interprets `[0]` as the outermost layer (exterior or top side)
/// with subsequent layers stacked inward.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLayerSet {
    pub name: String,
    pub layers: Vec<MaterialLayer>,
    /// Optional description; emitted as `IfcMaterialLayerSet.Description`.
    #[serde(default)]
    pub description: Option<String>,
}

impl MaterialLayerSet {
    /// Total thickness in feet (sum of layer thicknesses). Useful
    /// for sanity-checking that the declared wall thickness matches
    /// the layer-set composition.
    pub fn total_thickness_feet(&self) -> f64 {
        self.layers.iter().map(|l| l.thickness_feet).sum()
    }
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
///
/// Quantity variants (`AreaSquareFeet`, `VolumeCubicFeet`, `CountValue`,
/// `TimeSeconds`, `MassPounds`) are the measurement-flavoured siblings
/// of the primitive variants. They emit as the matching
/// `IfcAreaMeasure` / `IfcVolumeMeasure` / `IfcCountMeasure` / etc.
/// constructors — semantically they correspond to the IFC4 `IfcQuantity*`
/// family, but we route them through the existing
/// `IfcPropertySingleValue` carrier so the writer doesn't need a
/// parallel `IfcElementQuantity` path yet (tracked separately).
/// Feet / cubic-feet / pounds inputs are converted to metres /
/// cubic-metres / kilograms at emit time so the STEP output is
/// unit-consistent with the project-level `IfcUnitAssignment`.
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
    /// Area in square feet (writer converts to square metres).
    /// Maps to `IfcAreaMeasure`. (IFC-32)
    AreaSquareFeet(f64),
    /// Volume in cubic feet (writer converts to cubic metres).
    /// Maps to `IfcVolumeMeasure`. (IFC-32)
    VolumeCubicFeet(f64),
    /// Unitless discrete count — occupancy, rebar count, fixture count.
    /// Maps to `IfcCountMeasure`. (IFC-32)
    CountValue(i64),
    /// Time measurement in seconds. Maps to `IfcTimeMeasure`. (IFC-32)
    TimeSeconds(f64),
    /// Mass in pounds (writer converts to kilograms).
    /// Maps to `IfcMassMeasure`. (IFC-32)
    MassPounds(f64),
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
            PropertyValue::AreaSquareFeet(sqft) => {
                // 1 ft² = 0.09290304 m² (exact, from international foot).
                let sqm = sqft * 0.09290304;
                format!("IFCAREAMEASURE({sqm:.6})")
            }
            PropertyValue::VolumeCubicFeet(cuft) => {
                // 1 ft³ = 0.028316846592 m³ (exact).
                let cum = cuft * 0.028316846592;
                format!("IFCVOLUMEMEASURE({cum:.6})")
            }
            PropertyValue::CountValue(n) => format!("IFCCOUNTMEASURE({n})"),
            PropertyValue::TimeSeconds(s) => format!("IFCTIMEMEASURE({s:.6})"),
            PropertyValue::MassPounds(lb) => {
                // 1 lb = 0.45359237 kg (exact, international avoirdupois pound).
                let kg = lb * 0.45359237;
                format!("IFCMASSMEASURE({kg:.6})")
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_value_to_step_primitives() {
        assert_eq!(PropertyValue::Integer(42).to_step(), "IFCINTEGER(42)");
        assert_eq!(PropertyValue::Real(1.25).to_step(), "IFCREAL(1.250000)");
        assert_eq!(PropertyValue::Boolean(true).to_step(), "IFCBOOLEAN(.T)");
        assert_eq!(PropertyValue::Boolean(false).to_step(), "IFCBOOLEAN(.F)");
        assert_eq!(
            PropertyValue::Text("hello".into()).to_step(),
            "IFCTEXT('hello')"
        );
        assert_eq!(
            PropertyValue::Text("it's".into()).to_step(),
            "IFCTEXT('it''s')"
        );
    }

    #[test]
    fn property_value_to_step_length_and_angle() {
        // 10 ft = 3.048 m.
        assert_eq!(
            PropertyValue::LengthFeet(10.0).to_step(),
            "IFCLENGTHMEASURE(3.048000)"
        );
        // π/2 rad comes out to ~1.570796.
        let step = PropertyValue::AngleRadians(std::f64::consts::FRAC_PI_2).to_step();
        assert!(step.starts_with("IFCPLANEANGLEMEASURE(1.570796"));
    }

    /// IFC-32: quantity variants emit IfcAreaMeasure / IfcVolumeMeasure
    /// / IfcCountMeasure / IfcTimeMeasure / IfcMassMeasure, with the
    /// Imperial → SI conversion applied at emit time so downstream
    /// tools see unit-consistent output against the project's SI
    /// `IfcUnitAssignment`.
    /// IFC-28: MaterialLayerSet totals its layer thicknesses.
    #[test]
    fn material_layer_set_total_thickness() {
        let lset = MaterialLayerSet {
            name: "Ext - Generic 8\" Wall".into(),
            description: None,
            layers: vec![
                MaterialLayer {
                    material_index: 0,
                    thickness_feet: 5.0 / 12.0, // 5"
                    name: Some("Finish".into()),
                },
                MaterialLayer {
                    material_index: 1,
                    thickness_feet: 2.0 / 12.0, // 2"
                    name: Some("Structure".into()),
                },
                MaterialLayer {
                    material_index: 2,
                    thickness_feet: 1.0 / 12.0, // 1"
                    name: Some("Air Gap".into()),
                },
            ],
        };
        let total = lset.total_thickness_feet();
        assert!((total - (8.0 / 12.0)).abs() < 1e-9);
    }

    #[test]
    fn property_value_to_step_quantities() {
        // 1 ft² = 0.09290304 m² exact.
        assert_eq!(
            PropertyValue::AreaSquareFeet(1.0).to_step(),
            "IFCAREAMEASURE(0.092903)"
        );
        // 100 ft² = 9.290304 m².
        assert_eq!(
            PropertyValue::AreaSquareFeet(100.0).to_step(),
            "IFCAREAMEASURE(9.290304)"
        );

        // 1 ft³ = 0.028316846592 m³ exact.
        assert_eq!(
            PropertyValue::VolumeCubicFeet(1.0).to_step(),
            "IFCVOLUMEMEASURE(0.028317)"
        );

        // Counts are unitless integers.
        assert_eq!(
            PropertyValue::CountValue(12).to_step(),
            "IFCCOUNTMEASURE(12)"
        );

        // Time in seconds is already SI.
        assert_eq!(
            PropertyValue::TimeSeconds(3.0).to_step(),
            "IFCTIMEMEASURE(3.000000)"
        );

        // 1 lb = 0.45359237 kg exact.
        assert_eq!(
            PropertyValue::MassPounds(1.0).to_step(),
            "IFCMASSMEASURE(0.453592)"
        );
    }
}
