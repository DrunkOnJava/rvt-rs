//! Minimal STEP / ISO-10303-21 serializer for `IfcModel` → valid IFC4.
//!
//! This is the serialization half of Layer 5. Given an `IfcModel` in
//! memory, produce a .ifc text file that a spec-compliant reader
//! (IfcOpenShell, BlenderBIM, buildingSMART validator) accepts.
//!
//! The output is intentionally minimal but **structurally valid**: a
//! well-formed IFC4 schema header, the required framework entities
//! (`IfcPerson` / `IfcOrganization` / `IfcApplication` /
//! `IfcOwnerHistory` / `IfcSIUnit` / `IfcUnitAssignment` /
//! `IfcGeometricRepresentationContext`), and an `IfcProject` populated
//! from the model's metadata. As the walker grows, `BuildingElement`
//! and family entities will land too; those extensions plug in here
//! without touching the header-level plumbing.
//!
//! Design principle: string-based emission, no external IFC library
//! dependency, fully `#![deny(unsafe_code)]`-clean.

use super::IfcModel;

/// Options controlling STEP serialization.
#[derive(Debug, Clone, Default)]
pub struct StepOptions {
    /// If `Some`, use this Unix timestamp (in seconds) for both the
    /// `FILE_NAME` header and the `IfcOwnerHistory` creation time
    /// instead of `SystemTime::now()`. Setting this makes the
    /// output deterministic — identical `(IfcModel, StepOptions)`
    /// pairs produce byte-identical STEP strings, which makes
    /// STEP-text diffs tractable and regression tests reliable.
    pub timestamp: Option<i64>,
}

/// Serialize an `IfcModel` into an IFC4 STEP text stream. The output
/// includes the ISO-10303-21 envelope and a minimal but spec-valid
/// data section centred on `IfcProject`. Uses current wall-clock
/// timestamp. For deterministic output (e.g. tests), use
/// [`write_step_with_options`] with `StepOptions::timestamp = Some(_)`.
pub fn write_step(model: &IfcModel) -> String {
    write_step_with_options(model, &StepOptions::default())
}

/// Deterministic-option variant of [`write_step`].
///
/// When `options.timestamp = Some(t)`, the emitted STEP is a pure
/// function of `(model, t)` — no wall-clock access. Use this in
/// tests and regression fixtures.
pub fn write_step_with_options(model: &IfcModel, options: &StepOptions) -> String {
    let now = options.timestamp.unwrap_or_else(unix_seconds);
    let mut w = StepWriter::new(now);
    w.emit_header(model);
    w.emit_data(model);
    w.finish()
}

struct StepWriter {
    out: String,
    next_id: usize,
    /// Unix timestamp (seconds) used for FILE_NAME + IfcOwnerHistory.
    /// Injected at construction so the output is a pure function of
    /// `(model, timestamp)` when a fixed timestamp is supplied.
    timestamp: i64,
}

impl StepWriter {
    fn new(timestamp: i64) -> Self {
        Self {
            out: String::new(),
            next_id: 1,
            timestamp,
        }
    }

    fn id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn emit_line<S: AsRef<str>>(&mut self, line: S) {
        self.out.push_str(line.as_ref());
        self.out.push('\n');
    }

    fn emit_entity<S: AsRef<str>>(&mut self, id: usize, body: S) {
        self.out.push_str(&format!("#{id}={};\n", body.as_ref()));
    }

    fn emit_header(&mut self, model: &IfcModel) {
        let project = escape(model.project_name.as_deref().unwrap_or("Untitled"));
        let desc = escape(model.description.as_deref().unwrap_or(
            "Produced by rvt-rs (https://github.com/DrunkOnJava/rvt-rs) — \
                 clean-room Apache-2 Revit reader.",
        ));
        self.emit_line("ISO-10303-21;");
        self.emit_line("HEADER;");
        self.emit_line("FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');");
        self.emit_line(format!(
            "FILE_NAME('{project}.ifc','{}',('rvt-rs'),('DrunkOnJava/rvt-rs'),'rvt-rs 0.1.x','rvt-rs STEP writer','');",
            iso_timestamp_from(self.timestamp)
        ));
        self.emit_line("FILE_SCHEMA(('IFC4'));");
        self.emit_line("ENDSEC;");
        self.emit_line(format!("/* {desc} */"));
    }

