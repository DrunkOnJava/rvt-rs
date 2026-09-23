//! RE-38: element names from the partition name entries match Revit's.
//!
//! Revit's own IFC export names an element `Family:Type:ElementId`. rvt-rs
//! writes that name when the element's type and the type's family both
//! resolve from the partition name entries, and the element's class and id
//! otherwise. Every name it writes in that form must be exactly the name
//! Revit's export gives the same `Tag`, and its `ObjectType` exactly
//! Revit's `Family:Type`.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `2024_Core_Interior.rvt` with
//! `../IFC Exports/2024_Core_Interior_slim.ifc`, and the RE1 models with
//! their exports. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Split a STEP argument list on top-level commas, keeping quoted strings
/// and nested aggregates whole.
fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut depth) = (false, 0usize);
    for c in args.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                current.push(c);
            }
            '(' if !quoted => {
                depth += 1;
                current.push(c);
            }
            ')' if !quoted => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if !quoted && depth == 0 => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    out.push(current);
    out
}

/// Decode a STEP string literal's `\X\hh` and `\X2\hhhh…\X0\` escapes.
fn decode_step_string(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(tail) = rest.strip_prefix("\\X2\\") {
            let end = tail.find("\\X0\\").unwrap_or(tail.len());
            let units: Vec<u16> = tail.as_bytes()[..end]
                .chunks(4)
                .filter_map(|c| u16::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok())
                .collect();
            out.push_str(&String::from_utf16_lossy(&units));
            rest = tail.get(end + 4..).unwrap_or("");
        } else if let Some(tail) = rest.strip_prefix("\\X\\") {
            let byte = tail.get(..2).and_then(|h| u8::from_str_radix(h, 16).ok());
            out.push(byte.map_or('?', char::from));
            rest = tail.get(2..).unwrap_or("");
        } else {
            out.push('\\');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out.replace("''", "'")
}

/// `Tag` -> (`Name`, `ObjectType`) for every element entity (the 8th
/// attribute a numeric string), openings and owner history excluded.
fn names_by_tag(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let entity = entity.trim();
        if entity.ends_with("TYPE") || entity == "IFCOPENINGELEMENT" || entity == "IFCOWNERHISTORY"
        {
            continue;
        }
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        let fields = split_args(args);
        let (Some(name), Some(object_type), Some(tag)) =
            (fields.get(2), fields.get(4), fields.get(7))
        else {
            continue;
        };
        let Ok(tag) = tag.trim_matches('\'').parse() else {
            continue;
        };
        let unquote = |raw: &str| {
            decode_step_string(
                raw.strip_prefix('\'')
                    .and_then(|n| n.strip_suffix('\''))
                    .unwrap_or(raw),
            )
        };
        out.insert(tag, (unquote(name), unquote(object_type)));
    }
    out
}

/// How many names of the `Family:Type:ElementId` form rvt-rs wrote, after
/// checking each against the reference.
fn check(rvt: &Path, reference: &Path) -> usize {
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = names_by_tag(&write_step(&result.model));
    let theirs = names_by_tag(&std::fs::read_to_string(reference).expect("reference IFC"));
    let mut named = 0;
    for (tag, (name, object_type)) in &ours {
        if !(name.ends_with(&format!(":{tag}")) && name.matches(':').count() >= 2) {
            continue;
        }
        named += 1;
        assert_eq!(
            Some(object_type.as_str()),
            name.strip_suffix(&format!(":{tag}")),
            "ObjectType of {tag} in {}",
            rvt.display()
        );
        if let Some(reference) = theirs.get(tag) {
            assert_eq!(
                (name, object_type),
                (&reference.0, &reference.1),
                "name of {tag} in {}",
                rvt.display()
            );
        }
    }
    named
}

fn corpus() -> Option<PathBuf> {
    std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from)
}

#[test]
fn core_interior_names_are_revits() {
    let Some(dir) = corpus() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("2024_Core_Interior.rvt");
    let reference = dir.join("../IFC Exports/2024_Core_Interior_slim.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no Core Interior model and reference export");
        return;
    }
    // Doors, windows and columns: every family instance of the recovered
    // categories. Walls and slabs are system families, named elsewhere.
    assert_eq!(check(&rvt, &reference), 394);
}

