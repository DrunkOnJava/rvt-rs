//! Membership in the witnessed DDC18.4.3 exporter-created standard 3D profile.
//!
//! This is not visibility in an arbitrary saved Revit view. The converter creates
//! a fresh DBView3d, with Show Complete, fine detail, no user clipping or overrides.
//! Native runtime witnesses qualify the three physical owner classes below;
//! options, phase transitions and view-specific elements remain unsupported.
use crate::native_parameters::ObjectGraph;
use serde_json::{Value, json};

/// `current_phase_id` must come from the current native document phase catalog.
/// A positive result applies only to the explicitly named newly created profile.
/// A false result excludes an unplaced group-definition owner; callers should not
/// turn it into a serialized false metadata field when their export omits it.
pub fn evaluate_created_standard_3d(
    owner: &ObjectGraph,
    current_phase_id: i64,
) -> Result<(bool, Value), String> {
    let o = owner.objects.first().ok_or("missing current owner")?;
    if !matches!(o.class_name.as_str(), "Floor" | "SWall" | "FamilyInstance") {
        return Err("owner class outside witnessed standard-3D profile".into());
    }
    let f = &o.fields;
    let id = |name: &str| {
        f[name]["m_id"]["m_id64"]
            .as_i64()
            .ok_or_else(|| format!("missing {name}"))
    };
    let owner_id = id("m_id")?;
    let unplaced = id("m_unplacedOwnerId")?;
    if owner_id <= 0 || current_phase_id <= 0 {
        return Err("invalid owner or current phase identity".into());
    }
    let provenance = |visible| {
        json!({
            "method":"ddc_18_4_3_created_standard_3d_membership",
            "scope":"exporter-created DBView3d; Show Complete; default category/class/rule/clipping state",
            "owner_id":owner_id,"owner_class":o.class_name,
            "unplaced_owner_id":unplaced,"current_phase_id":current_phase_id,
            "visible":visible,
            "qualification":"native isVisibleInThisView runtime; Floor/SWall/FamilyInstance; no universal saved-view claim"
        })
    };
    if unplaced > 0 {
        return Ok((false, provenance(false)));
    }
    if unplaced != -1 {
        return Err("unknown unplaced-owner sentinel".into());
    }
    for field in ["m_ownerDBViewId", "m_designOptionId", "m_demolishedPhaseId"] {
        if id(field)? != -1 {
            return Err(format!("{field} outside witnessed default-view scope"));
        }
    }
    if id("m_createdPhaseId")? != current_phase_id {
        return Err("phase transition outside witnessed Show Complete scope".into());
    }
    for field in ["m_dummy", "m_moribund"] {
        if f[field].as_bool() != Some(false) {
            return Err(format!("{field} is not explicitly false"));
        }
    }
    if o.class_name == "FamilyInstance" {
        for field in ["m_invisible", "m_instDormant"] {
            if f[field].as_bool() != Some(false) {
                return Err(format!("{field} outside witnessed family visibility scope"));
            }
        }
    }
    Ok((true, provenance(true)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_parameters::GraphObject;
    fn owner() -> ObjectGraph {
        let mut f = json!({"m_dummy":false,"m_moribund":false});
        for (k, v) in [
            ("m_id", 71),
            ("m_unplacedOwnerId", -1),
            ("m_ownerDBViewId", -1),
            ("m_designOptionId", -1),
            ("m_demolishedPhaseId", -1),
            ("m_createdPhaseId", 17),
        ] {
            f[k] = json!({"m_id":{"m_id64":v}});
        }
        ObjectGraph {
            objects: vec![GraphObject {
                class_tag: 1,
                class_name: "Floor".into(),
                token: 0,
                start: 0,
                fields_end: 0,
                fields: f,
            }],
            edges: vec![],
            consumed_bytes: 0,
        }
    }
    #[test]
    fn excludes_definitions_and_refuses_unqualified_view_states() {
        let mut g = owner();
        assert!(evaluate_created_standard_3d(&g, 17).unwrap().0);
        assert!(evaluate_created_standard_3d(&g, 18).is_err());
        g.objects[0].fields["m_designOptionId"] = json!({"m_id":{"m_id64":19}});
        assert!(evaluate_created_standard_3d(&g, 17).is_err());
        g.objects[0].fields["m_unplacedOwnerId"] = json!({"m_id":{"m_id64":900}});
        assert!(!evaluate_created_standard_3d(&g, 17).unwrap().0);
    }
}
