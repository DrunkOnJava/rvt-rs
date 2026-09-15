//! Bounded native Element base fields, using the structurally decoded schema.
//! Pointer targets are retained as serialized tokens; no graph target is guessed.
use crate::schema_registry::Registry;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointerToken {
    pub field: String,
    pub offset: usize,
    pub token: u32,
    pub class_tag: Option<u16>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementBase {
    pub class_tag: u16,
    pub class_name: String,
    pub pointers: Vec<PointerToken>,
    pub document_token: u32,
    pub id_offset: usize,
    pub id: i64,
    pub associated_level_id: i64,
    pub family_id: i64,
    pub unplaced_owner_id: i64,
    pub owner_view_id: i64,
    pub created_phase_id: i64,
    pub demolished_phase_id: i64,
    pub design_option_id: i64,
    pub locked: bool,
    pub moribund: bool,
    pub dummy: bool,
    pub end: usize,
}
/// Decode the base prefix of an independently bounded record body. Checks its
/// declared class inherits Element and the registry's base-field layout matches
/// the measured contract. Derived fields and deferred pointer bodies are not
/// decoded by this function; `end` identifies the first derived-field byte.
pub fn decode_base(body: &[u8], registry: &Registry) -> Result<ElementBase> {
    let take = |p: &mut usize, n: usize| -> Result<&[u8]> {
        let end = p
            .checked_add(n)
            .ok_or_else(|| anyhow::anyhow!("element prefix length overflow"))?;
        let bytes = body
            .get(*p..end)
            .ok_or_else(|| anyhow::anyhow!("truncated Element base at {}", *p))?;
        *p = end;
        Ok(bytes)
    };
    let mut p = 0;
    let class_tag = u16::from_le_bytes(take(&mut p, 2)?.try_into()?);
    let class = registry
        .class(class_tag)
        .ok_or_else(|| anyhow::anyhow!("unknown native class {class_tag}"))?;
    let element = registry
        .named("Element")
        .ok_or_else(|| anyhow::anyhow!("Element schema absent"))?;
    let mut parent = class;
    let mut found = false;
    for _ in 0..128 {
        if parent.tag == element.tag {
            found = true;
            break;
        }
        let Some(next) = registry.class(parent.parent_reference.tag) else {
            break;
        };
        parent = next;
    }
    ensure!(
        found,
        "{} does not have a bounded Element ancestry",
        class.name
    );
    let pointers = [
        "m_pParamValueSetDouble",
        "m_pParamValueSetInt",
        "m_pParamValueSetAString",
        "m_pParamValueSetElementId",
        "m_geomSteps",
        "m_pGeomTable",
        "m_constrInfo",
        "m_cellList",
    ];
    let ids = [
        "m_id",
        "m_assocLevelId",
        "m_famId",
        "m_unplacedOwnerId",
        "m_ownerDBViewId",
        "m_createdPhaseId",
        "m_demolishedPhaseId",
        "m_designOptionId",
    ];
    ensure!(
        element.fields.len() == 20,
        "unsupported Element field count"
    );
    for (i, name) in pointers.iter().enumerate() {
        let f = &element.fields[i];
        ensure!(
            f.name == *name && f.base == 14 && f.modifier == if i == 6 { 81 } else { 1 },
            "unsupported Element pointer field {i}"
        );
    }
    let doc = &element.fields[8];
    ensure!(
        doc.name == "m_docAccess" && doc.raw_descriptor == 14 && doc.references.len() == 1,
        "unsupported document access field"
    );
    let doc_class = registry
        .class(doc.references[0].tag)
        .ok_or_else(|| anyhow::anyhow!("document access schema missing"))?;
    ensure!(
        doc_class.fields.len() == 1
            && doc_class.fields[0].name == "m_pDoc"
            && doc_class.fields[0].raw_descriptor == 0x30e,
        "unsupported document pointer contract"
    );
    for (i, name) in ids.iter().enumerate() {
        let f = &element.fields[9 + i];
        ensure!(
            f.name == *name && f.raw_descriptor == 14 && f.references.len() == 1,
            "unsupported Element identity field {i}"
        );
        let wrapper = registry
            .class(f.references[0].tag)
            .ok_or_else(|| anyhow::anyhow!("identity wrapper missing"))?;
        ensure!(
            wrapper.name == "ElementId"
                && wrapper.fields.len() == 1
                && wrapper.fields[0].raw_descriptor == 14
                && wrapper.fields[0].references.len() == 1,
            "unsupported ElementId wrapper"
        );
        let id = registry
            .class(wrapper.fields[0].references[0].tag)
            .ok_or_else(|| anyhow::anyhow!("Identifier schema missing"))?;
        ensure!(
            id.fields.len() == 1
                && id.fields[0].name == "m_id64"
                && id.fields[0].raw_descriptor == 11,
            "unsupported Identifier value"
        );
    }
    for (i, name) in ["m_locked", "m_moribund", "m_dummy"].iter().enumerate() {
        ensure!(
            element.fields[17 + i].name == *name && element.fields[17 + i].raw_descriptor == 1,
            "unsupported Element flag layout"
        );
    }
    let decoded = crate::native_parameters::decode_object_fields(body, registry, 2, element.tag)?;
    let fields = &decoded.fields;
    let mut tokens = Vec::new();
    for name in pointers {
        let field = &fields[name];
        let list = if name == "m_constrInfo" {
            field
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("constraint reference container missing"))?
                .clone()
        } else {
            vec![field.clone()]
        };
        for (i, value) in list.iter().enumerate() {
            tokens.push(PointerToken {
                field: if name == "m_constrInfo" {
                    format!("{name}[{i}]")
                } else {
                    name.into()
                },
                offset: value["offset"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("pointer offset absent"))?
                    as usize,
                token: value["pointer_token"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("pointer token absent"))?
                    as u32,
                class_tag: value["class_tag"].as_u64().map(|v| v as u16),
            });
        }
    }
    let document_token = fields["m_docAccess"]["m_pDoc"]["pointer_token"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("document pointer absent"))? as u32;
    let id_offset = decoded
        .field_spans
        .iter()
        .find(|s| s.class_tag == element.tag && s.name == "m_id")
        .ok_or_else(|| anyhow::anyhow!("Element identity field span absent"))?
        .start;
    let mut values = [0i64; 8];
    for (i, name) in ids.iter().enumerate() {
        values[i] = fields[*name]["m_id"]["m_id64"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("Element identity value absent: {name}"))?;
    }
    Ok(ElementBase {
        class_tag,
        class_name: class.name.clone(),
        pointers: tokens,
        document_token,
        id_offset,
        id: values[0],
        associated_level_id: values[1],
        family_id: values[2],
        unplaced_owner_id: values[3],
        owner_view_id: values[4],
        created_phase_id: values[5],
        demolished_phase_id: values[6],
        design_option_id: values[7],
        locked: fields["m_locked"].as_bool().unwrap(),
        moribund: fields["m_moribund"].as_bool().unwrap(),
        dummy: fields["m_dummy"].as_bool().unwrap(),
        end: decoded.end,
    })
}