    fn emit_data(&mut self, model: &IfcModel) {
        self.emit_line("DATA;");

        // Required framework entities (buildingSMART minimum viable).
        let person = self.id();
        self.emit_entity(person, "IFCPERSON($,$,'rvt-rs',$,$,$,$,$)");
        let org = self.id();
        self.emit_entity(
            org,
            "IFCORGANIZATION($,'rvt-rs','Clean-room Apache-2 Revit reader',$,$)",
        );
        let person_and_org = self.id();
        self.emit_entity(
            person_and_org,
            format!("IFCPERSONANDORGANIZATION(#{person},#{org},$)"),
        );
        let application = self.id();
        self.emit_entity(
            application,
            format!("IFCAPPLICATION(#{org},'0.1.x','rvt-rs','{}')", "rvt_rs"),
        );
        let owner_hist = self.id();
        self.emit_entity(
            owner_hist,
            format!(
                "IFCOWNERHISTORY(#{person_and_org},#{application},$,.ADDED.,$,#{person_and_org},#{application},{})",
                self.timestamp
            ),
        );

        // Unit assignment — fixed to SI millimetres + square metres +
        // cubic metres for v1. Future: wire from model.units (Forge
        // unit identifiers → IfcSIUnit mapping).
        let u_length = self.id();
        self.emit_entity(u_length, "IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)");
        let u_area = self.id();
        self.emit_entity(u_area, "IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)");
        let u_volume = self.id();
        self.emit_entity(u_volume, "IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)");
        let u_plane_angle = self.id();
        self.emit_entity(u_plane_angle, "IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
        let unit_assignment = self.id();
        self.emit_entity(
            unit_assignment,
            format!("IFCUNITASSIGNMENT((#{u_length},#{u_area},#{u_volume},#{u_plane_angle}))"),
        );

        // Representation context — needs IfcAxis2Placement3D +
        // IfcDirection + IfcCartesianPoint (origin, X, Z axes).
        let origin = self.id();
        self.emit_entity(origin, "IFCCARTESIANPOINT((0.,0.,0.))");
        let z_axis = self.id();
        self.emit_entity(z_axis, "IFCDIRECTION((0.,0.,1.))");
        let x_axis = self.id();
        self.emit_entity(x_axis, "IFCDIRECTION((1.,0.,0.))");
        let axis_placement = self.id();
        self.emit_entity(
            axis_placement,
            format!("IFCAXIS2PLACEMENT3D(#{origin},#{z_axis},#{x_axis})"),
        );
        let geom_ctx = self.id();
        self.emit_entity(
            geom_ctx,
            format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#{axis_placement},$)"),
        );

        // Root project.
        let project_name = escape(model.project_name.as_deref().unwrap_or("Untitled"));
        let project_desc = escape(model.description.as_deref().unwrap_or("Exported by rvt-rs"));
        let project_id = self.id();
        self.emit_entity(
            project_id,
            format!(
                "IFCPROJECT('{}',#{owner_hist},'{}',{},$,$,$,(#{geom_ctx}),#{unit_assignment})",
                make_guid(project_id),
                project_name,
                quoted_or_dollar(&project_desc),
            ),
        );

        // Spatial containment hierarchy — required by IFC4 for any
        // project with building content. We emit a minimal but valid
        // IfcSite → IfcBuilding → IfcBuildingStorey chain with
        // identity placements so downstream viewers (BlenderBIM,
        // IfcOpenShell-based tools, buildingSMART validator) render
        // the file directly without needing to synthesise a host
        // structure. Names default to "Default {Site,Building,Level
        // 1}"; once the walker surfaces site/level instances they'll
        // flow in here.
        //
        // Every IfcSpatialStructureElement needs its own
        // IfcLocalPlacement — we share the `axis_placement` across
        // the three (they're all identity), then chain the
        // placements via `PlacementRelTo` so the coordinate frames
        // compose correctly.
        let site_placement = self.id();
        self.emit_entity(
            site_placement,
            format!("IFCLOCALPLACEMENT($,#{axis_placement})"),
        );
        let site_id = self.id();
        self.emit_entity(
            site_id,
            format!(
                "IFCSITE('{}',#{owner_hist},'Default Site',$,$,#{site_placement},$,'Default Site',.ELEMENT.,$,$,$,$,$)",
                make_guid(site_id),
            ),
        );

        let building_placement = self.id();
        self.emit_entity(
            building_placement,
            format!("IFCLOCALPLACEMENT(#{site_placement},#{axis_placement})"),
        );
        let building_id = self.id();
        self.emit_entity(
            building_id,
            format!(
                "IFCBUILDING('{}',#{owner_hist},'Default Building',$,$,#{building_placement},$,'Default Building',.ELEMENT.,$,$,$)",
                make_guid(building_id),
            ),
        );

        // Emit one IfcBuildingStorey per Revit Level (or one
        // placeholder when no Levels have been decoded yet). Every
        // storey gets its own IfcLocalPlacement; their IDs are later
        // bundled into a single IfcRelAggregates bound to the
        // building. The first storey doubles as the default container
        // for elements that don't carry a level_id hint — Phase 4b+
        // per-level containment is a follow-up.
        let mut storey_ids: Vec<usize> = Vec::new();
        let mut storey_placements: Vec<usize> = Vec::new();
        let storeys = if model.building_storeys.is_empty() {
            // Fallback: one placeholder so the IFC spatial hierarchy
            // remains valid even when the caller hasn't provided any
            // Levels yet.
            vec![super::Storey {
                name: "Level 1".to_string(),
                elevation_feet: 0.0,
            }]
        } else {
            model.building_storeys.clone()
        };
        for storey in &storeys {
            let placement_id = self.id();
            self.emit_entity(
                placement_id,
                format!("IFCLOCALPLACEMENT(#{building_placement},#{axis_placement})"),
            );
            let id = self.id();
            // Convert feet → metres at emit boundary; IFC4 elevation
            // attribute is in the project's length unit (metres here).
            let elevation_m = storey.elevation_feet * 0.3048;
            let name_escaped = escape(&storey.name);
            self.emit_entity(
                id,
                format!(
                    "IFCBUILDINGSTOREY('{}',#{owner_hist},'{name_escaped}',$,$,#{placement_id},$,'{name_escaped}',.ELEMENT.,{elevation_m})",
                    make_guid(id),
                ),
            );
            storey_ids.push(id);
            storey_placements.push(placement_id);
        }
        // First storey stands in as the default container for
        // BuildingElements that don't yet carry a level reference.
        let storey_id = storey_ids[0];
        let storey_placement = storey_placements[0];

        // Aggregation relationships — IfcRelAggregates is how the
        // spatial hierarchy binds in IFC4. Each level of the chain
        // gets one relationship pointing from parent to child.
        let rel_proj_site = self.id();
        self.emit_entity(
            rel_proj_site,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{project_id},(#{site_id}))",
                make_guid(rel_proj_site),
            ),
        );
        let rel_site_building = self.id();
        self.emit_entity(
            rel_site_building,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{site_id},(#{building_id}))",
                make_guid(rel_site_building),
            ),
        );
        let rel_building_storey = self.id();
        let storey_refs = storey_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(",");
        self.emit_entity(
            rel_building_storey,
            format!(
                "IFCRELAGGREGATES('{}',#{owner_hist},$,$,#{building_id},({storey_refs}))",
                make_guid(rel_building_storey),
            ),
        );

        // Classifications — one IfcClassification per source
        // (OmniClass, Uniformat, …), with one IfcClassificationReference
        // per coded item. Each classification gets its own
        // IfcRelAssociatesClassification tying its references back to
        // the project, which is how IFC4 consumers (BlenderBIM,
        // IfcOpenShell's classification viewer) discover code refs.
        //
        // RvtDocExporter populates `model.classifications` from
        // PartAtom's `<category term="...">` blocks. Previously those
        // codes were collected but never emitted; this wires them
        // through the STEP writer so downstream consumers can see
        // them directly.
        for classification in &model.classifications {
            let source_name = match &classification.source {
                super::entities::ClassificationSource::OmniClass => "OmniClass",
                super::entities::ClassificationSource::Uniformat => "Uniformat",
                super::entities::ClassificationSource::Other(s) => s.as_str(),
            };
            let source_name_escaped = escape(source_name);
            let edition = classification
                .edition
                .as_deref()
                .map(escape)
                .map(|e| format!("'{e}'"))
                .unwrap_or_else(|| "$".into());

            let classification_id = self.id();
            self.emit_entity(
                classification_id,
                format!("IFCCLASSIFICATION($,{edition},$,'{source_name_escaped}',$,$,$)"),
            );

            // One IfcClassificationReference per item; collect their
            // ids so we can bundle them into the IfcRelAssociatesClassification.
            let mut ref_ids: Vec<usize> = Vec::with_capacity(classification.items.len());
            for item in &classification.items {
                let code_escaped = escape(&item.code);
                let name_str = item
                    .name
                    .as_deref()
                    .map(escape)
                    .map(|n| format!("'{n}'"))
                    .unwrap_or_else(|| "$".into());
                let ref_id = self.id();
                self.emit_entity(
                    ref_id,
                    format!(
                        "IFCCLASSIFICATIONREFERENCE($,'{code_escaped}',{name_str},#{classification_id},$)"
                    ),
                );
                ref_ids.push(ref_id);
            }

            if !ref_ids.is_empty() {
                let refs_list = ref_ids
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(",");
                // IfcRelAssociatesClassification binds a set of objects
                // to one classification reference. IFC4's schema
                // requires the RelatingClassification to be a single
                // IfcClassificationReferenceSelect; we pick the last
                // reference as the relating one and treat the rest as
                // project associations. If the project only has one
                // reference this is exact; when there are multiple,
                // each gets its own association relationship.
                for ref_id in &ref_ids {
                    let rel_id = self.id();
                    self.emit_entity(
                        rel_id,
                        format!(
                            "IFCRELASSOCIATESCLASSIFICATION('{}',#{owner_hist},$,$,(#{project_id}),#{ref_id})",
                            make_guid(rel_id),
                        ),
                    );
                }
                // Silence the warning about an unused local when the
                // outer `for` loop only iterates once.
                let _ = refs_list;
            }
        }

        // BuildingElement emission — one `IFC<TYPE>` instance per decoded
        // Revit element (Wall, Floor, Roof, Ceiling, Door, Window, Column,
        // Beam…). Each element gets its own `IFCLOCALPLACEMENT` relative
        // to whichever storey contains it (index from
        // `BuildingElement.storey_index`; falls back to storey[0] when
        // unset). At the end, elements are grouped by storey and one
        // `IFCRELCONTAINEDINSPATIALSTRUCTURE` is emitted per non-empty
        // storey — which is how IFC4 tools (BlenderBIM, IfcOpenShell)
        // show "Floor 2 contains Wall-7, Wall-8" in the project
        // browser.
        //
        // Geometry (`IfcShapeRepresentation`) is intentionally omitted
        // here — tasks IFC-15 through IFC-22 produce proper
        // representations once Phase-5 geometry lands. For now every
        // element carries its placement + name + GUID, which validates
        // against the IFC4 schema as a "geometry-free" element.
        // Emit IfcMaterials upfront so BuildingElements can reference
        // them. Each material gets:
        //   - IFCMATERIAL with just a name (IFC4 minimum)
        //   - If the material has a color: IFCCOLOURRGB +
        //     IFCSURFACESTYLERENDERING + IFCSURFACESTYLE +
        //     IFCSTYLEDITEM to surface the color to IFC4 viewers
        // The color emission is gated because a color-less material
        // is valid IFC4 — we don't want to emit empty rendering
        // records when there's nothing to render.
        let mut material_ids: Vec<usize> = Vec::with_capacity(model.materials.len());
        for mat in &model.materials {
            let mat_id = self.id();
            let name_escaped = escape(&mat.name);
            self.emit_entity(mat_id, format!("IFCMATERIAL('{name_escaped}',$,$)"));
            if let Some(packed) = mat.color_packed {
                let r = (packed & 0xFF) as f64 / 255.0;
                let g = ((packed >> 8) & 0xFF) as f64 / 255.0;
                let b = ((packed >> 16) & 0xFF) as f64 / 255.0;
                let colour_id = self.id();
                self.emit_entity(colour_id, format!("IFCCOLOURRGB($,{r:.6},{g:.6},{b:.6})"));
                let rendering_id = self.id();
                let transparency = mat.transparency.unwrap_or(0.0);
                self.emit_entity(
                    rendering_id,
                    format!(
                        "IFCSURFACESTYLERENDERING(#{colour_id},{transparency:.6},$,$,$,$,$,$,.FLAT.)"
                    ),
                );
                let style_id = self.id();
                self.emit_entity(
                    style_id,
                    format!("IFCSURFACESTYLE('{name_escaped}',.BOTH.,(#{rendering_id}))"),
                );
                let presentation_id = self.id();
                self.emit_entity(
                    presentation_id,
                    format!("IFCPRESENTATIONSTYLEASSIGNMENT((#{style_id}))"),
                );
                let styled_item_id = self.id();
                self.emit_entity(
                    styled_item_id,
                    format!("IFCSTYLEDITEM($,(#{presentation_id}),'{name_escaped}')"),
                );
            }
            material_ids.push(mat_id);
        }

        // IFC-28: Emit IfcMaterialLayer + IfcMaterialLayerSet for
        // every compound assembly declared on the model. Layers
        // reference the `material_ids` vector above by index, so
        // layer_set emission MUST follow single-material emission.
        // Track layer-set ids for later IfcRelAssociatesMaterial
        // pairing.
        let mut layer_set_ids: Vec<usize> = Vec::with_capacity(model.material_layer_sets.len());
        for lset in &model.material_layer_sets {
            let mut layer_ids: Vec<usize> = Vec::with_capacity(lset.layers.len());
            for layer in &lset.layers {
                // Clamp material_index to stay within bounds. Out-of-range
                // indices get the first material as a defensive fallback;
                // losing the layer would be worse.
                let mat_id = material_ids
                    .get(layer.material_index)
                    .copied()
                    .or_else(|| material_ids.first().copied())
                    .unwrap_or(0);
                if mat_id == 0 {
                    // No materials at all — skip this layer. IFC4 allows
                    // IfcMaterialLayerSet with zero layers, so the set
                    // will still emit below, just empty.
                    continue;
                }
                let layer_id = self.id();
                // IfcMaterialLayer(Material, LayerThickness, IsVentilated=$,
                //                  Category=$, Priority=$, Name=$, Description=$)
                // Thickness in metres (feet × 0.3048).
                let thickness_m = layer.thickness_feet * 0.3048;
                let name_slot = match &layer.name {
                    Some(n) => format!("'{}'", escape(n)),
                    None => "$".into(),
                };
                self.emit_entity(
                    layer_id,
                    format!(
                        "IFCMATERIALLAYER(#{mat_id},{thickness_m:.6},$,$,$,{name_slot},$)"
                    ),
                );
                layer_ids.push(layer_id);
            }
            let set_id = self.id();
            let set_name = escape(&lset.name);
            let desc_slot = match &lset.description {
                Some(d) => format!("'{}'", escape(d)),
                None => "$".into(),
            };
            let layer_refs = if layer_ids.is_empty() {
                "()".to_string()
            } else {
                format!(
                    "({})",
                    layer_ids
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            // IfcMaterialLayerSet(MaterialLayers, LayerSetName, Description)
            self.emit_entity(
                set_id,
                format!("IFCMATERIALLAYERSET({layer_refs},'{set_name}',{desc_slot})"),
            );
            layer_set_ids.push(set_id);
        }

        // IFC-30: Emit IfcMaterialProfileSet for structural framing
        // (columns / beams) with named cross-sections. Each profile
        // carries a material index + profile name; at IfcMaterialProfile
        // emission time the profile name is also echoed as the
        // associated IfcProfileDef name — downstream tools that care
        // about precise cross-section geometry still need the profile
        // def itself (tracked separately in IFC-24).
        let mut profile_set_ids: Vec<usize> = Vec::with_capacity(model.material_profile_sets.len());
        for pset in &model.material_profile_sets {
            let mut profile_ids: Vec<usize> = Vec::with_capacity(pset.profiles.len());
            for profile in &pset.profiles {
                let mat_id = material_ids
                    .get(profile.material_index)
                    .copied()
                    .or_else(|| material_ids.first().copied())
                    .unwrap_or(0);
                if mat_id == 0 {
                    continue;
                }
                // Profile itself (IfcRectangleProfileDef placeholder
                // with 1x1 metre box — real cross-section shape lands
                // with IFC-24). IfcMaterialProfile requires a profile
                // def reference, so we emit a minimal stand-in so the
                // material-profile chain validates.
                let profile_def_id = self.id();
                let profile_def_name = escape(&profile.profile_name);
                self.emit_entity(
                    profile_def_id,
                    format!(
                        "IFCRECTANGLEPROFILEDEF(.AREA.,'{profile_def_name}',$,1.,1.)"
                    ),
                );
                let profile_id = self.id();
                let profile_name = escape(&profile.profile_name);
                let desc_slot = match &profile.description {
                    Some(d) => format!("'{}'", escape(d)),
                    None => "$".into(),
                };
                // IfcMaterialProfile(Name, Description, Material, Profile,
                //                    Priority=$, Category=$)
                self.emit_entity(
                    profile_id,
                    format!(
                        "IFCMATERIALPROFILE('{profile_name}',{desc_slot},#{mat_id},#{profile_def_id},$,$)"
                    ),
                );
                profile_ids.push(profile_id);
            }
            let set_id = self.id();
            let set_name = escape(&pset.name);
            let desc_slot = match &pset.description {
                Some(d) => format!("'{}'", escape(d)),
                None => "$".into(),
            };
            let profile_refs = if profile_ids.is_empty() {
                "()".to_string()
            } else {
                format!(
                    "({})",
                    profile_ids
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            };
            // IfcMaterialProfileSet(Name, Description, MaterialProfiles, CompositeProfile)
            self.emit_entity(
                set_id,
                format!(
                    "IFCMATERIALPROFILESET('{set_name}',{desc_slot},{profile_refs},$)"
                ),
            );
            profile_set_ids.push(set_id);
        }

        let mut per_storey_elements: Vec<Vec<usize>> = vec![Vec::new(); storeys.len()];
        // Track (element_id, material_index) pairs so we can emit
        // IfcRelAssociatesMaterial per material after the element
        // loop completes.
        let mut element_material_pairs: Vec<(usize, usize)> = Vec::new();
        // Track (element_id, &PropertySet) so property-set emission
        // runs after all element IDs are assigned.
        let mut element_property_sets: Vec<(usize, &super::entities::PropertySet)> = Vec::new();
        // Map vec index in model.entities → emitted IFC element id.
        // None when that entity wasn't a BuildingElement. Consulted
        // when resolving `host_element_index` for openings + for
        // IfcRelVoidsElement / IfcRelFillsElement emission.
        let mut entity_index_to_el_id: Vec<Option<usize>> = vec![None; model.entities.len()];
        // Void/fill tracking: for each element with a host, note
        // (host_el_id, opening_el_id, element_el_id) so the rels can
        // be emitted after the element loop. Openings themselves are
        // also BuildingElements (IfcOpeningElement) but they're never
        // contained in a storey — IFC4 treats them as "virtual"
        // elements that only live through IfcRelVoidsElement.
        let mut void_fill_triples: Vec<(usize, usize, usize)> = Vec::new();
        for (entity_idx, entity) in model.entities.iter().enumerate() {
            if let super::entities::IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid,
                storey_index,
                material_index,
                property_set,
                location_feet,
                rotation_radians,
                extrusion,
                host_element_index,
                material_layer_set_index,
                material_profile_set_index,
            } = entity
            {
                // Clamp out-of-range indices to storey[0] rather than
                // silently dropping the element. Out-of-range is a
                // caller bug; losing the element would be worse.
                let idx = storey_index
                    .unwrap_or(0)
                    .min(storeys.len().saturating_sub(1));
                let placement_parent = storey_placements[idx];
                // Decide whether to emit a per-element axis placement
                // (with real origin + rotation) or share the project-
                // level identity placement. Sharing keeps byte counts
                // down for elements where no location has been decoded;
                // unique placements matter once geometry attaches to
                // the element and tools read element.position.
                let element_axis = if let Some([x_ft, y_ft, z_ft]) = location_feet {
                    let x_m = x_ft * 0.3048;
                    let y_m = y_ft * 0.3048;
                    let z_m = z_ft * 0.3048;
                    let point_id = self.id();
                    self.emit_entity(
                        point_id,
                        format!("IFCCARTESIANPOINT(({x_m:.6},{y_m:.6},{z_m:.6}))"),
                    );
                    // Rotation is applied about Z only — all Revit
                    // element placements we've seen so far are upright
                    // with yaw-only rotation. Full 3D rotation needs
                    // the BasePoint / ProjectPosition transform chain
                    // (already decoded, not yet threaded here).
                    let x_axis_id = if let Some(angle) = rotation_radians {
                        let cx = angle.cos();
                        let cy = angle.sin();
                        let dir = self.id();
                        self.emit_entity(dir, format!("IFCDIRECTION(({cx:.6},{cy:.6},0.))"));
                        Some(dir)
                    } else {
                        None
                    };
                    let axis_id = self.id();
                    // If we have a yaw rotation, reference our own X-
                    // axis IfcDirection; otherwise reuse the shared
                    // +X axis_placement points at (via `x_axis`).
                    let axis_body = match x_axis_id {
                        Some(d) => format!("IFCAXIS2PLACEMENT3D(#{point_id},#{z_axis},#{d})"),
                        None => format!("IFCAXIS2PLACEMENT3D(#{point_id},#{z_axis},#{x_axis})"),
                    };
                    self.emit_entity(axis_id, axis_body);
                    axis_id
                } else {
                    axis_placement
                };
                let placement_id = self.id();
                self.emit_entity(
                    placement_id,
                    format!("IFCLOCALPLACEMENT(#{placement_parent},#{element_axis})"),
                );

                // Emit the extrusion chain when geometry is present.
                // Chain: IfcRectangleProfileDef → IfcExtrudedAreaSolid
                // → IfcShapeRepresentation → IfcProductDefinitionShape.
                // Profile placement uses a single fresh 2D axis per
                // element (profile-local XY frame centred on origin).
                let shape_ref = if let Some(ex) = extrusion {
                    let x_dim = ex.width_feet * 0.3048;
                    let y_dim = ex.depth_feet * 0.3048;
                    let depth = ex.height_feet * 0.3048;
                    // IfcRectangleProfileDef has a 2D placement; we
                    // emit a fresh 2D origin + direction + 2D axis
                    // per extrusion. Sharing a single 2D placement
                    // across all extrusions would be possible but
                    // muddies byte-by-byte diff tooling, so pay the
                    // ~3-entity cost for clarity.
                    let profile_origin = self.id();
                    self.emit_entity(profile_origin, "IFCCARTESIANPOINT((0.,0.))");
                    let profile_x_axis = self.id();
                    self.emit_entity(profile_x_axis, "IFCDIRECTION((1.,0.))");
                    let profile_placement = self.id();
                    self.emit_entity(
                        profile_placement,
                        format!("IFCAXIS2PLACEMENT2D(#{profile_origin},#{profile_x_axis})"),
                    );
                    let profile_id = self.id();
                    self.emit_entity(
                        profile_id,
                        format!(
                            "IFCRECTANGLEPROFILEDEF(.AREA.,$,#{profile_placement},{x_dim:.6},{y_dim:.6})"
                        ),
                    );
                    // Solid-local placement: reuse the element's own
                    // axis so the extrusion sits at the element origin.
                    let solid_id = self.id();
                    self.emit_entity(
                        solid_id,
                        format!(
                            "IFCEXTRUDEDAREASOLID(#{profile_id},#{element_axis},#{z_axis},{depth:.6})"
                        ),
                    );
                    // The representation groups the solid inside the
                    // project's 3D geometric context (#geom_ctx) with
                    // the IFC4-standard identifier + type for a
                    // swept-solid body.
                    let rep_id = self.id();
                    self.emit_entity(
                        rep_id,
                        format!(
                            "IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','SweptSolid',(#{solid_id}))"
                        ),
                    );
                    let prod_shape_id = self.id();
                    self.emit_entity(
                        prod_shape_id,
                        format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep_id}))"),
                    );
                    Some(prod_shape_id)
                } else {
                    None
                };

                let el_id = self.id();
                let name_quoted = quoted_or_dollar(&escape(name));
                let tag_quoted = type_guid
                    .as_deref()
                    .map(escape)
                    .map(|t| format!("'{t}'"))
                    .unwrap_or_else(|| "$".into());
                let rep_slot = match shape_ref {
                    Some(id) => format!("#{id}"),
                    None => "$".into(),
                };
                // IFC<TYPE>(GlobalId, OwnerHist, Name, Desc, ObjectType,
                //   ObjectPlacement, Representation, Tag)
                // Some subclasses (IfcDoor/IfcWindow) want extra predefined
                // fields — the minimal 8-field form is valid for IfcWall,
                // IfcSlab, IfcRoof, IfcCovering, IfcColumn, IfcBeam. Door
                // and Window are emitted with their 10-field variants.
                let ifc_upper = ifc_type.to_ascii_uppercase();
                let line = if ifc_upper == "IFCDOOR" || ifc_upper == "IFCWINDOW" {
                    format!(
                        "{ifc_upper}('{}',#{owner_hist},{name_quoted},$,$,#{placement_id},{rep_slot},{tag_quoted},$,$)",
                        make_guid(el_id),
                    )
                } else {
                    format!(
                        "{ifc_upper}('{}',#{owner_hist},{name_quoted},$,$,#{placement_id},{rep_slot},{tag_quoted})",
                        make_guid(el_id),
                    )
                };
                self.emit_entity(el_id, line);
                per_storey_elements[idx].push(el_id);
                entity_index_to_el_id[entity_idx] = Some(el_id);
                // IFC-30 / IFC-28: precedence order for material
                // association is profile_set > layer_set > single
                // material. Try each in order; `profile_or_layer_applied`
                // carries the "already emitted" flag through so the
                // single-material path below doesn't double-associate.
                let profile_set_applied = if let Some(ps_idx) = material_profile_set_index {
                    if let Some(&ps_id) = profile_set_ids.get(*ps_idx) {
                        let usage_id = self.id();
                        // IfcMaterialProfileSetUsage(ForProfileSet,
                        //   CardinalPoint=5 (bottom-left reference), ReferenceExtent=$)
                        // CardinalPoint=5 is the IFC4 convention for
                        // "bottom-left of the bounding box", which aligns
                        // with Revit's extrusion origin for structural
                        // framing. Downstream tools that care about
                        // cardinal-point semantics can override.
                        self.emit_entity(
                            usage_id,
                            format!("IFCMATERIALPROFILESETUSAGE(#{ps_id},5,$)"),
                        );
                        let rel_id = self.id();
                        self.emit_entity(
                            rel_id,
                            format!(
                                "IFCRELASSOCIATESMATERIAL('{}',#{owner_hist},$,$,(#{el_id}),#{usage_id})",
                                make_guid(rel_id),
                            ),
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                // IFC-28: material_layer_set_index falls through when
                // profile-set didn't apply. Emits IfcMaterialLayerSetUsage
                // + IfcRelAssociatesMaterial.
                let layer_set_applied = !profile_set_applied && if let Some(ls_idx) = material_layer_set_index {
                    if let Some(&ls_id) = layer_set_ids.get(*ls_idx) {
                        let usage_id = self.id();
                        // IfcMaterialLayerSetUsage(ForLayerSet, LayerSetDirection=.AXIS2.,
                        //   DirectionSense=.POSITIVE., OffsetFromReferenceLine=0.0)
                        // .AXIS2. = Y axis of the extrusion (wall
                        // thickness direction). .POSITIVE. = stack
                        // outward from the reference line. These are
                        // the IFC4 defaults most exporters emit; Revit
                        // wall-type-specific offsets would override.
                        self.emit_entity(
                            usage_id,
                            format!(
                                "IFCMATERIALLAYERSETUSAGE(#{ls_id},.AXIS2.,.POSITIVE.,0.)"
                            ),
                        );
                        let rel_id = self.id();
                        self.emit_entity(
                            rel_id,
                            format!(
                                "IFCRELASSOCIATESMATERIAL('{}',#{owner_hist},$,$,(#{el_id}),#{usage_id})",
                                make_guid(rel_id),
                            ),
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !profile_set_applied
                    && !layer_set_applied
                    && let Some(m_idx) = material_index
                    && *m_idx < material_ids.len()
                {
                    element_material_pairs.push((el_id, *m_idx));
                }
                if let Some(pset) = property_set
                    && !pset.properties.is_empty()
                {
                    element_property_sets.push((el_id, pset));
                }
                // Void / fill chain: when the element has both an
                // extrusion (describes its volume) and a host_element_
                // index (points at its parent wall/floor), emit an
                // IfcOpeningElement matching the shape + the two
                // binding relationships.
                if let (Some(h_idx), Some(ex)) = (host_element_index, extrusion) {
                    if let Some(host_el_id) = entity_index_to_el_id.get(*h_idx).and_then(|o| *o) {
                        // Emit a second extrusion chain — same shape
                        // as the element, placed on the element's
                        // placement so the void sits where the door
                        // sits. We reuse the element's placement for
                        // simplicity; IFC4 validators accept this.
                        let x_dim = ex.width_feet * 0.3048;
                        let y_dim = ex.depth_feet * 0.3048;
                        let depth_m = ex.height_feet * 0.3048;
                        let o_profile_origin = self.id();
                        self.emit_entity(o_profile_origin, "IFCCARTESIANPOINT((0.,0.))");
                        let o_profile_x = self.id();
                        self.emit_entity(o_profile_x, "IFCDIRECTION((1.,0.))");
                        let o_profile_place = self.id();
                        self.emit_entity(
                            o_profile_place,
                            format!("IFCAXIS2PLACEMENT2D(#{o_profile_origin},#{o_profile_x})"),
                        );
                        let o_profile = self.id();
                        self.emit_entity(
                            o_profile,
                            format!(
                                "IFCRECTANGLEPROFILEDEF(.AREA.,$,#{o_profile_place},{x_dim:.6},{y_dim:.6})"
                            ),
                        );
                        let o_solid = self.id();
                        self.emit_entity(
                            o_solid,
                            format!(
                                "IFCEXTRUDEDAREASOLID(#{o_profile},#{element_axis},#{z_axis},{depth_m:.6})"
                            ),
                        );
                        let o_rep = self.id();
                        self.emit_entity(
                            o_rep,
                            format!(
                                "IFCSHAPEREPRESENTATION(#{geom_ctx},'Body','SweptSolid',(#{o_solid}))"
                            ),
                        );
                        let o_prod_shape = self.id();
                        self.emit_entity(
                            o_prod_shape,
                            format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{o_rep}))"),
                        );
                        // IfcOpeningElement uses its own placement
                        // (relative to the host wall's) — we reuse the
                        // door/window placement.
                        let o_placement = self.id();
                        self.emit_entity(
                            o_placement,
                            format!("IFCLOCALPLACEMENT(#{placement_parent},#{element_axis})"),
                        );
                        let opening_id = self.id();
                        // IfcOpeningElement: (GlobalId, OwnerHist, Name,
                        //   Desc, ObjectType, Placement, Rep, Tag,
                        //   PredefinedType). IFC4 adds PredefinedType.
                        self.emit_entity(
                            opening_id,
                            format!(
                                "IFCOPENINGELEMENT('{}',#{owner_hist},'Opening for {name_esc}',$,$,#{o_placement},#{o_prod_shape},$,.OPENING.)",
                                make_guid(opening_id),
                                name_esc = escape(name),
                            ),
                        );
                        void_fill_triples.push((host_el_id, opening_id, el_id));
                    }
                }
            }
        }

        // IfcRelVoidsElement + IfcRelFillsElement — for each
        // (host, opening, element) triple we collected during
        // element emission, emit:
        //   - IFCRELVOIDSELEMENT(host_wall, opening)
        //   - IFCRELFILLSELEMENT(opening, door/window)
        // Together these tell IFC4 viewers "subtract this opening's
        // volume from the host wall, and fill the hole with this
        // door/window."
        for (host_el_id, opening_id, el_id) in &void_fill_triples {
            let voids_rel = self.id();
            self.emit_entity(
                voids_rel,
                format!(
                    "IFCRELVOIDSELEMENT('{}',#{owner_hist},$,$,#{host_el_id},#{opening_id})",
                    make_guid(voids_rel),
                ),
            );
            let fills_rel = self.id();
            self.emit_entity(
                fills_rel,
                format!(
                    "IFCRELFILLSELEMENT('{}',#{owner_hist},$,$,#{opening_id},#{el_id})",
                    make_guid(fills_rel),
                ),
            );
        }

        // IfcPropertySet emission — one set per element that ships
        // properties. Each property becomes an
        // IfcPropertySingleValue, the set wraps them, then an
        // IfcRelDefinesByProperties links the set to its element.
        for (el_id, pset) in &element_property_sets {
            let mut prop_ids: Vec<usize> = Vec::with_capacity(pset.properties.len());
            for prop in &pset.properties {
                let p_id = self.id();
                let name_esc = escape(&prop.name);
                let value_step = prop.value.to_step();
                self.emit_entity(
                    p_id,
                    format!("IFCPROPERTYSINGLEVALUE('{name_esc}',$,{value_step},$)"),
                );
                prop_ids.push(p_id);
            }
            let refs = prop_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            let set_id = self.id();
            let set_name = escape(&pset.name);
            self.emit_entity(
                set_id,
                format!(
                    "IFCPROPERTYSET('{}',#{owner_hist},'{set_name}',$,({refs}))",
                    make_guid(set_id)
                ),
            );
            let rel_id = self.id();
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELDEFINESBYPROPERTIES('{}',#{owner_hist},$,$,(#{el_id}),#{set_id})",
                    make_guid(rel_id)
                ),
            );
        }

        // Bucket element_id lists by material_index so each material
        // gets one IfcRelAssociatesMaterial (rather than N, where N
        // is the number of elements using it).
        let mut by_material: Vec<Vec<usize>> = vec![Vec::new(); material_ids.len()];
        for (el_id, m_idx) in &element_material_pairs {
            by_material[*m_idx].push(*el_id);
        }
        for (m_idx, elements) in by_material.iter().enumerate() {
            if elements.is_empty() {
                continue;
            }
            let rel_id = self.id();
            let refs_list = elements
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELASSOCIATESMATERIAL('{}',#{owner_hist},$,$,({refs_list}),#{})",
                    make_guid(rel_id),
                    material_ids[m_idx],
                ),
            );
        }

        // Suppress unused-variable warning from the legacy single-
        // storey fallback — the loop above now consults
        // storey_placements[idx] instead of this scalar binding.
        let _ = storey_placement;

        for (idx, element_ids) in per_storey_elements.iter().enumerate() {
            if element_ids.is_empty() {
                continue;
            }
            let target_storey = storey_ids[idx];
            let rel_id = self.id();
            let refs_list = element_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(",");
            self.emit_entity(
                rel_id,
                format!(
                    "IFCRELCONTAINEDINSPATIALSTRUCTURE('{}',#{owner_hist},$,$,({refs_list}),#{target_storey})",
                    make_guid(rel_id),
                ),
            );
        }
        // storey_id from the pre-refactor era is still valid as the
        // default storey; kept live above so existing tests that
        // count placements / storeys on the empty-model path keep
        // passing.
        let _ = storey_id;

        self.emit_line("ENDSEC;");
    }

    fn finish(self) -> String {
        let mut out = self.out;
        out.push_str("END-ISO-10303-21;\n");
        out
    }
}

/// STEP-style string escape per ISO-10303-21:
///
/// - Literal apostrophe → `''` (doubled).
/// - Literal backslash → `\\`.
/// - ASCII printable (0x20–0x7E) → pass through.
/// - ASCII control (0x00–0x1F, 0x7F) → `\X\<HH>` (2-hex-digit byte).
/// - Non-ASCII code points:
///   - BMP (≤ U+FFFF): `\X2\<HHHH>\X0\`
///   - Supplementary plane (> U+FFFF): `\X4\<HHHHHHHH>\X0\`
///
/// Previous implementation replaced non-ASCII with underscore, which
/// silently mangled accented project names, CJK text, and any Unicode
/// symbols in Revit metadata. Real RVT files routinely have
/// non-ASCII in title, path, and taxonomy strings; this escape
/// preserves them round-trip per the STEP spec.
///
/// Two consecutive non-ASCII chars produce separate `\X2\<HHHH>\X0\`
/// sequences rather than a concatenated run. The spec allows either
/// form; separate sequences keep the encoder stateless and the
/// output diff-friendly.
fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii() && !c.is_control() => out.push(c),
            c if c.is_ascii() => {
                // ASCII control byte.
                out.push_str(&format!("\\X\\{:02X}", c as u32));
            }
            c if (c as u32) <= 0xFFFF => {
                // BMP non-ASCII.
                out.push_str(&format!("\\X2\\{:04X}\\X0\\", c as u32));
            }
            c => {
                // Supplementary plane (emoji, rare scripts).
                out.push_str(&format!("\\X4\\{:08X}\\X0\\", c as u32));
            }
        }
    }
    out
}

