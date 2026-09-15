//! Bounded evaluation of current embedded extrusion recipes and qualified constraints.
//! This is not a general Revit regeneration engine: unsupported dependencies
//! produce diagnostics, and no bounding box is substituted for a profile.
#[path = "native_family_arc.rs"]
mod arc;
#[path = "native_family_constraints.rs"]
mod constraints;
#[path = "native_family_expressions.rs"]
pub mod expressions;
#[path = "native_family_tangent.rs"]
mod tangent;
use crate::{
    native_document::Record,
    native_embedded,
    native_equipment::Identity,
    native_metadata::identifier,
    native_parameters::ObjectGraph,
    native_representations::{self, Source},
    native_spatial_context,
};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
type MeshBuffers = (Vec<[f64; 3]>, Vec<[usize; 3]>);

#[derive(Debug, Serialize)]
pub struct Mesh {
    pub content_key_raw_hex: String,
    pub local_form_id: u64,
    pub current_selection: native_embedded::CurrentSelection,
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    pub volume_cubic_feet: f64,
    pub surface_area_square_feet: f64,
    pub source: Source,
    pub parameter_sources: Vec<Value>,
    pub effective_material_id: Option<i64>,
    pub material_sources: Vec<Value>,
    pub analytic_extrusion: Option<Value>,
}
#[derive(Debug, Serialize)]
pub struct Evaluation {
    pub instance: Identity,
    pub type_identity: Identity,
    pub family_identity: Identity,
    pub detail_level: String,
    pub status: String,
    pub meshes: Vec<Mesh>,
    pub volume_cubic_feet: Option<f64>,
    pub surface_area_square_feet: Option<f64>,
    pub diagnostics: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_supported_geometry: bool,
    pub complete_family_geometry: bool,
    pub coordinate_space: &'static str,
    pub length_unit: &'static str,
    pub evaluations: Vec<Evaluation>,
    pub diagnostics: Vec<String>,
    pub unresolved_scope: Vec<&'static str>,
}
#[derive(Clone)]
struct TypeValue {
    value: Value,
    source: Value,
    expression: Option<expressions::Expression>,
    expression_error: Option<String>,
    parameter_keys: BTreeMap<i64, String>,
    is_boolean: bool,
}
// External/shared definitions can lack the family-only scope field. Retain
// absence as unknown; resolve_parameter refuses it only when evaluation needs it.
fn definition_scope(owner_fields: &BTreeMap<String, Value>) -> Value {
    owner_fields
        .get("m_instanceParam")
        .cloned()
        .unwrap_or(Value::Null)
}

