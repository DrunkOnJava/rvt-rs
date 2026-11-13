//! RE-456: the steps of a wall's native record that name an element cutting
//! it, and the rectangle a rectangular wall opening saves, on Revit 2015 to
//! 2027, by the file's own native records.
//!
//! FACT under test: `native_wall_cuts` read no step on Revit 2023 and
//! earlier, so the openings RE-321 writes (a rectangular wall opening's, a
//! door's in a second wall, a cabinet's, a curtain wall's) were not written
//! there. Hypothesis: the steps are in the records on 2015 to 2023 as on
//! 2024, and only the shape of their ElementId differs: `m_instId` is
//! `{"m_id":{"m_id":n}}` (4 bytes) where 2024 saves `{"m_id":{"m_id64":n}}`.
//!
//! Output, one record per line:
//!   file <path> release <year>
//!   shapes m_id <count> m_id64 <count>
//!   step <wall ElementId> <kind> <cutter ElementId> <m_bCutSomething|->
//!   rect <opening ElementId> <min x y z> <max x y z>
//!   face <opening ElementId> <host> <min x y z> <max x y z> saved <agrees|differs|none>
//!   walls <count> with_steps <count> steps <count> rect <count>
//!   faces <count> agree <count> differ <count> face_only <count>
//!
//! `shapes` counts the `m_instId` fields of the cut steps of every `SWall`
//! and `ArcWall` record by the key their Identifier holds. `step` and `rect`
//! are what `native_wall_cuts::wall_cut_steps` and `rect_opening_boxes`
//! read, so the probe shows the library's reading, not its own. Run it on the
//! same project saved by two releases to compare them.
//!
//! `face` is the probe's own reading of a rectangle's face corners
//! (`RectOpeningData.m_lowerLeftCorner`, `m_upperRightCorner`, `[u, v]`):
//! the host wall's location line (`VWallDriver.m_pCrv`, a `GLine`) at `u`,
//! raised by `v`. Where the save also holds `m_aPoint` (2019 on) it says
//! whether the two boxes agree to 1e-6 ft; 2015 to 2018 save only the face
//! corners (`none`).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re456_wall_cut_steps -- MODEL.rvt...

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_wall_cuts::{WallCutKind, rect_opening_boxes, wall_cut_steps};

const CUT_STEPS: [&str; 4] = [
    "WallEmbedWallGStep",
    "WallCutoutGStep",
    "WallRectOpeningGStep",
    "WallShadowCutoutGStep",
];

fn probe(path: &str) -> Result<()> {
    let mut rf = RevitFile::open(path)?;
    println!("file {path} release {}", rf.layout_release()?);

    // Every wall, and the shape of each cut step's `m_instId`.
    let mut walls = BTreeSet::new();
    let (mut narrow, mut wide) = (0usize, 0usize);
    let options = Options {
        selected_classes: BTreeSet::from(["SWall".to_string(), "ArcWall".to_string()]),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        walls.insert(record.identity.element_id);
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        for object in &graph.objects {
            if !CUT_STEPS.contains(&object.class_name.as_str()) {
                continue;
            }
            let Some(identifier) = object.fields.get("m_instId").and_then(|f| f.get("m_id")) else {
                continue;
            };
            if identifier.get("m_id64").is_some() {
                wide += 1;
            } else if identifier.get("m_id").is_some() {
                narrow += 1;
            }
        }
        Ok(())
    })?;
    println!("shapes m_id {narrow} m_id64 {wide}");

    let steps = wall_cut_steps(&mut rf, &walls)?;
    let mut rects = BTreeSet::new();
    let mut count = 0;
    for (wall, list) in &steps {
        for step in list {
            count += 1;
            let kind = match step.kind {
                WallCutKind::Embed => "embed",
                WallCutKind::Cutout => "cutout",
                WallCutKind::RectOpening => {
                    rects.insert(step.cutter);
                    "rect_opening"
                }
                WallCutKind::ShadowCutout => "shadow_cutout",
            };
            let flag = step
                .cuts_something
                .map_or("-".to_string(), |flag| flag.to_string());
            println!("step {wall} {kind} {} {flag}", step.cutter);
        }
    }
    let boxes = rect_opening_boxes(&mut rf, &rects)?;
    for (id, b) in &boxes {
        println!(
            "rect {id} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        );
    }
    let (faces, agree, differ, face_only) = face_rectangles(&mut rf, &rects)?;
    let with_steps = steps.values().filter(|list| !list.is_empty()).count();
    println!(
        "walls {} with_steps {with_steps} steps {count} rect {}",
        walls.len(),
        boxes.len()
    );
    println!("faces {faces} agree {agree} differ {differ} face_only {face_only}");
    Ok(())
}

