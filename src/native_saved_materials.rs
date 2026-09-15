//! Saved material ownership and the qualified DDC Generic viewport conversion.
use crate::{native_document::Record, native_metadata::identifier, native_parameters::ObjectGraph};
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct RenderMaterial {
    pub material_id: Option<i64>,
    pub name: String,
    pub base_color: [f64; 4],
    pub saved_opacity: Option<f64>,
    pub metallic_factor: f64,
    pub roughness_factor: f64,
    pub diagnostics: Vec<String>,
    pub provenance: Vec<Value>,
}
#[derive(Debug, Clone, Serialize)]
pub struct QuantityMaterial {
    pub material_id: i64,
    pub provenance: Vec<Value>,
}
#[derive(Default)]
pub struct Resolver {
    ddc_initialized_generic_traits: bool,
    materials: BTreeMap<i64, RenderMaterial>,
    material_identities: BTreeMap<i64, Value>,
    appearance_colors: BTreeMap<i64, ([f64; 4], Value)>,
    appearance_refs: BTreeMap<i64, i64>,
    styles: BTreeMap<i64, (i64, Value)>,
    category_styles: BTreeMap<(i64, i64), Vec<(i64, Value)>>,
    owner_types: BTreeMap<u64, (i64, Value)>,
    type_materials: BTreeMap<i64, (i64, Value)>,
    paint: BTreeMap<(u64, i64), (i64, Value)>,
    pub diagnostics: Vec<String>,
}
impl Resolver {
    /// Opt into the independently measured DDC 18.4.3 default viewport trait context.
    pub fn ddc_viewport_profile() -> Self {
        Self {
            ddc_initialized_generic_traits: true,
            ..Self::default()
        }
    }