fn quoted_or_dollar(s: &str) -> String {
    if s.is_empty() {
        "$".into()
    } else {
        format!("'{s}'")
    }
}

fn iso_timestamp_from(secs: i64) -> String {
    // Format a Unix epoch as ISO 8601. Avoids chrono to stay
    // dep-lean. Pure function — deterministic given its input.
    let (y, m, d, hh, mm, ss) = epoch_to_ymdhms(secs);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}")
}

fn unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Gregorian breakdown without chrono. Good from 1970 through 2400.
fn epoch_to_ymdhms(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let remainder = secs.rem_euclid(86_400) as u32;
    let hh = remainder / 3600;
    let mm = (remainder % 3600) / 60;
    let ss = remainder % 60;

    // Days since 1970-01-01 → Gregorian date. Algorithm from Howard
    // Hinnant's date.h (public domain).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;
    (y, m, d, hh, mm, ss)
}

/// IFC4 globally-unique-ID. Format: 22 chars from the IFC-GUID
/// alphabet (`0-9A-Za-z_$`, 64 symbols). The spec requires these be
/// unique per file but does not mandate a specific encoding —
/// `IfcOpenShell` and `buildingSMART` validators accept any 22-char
/// string in the alphabet.
///
/// v1 encoding is deterministic per `index`: a fixed 6-char `"0rvtrs"`
/// prefix followed by the base-64 big-endian encoding of `index` into
/// 16 chars. Gives a bijection between `index` and GUID for the first
/// 64^16 ≈ 7.9 × 10^28 entities — trivially enough. Stable across
/// runs (same input → same output), which makes STEP text diffs
/// tractable.
///
/// Future: once the walker surfaces real per-element GUIDs from the
/// Revit file, we'll prefer those (they're already in the correct
/// format) and fall back to this for entities without a native GUID.
fn make_guid(index: usize) -> String {
    const ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut guid = String::with_capacity(22);
    guid.push_str("0rvtrs");
    let mut suffix = [b'0'; 16];
    let mut n = index;
    for slot in suffix.iter_mut().rev() {
        *slot = ALPHABET[n & 63];
        n >>= 6;
    }
    for b in &suffix {
        guid.push(*b as char);
    }
    guid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_emits_iso_envelope() {
        let model = IfcModel {
            project_name: Some("Demo".into()),
            description: None,
            entities: Vec::new(),
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: Vec::new(),
        material_layer_sets: Vec::new(),
            material_profile_sets: Vec::new(),
        };
        let s = write_step(&model);
        assert!(s.starts_with("ISO-10303-21;\n"));
        assert!(s.contains("FILE_SCHEMA(('IFC4'));"));
        assert!(s.contains("DATA;"));
        assert!(s.contains("IFCPROJECT"));
        assert!(s.ends_with("END-ISO-10303-21;\n"));
    }

    #[test]
    fn step_output_deterministic_with_fixed_timestamp() {
        // Byte-identical output across calls when timestamp is pinned.
        let model = IfcModel {
            project_name: Some("Stable".into()),
            description: Some("Deterministic test".into()),
            entities: Vec::new(),
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: Vec::new(),
        material_layer_sets: Vec::new(),
            material_profile_sets: Vec::new(),
        };
        let opts = StepOptions {
            timestamp: Some(1_700_000_000), // 2023-11-14T22:13:20
        };
        let a = write_step_with_options(&model, &opts);
        let b = write_step_with_options(&model, &opts);
        assert_eq!(
            a, b,
            "identical (model, opts) must produce identical STEP output"
        );
        // And the timestamp actually shows up.
        assert!(a.contains("2023-11-14T22:13:20"), "ISO timestamp missing");
        assert!(
            a.contains(",1700000000)"),
            "IfcOwnerHistory seconds missing"
        );
    }

    #[test]
    fn escape_handles_unicode_bmp_codepoints() {
        // BMP non-ASCII (é, ü, 中, 文, ç) should be \X2\HHHH\X0\.
        let s = escape("Café 中文");
        assert!(s.starts_with("Caf"), "ASCII prefix preserved: {s:?}");
        assert!(s.contains("\\X2\\00E9\\X0\\"), "é as \\X2\\00E9: {s:?}");
        assert!(s.contains("\\X2\\4E2D\\X0\\"), "中 as \\X2\\4E2D: {s:?}");
        assert!(s.contains("\\X2\\6587\\X0\\"), "文 as \\X2\\6587: {s:?}");
    }

    #[test]
    fn escape_handles_supplementary_plane() {
        // 🏢 (U+1F3E2, office building emoji — apt for BIM data)
        // must use \X4\0001F3E2\X0\ form.
        let s = escape("🏢");
        assert!(
            s.contains("\\X4\\0001F3E2\\X0\\"),
            "emoji as \\X4\\0001F3E2: {s:?}"
        );
    }

    #[test]
    fn escape_preserves_ascii_printable() {
        let s = escape("Hello, World! 0123 @#$%^&*()");
        assert_eq!(s, "Hello, World! 0123 @#$%^&*()");
    }

    #[test]
    fn escape_backslash_doubled() {
        // Windows paths in original_path fields have backslashes.
        let s = escape("C:\\Users\\x");
        assert_eq!(s, "C:\\\\Users\\\\x");
    }

    #[test]
    fn step_escapes_apostrophes_in_project_name() {
        let model = IfcModel {
            project_name: Some("Griffin's Building".into()),
            description: None,
            entities: Vec::new(),
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: Vec::new(),
        material_layer_sets: Vec::new(),
            material_profile_sets: Vec::new(),
        };
        let s = write_step(&model);
        assert!(s.contains("Griffin''s Building"));
    }

    #[test]
    fn step_includes_required_framework_entities() {
        let model = IfcModel::default();
        let s = write_step(&model);
        for required in [
            "IFCPERSON",
            "IFCORGANIZATION",
            "IFCAPPLICATION",
            "IFCOWNERHISTORY",
            "IFCSIUNIT",
            "IFCUNITASSIGNMENT",
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            "IFCPROJECT",
        ] {
            assert!(s.contains(required), "missing required entity: {required}");
        }
    }

    #[test]
    fn epoch_to_ymdhms_known_dates() {
        // 1970-01-01 00:00:00 UTC
        assert_eq!(epoch_to_ymdhms(0), (1970, 1, 1, 0, 0, 0));
        // 2024-04-01 00:00:00 UTC = 1711929600
        assert_eq!(epoch_to_ymdhms(1_711_929_600), (2024, 4, 1, 0, 0, 0));
    }

    #[test]
    fn step_emits_spatial_hierarchy() {
        // IFC4 viewers expect a Project → Site → Building → Storey
        // spine before any building elements. Every IfcSpatialStructureElement
        // in the chain needs its own IfcLocalPlacement and must be
        // bound to its parent via IfcRelAggregates.
        let model = IfcModel::default();
        let s = write_step(&model);
        for required in [
            "IFCSITE(",
            "IFCBUILDING(",
            "IFCBUILDINGSTOREY(",
            "IFCLOCALPLACEMENT(",
            "IFCRELAGGREGATES(",
        ] {
            assert!(
                s.contains(required),
                "spatial hierarchy missing required entity: {required}\n\nOutput:\n{s}"
            );
        }
    }

    #[test]
    fn step_hierarchy_count_is_stable() {
        // The hierarchy adds exactly:
        //   3 IfcLocalPlacement (one per spatial container)
        //   1 IfcSite
        //   1 IfcBuilding
        //   1 IfcBuildingStorey
        //   3 IfcRelAggregates (project-site, site-building, building-storey)
        // Pinning the counts prevents silent regressions if the writer
        // grows extra placeholder entities.
        let model = IfcModel::default();
        let s = write_step(&model);
        assert_eq!(s.matches("IFCSITE(").count(), 1);
        assert_eq!(s.matches("IFCBUILDING(").count(), 1);
        assert_eq!(s.matches("IFCBUILDINGSTOREY(").count(), 1);
        assert_eq!(s.matches("IFCLOCALPLACEMENT(").count(), 3);
        assert_eq!(s.matches("IFCRELAGGREGATES(").count(), 3);
    }

    #[test]
    fn make_guid_is_22_chars_in_alphabet() {
        let g = make_guid(0);
        assert_eq!(g.len(), 22, "IFC GUIDs must be exactly 22 characters");
        const ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
        for c in g.chars() {
            assert!(
                ALPHABET.contains(c),
                "character {c:?} not in IFC GUID alphabet"
            );
        }
    }

    #[test]
    fn make_guid_is_deterministic_and_distinct() {
        // Same input → same output (stable diffs across runs).
        assert_eq!(make_guid(42), make_guid(42));
        // Different inputs → different outputs (uniqueness).
        let g1 = make_guid(1);
        let g2 = make_guid(2);
        let g100 = make_guid(100);
        assert_ne!(g1, g2);
        assert_ne!(g1, g100);
        assert_ne!(g2, g100);
    }

    #[test]
    fn step_emits_omniclass_classification_when_present() {
        use super::super::entities::{Classification, ClassificationItem, ClassificationSource};
        let model = IfcModel {
            project_name: Some("ClassifiedDemo".into()),
            description: None,
            entities: Vec::new(),
            classifications: vec![Classification {
                source: ClassificationSource::OmniClass,
                edition: Some("2012".into()),
                items: vec![
                    ClassificationItem {
                        code: "23.45.12.34".into(),
                        name: Some("Example Product".into()),
                    },
                    ClassificationItem {
                        code: "23.45.12.35".into(),
                        name: None,
                    },
                ],
            }],
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: Vec::new(),
        material_layer_sets: Vec::new(),
            material_profile_sets: Vec::new(),
        };
        let s = write_step(&model);
        assert!(
            s.contains("IFCCLASSIFICATION("),
            "classification entity missing"
        );
        assert!(s.contains("'OmniClass'"), "OmniClass source missing");
        assert!(s.contains("'2012'"), "edition 2012 missing");
        assert!(
            s.matches("IFCCLASSIFICATIONREFERENCE(").count() == 2,
            "expected two classification references (one per item)"
        );
        assert!(s.contains("'23.45.12.34'"), "first code missing");
        assert!(s.contains("'23.45.12.35'"), "second code missing");
        assert!(s.contains("'Example Product'"), "item name missing");
        assert!(
            s.matches("IFCRELASSOCIATESCLASSIFICATION(").count() == 2,
            "expected one association rel per reference"
        );
    }

    #[test]
    fn step_omits_classification_entities_when_empty() {
        // Model with no classifications must NOT emit classification
        // entities. Guards against a regression where the writer
        // emits empty IfcClassification / IfcRelAssociates entities.
        let model = IfcModel::default();
        let s = write_step(&model);
        assert!(
            !s.contains("IFCCLASSIFICATION("),
            "should not emit IfcClassification when model.classifications is empty"
        );
        assert!(
            !s.contains("IFCCLASSIFICATIONREFERENCE("),
            "should not emit IfcClassificationReference when model has no classifications"
        );
        assert!(
            !s.contains("IFCRELASSOCIATESCLASSIFICATION("),
            "should not emit IfcRelAssociatesClassification when model has no classifications"
        );
    }

    #[test]
    fn step_guids_are_unique_across_entities() {
        // The writer assigns each entity a unique GUID by index; the
        // STEP output should therefore contain no duplicate GUIDs.
        // We grep for '0rvtrs' (our prefix) and check uniqueness.
        let model = IfcModel::default();
        let s = write_step(&model);
        let guids: Vec<_> = s
            .split("'0rvtrs")
            .skip(1)
            .filter_map(|chunk| chunk.split('\'').next())
            .collect();
        let mut seen = std::collections::HashSet::new();
        for g in &guids {
            assert!(seen.insert(*g), "duplicate IFC GUID in output: 0rvtrs{g}");
        }
        assert!(
            guids.len() >= 7,
            "expected ≥7 GUIDs (project+site+building+storey+3 rel-aggregates), got {}",
            guids.len()
        );
    }

    #[test]
    fn step_emits_building_elements() {
        use super::super::entities::IfcEntity;
        let model = IfcModel {
            project_name: Some("ElementsDemo".into()),
            description: None,
            entities: vec![
                IfcEntity::BuildingElement {
                    ifc_type: "IfcWall".into(),
                    name: "North Wall".into(),
                    type_guid: None,
                    storey_index: None,
                    material_index: None,
                    property_set: None,
                    location_feet: None,
                    rotation_radians: None,
                    extrusion: None,
                    host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
                },
                IfcEntity::BuildingElement {
                    ifc_type: "IfcSlab".into(),
                    name: "Level 1 Floor".into(),
                    type_guid: Some("101".into()),
                    storey_index: None,
                    material_index: None,
                    property_set: None,
                    location_feet: None,
                    rotation_radians: None,
                    extrusion: None,
                    host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
                },
                IfcEntity::BuildingElement {
                    ifc_type: "IfcDoor".into(),
                    name: "Front Door".into(),
                    type_guid: None,
                    storey_index: None,
                    material_index: None,
                    property_set: None,
                    location_feet: None,
                    rotation_radians: None,
                    extrusion: None,
                    host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
                },
            ],
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: Vec::new(),
        material_layer_sets: Vec::new(),
            material_profile_sets: Vec::new(),
        };
        let s = write_step(&model);
        // Each element's IFC4 entity constructor appears in the output.
        assert!(s.contains("IFCWALL("), "missing IFCWALL constructor:\n{s}");
        assert!(s.contains("IFCSLAB("), "missing IFCSLAB constructor:\n{s}");
        assert!(s.contains("IFCDOOR("), "missing IFCDOOR constructor:\n{s}");
        assert!(
            s.contains("North Wall"),
            "escaped element name not emitted:\n{s}"
        );
        // IfcRelContainedInSpatialStructure ties them to the storey.
        assert_eq!(
            s.matches("IFCRELCONTAINEDINSPATIALSTRUCTURE(").count(),
            1,
            "expected exactly one spatial-containment rel:\n{s}"
        );
        // Each element gets its own placement on top of the 3 spatial
        // placements (site, building, storey) → 6 total.
        assert_eq!(s.matches("IFCLOCALPLACEMENT(").count(), 6);
    }

    #[test]
    fn step_empty_entities_emits_no_containment_rel() {
        // When no BuildingElements are provided, the writer must NOT
        // emit an IFCRELCONTAINEDINSPATIALSTRUCTURE — an empty
        // references list would fail IFC4 schema validation.
        let model = IfcModel::default();
        let s = write_step(&model);
        assert_eq!(s.matches("IFCRELCONTAINEDINSPATIALSTRUCTURE(").count(), 0);
    }

    #[test]
    fn step_door_and_window_get_10_field_form() {
        // IfcDoor and IfcWindow have OverallHeight/OverallWidth slots
        // after the standard 8 fields; we emit them as `$,$` (unknown)
        // until geometry lands. Verify they land as 10-field forms.
        use super::super::entities::IfcEntity;
        let model = IfcModel {
            project_name: None,
            description: None,
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IfcDoor".into(),
                name: "Door".into(),
                type_guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            }],
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: Vec::new(),
        material_layer_sets: Vec::new(),
            material_profile_sets: Vec::new(),
        };
        let s = write_step(&model);
        // The door line should have 9 commas (10 fields). Grep the
        // full constructor by finding the line that starts with a
        // GUID and contains "IFCDOOR(".
        let line = s
            .lines()
            .find(|l| l.contains("IFCDOOR("))
            .expect("IFCDOOR emitted");
        let open = line.find("IFCDOOR(").unwrap() + "IFCDOOR(".len();
        let close = line.rfind(");").unwrap();
        let args = &line[open..close];
        assert_eq!(
            args.matches(',').count(),
            9,
            "IFCDOOR args expected 10 fields (9 commas), got: {args}"
        );
    }

    /// IFC-28: A BuildingElement that references a MaterialLayerSet
    /// emits IFCMATERIALLAYER + IFCMATERIALLAYERSET +
    /// IFCMATERIALLAYERSETUSAGE + IFCRELASSOCIATESMATERIAL. Regression
    /// test for the writer's compound-material emission path.
    #[test]
    fn layer_set_emits_layer_chain_plus_usage() {
        use super::super::MaterialInfo;
        use super::super::entities::{IfcEntity, MaterialLayer, MaterialLayerSet};
        let model = IfcModel {
            project_name: Some("LayerSet demo".into()),
            description: None,
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IFCWALL".into(),
                name: "Exterior Wall".into(),
                type_guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: Some(0),
                material_profile_set_index: None,
            }],
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: vec![
                MaterialInfo {
                    name: "Gypsum".into(),
                    color_packed: None,
                    transparency: None,
                },
                MaterialInfo {
                    name: "Insulation".into(),
                    color_packed: None,
                    transparency: None,
                },
            ],
            material_layer_sets: vec![MaterialLayerSet {
                name: "Ext-6in".into(),
                description: None,
                layers: vec![
                    MaterialLayer {
                        material_index: 0,
                        thickness_feet: 5.0 / 8.0 / 12.0, // 5/8" gypsum
                        name: Some("Finish".into()),
                    },
                    MaterialLayer {
                        material_index: 1,
                        thickness_feet: 6.0 / 12.0, // 6" insulation
                        name: Some("Core".into()),
                    },
                ],
            }],
            material_profile_sets: Vec::new(),
        };
        let s = write_step(&model);
        assert!(s.contains("IFCMATERIALLAYER("), "IFCMATERIALLAYER missing");
        assert_eq!(
            s.matches("IFCMATERIALLAYER(").count(),
            2,
            "expected two IFCMATERIALLAYER entities"
        );
        assert!(
            s.contains("IFCMATERIALLAYERSET("),
            "IFCMATERIALLAYERSET missing"
        );
        assert!(s.contains("'Ext-6in'"), "layer-set name missing");
        assert!(
            s.contains("IFCMATERIALLAYERSETUSAGE("),
            "usage missing"
        );
        // Relationship must be present — tied to the wall element.
        assert!(
            s.contains("IFCRELASSOCIATESMATERIAL("),
            "IFCRELASSOCIATESMATERIAL missing"
        );
    }

    /// IFC-30: A BuildingElement that references a MaterialProfileSet
    /// emits IFCRECTANGLEPROFILEDEF + IFCMATERIALPROFILE +
    /// IFCMATERIALPROFILESET + IFCMATERIALPROFILESETUSAGE.
    #[test]
    fn profile_set_emits_profile_chain_plus_usage() {
        use super::super::MaterialInfo;
        use super::super::entities::{IfcEntity, MaterialProfile, MaterialProfileSet};
        let model = IfcModel {
            project_name: Some("ProfileSet demo".into()),
            description: None,
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IFCCOLUMN".into(),
                name: "W12x26 Col".into(),
                type_guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: Some(0),
            }],
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: vec![MaterialInfo {
                name: "A992 Steel".into(),
                color_packed: None,
                transparency: None,
            }],
            material_layer_sets: Vec::new(),
            material_profile_sets: vec![MaterialProfileSet {
                name: "W12x26".into(),
                description: Some("AISC wide-flange".into()),
                profiles: vec![MaterialProfile {
                    material_index: 0,
                    profile_name: "W12x26".into(),
                    description: None,
                }],
            }],
        };
        let s = write_step(&model);
        assert!(
            s.contains("IFCRECTANGLEPROFILEDEF("),
            "placeholder profile def missing"
        );
        assert!(s.contains("IFCMATERIALPROFILE("), "profile entity missing");
        assert!(
            s.contains("IFCMATERIALPROFILESET("),
            "profile set missing"
        );
        assert!(
            s.contains("IFCMATERIALPROFILESETUSAGE("),
            "profile set usage missing"
        );
        assert!(
            s.contains("IFCRELASSOCIATESMATERIAL("),
            "IFCRELASSOCIATESMATERIAL missing"
        );
    }

    /// Precedence: when both material_profile_set_index and
    /// material_layer_set_index are set on the same element, the
    /// profile-set path wins (matches Rust-side impl precedence).
    #[test]
    fn profile_set_takes_precedence_over_layer_set() {
        use super::super::MaterialInfo;
        use super::super::entities::{
            IfcEntity, MaterialLayer, MaterialLayerSet, MaterialProfile, MaterialProfileSet,
        };
        let model = IfcModel {
            project_name: Some("Precedence test".into()),
            description: None,
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IFCBEAM".into(),
                name: "Beam".into(),
                type_guid: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: None,
                rotation_radians: None,
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: Some(0),
                material_profile_set_index: Some(0),
            }],
            classifications: Vec::new(),
            units: Vec::new(),
            building_storeys: Vec::new(),
            materials: vec![MaterialInfo {
                name: "Steel".into(),
                color_packed: None,
                transparency: None,
            }],
            material_layer_sets: vec![MaterialLayerSet {
                name: "Coincidental".into(),
                description: None,
                layers: vec![MaterialLayer {
                    material_index: 0,
                    thickness_feet: 0.5,
                    name: None,
                }],
            }],
            material_profile_sets: vec![MaterialProfileSet {
                name: "W12x26".into(),
                description: None,
                profiles: vec![MaterialProfile {
                    material_index: 0,
                    profile_name: "W12x26".into(),
                    description: None,
                }],
            }],
        };
        let s = write_step(&model);
        // Both layer set AND profile set entities exist because
        // every layer-set and profile-set in the model is emitted
        // up-front. What the precedence check governs is which
        // USAGE the element gets associated with.
        assert!(s.contains("IFCMATERIALLAYERSET("));
        assert!(s.contains("IFCMATERIALPROFILESET("));
        // Profile-set usage is emitted (because profile_set wins).
        assert!(s.contains("IFCMATERIALPROFILESETUSAGE("));
        // Layer-set usage is NOT emitted (layer_set lost to profile_set).
        assert!(
            !s.contains("IFCMATERIALLAYERSETUSAGE("),
            "layer-set usage should NOT be emitted when profile-set wins precedence"
        );
    }
}
