//! Typed appearance asset trees through explicit native property ownership.
//! Values retain stored units. This does not evaluate a rendering shader or fetch textures.
use crate::native_parameters::ObjectGraph;
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
#[derive(Debug, Serialize)]
pub struct Node {
    pub name: String,
    pub class_name: String,
    pub object_index: usize,
    pub value: Option<Value>,
    pub unit_type_id: Option<String>,
    pub asset_type: Option<i64>,
    pub library: Option<String>,
    pub scene: Option<String>,
    pub properties: Vec<Node>,
    pub connected: Vec<Node>,
    pub child: Option<Box<Node>>,
    pub image_reference: Option<Value>,
}
#[derive(Debug, Serialize)]
pub struct Appearance {
    pub status: &'static str,
    pub semantics: &'static str,
    pub display_name: Option<String>,
    pub root: Option<Node>,
    pub material_path_map: Value,
    pub diagnostic: Option<String>,
    pub generic_shader_binding: Option<crate::native_shader::GenericBinding>,
}
fn target(g: &ObjectGraph, owner: usize, p: &Value) -> Result<usize> {
    let edges: Vec<_> = g
        .edges
        .iter()
        .filter(|e| {
            e.source_object_index == owner && Some(e.pointer_offset as u64) == p["offset"].as_u64()
        })
        .collect();
    ensure!(
        edges.len() == 1,
        "appearance property ownership absent/ambiguous"
    );
    let i = edges[0].target_object_index;
    ensure!(i < g.objects.len(), "appearance pointer outside graph");
    Ok(i)
}
pub fn project(g: &ObjectGraph) -> Appearance {
    let mut result = Appearance {
        status: "unresolved",
        semantics: "typed_saved_appearance_properties; no_shader_evaluation_or_resource_fetch",
        display_name: None,
        root: None,
        material_path_map: Value::Null,
        diagnostic: None,
        generic_shader_binding: None,
    };
    let parsed = (|| -> Result<Node> {
        let owner = g
            .objects
            .first()
            .ok_or_else(|| anyhow::anyhow!("appearance graph empty"))?;
        ensure!(
            owner.class_name == "AppearanceAssetElem",
            "appearance owner class"
        );
        result.material_path_map = owner.fields["m_materialPathMap"].clone();
        let i = target(g, 0, &owner.fields["m_pAppearanceAsset"])?;
        let appearance = &g.objects[i];
        ensure!(
            appearance.class_name == "AppearanceAsset",
            "appearance payload class"
        );
        result.display_name = appearance.fields["m_Name"].as_str().map(str::to_owned);
        node(
            g,
            i,
            &appearance.fields["m_Asset"],
            "Asset",
            &mut BTreeSet::new(),
            &mut 10_000,
        )
    })();
    match parsed {
        Ok(root) => {
            result.generic_shader_binding = crate::native_shader::bind_generic(&root);
            result.root = Some(root);
            result.status = "decoded_owned_asset_tree";
        }
        Err(e) => result.diagnostic = Some(format!("{e:#}")),
    };
    result
}
fn node(
    g: &ObjectGraph,
    i: usize,
    f: &Value,
    class: &str,
    path: &mut BTreeSet<usize>,
    budget: &mut usize,
) -> Result<Node> {
    ensure!(
        path.len() < 64 && *budget > 0,
        "appearance traversal budget"
    );
    *budget -= 1;
    ensure!(path.insert(i), "cyclic appearance property ownership");
    let value = match class {
        "Asset" => None,
        "APropertyString" => {
            ensure!(f["m_value"].is_string(), "appearance string shape");
            Some(f["m_value"].clone())
        }
        "APropertyBoolean" => {
            ensure!(f["m_value"].is_boolean(), "appearance boolean shape");
            Some(f["m_value"].clone())
        }
        "APropertyInteger" => {
            ensure!(f["m_value"].as_i64().is_some(), "appearance integer shape");
            Some(f["m_value"].clone())
        }
        "APropertyDouble1" | "APropertyDistance" => {
            ensure!(
                f["m_value"].as_f64().is_some_and(f64::is_finite),
                "appearance scalar shape"
            );
            Some(f["m_value"].clone())
        }
        "APropertyDouble2" | "APropertyDouble3" | "APropertyDouble4" => {
            let n = class.as_bytes()[class.len() - 1] - b'0';
            ensure!(
                f["m_value"]
                    .as_array()
                    .is_some_and(|a| a.len() == n as usize
                        && a.iter().all(|v| v.as_f64().is_some_and(f64::is_finite))),
                "appearance vector shape"
            );
            Some(f["m_value"].clone())
        }
        _ => anyhow::bail!("unsupported appearance property class {class}"),
    };
    let unit_type_id = if class == "APropertyDistance" {
        Some(
            f["m_unitTypeId"]["m_typeId"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| anyhow::anyhow!("appearance distance unit absent"))?
                .to_owned(),
        )
    } else {
        None
    };
    let mut out = Node {
        name: f["m_sName"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("appearance property name absent"))?
            .into(),
        class_name: class.into(),
        object_index: i,
        value,
        unit_type_id,
        asset_type: f["m_eAssetType"].as_i64(),
        library: f["m_sLibrary"].as_str().map(str::to_owned),
        scene: f["m_sScene"].as_str().map(str::to_owned),
        properties: vec![],
        connected: vec![],
        child: None,
        image_reference: None,
    };
    if class == "Asset" {
        ensure!(out.asset_type.is_some(), "appearance asset type absent");
        for p in f["m_aAProperties"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("asset property array absent"))?
        {
            out.properties.push(follow(g, i, p, path, budget)?);
        }
        if f["m_oImage"]["pointer_token"].as_u64() != Some(0) {
            let j = target(g, i, &f["m_oImage"])?;
            out.image_reference = Some(
                serde_json::json!({"object_index":j,"class_name":g.objects[j].class_name,"status":"preserved_native_graph_reference_not_decoded_image"}),
            );
        }
    }
    for p in f["m_aConnected"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("appearance connection array absent"))?
    {
        out.connected.push(follow(g, i, p, path, budget)?);
    }
    if f["m_aChild"]["pointer_token"].as_u64() != Some(0) {
        out.child = Some(Box::new(follow(g, i, &f["m_aChild"], path, budget)?));
    }
    path.remove(&i);
    Ok(out)
}
fn follow(
    g: &ObjectGraph,
    i: usize,
    p: &Value,
    path: &mut BTreeSet<usize>,
    budget: &mut usize,
) -> Result<Node> {
    let j = target(g, i, p)?;
    let o = &g.objects[j];
    node(g, j, &o.fields, &o.class_name, path, budget)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_parameters::{GraphEdge, GraphObject};
    use serde_json::json;
    #[test]
    fn distance_keeps_its_declared_unit_and_missing_unit_is_rejected() {
        let graph = ObjectGraph {
            consumed_bytes: 0,
            objects: vec![],
            edges: vec![],
        };
        let mut f = json!({"m_sName":"texture_RealWorldScaleX","m_value":4.25,"m_unitTypeId":{"m_typeId":"autodesk.unit.unit:inches-1.0.1"},"m_aChild":{"pointer_token":0},"m_aConnected":[]});
        let parsed = node(
            &graph,
            0,
            &f,
            "APropertyDistance",
            &mut BTreeSet::new(),
            &mut 10,
        )
        .unwrap();
        assert_eq!(parsed.value, Some(json!(4.25)));
        assert_eq!(
            parsed.unit_type_id.as_deref(),
            Some("autodesk.unit.unit:inches-1.0.1")
        );
        f["m_unitTypeId"] = Value::Null;
        assert!(
            node(
                &graph,
                0,
                &f,
                "APropertyDistance",
                &mut BTreeSet::new(),
                &mut 10
            )
            .is_err()
        );
    }
    #[test]
    fn cyclic_connected_property_is_rejected() {
        let f = json!({"m_sName":"loop","m_value":true,"m_aChild":{"pointer_token":0},"m_aConnected":[{"pointer_token":1,"offset":20}]});
        let graph = ObjectGraph {
            consumed_bytes: 24,
            objects: vec![GraphObject {
                class_tag: 44,
                class_name: "APropertyBoolean".into(),
                token: 1,
                start: 0,
                fields_end: 24,
                fields: f.clone(),
            }],
            edges: vec![GraphEdge {
                source_object_index: 0,
                pointer_offset: 20,
                pointer_token: 1,
                target_object_index: 0,
                target_class_tag: 44,
            }],
        };
        assert!(
            node(
                &graph,
                0,
                &f,
                "APropertyBoolean",
                &mut BTreeSet::new(),
                &mut 10
            )
            .unwrap_err()
            .to_string()
            .contains("cyclic")
        );
    }
}
