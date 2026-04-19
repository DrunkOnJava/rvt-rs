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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitAssignment {
    /// e.g. "autodesk.unit.unit:millimeters-1.0.1"
    pub forge_identifier: String,
    /// IFC base unit name, e.g. "MILLI" + "METRE"
    pub ifc_mapping: Option<String>,
}