#[derive(Default)]
pub(crate) struct ParameterContext {
    values: BTreeMap<u64, BTreeMap<String, TypeValue>>,
}
impl ParameterContext {
    pub(crate) fn ingest(&mut self, record: &Record) -> Result<()> {
        if matches!(
            record.class_name.as_deref(),
            Some("FamilySymbol" | "FamilyInstance")
        ) {
            let mut values = BTreeMap::new();
            if let Some(meta) = &record.saved_metadata {
                for parameter in &meta.resolved_family_parameters {
                    if let Some(def) = meta
                        .parameter_definitions
                        .iter()
                        .find(|d| d.parameter_id == parameter.definition_parameter_id)
                        && let Some(key) = &def.type_id
                    {
                        let (expression, expression_error) =
                            (|| -> Result<Option<expressions::Expression>> {
                                let graph = record
                                    .graph
                                    .as_ref()
                                    .ok_or_else(|| anyhow::anyhow!("parameter graph absent"))?;
                                let object =
                                    graph.objects.get(parameter.source_object).ok_or_else(
                                        || anyhow::anyhow!("parameter source outside graph"),
                                    )?;
                                let rows =
                                    object.fields["m_params"].as_array().ok_or_else(|| {
                                        anyhow::anyhow!("family parameter rows absent")
                                    })?;
                                let rows = rows
                                    .iter()
                                    .filter(|r| {
                                        identifier(&r["m_paramId"]).ok()
                                            == Some(parameter.parameter_id)
                                    })
                                    .collect::<Vec<_>>();
                                ensure!(rows.len() == 1, "ambiguous expression parameter source");
                                ensure!(
                                    rows[0]["m_reporting"].as_bool() == Some(false),
                                    "reporting parameter evaluation unsupported"
                                );
                                let pointer = &rows[0]["m_oExpression"];
                                if pointer["pointer_token"].as_u64() == Some(0) {
                                    Ok(None)
                                } else {
                                    Ok(Some(expressions::read(
                                        graph,
                                        parameter.source_object,
                                        pointer,
                                    )?))
                                }
                            })()
                            .map(|v| (v, None))
                            .unwrap_or_else(|e| (None, Some(e.to_string())));
                        let value = TypeValue {
                            value: parameter.raw_value.clone(),
                            expression,
                            expression_error,
                            parameter_keys: meta
                                .parameter_definitions
                                .iter()
                                .filter_map(|d| {
                                    d.type_id.as_ref().map(|key| (d.parameter_id, key.clone()))
                                })
                                .collect(),
                            is_boolean: def.spec_type_id.as_deref()
                                == Some("autodesk.spec:spec.bool-1.0.0"),
                            source: serde_json::json!({
                            "source_owner_id":record.identity.element_id,"source_owner_unique_id":record.identity.unique_id,
                            "parameter_id":parameter.parameter_id,"definition_type_id":key,"definition_instance_parameter":definition_scope(&def.owner_fields),"stored_instance_default":parameter.stored_instance_flag,
                            "source_object":parameter.source_object,"body_sha256":record.source.body_sha256,
                            "value_unit_basis":"revit_internal_units_for_spec","spec_type_id":def.spec_type_id}),
                        };
                        ensure!(
                            values.insert(key.clone(), value).is_none(),
                            "duplicate semantic parameter key on type"
                        );
                    }
                }
            }
            self.values.insert(record.identity.element_id, values);
        }
        Ok(())
    }
    pub(crate) fn resolve_id(
        &self,
        parameter: i64,
        instance: u64,
        symbol: u64,
    ) -> Result<(Value, Value)> {
        let candidates = self
            .values
            .get(&symbol)
            .into_iter()
            .flat_map(|v| v.iter())
            .filter(|(_, v)| v.source["parameter_id"].as_i64() == Some(parameter))
            .collect::<Vec<_>>();
        ensure!(
            candidates.len() == 1,
            "connector associated parameter identity ambiguous or absent"
        );
        resolve_parameter(
            candidates[0].0,
            instance,
            symbol,
            &self.values,
            &mut Vec::new(),
        )
    }
}
#[derive(Default)]
pub struct InventoryBuilder {
    representations: native_representations::InventoryBuilder,
    spatial: native_spatial_context::InventoryBuilder,
    graphs: BTreeMap<(String, u64), ObjectGraph>,
    definitions: BTreeMap<(String, i64), String>,
    parameters: ParameterContext,
    planes: BTreeMap<(String, i64), Vec<(ObjectGraph, Value)>>,
    sketches: BTreeMap<(String, i64), (ObjectGraph, Value)>,
    blockers: BTreeMap<String, Vec<String>>,
    dimensions: BTreeMap<String, Vec<(ObjectGraph, Value)>>,
}
impl InventoryBuilder {
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        self.representations.ingest(record)?;
        self.spatial.ingest(record)?;
        self.parameters.ingest(record)?;
        Ok(())
    }
    pub fn ingest_embedded(&mut self, record: &native_embedded::Record) -> Result<()> {
        self.representations.ingest_embedded(record)?;
        if record.current_selection.is_none() {
            return Ok(());
        }
        let class = record.class_name.as_deref().unwrap_or("<unknown>");
        let other_form = record
            .graph
            .as_ref()
            .and_then(|g| g.objects.first())
            .is_some_and(|o| o.fields.get("m_cutting").is_some())
            && class != "ExtrusionElem";
        if other_form
            || class == "FamilyInstance"
            || (class == "ExtrusionElem" && record.graph.is_none())
        {
            self.blockers.entry(record.content_key_raw_hex.clone()).or_default().push(format!("current content owner {} class {class} requires unsupported geometry regeneration",record.local_element_id));
        }
        if let Some(graph) = &record.graph {
            if matches!(
                record.class_name.as_deref(),
                Some("RefPlane" | "CurveElem" | "DimensionStyle")
            ) || graph.objects[0].fields.get("m_witnessRefs").is_some()
            {
                self.dimensions.entry(record.content_key_raw_hex.clone()).or_default().push((graph.clone(),serde_json::json!({"local_element_id":record.local_element_id,"source":record.source,"current_selection":record.current_selection})));
            }
            if record.class_name.as_deref() == Some("VarSketch") {
                self.sketches.insert(
                    (
                        record.content_key_raw_hex.clone(),
                        record.local_element_id as i64,
                    ),
                    (graph.clone(), serde_json::to_value(record)?),
                );
            }
            if record.class_name.as_deref() == Some("SketchPlane") {
                let user = identifier(&graph.objects[0].fields["m_userId"])?;
                if user > 0 {
                    self.planes
                        .entry((record.content_key_raw_hex.clone(), user))
                        .or_default()
                        .push((graph.clone(), serde_json::to_value(record)?));
                }
            }
            if record.class_name.as_deref() == Some("ExtrusionElem") {
                ensure!(
                    self.graphs
                        .insert(
                            (record.content_key_raw_hex.clone(), record.local_element_id),
                            graph.clone()
                        )
                        .is_none(),
                    "duplicate selected form graph"
                );
            }
            if let Some(def) = crate::native_parameter_definitions::project(graph)?
                && let Some(key) = def.type_id
            {
                self.definitions
                    .insert((record.content_key_raw_hex.clone(), def.parameter_id), key);
            }
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Inventory> {
        let representations = self.representations.finish()?;
        let spatial = self.spatial.finish()?;
        let mut evaluations = Vec::new();
        let mut diagnostics = Vec::new();
        for instance_ref in representations
            .owner_references
            .iter()
            .filter(|r| r.role == "instance_symbol")
        {
            let Some(type_identity) = &instance_ref.target_identity else {
                continue;
            };
            let Some(family_ref) = representations.owner_references.iter().find(|r| {
                r.role == "symbol_family" && r.owner.element_id == type_identity.element_id
            }) else {
                continue;
            };
            let Some(family_identity) = &family_ref.target_identity else {
                continue;
            };
            let Some(content) = representations
                .family_contents
                .iter()
                .find(|f| f.family.element_id == family_identity.element_id)
            else {
                continue;
            };
            let forms: Vec<_> = representations
                .declarations
                .iter()
                .filter(|d| {
                    d.class_name == "ExtrusionElem"
                        && d.owner.content_key_raw_hex.as_deref()
                            == Some(&content.content_key_raw_hex)
                        && d.owner.current_state_established
                })
                .collect();
            if forms.is_empty() {
                continue;
            }
            let transform = spatial
                .elements
                .iter()
                .find(|e| e.identity.element_id == instance_ref.owner.element_id)
                .and_then(|e| e.transform.as_ref());
            for detail in ["Coarse", "Medium", "Fine"] {
                let mut evaluation = Evaluation {
                    instance: instance_ref.owner.clone(),
                    type_identity: type_identity.clone(),
                    family_identity: family_identity.clone(),
                    detail_level: detail.into(),
                    status: "evaluated".into(),
                    meshes: Vec::new(),
                    volume_cubic_feet: None,
                    surface_area_square_feet: None,
                    diagnostics: self
                        .blockers
                        .get(&content.content_key_raw_hex)
                        .cloned()
                        .unwrap_or_default(),
                };
                for form in &forms {
                    let result = (|| -> Result<Option<Mesh>> {
                        let visibility = form
                            .detail_visibility
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("unqualified detail visibility"))?;
                        if !visibility[detail] {
                            return Ok(None);
                        }
                        ensure!(
                            form.saved_root_fields["m_cutting"].as_bool() == Some(false),
                            "void extrusion requires boolean evaluation"
                        );
                        let graph = self
                            .graphs
                            .get(&(
                                content.content_key_raw_hex.clone(),
                                form.owner.local_element_id,
                            ))
                            .ok_or_else(|| anyhow::anyhow!("current form graph unavailable"))?;
                        let (mut start, mut end, mut visible) = saved_controls(graph)?;
                        let mut parameter_sources = Vec::new();
                        let mut effective_material_id = None;
                        let mut material_sources = Vec::new();
                        for association in &form.parameter_associations {
                            ensure!(
                                association.raw_fields["m_bIsSymbol"].as_bool() == Some(false),
                                "symbol-target form association unsupported"
                            );
                            let key = self
                                .definitions
                                .get(&(
                                    content.content_key_raw_hex.clone(),
                                    association.family_parameter_local_id,
                                ))
                                .ok_or_else(|| {
                                    anyhow::anyhow!("associated local definition absent")
                                })?;
                            let (value, source) = resolve_parameter(
                                key,
                                instance_ref.owner.element_id,
                                type_identity.element_id,
                                &self.parameters.values,
                                &mut Vec::new(),
                            )?;
                            match association.element_property_id {
                                -1001800 => start = number(&value)?,
                                -1001801 => end = number(&value)?,
                                -1002107 => {
                                    let material = value.as_i64().ok_or_else(|| {
                                        anyhow::anyhow!(
                                            "material association is not element ID storage"
                                        )
                                    })?;
                                    ensure!(
                                        material > 0,
                                        "ByCategory or invalid material association requires category evaluation"
                                    );
                                    effective_material_id = Some(material);
                                    material_sources.push(serde_json::json!({"role":"associated_project_material","association":association,"parameter_value":source}));
                                }
                                -1006205 => {
                                    let v = value.as_i64().ok_or_else(|| {
                                        anyhow::anyhow!("visibility is not integer")
                                    })?;
                                    ensure!(v == 0 || v == 1, "nonboolean visibility");
                                    visible = v == 1;
                                }
                                _ => anyhow::bail!(
                                    "unsupported associated form property {}",
                                    association.element_property_id
                                ),
                            }
                            parameter_sources.push(serde_json::json!({"association":association,"parameter_value":source}));
                        }
                        if !visible {
                            return Ok(None);
                        }
                        let (mut polygon, sketch_id, curved) = profile(graph)?;
                        let mut analytic_extrusion = None;
                        let (sketch, sketch_source) = self
                            .sketches
                            .get(&(content.content_key_raw_hex.clone(), sketch_id))
                            .ok_or_else(|| {
                                anyhow::anyhow!("current sketch constraint graph unavailable")
                            })?;
                        if sketch.objects[0].fields["m_dimIds"]
                            .as_array()
                            .is_some_and(|a| !a.is_empty())
                        {
                            let empty = Vec::new();
                            let mut resolver = |id| {
                                let key = self
                                    .definitions
                                    .get(&(content.content_key_raw_hex.clone(), id))
                                    .ok_or_else(|| {
                                        anyhow::anyhow!("labeled dimension definition missing")
                                    })?;
                                let (value, source) = resolve_parameter(
                                    key,
                                    instance_ref.owner.element_id,
                                    type_identity.element_id,
                                    &self.parameters.values,
                                    &mut Vec::new(),
                                )?;
                                Ok((number(&value)?, source))
                            };
                            let dimensions = self
                                .dimensions
                                .get(&content.content_key_raw_hex)
                                .unwrap_or(&empty);
                            let (evaluated, proof) = if curved.is_empty() {
                                constraints::reuse(
                                    sketch,
                                    graph,
                                    form.owner.local_element_id,
                                    dimensions,
                                    &polygon,
                                    &mut resolver,
                                )?
                            } else if curved.len() == 4 {
                                tangent::solve(
                                    sketch,
                                    form.owner.local_element_id,
                                    dimensions,
                                    &curved,
                                    &mut resolver,
                                )?
                            } else {
                                arc::solve(
                                    sketch,
                                    form.owner.local_element_id,
                                    dimensions,
                                    &curved,
                                    &mut resolver,
                                )?
                            };
                            analytic_extrusion = proof.get("analytic_profile").cloned();
                            polygon = evaluated;
                            parameter_sources.push(proof);
                        } else {
                            qualify_sketch(sketch, form.owner.local_element_id)?;
                        }
                        parameter_sources.push(serde_json::json!({"role":"current_profile_constraint_graph","source":sketch_source["source"],"current_selection":sketch_source["current_selection"]}));
                        let planes = self
                            .planes
                            .get(&(content.content_key_raw_hex.clone(), sketch_id))
                            .ok_or_else(|| anyhow::anyhow!("current sketch plane unavailable"))?;
                        ensure!(
                            planes.len() == 1,
                            "ambiguous current sketch plane ownership"
                        );
                        let (plane, plane_source) = &planes[0];
                        let normal = plane_normal(plane, &polygon)?;
                        parameter_sources.push(serde_json::json!({"role":"extrusion_sketch_plane","local_sketch_id":sketch_id,"local_plane_id":plane_source["local_element_id"],"source":plane_source["source"],"current_selection":plane_source["current_selection"]}));
                        let transform = transform
                            .ok_or_else(|| anyhow::anyhow!("instance transform unavailable"))?;
                        if let Some(profile) = analytic_extrusion.take() {
                            let radius = number(&profile["radius"])?;
                            let center = vector(&profile["center"])?;
                            let bx = vector(&profile["basis_x"])?;
                            let by = vector(&profile["basis_y"])?;
                            let direction =
                                |v: [f64; 3]| sub(apply(transform, v), apply(transform, [0.; 3]));
                            let depth = (end - start).abs();
                            let capsule = profile["kind"] == "capsule";
                            let center_distance = if capsule {
                                number(&profile["center_distance"])?
                            } else {
                                0.
                            };
                            let section_area = if capsule {
                                std::f64::consts::PI * radius * radius
                                    + 2. * radius * center_distance
                            } else {
                                std::f64::consts::PI * radius * radius / 2.
                            };
                            let section_perimeter = if capsule {
                                2. * std::f64::consts::PI * radius + 2. * center_distance
                            } else {
                                (std::f64::consts::PI + 2.) * radius
                            };
                            let second_center = if capsule {
                                Some(apply(
                                    transform,
                                    add(vector(&profile["second_center"])?, scale(normal, start)),
                                ))
                            } else {
                                None
                            };
                            analytic_extrusion = Some(serde_json::json!({
                                "kind":if capsule {"capsule_linear_extrusion"} else {"semicircular_linear_extrusion"}, "coordinate_space":"host_document", "length_unit":"feet",
                                "start_center":apply(transform,add(center,scale(normal,start))),
                                "end_center":apply(transform,add(center,scale(normal,end))),
                                "basis_x":direction(bx),"basis_y":direction(by),"axis":direction(normal),
                                "radius":radius,"depth":depth,"sweep_radians":std::f64::consts::PI,
                                "volume_cubic_feet":section_area*depth,"second_start_center":second_center,"center_distance":center_distance,
                                "surface_area_square_feet":2.*section_area+section_perimeter*depth,
                                "chord_tolerance_feet":profile["chord_tolerance_feet"],"arc_segments":profile["arc_segments"],
                                "measure_basis":"exact analytic circular profile; mesh measures are chordal approximations"
                            }));
                        }
                        let (vertices, triangles) = extrude(&polygon, normal, start, end)?;
                        let vertices: Vec<_> =
                            vertices.into_iter().map(|p| apply(transform, p)).collect();
                        let mut triangles = triangles;
                        if dot(
                            cross(transform.basis_x, transform.basis_y),
                            transform.basis_z,
                        ) < 0.0
                        {
                            for triangle in &mut triangles {
                                triangle.swap(1, 2);
                            }
                        }
                        let (volume, area) = measure(&vertices, &triangles);
                        Ok(Some(Mesh {
                            content_key_raw_hex: content.content_key_raw_hex.clone(),
                            local_form_id: form.owner.local_element_id,
                            current_selection: form
                                .owner
                                .content_current_selection
                                .clone()
                                .ok_or_else(|| {
                                    anyhow::anyhow!("current selection evidence absent")
                                })?,
                            vertices,
                            triangles,
                            volume_cubic_feet: volume,
                            surface_area_square_feet: area,
                            source: form.source.clone(),
                            parameter_sources,
                            effective_material_id,
                            material_sources,
                            analytic_extrusion,
                        }))
                    })();
                    match result {
                        Ok(Some(mesh)) => evaluation.meshes.push(mesh),
                        Ok(None) => {}
                        Err(error) => evaluation
                            .diagnostics
                            .push(format!("form {}: {error}", form.owner.local_element_id)),
                    }
                }
                if evaluation.diagnostics.is_empty() {
                    evaluation.volume_cubic_feet =
                        Some(evaluation.meshes.iter().map(|m| m.volume_cubic_feet).sum());
                    evaluation.surface_area_square_feet = Some(
                        evaluation
                            .meshes
                            .iter()
                            .map(|m| m.surface_area_square_feet)
                            .sum(),
                    );
                } else {
                    evaluation.status = "unsupported".into();
                    diagnostics.push(format!(
                        "instance {} {detail}: {}",
                        evaluation.instance.element_id,
                        evaluation.diagnostics.join("; ")
                    ));
                }
                evaluations.push(evaluation);
            }
        }
        Ok(Inventory {
            format: "rvt-native-family-geometry/v1",
            complete_supported_geometry: diagnostics.is_empty(),
            complete_family_geometry: false,
            coordinate_space: "host_document",
            length_unit: "feet",
            evaluations,
            diagnostics,
            unresolved_scope: vec![
                "Single convex linear profiles and independently anchored semicircular or tangent-capsule extrusions are evaluated. Other curved or holed profiles, sweeps, blends, booleans, joins, nested regeneration and constraints outside the qualified subsets remain unsupported.",
                "Type and instance controls use saved definition identities and a qualified multiplication/conditional expression subset. Labeled profiles use locked datum constraints, qualified linear/true-distance/acute-angle/radial solves or uniquely verified saved solutions; arbitrary constraint solving and reporting remain unsupported.",
                "Meshes are derived recipe evaluations, not serialized Revit evaluated geometry.",
            ],
        })
    }
}
fn resolve_parameter(
    key: &str,
    instance: u64,
    symbol: u64,
    values: &BTreeMap<u64, BTreeMap<String, TypeValue>>,
    active: &mut Vec<String>,
) -> Result<(Value, Value)> {
    ensure!(
        active.len() < 64 && !active.iter().any(|k| k == key),
        "cyclic or oversized parameter expression dependency"
    );
    let type_value = values.get(&symbol).and_then(|v| v.get(key));
    let instance_value = values.get(&instance).and_then(|v| v.get(key));
    let base = instance_value
        .or(type_value)
        .ok_or_else(|| anyhow::anyhow!("parameter value absent for definition {key}"))?;
    let instance_scope = base.source["definition_instance_parameter"]
        .as_bool()
        .ok_or_else(|| anyhow::anyhow!("parameter definition scope absent"))?;
    ensure!(
        !instance_scope || instance_value.is_some(),
        "instance value absent; symbol default cannot substitute"
    );
    let formula = type_value
        .filter(|v| v.expression.is_some() || v.expression_error.is_some())
        .unwrap_or(base);
    if let Some(error) = &formula.expression_error {
        anyhow::bail!("parameter expression unresolved: {error}");
    }
    let Some(expression) = &formula.expression else {
        return Ok((base.value.clone(), base.source.clone()));
    };
    active.push(key.into());
    let mut dependencies = Vec::new();
    let evaluated = expressions::evaluate(expression, &mut |id| {
        let dependency_key = formula
            .parameter_keys
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("expression dependency definition {id} unavailable"))?;
        let (value, source) = resolve_parameter(dependency_key, instance, symbol, values, active)?;
        dependencies.push(source);
        let dependency = values
            .get(&instance)
            .and_then(|v| v.get(dependency_key))
            .or_else(|| values.get(&symbol).and_then(|v| v.get(dependency_key)))
            .ok_or_else(|| anyhow::anyhow!("expression dependency value unavailable"))?;
        if dependency.is_boolean {
            match value.as_i64() {
                Some(0) => Ok(expressions::Scalar::Boolean(false)),
                Some(1) => Ok(expressions::Scalar::Boolean(true)),
                _ => anyhow::bail!("invalid boolean dependency"),
            }
        } else {
            Ok(expressions::Scalar::Number(number(&value)?))
        }
    });
    active.pop();
    let value = match evaluated? {
        expressions::Scalar::Number(v) => serde_json::json!(v),
        expressions::Scalar::Boolean(v) => serde_json::json!(i32::from(v)),
    };
    Ok((
        value,
        serde_json::json!({"role":"evaluated_saved_expression","expression":expression,"expression_owner":formula.source,"cached_value_owner":base.source,"cached_value":base.value,"dependencies":dependencies}),
    ))
}
fn number(v: &Value) -> Result<f64> {
    let x = v
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("finite numeric value absent"))?;
    ensure!(x.is_finite(), "nonfinite number");
    Ok(x)
}
fn vector(v: &Value) -> Result<[f64; 3]> {
    let a = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("vector absent"))?;
    ensure!(a.len() == 3, "vector dimension");
    Ok([number(&a[0])?, number(&a[1])?, number(&a[2])?])
}
fn target(graph: &ObjectGraph, owner: usize, pointer: &Value) -> Result<usize> {
    let offset = pointer["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("pointer offset absent"))? as usize;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == owner && e.pointer_offset == offset)
        .collect();
    ensure!(edges.len() == 1, "owning pointer has no unique target");
    Ok(edges[0].target_object_index)
}
fn saved_controls(graph: &ObjectGraph) -> Result<(f64, f64, bool)> {
    let metadata = crate::native_metadata::project(graph)?;
    let values: BTreeMap<_, _> = metadata
        .parameter_sets
        .iter()
        .flat_map(|s| s.parameters.iter())
        .map(|p| (p.serialized_parameter_id, &p.raw_value))
        .collect();
    let get = |id| {
        values
            .get(&id)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("saved extrusion control {id} absent"))
    };
    let visible = get(-1006205)?
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("visibility storage"))?;
    ensure!(visible == 0 || visible == 1, "visibility is not boolean");
    Ok((
        number(get(-1001800)?)?,
        number(get(-1001801)?)?,
        visible == 1,
    ))
}
fn profile(graph: &ObjectGraph) -> Result<(Vec<[f64; 3]>, i64, Vec<Value>)> {
    let root = &graph.objects[0];
    ensure!(
        root.fields["m_alwaysRefPlaneNorm"].as_bool() == Some(true),
        "unqualified extrusion normal mode"
    );
    ensure!(
        root.fields["m_constrInfo"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "form constraints require solver"
    );
    let steps = &graph.objects[target(graph, 0, &root.fields["m_geomSteps"])?];
    ensure!(
        steps.class_name == "GeomStepList",
        "geometry recipe steps unavailable"
    );
    for field in [
        "m_bRepAdjustGList",
        "m_bRepCutOutGList",
        "m_bRepPostCutOutGList",
        "m_bRepTweakGList",
    ] {
        ensure!(
            steps.fields[field].as_array().is_some_and(Vec::is_empty),
            "post-extrusion geometry step {field} unsupported"
        );
    }
    let list = target(graph, 0, &root.fields["m_cellList"])?;
    ensure!(
        graph.objects[list].class_name == "CellList",
        "cell list class"
    );
    let mut helpers = Vec::new();
    for pointer in graph.objects[list].fields["m_cells"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("cell list absent"))?
    {
        let i = target(graph, list, pointer)?;
        if graph.objects[i].class_name == "ExtrusionElemExtrusionHelper" {
            helpers.push(i);
        }
    }
    ensure!(helpers.len() == 1, "unique extrusion helper required");
    let helper = helpers[0];
    ensure!(
        graph.objects[helper].fields["m_complete"].as_bool() == Some(true),
        "incomplete extrusion helper"
    );
    let loops = graph.objects[helper].fields["m_pCurveLoops"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("profile loops absent"))?;
    ensure!(loops.len() == 1, "multiple profile loops unsupported");
    let index = target(graph, helper, &loops[0])?;
    let curve_loop = &graph.objects[index];
    ensure!(
        curve_loop.class_name == "CurveLoop"
            && curve_loop.fields["m_open"].as_bool() == Some(false),
        "closed curve loop required"
    );
    let mut segments = Vec::new();
    let mut curved = Vec::new();
    for pointer in curve_loop.fields["m_curves"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("profile curves absent"))?
    {
        let curve = &graph.objects[target(graph, index, pointer)?];
        ensure!(
            matches!(curve.class_name.as_str(), "GLine" | "GArc"),
            "unsupported profile curve"
        );
        curved.push(serde_json::json!({"class_name":curve.class_name,"fields":curve.fields}));
        if curve.class_name == "GArc" {
            continue;
        }
        let origin = vector(&curve.fields["m_origin"])?;
        let direction = vector(&curve.fields["m_dirVec"])?;
        let params = curve.fields["m_endParams"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("line bounds absent"))?;
        ensure!(params.len() == 2, "line bound count");
        segments.push((
            add(origin, scale(direction, number(&params[0])?)),
            add(origin, scale(direction, number(&params[1])?)),
        ));
    }
    if curved.iter().any(|c| c["class_name"] == "GArc") {
        ensure!(
            (curved.len() == 2 && segments.len() == 1)
                || (curved.len() == 4 && segments.len() == 2),
            "only constrained arc-line profile supported"
        );
        return Ok((
            Vec::new(),
            identifier(&graph.objects[helper].fields["m_sketchId"])?,
            curved,
        ));
    }
    ensure!(
        segments.len() >= 3 && segments.len() <= 1024,
        "bounded polygon edge count"
    );
    for i in 0..segments.len() {
        ensure!(
            norm(sub(segments[i].1, segments[(i + 1) % segments.len()].0)) < 1e-8,
            "profile segments not connected in serialized order"
        );
    }
    Ok((
        segments.into_iter().map(|s| s.0).collect(),
        identifier(&graph.objects[helper].fields["m_sketchId"])?,
        Vec::new(),
    ))
}
fn qualify_sketch(graph: &ObjectGraph, form_id: u64) -> Result<()> {
    let root = &graph.objects[0];
    ensure!(root.class_name == "VarSketch", "sketch class");
    ensure!(
        identifier(&root.fields["m_userId"])? == form_id as i64,
        "sketch does not belong to extrusion"
    );
    for field in [
        "m_dimIds",
        "m_weakDimIds",
        "m_dimData",
        "m_clineIds",
        "m_customDatumPlanes",
        "m_constrInfo",
    ] {
        ensure!(
            root.fields[field].as_array().is_some_and(Vec::is_empty),
            "sketch dimension or external dependency {field} requires constraint solver"
        );
    }
    for flag in ["m_flipXY", "m_flipZ"] {
        ensure!(
            root.fields[flag].as_bool() == Some(false),
            "sketch orientation flag {flag} unqualified"
        );
    }
    ensure!(
        root.fields["m_pPlaneRef"]["pointer_token"].as_u64() == Some(0),
        "external sketch plane dependency"
    );
    for object in &graph.objects {
        ensure!(
            !object.class_name.starts_with("Var")
                || [
                    "VarSketch",
                    "VarSketchLineSegObj",
                    "VarSketchHorVerConstrObj",
                    "VarSketchPPConstrObj",
                    "VarParam"
                ]
                .contains(&object.class_name.as_str()),
            "unsupported sketch expression or constraint {}",
            object.class_name
        );
        ensure!(
            object.class_name != "FamilyParametrizedElemParamsCell",
            "nonliteral sketch association"
        );
    }
    Ok(())
}
fn plane_normal(graph: &ObjectGraph, polygon: &[[f64; 3]]) -> Result<[f64; 3]> {
    let root = &graph.objects[0];
    ensure!(root.class_name == "SketchPlane", "sketch plane class");
    ensure!(
        root.fields["m_flipZ"].as_bool() == Some(false),
        "flipped sketch plane unqualified"
    );
    let trf = &graph.objects[target(graph, 0, &root.fields["m_oTrf"])?];
    ensure!(trf.class_name == "Trf", "sketch transform class");
    ensure!(
        trf.fields["m_3x3"]
            == serde_json::json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
            && norm(vector(&trf.fields["m_or"])?) < 1e-10,
        "nonidentity sketch transform unqualified"
    );
    let reference = target(graph, 0, &root.fields["m_oPlaneRef"])?;
    ensure!(
        graph.objects[reference].class_name == "DummyPlaneRef",
        "referenced host plane unqualified"
    );
    let plane = &graph.objects[target(
        graph,
        reference,
        &graph.objects[reference].fields["m_pPlane"],
    )?];
    ensure!(
        plane.class_name == "Plane" && plane.fields["m_orientFlag"].as_bool() == Some(true),
        "plane orientation unqualified"
    );
    let x = vector(&plane.fields["m_xVec"])?;
    let y = vector(&plane.fields["m_yVec"])?;
    let origin = vector(&plane.fields["m_origin"])?;
    let n = cross(x, y);
    ensure!((norm(n) - 1.0).abs() < 1e-8, "nonorthonormal sketch basis");
    for point in polygon {
        ensure!(
            dot(sub(*point, origin), n).abs() < 1e-8,
            "profile does not lie on saved sketch plane"
        );
    }
    Ok(n)
}
fn extrude(polygon: &[[f64; 3]], normal: [f64; 3], start: f64, end: f64) -> Result<MeshBuffers> {
    ensure!(polygon.len() >= 3, "profile too small");
    ensure!(
        (norm(normal) - 1.0).abs() < 1e-8,
        "invalid extrusion normal"
    );
    for i in 0..polygon.len() {
        ensure!(
            dot(sub(polygon[i], polygon[0]), normal).abs() < 1e-8,
            "nonplanar profile"
        );
        ensure!(
            dot(
                cross(
                    sub(polygon[(i + 1) % polygon.len()], polygon[i]),
                    sub(
                        polygon[(i + 2) % polygon.len()],
                        polygon[(i + 1) % polygon.len()]
                    )
                ),
                normal
            ) > 1e-10,
            "nonconvex or degenerate profile unsupported"
        );
    }
    ensure!(end > start, "nonpositive extrusion depth unsupported");
    let n = polygon.len();
    let mut vertices = Vec::with_capacity(n * 2);
    for offset in [start, end] {
        vertices.extend(polygon.iter().map(|p| add(*p, scale(normal, offset))));
    }
    let mut triangles = Vec::new();
    for i in 1..n - 1 {
        triangles.push([0, i + 1, i]);
        triangles.push([n, n + i, n + i + 1]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        triangles.push([i, j, n + j]);
        triangles.push([i, n + j, n + i]);
    }
    Ok((vertices, triangles))
}
fn apply(t: &native_spatial_context::Transform, p: [f64; 3]) -> [f64; 3] {
    add(
        t.origin,
        add(
            scale(t.basis_x, p[0]),
            add(scale(t.basis_y, p[1]), scale(t.basis_z, p[2])),
        ),
    )
}
fn measure(v: &[[f64; 3]], t: &[[usize; 3]]) -> (f64, f64) {
    let mut volume = 0.0;
    let mut area = 0.0;
    for [a, b, c] in t {
        let (a, b, c) = (v[*a], v[*b], v[*c]);
        volume += dot(a, cross(b, c)) / 6.0;
        area += norm(cross(sub(b, a), sub(c, a))) * 0.5;
    }
    (volume.abs(), area)
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn parameter(value: Value, instance: bool) -> TypeValue {
        TypeValue {
            value,
            source: serde_json::json!({"definition_instance_parameter":instance}),
            expression: None,
            expression_error: None,
            parameter_keys: BTreeMap::new(),
            is_boolean: false,
        }
    }
    #[test]
    fn absent_family_scope_is_retained_and_refused_without_panicking() {
        let scope = definition_scope(&BTreeMap::new());
        assert_eq!(scope, Value::Null);
        let mut value = parameter(json!(7.0), false);
        value.source["definition_instance_parameter"] = scope;
        let values = BTreeMap::from([(2, BTreeMap::from([("external".into(), value)]))]);
        assert!(
            resolve_parameter("external", 1, 2, &values, &mut Vec::new())
                .unwrap_err()
                .to_string()
                .contains("parameter definition scope absent")
        );
        assert_eq!(
            definition_scope(&BTreeMap::from([("m_instanceParam".into(), json!(false))])),
            json!(false)
        );
    }

    #[test]
    fn scoped_expression_dependencies_use_instance_values_and_refuse_defaults() {
        let mut values = BTreeMap::new();
        let mut formula = parameter(serde_json::json!(6.0), true);
        formula.expression = Some(expressions::Expression::Binary {
            operator: 3,
            left: Box::new(expressions::Expression::Parameter {
                parameter_id: 99,
                source_object: 1,
            }),
            right: Box::new(expressions::Expression::Number {
                value: 2.0,
                spec_type_id: "number".into(),
                source_object: 2,
            }),
            source_object: 0,
        });
        formula.parameter_keys.insert(99, "input".into());
        values.insert(
            2,
            BTreeMap::from([
                ("result".into(), formula),
                ("input".into(), parameter(serde_json::json!(3.0), true)),
            ]),
        );
        assert!(resolve_parameter("result", 1, 2, &values, &mut Vec::new()).is_err());
        values.insert(
            1,
            BTreeMap::from([
                ("result".into(), parameter(serde_json::json!(6.0), true)),
                ("input".into(), parameter(serde_json::json!(7.0), true)),
            ]),
        );
        let (value, source) = resolve_parameter("result", 1, 2, &values, &mut Vec::new()).unwrap();
        assert_eq!(value, serde_json::json!(14.0));
        assert_eq!(source["cached_value"], serde_json::json!(6.0));
        values
            .get_mut(&2)
            .unwrap()
            .get_mut("result")
            .unwrap()
            .parameter_keys
            .insert(99, "result".into());
        assert!(
            resolve_parameter("result", 1, 2, &values, &mut Vec::new())
                .unwrap_err()
                .to_string()
                .contains("cyclic")
        );
    }
    #[test]
    fn extruded_profile_has_closed_oriented_topology_and_correct_measures() {
        let polygon = [
            [2.0, 3.0, 0.0],
            [4.0, 3.0, 0.0],
            [4.0, 6.0, 0.0],
            [2.0, 6.0, 0.0],
        ];
        let (vertices, triangles) = extrude(&polygon, [0.0, 0.0, 1.0], 1.0, 5.0).unwrap();
        let (volume, area) = measure(&vertices, &triangles);
        assert!((volume - 24.0).abs() < 1e-9);
        assert!((area - 52.0).abs() < 1e-9);
        let mut edges = BTreeMap::<(usize, usize), Vec<(usize, usize)>>::new();
        for triangle in &triangles {
            for i in 0..3 {
                let a = triangle[i];
                let b = triangle[(i + 1) % 3];
                edges.entry((a.min(b), a.max(b))).or_default().push((a, b));
            }
        }
        for occurrences in edges.values() {
            assert_eq!(occurrences.len(), 2);
            assert_eq!(occurrences[0], (occurrences[1].1, occurrences[1].0));
        }
    }
    #[test]
    fn nonconvex_nonplanar_and_reversed_profiles_refuse_mesh_substitution() {
        let polygon = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, 0.5, 0.0],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
        ];
        assert!(extrude(&polygon, [0.0, 0.0, 1.0], 0.0, 1.0).is_err());
        let bad = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 1.0],
            [0.0, 2.0, 0.0],
        ];
        assert!(extrude(&bad, [0.0, 0.0, 1.0], 0.0, 1.0).is_err());
        let reversed = [
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        ];
        assert!(extrude(&reversed, [0.0, 0.0, 1.0], 0.0, 1.0).is_err());
    }
    #[test]
    fn dimension_dependency_requires_constraint_evaluation() {
        let mut graph:ObjectGraph=serde_json::from_value(json!({"consumed_bytes":20,"objects":[{"class_tag":12,"class_name":"VarSketch","token":0,"start":2,"fields_end":20,"fields":{
            "m_userId":42,"m_dimIds":[],"m_weakDimIds":[],"m_dimData":[],"m_clineIds":[],"m_customDatumPlanes":[],"m_constrInfo":[],"m_flipXY":false,"m_flipZ":false,"m_pPlaneRef":{"pointer_token":0}}}],"edges":[]})).unwrap();
        qualify_sketch(&graph, 42).unwrap();
        graph.objects[0].fields["m_dimIds"] = json!([71]);
        assert!(
            qualify_sketch(&graph, 42)
                .unwrap_err()
                .to_string()
                .contains("constraint solver")
        );
    }
}