#[test]
fn re1_names_are_revits() {
    let Some(dir) = corpus() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut checked = 0;
    // Mechanical's 6 "300x150" fittings and Electrical's 12 lighting
    // fixtures have types whose records also name a family nested in their
    // own (RE-42).
    for (model, expected) in [
        ("Architecture", 57),
        ("Mechanical", 42),
        ("Plumbing", 61),
        ("Electrical", 12),
    ] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference.exists() {
            continue;
        }
        assert_eq!(check(&rvt, &reference), expected, "RE1 {model}");
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipping: no RE1 model under RVT_PROJECT_CORPUS_DIR");
    }
}

/// #322: every element whose type is a system-family type (walls, floors,
/// ceilings, roofs, railings) carries the type name its type's own data
/// gives, and it equals the type half of Revit's `ObjectType`
/// (`Basic Wall:<type>`, `Floor:<type>`) for the same Tag. Returns how many
/// were checked per class.
fn check_system_type_names(rvt: &Path, reference: &Path, release: u32) -> Vec<(String, usize)> {
    let theirs = names_by_tag(&std::fs::read_to_string(reference).expect("reference IFC"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        release,
        rvt::walker::WalkerLimits::default(),
    )
    .expect("partition MVP");
    let field = |element: &rvt::walker::DecodedElement, wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            rvt::walker::InstanceField::String(v) if name == wanted => Some(v.clone()),
            _ => None,
        })
    };
    let mut checked = BTreeMap::new();
    for element in mvp
        .walls
        .iter()
        .chain(mvp.slabs.iter())
        .chain(mvp.products.iter())
    {
        if field(element, rvt::partition_schema_mvp::FAMILY_NAME_FIELD).is_some() {
            continue;
        }
        let Some(ours) = field(element, rvt::partition_schema_mvp::TYPE_NAME_FIELD) else {
            continue;
        };
        let id = element.id.expect("record id");
        let (_, object_type) = theirs.get(&u64::from(id)).expect("Revit exports it");
        let want = object_type
            .split_once(':')
            .map(|(_, t)| t)
            .expect("Family:Type");
        assert_eq!(ours, want, "{} {id}", element.class);
        *checked.entry(element.class.clone()).or_insert(0) += 1;
    }
    checked.into_iter().collect()
}

#[test]
fn core_interior_system_type_names_are_revits() {
    let Some(dir) = corpus() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("2024_Core_Interior.rvt");
    let reference = dir.join("../IFC Exports/2024_Core_Interior_slim.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no Core Interior model and reference export");
        return;
    }
    // All 360 walls, 99 floors (79 exported as IfcSlab, 20 as
    // IfcShadingDevice) and the building pad.
    assert_eq!(
        check_system_type_names(&rvt, &reference, 2024),
        [
            ("BuildingPad".to_string(), 1),
            ("Floor".to_string(), 99),
            ("Wall".to_string(), 360),
        ]
    );
}

/// The same on Revit 2025, whose element data opens with `0x02ef` where
/// 2024's opens with `0x02d3`. RE1's two floor types are named `-` and
/// `--`, so a floor given the other type's name fails.
#[test]
fn re1_system_type_names_are_revits() {
    let Some(dir) = corpus() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("RE1-Architecture.rvt");
    let reference = dir.join("RE1-Architecture.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no RE1 Architecture model and reference export");
        return;
    }
    // Seven basic walls and the curtain wall (RE-46), whose type is also `-`.
    assert_eq!(
        check_system_type_names(&rvt, &reference, 2025),
        [
            ("Ceiling".to_string(), 6),
            ("CurtainWall".to_string(), 1),
            ("Floor".to_string(), 2),
            ("Wall".to_string(), 7),
        ]
    );
}

#[test]
fn step_string_escapes_decode() {
    assert_eq!(decode_step_string("M_Folha \\X\\FAnica"), "M_Folha única");
    assert_eq!(
        decode_step_string("M_Folha \\X2\\00FA\\X0\\nica"),
        "M_Folha única"
    );
    assert_eq!(decode_step_string("36'' x 84''"), "36' x 84'");
}
