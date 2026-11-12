//! Bounded native Element base fields, using the structurally decoded schema.
//! Pointer targets are retained as serialized tokens; no graph target is guessed.
use crate::schema_registry::{Class, Field, Registry};
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
/// The Element fields that open every record after its class tag: eight
/// pointers (the seventh a container of them), the document access, the
/// eight identifiers of [`BASE_IDS`] and three flags.
const BASE_POINTERS: [&str; 8] = [
    "m_pParamValueSetDouble",
    "m_pParamValueSetInt",
    "m_pParamValueSetAString",
    "m_pParamValueSetElementId",
    "m_geomSteps",
    "m_pGeomTable",
    "m_constrInfo",
    "m_cellList",
];
const BASE_IDS: [&str; 8] = [
    "m_id",
    "m_assocLevelId",
    "m_famId",
    "m_unplacedOwnerId",
    "m_ownerDBViewId",
    "m_createdPhaseId",
    "m_demolishedPhaseId",
    "m_designOptionId",
];

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
    let pointers = BASE_POINTERS;
    let ids = BASE_IDS;
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
                && matches!(
                    (id.fields[0].name.as_str(), id.fields[0].raw_descriptor),
                    ("m_id64", 11) | ("m_id", 4)
                ),
            "unsupported Identifier value"
        );
    }
    for (i, name) in ["m_locked", "m_moribund", "m_dummy"].iter().enumerate() {
        ensure!(
            element.fields[17 + i].name == *name && element.fields[17 + i].raw_descriptor == 1,
            "unsupported Element flag layout"
        );
    }
    if let Some(base) = read_base(body, registry, class_tag, class, element) {
        return Ok(base);
    }
    read_base_by_graph(body, registry, class_tag, class, element)
}

/// [`decode_base`]'s fields as the graph decoder reads them, which also
/// says what is wrong with a prefix [`read_base`] does not take.
fn read_base_by_graph(
    body: &[u8],
    registry: &Registry,
    class_tag: u16,
    class: &Class,
    element: &Class,
) -> Result<ElementBase> {
    let pointers = BASE_POINTERS;
    let ids = BASE_IDS;
    // Only the identity field's span is read below: spans of the nested
    // objects' fields would copy a name each (#1237).
    let decoded = crate::native_parameters::decode_object_fields_spanning(
        body,
        registry,
        2,
        element.tag,
        &crate::native_parameters::GraphLimits::default(),
        crate::native_parameters::Spans::Class(element.tag),
    )?;
    let fields = &decoded.fields;
    let mut tokens = Vec::new();
    for name in pointers {
        let field = &fields[name];
        let list = if name == "m_constrInfo" {
            field
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("constraint reference container missing"))?
                .to_vec()
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
        values[i] = crate::native_json::identifier_i64(&fields[*name])
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
        locked: fields["m_locked"]
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("Element flag absent: m_locked"))?,
        moribund: fields["m_moribund"]
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("Element flag absent: m_moribund"))?,
        dummy: fields["m_dummy"]
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("Element flag absent: m_dummy"))?,
        end: decoded.end,
    })
}

/// Whether `field` is a field this reader takes: no flags, and this base
/// type and modifier.
fn plain(field: &Field, base: u8, modifier: u8) -> bool {
    field.uninterpreted_flags == 0 && field.base == base && field.modifier == modifier
}