fn triple(value: Option<&Fv>) -> Option<[f64; 3]> {
    match value?.as_array()? {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?]),
        _ => None,
    }
}

fn pair(value: Option<&Fv>) -> Option<[f64; 2]> {
    match value?.as_array()? {
        [u, v] => Some([u.as_f64()?, v.as_f64()?]),
        _ => None,
    }
}

fn identifier(value: Option<&Fv>) -> Option<u64> {
    let id = value?.get("m_id")?;
    id.get("m_id64")
        .or_else(|| id.get("m_id"))?
        .as_i64()
        .and_then(|id| u64::try_from(id).ok())
}

fn corner_box(a: [f64; 3], b: [f64; 3]) -> [f64; 6] {
    [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].min(b[2]),
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].max(b[2]),
    ]
}

/// Each rectangle's face corners placed on its host wall's location line,
/// against its `m_aPoint` where saved.
fn face_rectangles(
    rf: &mut RevitFile,
    rects: &BTreeSet<u64>,
) -> Result<(usize, usize, usize, usize)> {
    type Face = (u64, [f64; 2], [f64; 2], Option<[f64; 6]>);
    let mut found: BTreeMap<u64, Face> = BTreeMap::new();
    if rects.is_empty() {
        return Ok((0, 0, 0, 0));
    }
    let options = Options {
        selected_ids: rects.clone(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let find = |class: &str| graph.objects.iter().find(|o| o.class_name == class);
        let (Some(data), Some(opening)) = (find("RectOpeningData"), find("SWallRectOpening"))
        else {
            return Ok(());
        };
        let saved = data
            .fields
            .get("m_aPoint")
            .and_then(Fv::as_array)
            .and_then(|points| match points {
                [a, b] => Some(corner_box(triple(Some(a))?, triple(Some(b))?)),
                _ => None,
            });
        if let (Some(host), Some(low), Some(high)) = (
            identifier(opening.fields.get("m_hostId")),
            pair(data.fields.get("m_lowerLeftCorner")),
            pair(data.fields.get("m_upperRightCorner")),
        ) {
            found.insert(record.identity.element_id, (host, low, high, saved));
        }
        Ok(())
    })?;
    let hosts: BTreeSet<u64> = found.values().map(|face| face.0).collect();
    let mut lines: BTreeMap<u64, ([f64; 3], [f64; 3])> = BTreeMap::new();
    let options = Options {
        selected_ids: hosts,
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let token = graph
            .objects
            .iter()
            .find(|o| o.class_name == "VWallDriver")
            .and_then(|o| o.fields.pointer("/m_pCrv/pointer_token"))
            .and_then(Fv::as_u64);
        let named: Vec<_> = graph
            .objects
            .iter()
            .filter(|o| Some(u64::from(o.token)) == token)
            .collect();
        if let [line] = named.as_slice() {
            if line.class_name == "GLine" {
                if let (Some(origin), Some(direction)) = (
                    triple(line.fields.get("m_origin")),
                    triple(line.fields.get("m_dirVec")),
                ) {
                    lines.insert(record.identity.element_id, (origin, direction));
                }
            }
        }
        Ok(())
    })?;
    let (mut faces, mut agree, mut differ, mut face_only) = (0, 0, 0, 0);
    for (id, (host, low, high, saved)) in &found {
        let Some((origin, direction)) = lines.get(host) else {
            println!("face {id} {host} no_line");
            continue;
        };
        let at = |[u, v]: [f64; 2]| {
            [
                origin[0] + u * direction[0],
                origin[1] + u * direction[1],
                origin[2] + u * direction[2] + v,
            ]
        };
        let b = corner_box(at(*low), at(*high));
        faces += 1;
        let verdict = match saved {
            Some(saved) if saved.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-6) => {
                agree += 1;
                "agrees"
            }
            Some(_) => {
                differ += 1;
                "differs"
            }
            None => {
                face_only += 1;
                "none"
            }
        };
        println!(
            "face {id} {host} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} saved {verdict}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        );
    }
    Ok((faces, agree, differ, face_only))
}

fn main() -> Result<()> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        anyhow::bail!("usage: probe_re456_wall_cut_steps MODEL.rvt...");
    }
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("file {path} error {error:#}");
        }
    }
    Ok(())
}
