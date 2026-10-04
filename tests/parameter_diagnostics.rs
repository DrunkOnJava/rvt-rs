//! B49 (#35): the export diagnostics count the Revit parameters the export
//! writes.
//!
//! Revit's exporter fills the IFC common property sets (`Pset_*`) from an
//! element's parameters, and rvt-rs writes the values it reads into the same
//! sets. The diagnostics `rvt-ifc` writes beside an export must agree with that
//! file: `decoded.parameter_value_count` is the number of property values in
//! its `Pset_` property sets, and `revit_element_parameters_to_ifc_property_sets`
//! (no Revit parameter reaches a property set) is claimed exactly when that
//! number is 0.
//!
//! Runs the built `rvt-ifc` against `RVT_PROJECT_CORPUS_DIR`: Core Interior,
//! Einhoven and the four RE1 models. Skips what is absent.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

const NO_PARAMETERS: &str = "revit_element_parameters_to_ifc_property_sets";

/// `#id -> (entity, args)` for every line of a STEP file.
fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
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
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

/// Split a STEP argument list on top-level commas.
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

/// The property values of every `IFCPROPERTYSET` whose name starts `Pset_`:
/// the length of its `HasProperties` list, summed over the sets.
fn pset_property_values(step: &str) -> usize {
    entities(step)
        .values()
        .filter(|(entity, _)| entity == "IFCPROPERTYSET")
        .map(|(_, args)| split_args(args))
        .filter(|f| f.get(2).is_some_and(|n| n.trim().starts_with("'Pset_")))
        .map(|f| {
            let list = f.get(4).map(|l| l.trim()).unwrap_or("()");
            let inner = list.trim_start_matches('(').trim_end_matches(')');
            split_args(inner)
                .iter()
                .filter(|p| p.trim().starts_with('#'))
                .count()
        })
        .sum()
}

#[test]
fn diagnostics_count_the_revit_parameters_the_export_writes() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let models = [
        "2024_Core_Interior.rvt",
        "Revit_IFC5_Einhoven.rvt",
        "RE1-Architecture.rvt",
        "RE1-Mechanical.rvt",
        "RE1-Plumbing.rvt",
        "RE1-Electrical.rvt",
    ];
    let out_dir = std::env::temp_dir().join(format!(
        "rvt-rs-parameter-diagnostics-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&out_dir).expect("create output dir");
    let mut with_parameters = 0;
    let mut failures = Vec::new();
    for model in models {
        let rvt = dir.join(model);
        if !rvt.exists() {
            eprintln!("skipping {model}: absent");
            continue;
        }
        let ifc = out_dir.join(format!("{model}.ifc"));
        let diagnostics = out_dir.join(format!("{model}.diagnostics.json"));
        let output = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
            .arg(&rvt)
            .arg("-o")
            .arg(&ifc)
            .arg("--diagnostics")
            .arg(&diagnostics)
            .output()
            .expect("run rvt-ifc");
        assert!(
            output.status.success(),
            "{model}: rvt-ifc failed\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let written = pset_property_values(&std::fs::read_to_string(&ifc).expect("read IFC"));
        let json: Value =
            serde_json::from_slice(&std::fs::read(&diagnostics).expect("read diagnostics"))
                .expect("parse diagnostics");
        let counted = json["decoded"]["parameter_value_count"].as_u64();
        let claimed = json["unsupported_features"]
            .as_array()
            .is_some_and(|features| features.iter().any(|f| f == NO_PARAMETERS));
        eprintln!(
            "{model}: {written} property values in Pset_ sets; parameter_value_count {counted:?}; claims {NO_PARAMETERS}: {claimed}"
        );
        if counted != Some(written as u64) {
            failures.push(format!(
                "{model}: parameter_value_count is {counted:?}, the file's Pset_ sets hold {written} values"
            ));
        }
        if claimed != (written == 0) {
            failures.push(format!(
                "{model}: {NO_PARAMETERS} claimed {claimed} with {written} Pset_ values written"
            ));
        }
        if written > 0 {
            with_parameters += 1;
        }
    }
    let _ = std::fs::remove_dir_all(&out_dir);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        with_parameters > 0 || !dir.join("2024_Core_Interior.rvt").exists(),
        "no model exported a Pset_ value, so nothing was compared"
    );
}
