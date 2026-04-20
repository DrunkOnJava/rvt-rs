//! Bridge from Layer-5b decoded elements into the IFC model.
//!
//! This is the "one call" integration layer: feed in `DecodedElement`
//! values (produced by the per-class decoders in `src/elements/`) and
//! get back an `IfcModel` that `write_step` will serialise as a valid
//! IFC4 STEP file with per-element `IfcWall` / `IfcSlab` / `IfcDoor`
//! / … entities wired to the storey.
//!
//! Callers bring their own decoded elements; we handle the class-name
//! → IFC-type mapping via [`super::category_map::lookup`] and the
//! spatial-containment wiring happens in the STEP writer.
//!
//! # What this does NOT do yet
//!
//! - **Geometry.** Every emitted element has no `IfcShapeRepresentation`
//!   attached. Phase 5 produces the solids / curves; then we add
//!   `IfcExtrudedAreaSolid` / `IfcFacetedBrep` here.
//! - **Materials.** Need the Material / FillPattern decoders' output
//!   threaded through — straightforward once we accept a
//!   `StylingCatalog` alongside the decoded elements.
//! - **Property sets.** Parameter decoding is L5B-53..56.
//! - **Type → instance linking.** `IfcTypeObject` is IFC-21 / IFC-22.
//!
//! The deliberate minimalism here keeps the integration surface tiny
//! so each future layer (geometry, materials, properties) attaches
//! orthogonally.

use super::IfcModel;
use super::entities::{Classification, IfcEntity, UnitAssignment};
use crate::walker::DecodedElement;

/// Input record for the bridge: one decoded element plus a display
/// name resolved by the caller (usually the decoded element's `name`
/// field, an instance tag like "Wall-1"/"Wall-2", or a category
/// label). Keeping the display-name resolution out-of-band means this
/// bridge doesn't need to know about the caller's naming scheme.
///
/// The `guid` field is optional and, when present, is carried into
/// the IFC entity's `Tag` attribute — useful for round-tripping
/// Revit element IDs.
#[derive(Debug, Clone)]
pub struct ElementInput<'a> {
    pub decoded: &'a DecodedElement,
    pub display_name: String,
    pub guid: Option<String>,
}

/// Options controlling the bridge's output.
#[derive(Debug, Clone, Default)]
pub struct BuilderOptions {
    /// Classifications to carry through (usually populated from
    /// PartAtom taxonomies).
    pub classifications: Vec<Classification>,
    /// Unit assignments (usually populated by
    /// `RvtDocExporter::build_unit_list`).
    pub units: Vec<UnitAssignment>,
    /// Project name override. If `None`, falls back to the first
    /// element's class name.
    pub project_name: Option<String>,
    /// Project description.
    pub description: Option<String>,
}

