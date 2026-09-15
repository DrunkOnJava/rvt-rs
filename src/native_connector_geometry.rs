//! Connector geometry derived from explicit saved frames and curve parameters.
use super::{Port, Source, source, target};
use crate::{native_document::Record, native_metadata::identifier, native_parameters::ObjectGraph};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize)]
pub struct Frame {
    pub origin: [f64; 3],
    pub basis_x: [f64; 3],
    pub basis_y: [f64; 3],
    pub basis_z: [f64; 3],
}
#[derive(Debug, Clone, Serialize)]
pub struct Geometry {
    pub status: String,
    pub coordinate_space: &'static str,
    pub length_unit: &'static str,
    pub frame: Option<Frame>,
    pub shape: Option<&'static str>,
    pub radius: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub sources: Vec<Source>,
    pub diagnostics: Vec<String>,
    pub parameter_sources: Vec<Value>,
}
impl Geometry {
    pub(super) fn unresolved() -> Self {
        Self {
            status: "unavailable".into(),
            coordinate_space: "host_document",
            length_unit: "feet",
            frame: None,
            shape: None,
            radius: None,
            width: None,
            height: None,
            sources: Vec::new(),
            diagnostics: Vec::new(),
            parameter_sources: Vec::new(),
        }
    }
}
struct Instance {
    symbol_id: i64,
    computed_symbol_id: i64,
    frame: Frame,
    source: Source,
}
struct Symbol {
    family_id: i64,
    master_id: i64,
    frames: BTreeMap<i64, Frame>,
    source: Source,
}
struct Definition {
    domain: i64,
    profile: i64,
    parameters: BTreeMap<i64, Value>,
    bound: BTreeSet<i64>,
    bindings: BTreeMap<i64, i64>,
    sources: Vec<Source>,
}
struct Pipe {
    origin: [f64; 3],
    direction: [f64; 3],
    start: f64,
    end: f64,
    normal: [f64; 3],
    diameter: f64,
    sources: Vec<Source>,
}
#[derive(Default)]
pub(super) struct Context {
    instances: BTreeMap<i64, Result<Instance, String>>,
    symbols: BTreeMap<i64, Result<Symbol, String>>,
    families: BTreeMap<i64, Result<BTreeMap<i64, Definition>, String>>,
    pipes: BTreeMap<i64, Result<Pipe, String>>,
    parameters: crate::native_family_geometry::ParameterContext,
    parameter_errors: BTreeMap<i64, String>,
    dependencies: BTreeSet<i64>,
}
impl Context {
    pub fn ingest(&mut self, record: &Record) {
        let id = record.identity.element_id as i64;
        if let Err(error) = self.parameters.ingest(record) {
            self.parameter_errors.insert(id, error.to_string());
        }
        let graph = record
            .graph
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("native geometry context graph unavailable"));
        match record.class_name.as_deref() {
            Some("FamilyInstance") => {
                let parsed = record
                    .graph
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("native geometry context graph unavailable"))
                    .and_then(|g| instance(record, g))
                    .map_err(|e| e.to_string());
                if let Ok(value) = &parsed {
                    self.dependencies.insert(value.symbol_id);
                    self.dependencies.insert(value.computed_symbol_id);
                }
                self.instances.insert(id, parsed);
            }
            Some("FamilySymbol") => {
                let parsed = record
                    .graph
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("native geometry context graph unavailable"))
                    .and_then(|g| symbol(record, g))
                    .map_err(|e| e.to_string());
                if let Ok(value) = &parsed {
                    self.dependencies.insert(value.family_id);
                    self.dependencies.insert(value.master_id);
                }
                self.symbols.insert(id, parsed);
            }
            Some("Family") => {
                self.families.insert(
                    id,
                    graph
                        .and_then(|g| family(record, g))
                        .map_err(|e| e.to_string()),
                );
            }
            Some("RbsPipeCurve") => {
                self.pipes.insert(
                    id,
                    graph
                        .and_then(|g| pipe(record, g))
                        .map_err(|e| e.to_string()),
                );
            }
            _ => {}
        }
    }

    pub fn dependency_ids(&self) -> BTreeSet<i64> {
        self.dependencies.clone()
    }
    pub fn project(&self, port: &Port) -> Geometry {
        let mut result = Geometry::unresolved();
        if port.mode == 4 {
            result.status = "not_applicable_logical".into();
            return result;
        }
        let outcome = (|| -> Result<()> {
            ensure!(
                port.mode == 1,
                "connector mode has no qualified physical frame"
            );
            if port.owner_class == "FamilyInstance" {
                ensure!(
                    port.modifiers
                        .iter()
                        .any(|m| m.class_name == "FamilyConnectorPosition"),
                    "family position modifier absent"
                );
                let instance = lookup(&self.instances, port.key.owner_element_id)?;
                ensure!(
                    dot(
                        cross(instance.frame.basis_x, instance.frame.basis_y),
                        instance.frame.basis_z
                    ) > 0.0,
                    "mirrored connector frame composition is not yet qualified"
                );
                let symbol = lookup(&self.symbols, instance.symbol_id)?;
                let computed_symbol = lookup(&self.symbols, instance.computed_symbol_id)?;
                ensure!(
                    computed_symbol.family_id == symbol.family_id,
                    "computed connector symbol belongs to another family"
                );
                ensure!(
                    instance.computed_symbol_id == instance.symbol_id
                        || computed_symbol.master_id == instance.symbol_id,
                    "computed connector symbol master mismatch"
                );
                let local = computed_symbol
                    .frames
                    .get(&port.key.connector_id)
                    .ok_or_else(|| anyhow::anyhow!("symbol connector frame unavailable"))?;
                result.frame = Some(compose(&instance.frame, local));
                result.sources.extend([
                    instance.source.clone(),
                    symbol.source.clone(),
                    computed_symbol.source.clone(),
                ]);
                let definitions = lookup(&self.families, symbol.family_id)?;
                let definition = definitions
                    .get(&port.key.connector_id)
                    .ok_or_else(|| anyhow::anyhow!("family connector definition unavailable"))?;
                result.sources.extend(definition.sources.clone());
                ensure!(
                    definition.domain == 3 && definition.profile == 0,
                    "connector profile/domain has no qualified dimensions"
                );
                result.shape = Some("Round");
                let radius = if definition.bindings.contains_key(&-1133401)
                    || definition.bindings.contains_key(&-1133415)
                {
                    ensure!(
                        !(definition.bindings.contains_key(&-1133401)
                            && definition.bindings.contains_key(&-1133415)),
                        "simultaneous radius/diameter bindings ambiguous"
                    );
                    ensure!(
                        !self
                            .parameter_errors
                            .contains_key(&port.key.owner_element_id)
                            && !self.parameter_errors.contains_key(&instance.symbol_id),
                        "connector parameter context incomplete"
                    );
                    let (property, factor) = if definition.bindings.contains_key(&-1133401) {
                        (-1133401, 1.0)
                    } else {
                        (-1133415, 0.5)
                    };
                    let (value, proof) = self.parameters.resolve_id(
                        definition.bindings[&property],
                        port.key.owner_element_id as u64,
                        instance.symbol_id as u64,
                    )?;
                    result.parameter_sources.push(serde_json::json!({"role":"evaluated_connector_dimension_association","property_id":property,"family_parameter_id":definition.bindings[&property],"radius_conversion_factor":factor,"parameter_value":proof}));
                    number(&value)? * factor
                } else {
                    ensure!(
                        !definition.bound.contains(&-1133401),
                        "associated radius unresolved"
                    );
                    let parameter = definition
                        .parameters
                        .get(&-1133401)
                        .ok_or_else(|| anyhow::anyhow!("saved connector radius absent"))?;
                    ensure!(
                        parameter["m_oExpression"]["pointer_token"].as_u64() == Some(0)
                            && parameter["m_reporting"].as_bool() == Some(false),
                        "connector radius expression/reporting dependency"
                    );
                    number(&parameter["m_value"])?
                };
                ensure!(radius > 0.0, "nonpositive connector radius");
                result.radius = Some(radius);
            } else if port.owner_class == "RbsPipeCurve" {
                let pipe = lookup(&self.pipes, port.key.owner_element_id)?;
                let modifiers: Vec<_> = port
                    .modifiers
                    .iter()
                    .filter(|m| m.class_name == "SegmentConnectorPosition")
                    .collect();
                ensure!(
                    modifiers.len() == 1,
                    "unique segment position modifier required"
                );
                let t = number(&modifiers[0].fields["m_paramOnCurve"])?;
                ensure!(
                    t == 0.0 || t == 1.0,
                    "interior segment port frame unqualified"
                );
                let tangent = scale(pipe.direction, 1.0 / norm(pipe.direction));
                let z = scale(tangent, if t == 0.0 { -1.0 } else { 1.0 });
                let y = scale(pipe.normal, -1.0);
                let x = cross(y, z);
                ensure!(
                    (norm(y) - 1.0).abs() < 1e-8 && dot(y, z).abs() < 1e-8,
                    "segment normal does not establish orthonormal frame"
                );
                result.frame = Some(Frame {
                    origin: add(
                        pipe.origin,
                        scale(pipe.direction, pipe.start + (pipe.end - pipe.start) * t),
                    ),
                    basis_x: x,
                    basis_y: y,
                    basis_z: z,
                });
                result.shape = Some("Round");
                result.radius = Some(pipe.diameter / 2.0);
                result.sources.extend(pipe.sources.clone());
                result.sources.push(modifiers[0].source.clone());
            } else {
                anyhow::bail!("owner class has no qualified physical connector geometry");
            }
            Ok(())
        })();
        match outcome {
            Ok(()) => result.status = "resolved".into(),
            Err(error) => {
                result.status = if result.frame.is_some() {
                    "partial"
                } else {
                    "unavailable"
                }
                .into();
                result.diagnostics.push(error.to_string());
            }
        }
        result
    }
}
fn lookup<T>(map: &BTreeMap<i64, Result<T, String>>, id: i64) -> Result<&T> {
    map.get(&id)
        .ok_or_else(|| anyhow::anyhow!("geometry context owner {id} absent"))?
        .as_ref()
        .map_err(|e| anyhow::anyhow!("owner {id}: {e}"))
}
fn required(graph: &ObjectGraph, owner: usize, value: &Value) -> Result<usize> {
    target(graph, owner, value)?.ok_or_else(|| anyhow::anyhow!("geometry context pointer is null"))
}
fn instance(record: &Record, graph: &ObjectGraph) -> Result<Instance> {
    let root = &graph.objects[0];
    let index = required(graph, 0, &root.fields["m_pInstanceInfo"])?;
    ensure!(
        graph.objects[index].class_name == "InstanceInfo",
        "instance info class"
    );
    Ok(Instance {
        symbol_id: identifier(&root.fields["m_masterSymbolId"])?,
        computed_symbol_id: identifier(&graph.objects[index].fields["m_symbolId"])?,
        frame: frame(&graph.objects[index].fields["m_Trf"])?,
        source: source(
            record,
            index,
            "m_pInstanceInfo.InstanceInfo.m_Trf;m_symbolId (computed geometry symbol);root.m_masterSymbolId (declared parameter symbol)",
        ),
    })
}
fn symbol(record: &Record, graph: &ObjectGraph) -> Result<Symbol> {
    let root = &graph.objects[0];
    let mut frames = BTreeMap::new();
    for row in root.fields["m_arrConnectorData"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("symbol connector frames absent"))?
    {
        let id = number_i64(&row["m_index"])?;
        ensure!(
            frames.insert(id, frame(&row["m_trf"])?).is_none(),
            "duplicate symbol connector frame ID"
        );
    }
    Ok(Symbol {
        family_id: identifier(&root.fields["m_familyId"])?,
        master_id: identifier(&root.fields["m_masterId"])?,
        frames,
        source: source(record, 0, "m_arrConnectorData[m_index].m_trf"),
    })
}
fn family(record: &Record, graph: &ObjectGraph) -> Result<BTreeMap<i64, Definition>> {
    let root = &graph.objects[0];
    let mut result = BTreeMap::new();
    let Some(list) = target(graph, 0, &root.fields["m_cellList"])? else {
        return Ok(result);
    };
    ensure!(
        graph.objects[list].class_name == "CellList",
        "family cell list class"
    );
    for pointer in graph.objects[list].fields["m_cells"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("family cells absent"))?
    {
        let index = required(graph, list, pointer)?;
        let cell = &graph.objects[index];
        if cell.class_name != "ConnectorDataCell" {
            continue;
        }
        let params_index = required(graph, index, &cell.fields["m_oRelevantParams"])?;
        ensure!(
            graph.objects[params_index].class_name == "FamilyParams",
            "connector parameters class"
        );
        let mut parameters = BTreeMap::new();
        for row in graph.objects[params_index].fields["m_params"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("connector parameter slots absent"))?
        {
            ensure!(
                parameters
                    .insert(identifier(&row["m_paramId"])?, row.clone())
                    .is_none(),
                "duplicate connector parameter"
            );
        }
        let mut bound = BTreeSet::new();
        let mut bindings = BTreeMap::new();
        for row in cell.fields["m_propId2FamParamIdBindings"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("connector binding map absent"))?
        {
            bound.insert(identifier(&row["first"])?);
            ensure!(
                bindings
                    .insert(identifier(&row["first"])?, identifier(&row["second"])?)
                    .is_none(),
                "duplicate connector binding"
            );
        }
        let definition = Definition {
            domain: number_i64(&cell.fields["m_domain"])?,
            profile: number_i64(&cell.fields["m_profileType"])?,
            parameters,
            bound,
            bindings,
            sources: vec![
                source(record, index, "m_cellList.ConnectorDataCell[m_index]"),
                source(
                    record,
                    params_index,
                    "m_oRelevantParams.FamilyParams.m_params[-1133401]",
                ),
            ],
        };
        ensure!(
            result
                .insert(number_i64(&cell.fields["m_index"])?, definition)
                .is_none(),
            "duplicate family connector definition"
        );
    }
    Ok(result)
}
fn pipe(record: &Record, graph: &ObjectGraph) -> Result<Pipe> {
    let root = &graph.objects[0];
    let driver = required(graph, 0, &root.fields["m_pCurveDriver"])?;
    ensure!(
        graph.objects[driver].class_name == "RbsCurveDriver",
        "pipe curve driver class"
    );
    let fields = &graph.objects[driver].fields;
    ensure!(
        fields["m_flip"].as_bool() == Some(false),
        "flipped pipe driver unqualified"
    );
    let curve = required(graph, driver, &fields["m_pCrv"])?;
    ensure!(
        graph.objects[curve].class_name == "GLine",
        "curved pipe connector frame unqualified"
    );
    let fields = &graph.objects[curve].fields;
    let bounds = fields["m_endParams"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("pipe curve bounds absent"))?;
    ensure!(bounds.len() == 2, "pipe bound count");
    let direction = vector(&fields["m_dirVec"])?;
    ensure!(norm(direction) > 1e-10, "degenerate pipe direction");
    let diameter = number(&root.fields["m_dWidthOrDiameter"])?;
    ensure!(diameter > 0.0, "nonpositive pipe diameter");
    Ok(Pipe {
        origin: vector(&fields["m_origin"])?,
        direction,
        start: number(&bounds[0])?,
        end: number(&bounds[1])?,
        normal: vector(&root.fields["m_vNormal"])?,
        diameter,
        sources: vec![
            source(record, curve, "m_pCurveDriver.RbsCurveDriver.m_pCrv.GLine"),
            source(record, 0, "m_vNormal;m_dWidthOrDiameter"),
        ],
    })
}
fn frame(v: &Value) -> Result<Frame> {
    let rows = v["m_3x3"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("frame basis absent"))?;
    ensure!(rows.len() == 3, "frame row count");
    let a = vector(&rows[0])?;
    let b = vector(&rows[1])?;
    let c = vector(&rows[2])?;
    let frame = Frame {
        origin: vector(&v["m_or"])?,
        basis_x: [a[0], b[0], c[0]],
        basis_y: [a[1], b[1], c[1]],
        basis_z: [a[2], b[2], c[2]],
    };
    for axis in [frame.basis_x, frame.basis_y, frame.basis_z] {
        ensure!(
            (norm(axis) - 1.0).abs() < 1e-8,
            "non-unit connector frame basis"
        );
    }
    ensure!(
        dot(frame.basis_x, frame.basis_y).abs() < 1e-8
            && dot(frame.basis_x, frame.basis_z).abs() < 1e-8
            && dot(frame.basis_y, frame.basis_z).abs() < 1e-8,
        "nonorthogonal connector frame basis"
    );
    Ok(frame)
}
fn compose(a: &Frame, b: &Frame) -> Frame {
    Frame {
        origin: add(a.origin, rotate(a, b.origin)),
        basis_x: rotate(a, b.basis_x),
        basis_y: rotate(a, b.basis_y),
        basis_z: rotate(a, b.basis_z),
    }
}
fn rotate(f: &Frame, v: [f64; 3]) -> [f64; 3] {
    add(
        scale(f.basis_x, v[0]),
        add(scale(f.basis_y, v[1]), scale(f.basis_z, v[2])),
    )
}
fn number_i64(v: &Value) -> Result<i64> {
    v.as_i64().ok_or_else(|| anyhow::anyhow!("integer absent"))
}
fn number(v: &Value) -> Result<f64> {
    let v = v.as_f64().ok_or_else(|| anyhow::anyhow!("number absent"))?;
    ensure!(v.is_finite(), "nonfinite geometry number");
    Ok(v)
}
fn vector(v: &Value) -> Result<[f64; 3]> {
    let a = v
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("vector absent"))?;
    ensure!(a.len() == 3, "vector size");
    Ok([number(&a[0])?, number(&a[1])?, number(&a[2])?])
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
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
    #[test]
    fn row_major_frames_compose_position_and_all_basis_vectors() {
        let owner = frame(
            &json!({"m_or":[10.0,-5.0,2.0],"m_3x3":[[0.0,-1.0,0.0],[1.0,0.0,0.0],[0.0,0.0,1.0]]}),
        )
        .unwrap();
        let local = frame(
            &json!({"m_or":[4.0,1.0,1.0],"m_3x3":[[0.0,0.0,1.0],[1.0,0.0,0.0],[0.0,1.0,0.0]]}),
        )
        .unwrap();
        let world = compose(&owner, &local);
        assert_eq!(world.origin, [9.0, -1.0, 3.0]);
        assert_eq!(world.basis_x, [-1.0, 0.0, 0.0]);
        assert_eq!(world.basis_y, [0.0, 0.0, 1.0]);
        assert_eq!(world.basis_z, [0.0, 1.0, 0.0]);
    }
    #[test]
    fn scaled_or_sheared_basis_does_not_claim_unscaled_dimensions() {
        assert!(
            frame(
                &json!({"m_or":[0.0,0.0,0.0],"m_3x3":[[2.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]]})
            )
            .is_err()
        );
        assert!(
            frame(
                &json!({"m_or":[0.0,0.0,0.0],"m_3x3":[[1.0,1.0,0.0],[0.0,0.0,0.0],[0.0,0.0,1.0]]})
            )
            .is_err()
        );
    }
}
