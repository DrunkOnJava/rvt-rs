//! RE-46: a wall a curtain-wall mullion names is a curtain wall, exported
//! as a bodiless `IfcCurtainWall` that aggregates its panels and mullions.
//!
//! On `RE1-Architecture.rvt` (Revit 2025, MIT) Revit's export writes wall
//! 445961 as `IFCCURTAINWALL` with no representation and aggregates its 2
//! panels and 10 mullions (and a panel door, which is not read). Corpus-gated
//! through `RVT_PROJECT_CORPUS_DIR`.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Top-level STEP arguments, keeping quoted strings and aggregates whole.
fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quoted = false;
    for ch in args.chars() {
        match ch {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                out.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }
    out.push(current);
    out
}

#[test]
fn re1_curtain_wall_aggregates_its_panels_and_mullions() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let path = dir.join("RE1-Architecture.rvt");
    if !path.exists() {
        eprintln!("skipping: no RE1-Architecture.rvt");
        return;
    }
    let mut rf = RevitFile::open(&path).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let step = write_step(&result.model);

    // #id -> (entity, arguments)
    let mut entities: BTreeMap<u64, (String, Vec<String>)> = BTreeMap::new();
    for line in step.lines() {
        let Some((id, body)) = line.split_once('=') else {
            continue;
        };
        let Some(id) = id.trim().strip_prefix('#').and_then(|id| id.parse().ok()) else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        entities.insert(id, (entity.trim().to_string(), split_args(args)));
    }
    let tag = |args: &[String]| args.get(7).map(|t| t.trim_matches('\'').to_string());

    let curtain_walls: Vec<(u64, &Vec<String>)> = entities
        .iter()
        .filter(|(_, (entity, _))| entity == "IFCCURTAINWALL")
        .map(|(id, (_, args))| (*id, args))
        .collect();
    assert_eq!(curtain_walls.len(), 1);
    let (whole, args) = curtain_walls[0];
    assert_eq!(tag(args).as_deref(), Some("445961"));
    // Representation is `$`: the parts carry the geometry, as in Revit's export.
    assert_eq!(args[6], "$");
    assert_eq!(args[8], ".NOTDEFINED.");
    assert!(
        entities
            .values()
            .filter(|(entity, _)| entity == "IFCWALL")
            .all(|(_, args)| tag(args).as_deref() != Some("445961"))
    );

    let mut parts: BTreeMap<String, usize> = BTreeMap::new();
    for (entity, args) in entities.values() {
        if entity != "IFCRELAGGREGATES" || args[4] != format!("#{whole}") {
            continue;
        }
        for part in args[5].trim_matches(|c| c == '(' || c == ')').split(',') {
            let id: u64 = part.trim_start_matches('#').parse().expect("part id");
            *parts.entry(entities[&id].0.clone()).or_default() += 1;
        }
    }
    assert_eq!(
        parts,
        BTreeMap::from([("IFCMEMBER".to_string(), 10), ("IFCPLATE".to_string(), 2)])
    );
}
