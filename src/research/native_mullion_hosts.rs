//! Which curtain mullions Revit's export draws as an extrusion of their
//! section along their length (#1016, RE-243).
//!
//! A curtain mullion's `Qto_MemberBaseQuantities.NetVolume` is its solid's
//! volume. Revit's export draws a mullion as an extrusion of its section,
//! whose volume is that section's area times its length, except where the
//! mullion is curved or cut: on the BIM4LCA office as built every mullion it
//! writes as a mesh stands on a curved curtain wall (an `ArcWall` record) or
//! on one whose record holds an edited profile (an `ExtrusionGStep`). A
//! mullion's host is the `m_hostId` and its type the `m_masterSymbolId` its
//! native record names; a type whose `SysMullionFamSym.m_profileId` is set
//! draws a profile family's section rather than its own. A type's record
//! holds one section object whose class names its shape (RE-246), the same
//! in every language: [`BOX_SECTION_CLASSES`] are those whose section is a
//! full rectangle, which the mullion's box gives. The mullions of one curtain
//! wall are those that name it as their host (`m_hostId`), which
//! [`mullion_hosts`] returns with the plain ones (`NetSurfaceArea` groups
//! them by it, RE-268).

use crate::native_json::{agree, element_id_i64};
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// The section classes of a rectangular mullion type and a quad corner
/// mullion type's square (RE-246); the others are `SysCircMullionProfile`,
/// `SysLMullionProfile`, `SysVMullionProfile`, `SysTrapMullionProfile` and,
/// for a profile family's section, `SysCustMullionProfile`.
pub const BOX_SECTION_CLASSES: [&str; 2] = ["SysRectMullionProfile", "SysQuadMullionProfile"];

/// The first id `field` of any of `graph`'s objects names, where positive.
fn linked(graph: &crate::native_parameters::ObjectGraph, field: &str) -> Option<u64> {
    graph
        .objects
        .iter()
        .find_map(|object| object.fields.get(field).and_then(element_id_i64))
        .and_then(|id| u64::try_from(id).ok())
}

/// The curtain wall each mullion's native record names, and the mullions
/// among them that stand on plain hosts.
#[derive(Debug, Clone, Default)]
pub struct MullionHosts {
    /// A mullion's host: the `m_hostId` of its native record, where every
    /// copy of the record agrees on it and on its type.
    pub host: BTreeMap<u64, u64>,
    /// The mullions whose host, the curtain wall their native
    /// record names, is a straight wall (`SWall`) whose record holds neither an
    /// `ArcWall` nor an `ExtrusionGStep`, and whose type's record
    /// (`SysMullionFamSym`) sets no profile and holds a
    /// [`BOX_SECTION_CLASSES`] section.
    pub plain: BTreeSet<u64>,
}

/// The hosts of `mullions`, and those that stand on plain hosts, read in one
/// pass. A mullion, host or type whose copies disagree is left out.
pub fn mullion_hosts(rf: &mut RevitFile, mullions: &BTreeSet<u64>) -> anyhow::Result<MullionHosts> {
    if mullions.is_empty() {
        return Ok(MullionHosts::default());
    }
    let options = native_document::Options {
        selected_ids: mullions.clone(),
        selected_classes: ["SWall", "ArcWall", "SysMullionFamSym"]
            .map(String::from)
            .into_iter()
            .collect(),
        ..native_document::Options::default()
    };
    let mut links: BTreeMap<u64, Option<(u64, u64)>> = BTreeMap::new();
    let mut plain: BTreeMap<u64, Option<bool>> = BTreeMap::new();
    native_document::extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match class {
            "SWall" | "ArcWall" => {
                let straight = class == "SWall"
                    && !graph.objects.iter().any(|object| {
                        matches!(object.class_name.as_str(), "ArcWall" | "ExtrusionGStep")
                    });
                agree(&mut plain, id, Some(straight));
            }
            "SysMullionFamSym" => {
                let own = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "SysMullionFamSym")
                    .and_then(|object| object.fields.get("m_profileId").and_then(element_id_i64))
                    .map(|profile| profile < 0);
                let boxed = graph
                    .objects
                    .iter()
                    .any(|object| BOX_SECTION_CLASSES.contains(&object.class_name.as_str()));
                agree(&mut plain, id, own.map(|own| own && boxed));
            }
            _ => {}
        }
        if mullions.contains(&id) {
            let link = linked(graph, "m_hostId").zip(linked(graph, "m_masterSymbolId"));
            agree(&mut links, id, link);
        }
        Ok(())
    })?;
    let is_plain = |id: &u64| plain.get(id).copied().flatten() == Some(true);
    let mut out = MullionHosts::default();
    for (mullion, link) in links {
        let Some((host, mullion_type)) = link else {
            continue;
        };
        out.host.insert(mullion, host);
        if is_plain(&host) && is_plain(&mullion_type) {
            out.plain.insert(mullion);
        }
    }
    Ok(out)
}

/// The mullions of `mullions` that stand on plain hosts
/// ([`MullionHosts::plain`]).
pub fn mullions_on_plain_hosts(
    rf: &mut RevitFile,
    mullions: &BTreeSet<u64>,
) -> anyhow::Result<BTreeSet<u64>> {
    Ok(mullion_hosts(rf, mullions)?.plain)
}