    pub fn ingest(&mut self, r: &Record) -> Result<()> {
        if r.channel != 102 {
            return Ok(());
        }
        if let Err(e) = self.project(r) {
            self.diagnostics
                .push(format!("{}: {e:#}", r.identity.element_id));
        }
        Ok(())
    }
    fn project(&mut self, r: &Record) -> Result<()> {
        let Some(g) = &r.graph else {
            return Ok(());
        };
        let Some(root) = g.objects.first() else {
            return Ok(());
        };
        let f = &root.fields;
        let id = r.identity.element_id;
        match root.class_name.as_str() {
            "MaterialElem" => {
                let (i, m) = target(g, 0, &f["m_pMaterial"], "Material")?;
                ensure!(
                    self.material_identities
                        .insert(
                            id as i64,
                            source(r, i, "m_pMaterial/current MaterialElem identity")
                        )
                        .is_none(),
                    "duplicate material identity"
                );
                let material = project_material(r, g, i, m, self.ddc_initialized_generic_traits)?;
                if m["m_bUseRenderAppearance"].as_bool() == Some(true) {
                    self.appearance_refs
                        .insert(id as i64, identifier(&m["m_appearanceAssetId"])?);
                }
                ensure!(
                    self.materials.insert(id as i64, material).is_none(),
                    "duplicate material owner"
                );
            }
            "AppearanceAssetElem" => {
                let (i, a) = target(g, 0, &f["m_pAppearanceAsset"], "AppearanceAsset")?;
                self.appearance_colors.insert(
                    id as i64,
                    (packed_color(a)?, source(r, i, "m_color/m_transparency")),
                );
            }
            "GStyleElem" => {
                let (i, s) = target(g, 0, &f["m_pGStyle"], "GStyle")?;
                if let (Some(category), Some(kind)) = (
                    f.get("m_categoryId"),
                    f.get("m_gstyleType").and_then(Value::as_i64),
                ) {
                    self.category_styles.entry((identifier(category)?, kind)).or_default().push((identifier(&s["m_materialElemId"])?, json!({"style_owner": source(r, 0, "m_categoryId/m_gstyleType"), "material_reference": source(r, i, "m_materialElemId")})));
                }
                self.styles.insert(
                    id as i64,
                    (
                        identifier(&s["m_materialElemId"])?,
                        source(r, i, "m_materialElemId"),
                    ),
                );
            }
            "SWall" | "Floor" => {
                let field = if root.class_name == "SWall" {
                    "m_WallAttributesId"
                } else {
                    "m_floorAttributesId"
                };
                if let Some(v) = f.get(field) {
                    self.owner_types
                        .insert(id, (identifier(v)?, source(r, 0, field)));
                }
            }
            "BasicWallType" | "FloorAttributes" => {
                let (i, c) = target(g, 0, &f["m_pCompoundStructure"], "CompoundStructure")?;
                let layers = c["m_layers"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("missing layers"))?;
                if layers.len() == 1 && c["m_variableLayerIdx"].as_i64() == Some(-1) {
                    self.type_materials.insert(
                        id as i64,
                        (
                            identifier(&layers[0]["m_materialId"])?,
                            source(r, i, "m_layers[0].m_materialId"),
                        ),
                    );
                }
            }
            _ => {}
        }
        if let Some(p) = f.get("m_pGeomTable") {
            if p["pointer_token"].as_u64() != Some(0) {
                let (i, t) = target(g, 0, p, "GeomTable")?;
                if let Some(markers) = t["m_materialMarkers"].as_array() {
                    for p in markers {
                        let (mi, m) = target(g, i, p, "GeomMaterialMarker")?;
                        let tag = m["m_geomTag"]
                            .as_i64()
                            .ok_or_else(|| anyhow::anyhow!("invalid material marker tag"))?;
                        ensure!(
                            self.paint
                                .insert(
                                    (id, tag),
                                    (
                                        identifier(&m["m_materialId"])?,
                                        source(r, mi, "m_materialId")
                                    )
                                )
                                .is_none(),
                            "duplicate paint marker"
                        );
                    }
                }
            }
        }
        Ok(())
    }
    /// Quantity identity uses category material for an explicitly unassigned
    /// constant host layer; viewport rendering retains its separate context.
    pub fn resolve_quantity_material(
        &self,
        owner_id: u64,
        source_owner_id: Option<u64>,
        face_tag: i64,
        render_style_id: i64,
        explicit_material_id: Option<i64>,
        category_id: i64,
    ) -> Option<QuantityMaterial> {
        if !self.paint.contains_key(&(owner_id, face_tag))
            && explicit_material_id.is_none_or(|id| id <= 0)
        {
            if let Some((ty, owner_source)) = self.owner_types.get(&owner_id) {
                if let Some((-1, layer_source)) = self.type_materials.get(ty) {
                    let a = self.category_styles.get(&(category_id, 1))?;
                    let b = self.category_styles.get(&(category_id, 2))?;
                    if a.len() != 1 || b.len() != 1 || a[0].0 <= 0 || a[0].0 != b[0].0 {
                        return None;
                    }
                    let identity = self.material_identities.get(&a[0].0)?;
                    return Some(QuantityMaterial {
                        material_id: a[0].0,
                        provenance: vec![
                            identity.clone(),
                            owner_source.clone(),
                            layer_source.clone(),
                            a[0].1.clone(),
                            b[0].1.clone(),
                            json!({"resolution":"constant_unassigned_host_layer_category_material", "category_id":category_id,"style_kinds_agree":[1,2],"not_viewport_default":true}),
                        ],
                    });
                }
            }
        }
        let (selected, mut provenance) = self.select_material(
            owner_id,
            source_owner_id,
            face_tag,
            render_style_id,
            explicit_material_id,
        );
        provenance.push(self.material_identities.get(&selected)?.clone());
        provenance.push(json!({"resolution":"saved_material_identity_only", "appearance_evaluation_required":false}));
        Some(QuantityMaterial {
            material_id: selected,
            provenance,
        })
    }
    fn select_material(
        &self,
        owner_id: u64,
        source_owner_id: Option<u64>,
        face_tag: i64,
        render_style_id: i64,
        explicit_material_id: Option<i64>,
    ) -> (i64, Vec<Value>) {
        let mut provenance = Vec::new();
        let selected = if let Some((id, s)) = self.paint.get(&(owner_id, face_tag)) {
            provenance.push(s.clone());
            *id
        } else if let Some(id) = explicit_material_id.filter(|id| *id > 0) {
            id
        } else if let Some((id, src)) = self.owner_types.get(&owner_id).and_then(|(ty, s)| {
            self.type_materials
                .get(ty)
                .filter(|(id, _)| *id > 0)
                .map(|(id, t)| (*id, vec![s.clone(), t.clone()]))
        }) {
            provenance.extend(src);
            id
        } else if let Some((id, s)) = source_owner_id.and_then(|id| self.paint.get(&(id, face_tag)))
        {
            provenance.push(s.clone());
            *id
        } else if let Some((id, s)) = self.styles.get(&render_style_id) {
            provenance.push(s.clone());
            *id
        } else {
            render_style_id
        };
        (selected, provenance)
    }
    pub fn resolve(
        &self,
        owner_id: u64,
        source_owner_id: Option<u64>,
        face_tag: i64,
        render_style_id: i64,
        explicit_material_id: Option<i64>,
    ) -> Option<RenderMaterial> {
        let (selected, mut provenance) = self.select_material(
            owner_id,
            source_owner_id,
            face_tag,
            render_style_id,
            explicit_material_id,
        );
        let mut m = if selected == -1 && self.ddc_initialized_generic_traits {
            RenderMaterial {
                material_id: None,
                name: "DDC viewport default".to_string(),
                saved_opacity: None,
                base_color: [127.0 / 255.0, 127.0 / 255.0, 127.0 / 255.0, 1.0],
                metallic_factor: 0.0,
                roughness_factor: generic_roughness(0.5, 0.1).ok()?,
                diagnostics: vec![],
                provenance: vec![
                    json!({"source":"DDC18.4.3_initialized_viewport_traits","not_serialized_material_property":true,"diffuse_packed_rgb":8355711,"direct_reflectivity":0.5,"glossiness":0.1}),
                ],
            }
        } else {
            self.materials.get(&selected)?.clone()
        };
        if let Some(appearance) = self.appearance_refs.get(&selected) {
            let (color, src) = self.appearance_colors.get(appearance)?;
            m.base_color = *color;
            m.saved_opacity = Some(color[3]);
            provenance.push(src.clone());
        }
        provenance.push(json!({"owner_id":owner_id,"source_owner_id":source_owner_id,"face_tag":face_tag,"render_style_id":render_style_id,"explicit_material_id":explicit_material_id,"resolution":"saved_owner_type_paint_or_render_style"}));
        if self.ddc_initialized_generic_traits {
            provenance.push(json!({"source":"DDC18.4.3_opaque_default_viewport_profile","saved_material_opacity":m.base_color[3],"exported_opacity":1.0,"not_serialized_material_property":true}));
            m.base_color[3] = 1.0;
        }
        m.provenance.extend(provenance);
        Some(m)
    }
}
fn source(r: &Record, i: usize, field: &str) -> Value {
    json!({"owner_id":r.identity.element_id,"unique_id":r.identity.unique_id,"object_index":i,"field":field,"record_source":r.source})
}
fn target<'a>(
    g: &'a ObjectGraph,
    owner: usize,
    p: &Value,
    class: &str,
) -> Result<(usize, &'a Value)> {
    let offset = p["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing pointer offset"))?;
    let es: Vec<_> = g
        .edges
        .iter()
        .filter(|e| e.source_object_index == owner && e.pointer_offset as u64 == offset)
        .collect();
    ensure!(es.len() == 1, "pointer has no unique target");
    let i = es[0].target_object_index;
    let o = &g.objects[i];
    ensure!(
        o.class_name == class,
        "unexpected target class {}",
        o.class_name
    );
    Ok((i, &o.fields))
}
fn project_material(
    r: &Record,
    g: &ObjectGraph,
    i: usize,
    m: &Value,
    ddc_context: bool,
) -> Result<RenderMaterial> {
    ensure!(
        m["m_bUseRenderAppearance"].as_bool().is_some(),
        "missing diffuse source flag"
    );
    let base_color = packed_color(m)?;
    let asset = &m["m_asset"];
    let schema = asset["m_sName"].as_str().unwrap_or("");
    let mut provenance = vec![source(
        r,
        i,
        "m_color/m_transparency/m_bUseRenderAppearance/m_asset",
    )];
    let roughness = if schema == "Generic"
        && identifier(&m["m_appearanceAssetId"])? == -1
        && asset["m_aAProperties"]
            .as_array()
            .is_some_and(Vec::is_empty)
    {
        1.0
    } else if schema == "HardwoodSchema" && ddc_context {
        provenance.push(json!({"source":"DDC18.4.3_initialized_viewport_traits","not_serialized_material_property":true,"direct_reflectivity":0.5,"glossiness":0.1,"reason":"Hardwood structural handler preserves initialized specular gloss"}));
        generic_roughness(0.5, 0.1)?
    } else {
        ensure!(
            ["GenericSchema", "MetalSchema", "WallPaintSchema"].contains(&schema),
            "appearance schema {schema} not qualified for viewport roughness"
        );
        let mut props = BTreeMap::new();
        for p in asset["m_aAProperties"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing asset properties"))?
        {
            let offset = p["offset"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("property pointer absent"))?;
            let es: Vec<_> = g
                .edges
                .iter()
                .filter(|e| e.source_object_index == i && e.pointer_offset as u64 == offset)
                .collect();
            ensure!(es.len() == 1, "asset property has no unique owned target");
            let pi = es[0].target_object_index;
            let f = &g.objects[pi].fields;
            if let Some(name) = f["m_sName"].as_str() {
                ensure!(
                    props.insert(name, (pi, &f["m_value"])).is_none(),
                    "duplicate asset property"
                );
            }
        }
        if schema == "MetalSchema" || schema == "WallPaintSchema" {
            let key = if schema == "MetalSchema" {
                "metal_finish"
            } else {
                "wallpaint_finish"
            };
            let (pi, value) = props
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("missing finish property"))?;
            ensure!(
                value.as_i64() == Some(0),
                "finish variant outside measured render profile"
            );
            provenance.push(source(r, *pi, "m_value"));
            if schema == "MetalSchema" { 0.7 } else { 0.9 }
        } else {
            let (ri, rv) = props
                .get("generic_reflectivity_at_0deg")
                .ok_or_else(|| anyhow::anyhow!("missing direct reflectivity"))?;
            let (gi, gv) = props
                .get("generic_glossiness")
                .ok_or_else(|| anyhow::anyhow!("missing glossiness"))?;
            provenance.push(source(r, *ri, "m_value"));
            provenance.push(source(r, *gi, "m_value"));
            generic_roughness(
                rv.as_f64()
                    .ok_or_else(|| anyhow::anyhow!("invalid reflectivity"))?,
                gv.as_f64()
                    .ok_or_else(|| anyhow::anyhow!("invalid glossiness"))?,
            )?
        }
    };
    provenance.push(json!({"conversion_profile":"DDC18.4.3_default_viewport","saved_asset_schema":schema,"roughness_rules":{"GenericSchema":"1-sqrt(direct)*(1-min(glossiness,0.999)^4)","Generic_without_asset":"1","MetalSchema_finish0":"1-0.3","WallPaintSchema_finish0":"1-0.1","HardwoodSchema":"preserve explicitly selected initialized Generic trait context"},"metallic":"exporter zero factor","diffuse":"saved packed graphics RGB / 255"}));
    Ok(RenderMaterial {
        material_id: Some(r.identity.element_id as i64),
        name: m["m_name"].as_str().unwrap_or("").to_string(),
        base_color,
        saved_opacity: Some(base_color[3]),
        metallic_factor: 0.0,
        roughness_factor: roughness,
        diagnostics: Vec::new(),
        provenance,
    })
}
fn packed_color(m: &Value) -> Result<[f64; 4]> {
    let color = m["m_color"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("missing packed color"))?;
    ensure!(color <= 0xffffff, "packed color outside RGB range");
    let alpha = 1.0
        - m["m_transparency"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("missing transparency"))?;
    ensure!((0.0..=1.0).contains(&alpha), "invalid transparency");
    Ok([
        (color & 255) as f64 / 255.0,
        ((color >> 8) & 255) as f64 / 255.0,
        ((color >> 16) & 255) as f64 / 255.0,
        alpha,
    ])
}
fn generic_roughness(direct: f64, gloss: f64) -> Result<f64> {
    ensure!(
        direct.is_finite()
            && gloss.is_finite()
            && (0.0..=1.0).contains(&direct)
            && (0.0..=1.0).contains(&gloss),
        "Generic reflectivity/glossiness outside qualified domain"
    );
    Ok(1.0 - direct.sqrt() * (1.0 - gloss.min(0.999).powi(4)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quantity_category_identity_requires_both_unique_styles_and_null_layer() {
        let mut r = Resolver::ddc_viewport_profile();
        r.material_identities.insert(87, json!({"owner":87}));
        r.owner_types.insert(10, (20, json!({"owner":10})));
        r.type_materials.insert(20, (-1, json!({"layer":0})));
        r.category_styles
            .insert((-2000011, 1), vec![(87, json!({"style":113}))]);
        assert!(
            r.resolve_quantity_material(10, None, 1, -1, None, -2000011)
                .is_none()
        );
        r.category_styles
            .insert((-2000011, 2), vec![(87, json!({"style":114}))]);
        assert_eq!(
            r.resolve_quantity_material(10, None, 1, -1, None, -2000011)
                .unwrap()
                .material_id,
            87
        );
        assert_eq!(r.resolve(10, None, 1, -1, None).unwrap().material_id, None);
        r.category_styles.get_mut(&(-2000011, 2)).unwrap()[0].0 = 88;
        assert!(
            r.resolve_quantity_material(10, None, 1, -1, None, -2000011)
                .is_none()
        );
        r.category_styles.get_mut(&(-2000011, 2)).unwrap()[0].0 = 87;
        r.category_styles
            .get_mut(&(-2000011, 1))
            .unwrap()
            .push((87, json!({})));
        assert!(
            r.resolve_quantity_material(10, None, 1, -1, None, -2000011)
                .is_none()
        );
    }
    #[test]
    fn quantity_identity_does_not_require_qualified_appearance() {
        let mut r = Resolver::default();
        r.material_identities
            .insert(7, json!({"owner_id":7,"class":"MaterialElem"}));
        r.appearance_refs.insert(7, 99);
        r.styles.insert(42, (7, json!({"owner_id":42})));
        assert!(r.resolve(1, None, 0, 42, None).is_none());
        assert_eq!(
            r.resolve_quantity_material(1, None, 0, 42, None, -2000014)
                .unwrap()
                .material_id,
            7
        );
        assert!(
            r.resolve_quantity_material(1, None, 0, 8, None, -2000014)
                .is_none()
        );
        r.paint.insert((1, 0), (8, json!({"owner_id":1})));
        assert!(
            r.resolve_quantity_material(1, None, 0, 42, Some(7), -2000014)
                .is_none()
        );
    }
    #[test]
    fn generic_conversion_measured_nontrivial_values() {
        assert!((generic_roughness(0.5, 0.1).unwrap() - 0.29296392949157113).abs() < 1e-15);
        assert_eq!(generic_roughness(0.0, 0.5).unwrap(), 1.0);
        assert!(generic_roughness(-0.1, 0.5).is_err());
        assert!(generic_roughness(0.5, f64::NAN).is_err());
    }
    #[test]
    fn unresolved_style_is_not_a_gray_material() {
        assert!(Resolver::default().resolve(1, None, 0, 534, None).is_none());
        assert!(
            Resolver::ddc_viewport_profile()
                .resolve(1, None, 0, 534, None)
                .is_none()
        );
        assert!(Resolver::default().resolve(1, None, 0, -1, None).is_none());
        assert_eq!(
            Resolver::ddc_viewport_profile()
                .resolve(1, None, 0, -1, None)
                .unwrap()
                .material_id,
            None
        );
    }
    #[test]
    fn paint_and_single_layer_context_override_saved_default_style() {
        let mut r = Resolver::default();
        for id in [7, 8, 9] {
            r.materials.insert(
                id,
                RenderMaterial {
                    material_id: Some(id),
                    name: id.to_string(),
                    base_color: [0.0, 0.0, 0.0, 1.0],
                    saved_opacity: Some(1.0),
                    metallic_factor: 0.0,
                    roughness_factor: 1.0,
                    diagnostics: vec![],
                    provenance: vec![],
                },
            );
        }
        r.owner_types.insert(100, (200, json!({"field":"type"})));
        r.type_materials.insert(200, (8, json!({"field":"layer"})));
        assert_eq!(
            r.resolve(100, None, 3, 7, None).unwrap().material_id,
            Some(8)
        );
        r.type_materials
            .insert(200, (-1, json!({"field":"unassigned layer"})));
        assert_eq!(
            r.resolve(100, None, 3, 7, None).unwrap().material_id,
            Some(7)
        );
        r.type_materials.insert(200, (8, json!({"field":"layer"})));
        r.paint.insert((100, 3), (9, json!({"field":"paint"})));
        assert_eq!(
            r.resolve(100, None, 3, 7, Some(7)).unwrap().material_id,
            Some(9)
        );
        assert_eq!(
            r.resolve(100, None, 4, 7, Some(7)).unwrap().material_id,
            Some(7)
        );
    }
    #[test]
    fn appearance_color_reference_must_resolve() {
        let mut r = Resolver::default();
        r.materials.insert(
            7,
            RenderMaterial {
                material_id: Some(7),
                name: String::new(),
                base_color: [0.0; 4],
                saved_opacity: Some(0.0),
                metallic_factor: 0.0,
                roughness_factor: 1.0,
                diagnostics: vec![],
                provenance: vec![],
            },
        );
        r.appearance_refs.insert(7, 8);
        assert!(r.resolve(1, None, 0, 7, None).is_none());
        r.appearance_colors
            .insert(8, ([0.1, 0.2, 0.3, 1.0], json!({"owner_id":8})));
        assert_eq!(
            r.resolve(1, None, 0, 7, None).unwrap().base_color,
            [0.1, 0.2, 0.3, 1.0]
        );
    }
}