/// [`decode_base`]'s fields read straight from the bytes, for the layout its
/// checks measured. `None` for anything else, where [`read_base_by_graph`]
/// reads them and says what is wrong: a field with flags, a class with a
/// parent of its own, a pointer to a class the schema lacks, a flag above 1,
/// more than 1,024 constraint references, or a body that ends early. The
/// graph decoder builds a JSON value for each of the prefix's fields and for
/// the objects inside them, which was most of a phase read of every record of
/// a Revit 2015 to 2022 project (#1237).
fn read_base(
    body: &[u8],
    registry: &Registry,
    class_tag: u16,
    class: &Class,
    element: &Class,
) -> Option<ElementBase> {
    // A class with a parent of its own is read with the parent's fields
    // first.
    let alone = |class: &Class| class.parent_reference.tag < 12;
    let fields = &element.fields;
    if !alone(element) || fields.len() != 20 {
        return None;
    }
    for (i, field) in fields[..8].iter().enumerate() {
        if !plain(field, 14, if i == 6 { 81 } else { 1 }) {
            return None;
        }
    }
    let doc = &fields[8];
    if !plain(doc, 14, 0) || doc.references.len() != 1 {
        return None;
    }
    let doc_class = registry.class(doc.references[0].tag)?;
    if !alone(doc_class) || doc_class.fields.len() != 1 || !plain(&doc_class.fields[0], 14, 3) {
        return None;
    }
    // Whether each identifier is an 8-byte `m_id64` or a 4-byte `m_id`.
    let mut wide = [false; 8];
    for (i, is_wide) in wide.iter_mut().enumerate() {
        let field = &fields[9 + i];
        if !plain(field, 14, 0) || field.references.len() != 1 {
            return None;
        }
        let wrapper = registry.class(field.references[0].tag)?;
        let inner = wrapper.fields.first()?;
        if !alone(wrapper)
            || wrapper.fields.len() != 1
            || inner.name != "m_id"
            || !plain(inner, 14, 0)
            || inner.references.len() != 1
        {
            return None;
        }
        let identifier = registry.class(inner.references[0].tag)?;
        let value = identifier.fields.first()?;
        if !alone(identifier) || identifier.fields.len() != 1 {
            return None;
        }
        *is_wide = if value.name == "m_id64" && plain(value, 11, 0) {
            true
        } else if value.name == "m_id" && plain(value, 4, 0) {
            false
        } else {
            return None;
        };
    }
    if !fields[17..].iter().all(|flag| plain(flag, 1, 0)) {
        return None;
    }
    let u32_at = |pos: &mut usize| -> Option<u32> {
        let end = pos.checked_add(4)?;
        let value = u32::from_le_bytes(body.get(*pos..end)?.try_into().ok()?);
        *pos = end;
        Some(value)
    };
    let pointer = |pos: &mut usize, field: String| -> Option<PointerToken> {
        let offset = *pos;
        let token = u32_at(pos)?;
        let class_tag = if token != 0 {
            let end = pos.checked_add(2)?;
            let tag = u16::from_le_bytes(body.get(*pos..end)?.try_into().ok()?);
            *pos = end;
            registry.class(tag)?;
            Some(tag)
        } else {
            None
        };
        Some(PointerToken {
            field,
            offset,
            token,
            class_tag,
        })
    };
    let mut pos = 2;
    let mut pointers = Vec::with_capacity(BASE_POINTERS.len());
    for (i, name) in BASE_POINTERS.iter().enumerate() {
        if i == 6 {
            let count = u32_at(&mut pos)?;
            if count > 1024 {
                return None;
            }
            for item in 0..count {
                pointers.push(pointer(&mut pos, format!("{name}[{item}]"))?);
            }
        } else {
            pointers.push(pointer(&mut pos, (*name).into())?);
        }
    }
    let document_token = u32_at(&mut pos)?;
    let id_offset = pos;
    let mut values = [0i64; 8];
    for (value, is_wide) in values.iter_mut().zip(wide) {
        *value = if is_wide {
            let end = pos.checked_add(8)?;
            let id = i64::from_le_bytes(body.get(pos..end)?.try_into().ok()?);
            pos = end;
            id
        } else {
            let end = pos.checked_add(4)?;
            let id = i32::from_le_bytes(body.get(pos..end)?.try_into().ok()?);
            pos = end;
            i64::from(id)
        };
    }
    let mut flags = [false; 3];
    for flag in &mut flags {
        let byte = *body.get(pos)?;
        if byte > 1 {
            return None;
        }
        pos += 1;
        *flag = byte != 0;
    }
    Some(ElementBase {
        class_tag,
        class_name: class.name.clone(),
        pointers,
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
        locked: flags[0],
        moribund: flags[1],
        dummy: flags[2],
        end: pos,
    })
}
