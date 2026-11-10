//! RE-398 (#1165): the step a `WallShadowCutoutGStep` mirrors.
//!
//! FACT under test: a wall's `SWall` record lists a `WallShadowCutoutGStep` for
//! a cutter whose cut is held by another wall's record, and names that wall
//! (`m_otherWallId`) and the other wall's step (`m_ggId` is that step's
//! `m_id`). The other wall's step is an embed or a cutout of the same cutter.
//! `native_wall_cuts::resolve_shadow_steps` reads the shadow as that step; this
//! lists every shadow step of the file with what its names lead to, so the
//! rule is held to every shadow step and not to the ones that gave it.
//!
//! Output:
//!   shadow <wall> cutter=<id> <cutter's IFC class> other=<wall> gg=<m_ggId>
//!     <resolves kind=<Embed|Cutout> cuts_something=<..>|no: <why>>
//!   shadow <wall> cutter=<id> no m_otherWallId or m_ggId
//!   steps walls=<n> steps=<n> embed=<n> cutout=<n> rect=<n> shadow=<n>
//!     resolved=<n> unresolved=<n> ids_missing=<n>
//! where <why> is one of: the other wall is not an exported wall, the other wall
//! holds no step with that id, the step with that id is of another cutter, the
//! step with that id is of another kind.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re398_shadow_cutout_links -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_wall_cuts::{WallCutKind, wall_cut_steps};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut walls: BTreeSet<u64> = BTreeSet::new();
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        else {
            continue;
        };
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        if ifc_type.eq_ignore_ascii_case("IFCWALL") {
            walls.insert(id);
        }
        classes.entry(id).or_insert_with(|| ifc_type.clone());
    }
    let steps = wall_cut_steps(&mut rf, &walls)?;
    let (mut embed, mut cutout, mut rect, mut shadow) = (0usize, 0usize, 0usize, 0usize);
    let (mut resolved, mut unresolved, mut ids_missing) = (0usize, 0usize, 0usize);
    let mut total = 0usize;
    for (wall, list) in &steps {
        for step in list {
            total += 1;
            match step.kind {
                WallCutKind::Embed => embed += 1,
                WallCutKind::Cutout => cutout += 1,
                WallCutKind::RectOpening => rect += 1,
                WallCutKind::ShadowCutout => shadow += 1,
            }
            if step.kind != WallCutKind::ShadowCutout {
                continue;
            }
            let class = classes
                .get(&step.cutter)
                .map_or("none", std::string::String::as_str);
            let Some((other, gg)) = step.mirrors else {
                ids_missing += 1;
                println!(
                    "shadow {wall} cutter={} {class} no m_otherWallId or m_ggId",
                    step.cutter
                );
                continue;
            };
            let verdict = match steps.get(&other) {
                None => "no: the other wall is not an exported wall".to_string(),
                Some(held) => match held.iter().find(|candidate| candidate.id == Some(gg)) {
                    None => "no: the other wall holds no step with that id".to_string(),
                    Some(candidate) if candidate.cutter != step.cutter => {
                        "no: the step with that id is of another cutter".to_string()
                    }
                    Some(candidate)
                        if !matches!(candidate.kind, WallCutKind::Embed | WallCutKind::Cutout) =>
                    {
                        "no: the step with that id is of another kind".to_string()
                    }
                    Some(candidate) => format!(
                        "resolves kind={:?} cuts_something={:?}",
                        candidate.kind, candidate.cuts_something
                    ),
                },
            };
            if verdict.starts_with("resolves") {
                resolved += 1;
            } else {
                unresolved += 1;
            }
            println!(
                "shadow {wall} cutter={} {class} other={other} gg={gg} {verdict}",
                step.cutter
            );
        }
    }
    println!(
        "steps walls={} steps={total} embed={embed} cutout={cutout} rect={rect} shadow={shadow} \
         resolved={resolved} unresolved={unresolved} ids_missing={ids_missing}",
        steps.len()
    );
    Ok(())
}
