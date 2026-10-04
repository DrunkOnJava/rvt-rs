//! B74: an export without geometry types its elements as one with geometry.
//!
//! `typed-no-geometry` drops bodies, placements and host links, not what an
//! element is: its `ObjectType` (`Family:Type`) and its type object, of the
//! same entity, GlobalId and name, are the same as in the default export. For
//! every element both exports write (matched by GlobalId) that the default
//! export relates to a type, the export without geometry must relate it to
//! the same type and give it the same `ObjectType`.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: the four `RE1-*.rvt` models and
//! `2024_Core_Interior.rvt`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{ExportQualityMode, RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// An entity's top-level arguments, each as written: a quoted string keeps
/// its quotes, a list its parentheses.
fn arguments(args: &str) -> Vec<String> {
    let body = args.trim_end().trim_end_matches(';');
    let body = body.strip_suffix(')').unwrap_or(body);
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut depth, mut in_string) = (0usize, false);
    for c in body.chars() {
        match c {
            '\'' => in_string = !in_string,
            '(' if !in_string => depth += 1,
            ')' if !in_string => depth = depth.saturating_sub(1),
            ',' if !in_string && depth == 0 => {
                out.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    out.push(current);
    out
}

/// `#id` -> (entity name, top-level arguments) for every entity line.
fn entities(step: &str) -> BTreeMap<u64, (String, Vec<String>)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let Ok(id) = id.trim().parse() else {
            continue;
        };
        out.insert(id, (entity.trim().to_string(), arguments(args)));
    }
    out
}

/// The `#n` references in an argument.
fn refs(arg: &str) -> Vec<u64> {
    arg.split('#')
        .skip(1)
        .filter_map(|s| {
            let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// What an element is: its `ObjectType`, and its type's entity, GlobalId
/// and name, where it has a type.
#[derive(Debug, PartialEq, Eq)]
struct Typing {
    object_type: String,
    of_type: Option<(String, String, String)>,
}

/// Each object's GlobalId -> its typing, for every rooted object with an
/// `ObjectType` slot.
fn typings(step: &str) -> BTreeMap<String, Typing> {
    let ents = entities(step);
    let mut type_of: BTreeMap<u64, (String, String, String)> = BTreeMap::new();
    for (entity, args) in ents.values() {
        // IfcRelDefinesByType(GlobalId, OwnerHistory, Name, Description,
        // RelatedObjects, RelatingType).
        if entity != "IFCRELDEFINESBYTYPE" || args.len() < 6 {
            continue;
        }
        let Some(ty) = refs(&args[5]).first().copied() else {
            continue;
        };
        let Some((ty_entity, ty_args)) = ents.get(&ty) else {
            continue;
        };
        let (Some(global_id), Some(name)) = (ty_args.first(), ty_args.get(2)) else {
            continue;
        };
        for element in refs(&args[4]) {
            type_of.insert(
                element,
                (ty_entity.clone(), global_id.clone(), name.clone()),
            );
        }
    }
    let mut out = BTreeMap::new();
    for (id, (entity, args)) in &ents {
        if entity.starts_with("IFCREL") {
            continue;
        }
        // GlobalId, OwnerHistory, Name, Description, ObjectType, ...
        let (Some(global_id), Some(object_type)) = (args.first(), args.get(4)) else {
            continue;
        };
        if global_id.len() != 24 || !global_id.starts_with('\'') {
            continue;
        }
        out.insert(
            global_id.clone(),
            Typing {
                object_type: object_type.clone(),
                of_type: type_of.get(id).cloned(),
            },
        );
    }
    out
}

fn check(rvt: &Path, failures: &mut Vec<String>) -> usize {
    let export = |mode| {
        let mut rf = RevitFile::open(rvt).expect("open");
        let model = RvtDocExporter
            .export_with_mode_and_limits(&mut rf, mode, Default::default())
            .expect("export");
        typings(&write_step(&model))
    };
    let with = export(ExportQualityMode::Scaffold);
    let without = export(ExportQualityMode::TypedNoGeometry);
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    let mut wrong = Vec::new();
    let mut compared = 0;
    for (global_id, typing) in &with {
        if typing.of_type.is_none() {
            continue;
        }
        let Some(theirs) = without.get(global_id) else {
            continue;
        };
        compared += 1;
        if theirs != typing {
            wrong.push(format!(
                "{global_id}: with geometry {typing:?}, without {theirs:?}"
            ));
        }
    }
    eprintln!(
        "{name}: {compared} typed elements in both exports, {} typed otherwise without geometry",
        wrong.len()
    );
    if compared == 0 {
        failures.push(format!(
            "{name}: no element the default export types is in the export without geometry"
        ));
    }
    if !wrong.is_empty() {
        failures.push(format!(
            "{name}: {} of {compared} typed elements are typed otherwise without geometry; first: {:?}",
            wrong.len(),
            wrong.iter().take(3).collect::<Vec<_>>()
        ));
    }
    compared
}

#[test]
fn an_export_without_geometry_types_elements_as_one_with_geometry() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut models: Vec<PathBuf> = ["Architecture", "Mechanical", "Plumbing", "Electrical"]
        .iter()
        .map(|model| dir.join(format!("RE1-{model}.rvt")))
        .collect();
    models.push(dir.join("2024_Core_Interior.rvt"));
    let mut failures = Vec::new();
    let mut compared = 0;
    for rvt in &models {
        if !rvt.exists() {
            eprintln!("skipping {}: absent", rvt.display());
            continue;
        }
        compared += check(rvt, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        compared > 0 || !dir.join("RE1-Architecture.rvt").exists(),
        "no typed element was compared"
    );
}