/// Build an `IfcModel` from a slice of decoded elements.
///
/// Each input element is mapped to an `IfcEntity::BuildingElement`
/// via [`super::category_map::lookup`]. Unknown classes fall back to
/// `IFCBUILDINGELEMENTPROXY` (IFC4 catch-all) rather than being
/// silently dropped — round-tripping an unknown class is more useful
/// than losing it.
pub fn build_ifc_model(inputs: &[ElementInput<'_>], options: BuilderOptions) -> IfcModel {
    let mut entities: Vec<IfcEntity> = Vec::with_capacity(inputs.len());
    for input in inputs {
        let mapping = super::category_map::lookup(&input.decoded.class);
        let ifc_type = mapping
            .map(|m| m.ifc_type)
            .unwrap_or("IFCBUILDINGELEMENTPROXY");
        entities.push(IfcEntity::BuildingElement {
            ifc_type: ifc_type.to_string(),
            name: input.display_name.clone(),
            type_guid: input.guid.clone(),
        });
    }
    let project_name = options.project_name.or_else(|| {
        inputs
            .first()
            .map(|e| format!("RVT project ({})", e.decoded.class))
    });
    IfcModel {
        project_name,
        description: options.description,
        entities,
        classifications: options.classifications,
        units: options.units,
    }
}

/// Compute counts of each IFC entity type produced by the bridge.
/// Useful for end-to-end smoke tests: "did we actually get 47
/// walls out of a file that contains 47 walls?"
pub fn entity_type_histogram(model: &IfcModel) -> std::collections::BTreeMap<String, usize> {
    let mut out: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for e in &model.entities {
        if let IfcEntity::BuildingElement { ifc_type, .. } = e {
            *out.entry(ifc_type.clone()).or_insert(0) += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walker::{DecodedElement, InstanceField};

    fn mk_decoded(class: &str) -> DecodedElement {
        DecodedElement {
            id: None,
            class: class.to_string(),
            fields: vec![("name".to_string(), InstanceField::String(class.to_string()))],
            byte_range: 0..0,
        }
    }

    #[test]
    fn build_model_maps_known_classes() {
        let wall = mk_decoded("Wall");
        let floor = mk_decoded("Floor");
        let roof = mk_decoded("Roof");
        let inputs = vec![
            ElementInput {
                decoded: &wall,
                display_name: "Wall-1".into(),
                guid: None,
            },
            ElementInput {
                decoded: &floor,
                display_name: "Slab-1".into(),
                guid: None,
            },
            ElementInput {
                decoded: &roof,
                display_name: "Roof-1".into(),
                guid: None,
            },
        ];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        assert_eq!(model.entities.len(), 3);
        let hist = entity_type_histogram(&model);
        assert_eq!(hist.get("IFCWALL"), Some(&1));
        assert_eq!(hist.get("IFCSLAB"), Some(&1));
        assert_eq!(hist.get("IFCROOF"), Some(&1));
    }

    #[test]
    fn unknown_class_falls_back_to_proxy() {
        let custom = mk_decoded("SomeCustomAutodeskExtension");
        let inputs = vec![ElementInput {
            decoded: &custom,
            display_name: "Mystery-1".into(),
            guid: None,
        }];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        let hist = entity_type_histogram(&model);
        assert_eq!(hist.get("IFCBUILDINGELEMENTPROXY"), Some(&1));
    }

    #[test]
    fn project_name_default_uses_first_class() {
        let wall = mk_decoded("Wall");
        let inputs = vec![ElementInput {
            decoded: &wall,
            display_name: "Wall-1".into(),
            guid: None,
        }];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        assert!(
            model
                .project_name
                .as_deref()
                .unwrap()
                .starts_with("RVT project")
        );
    }

    #[test]
    fn project_name_override_wins() {
        let wall = mk_decoded("Wall");
        let inputs = vec![ElementInput {
            decoded: &wall,
            display_name: "Wall-1".into(),
            guid: None,
        }];
        let opts = BuilderOptions {
            project_name: Some("Acme HQ".into()),
            ..Default::default()
        };
        let model = build_ifc_model(&inputs, opts);
        assert_eq!(model.project_name.as_deref(), Some("Acme HQ"));
    }

    #[test]
    fn empty_input_produces_empty_model() {
        let model = build_ifc_model(&[], BuilderOptions::default());
        assert!(model.entities.is_empty());
        assert!(model.project_name.is_none());
    }

    #[test]
    fn histogram_counts_multiple_of_same_type() {
        let w1 = mk_decoded("Wall");
        let w2 = mk_decoded("Wall");
        let w3 = mk_decoded("Wall");
        let inputs = vec![
            ElementInput {
                decoded: &w1,
                display_name: "Wall-N".into(),
                guid: None,
            },
            ElementInput {
                decoded: &w2,
                display_name: "Wall-E".into(),
                guid: None,
            },
            ElementInput {
                decoded: &w3,
                display_name: "Wall-S".into(),
                guid: None,
            },
        ];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        let hist = entity_type_histogram(&model);
        assert_eq!(hist.get("IFCWALL"), Some(&3));
    }

    #[test]
    fn built_model_round_trips_through_step_writer() {
        // End-to-end: decoded elements → IfcModel → STEP text. We
        // don't parse the output, but we do check that the entity
        // names we expect land in the string. This is the tightest
        // unit test of the "one call" pipeline.
        let wall = mk_decoded("Wall");
        let door = mk_decoded("Door");
        let inputs = vec![
            ElementInput {
                decoded: &wall,
                display_name: "North Wall".into(),
                guid: None,
            },
            ElementInput {
                decoded: &door,
                display_name: "Front Door".into(),
                guid: Some("DOOR-GUID-42".into()),
            },
        ];
        let model = build_ifc_model(&inputs, BuilderOptions::default());
        let step = super::super::write_step(&model);
        assert!(step.contains("IFCWALL("));
        assert!(step.contains("IFCDOOR("));
        assert!(step.contains("North Wall"));
        assert!(step.contains("Front Door"));
        assert!(step.contains("DOOR-GUID-42"));
        // Exactly one containment rel regardless of element count.
        assert_eq!(
            step.matches("IFCRELCONTAINEDINSPATIALSTRUCTURE(").count(),
            1
        );
    }
}
