//! Cross-witness gate: IFClite (`ifc-lite-core`, MPL-2.0, Rust) parses a
//! Revit-authored reference IFC and must agree with the project-count manifest
//! that the rvt-rs decoder and IfcOpenShell are also gated against
//! (docs/verification-protocol.md).
//!
//! Usage: witness-ifc-lite <manifest.json> <corpus_dir> [--json OUT]
//!                         [--observation OUT]
//!
//! `--observation` additionally writes an OctetProof observation
//! (docs/octetproof-spec-draft.md §6.2): entity counts for every manifest
//! `source_ifc_type`, canonicalized (sorted keys, no whitespace, UTF-8) and
//! hashed so a replay can prove the witness saw the same thing.
//!
//! The manifest's `reference_ifc_file` is resolved under <corpus_dir>, its
//! SHA-256 is checked against `source.reference_ifc_sha256` (a golden artifact
//! must be the exact bytes the registry names), and every category carrying a
//! `source_ifc_type` is counted with `ifc_lite_core::EntityScanner` (exact
//! STEP keyword, no subtypes — the same semantics as IfcOpenShell's
//! `by_type(..., include_subtypes=False)` and the manifest's STEP-constructor
//! counts) and compared to `expected` within `tolerance`. Exit 1 on any drift
//! or hash mismatch.
//!
//! This is the third implementation lineage on the RVT → IFC edge: rvt-rs
//! reads the .rvt (source witness), IfcOpenShell and IFClite each read Revit's
//! .ifc (bridge witnesses) with no shared code.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ifc_lite_core::EntityScanner;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

/// Registry id of this witness. Must match the `id` of the `ifc-lite` entry in
/// research/witness-registry.json — witness-verdict.py resolves the lineage and
/// license from there by this key.
const WITNESS_ID: &str = "ifc-lite";

/// Exact pinned version of the reader (OctetProof §9.6). Kept in lockstep with
/// the `ifc-lite-core = "=X.Y.Z"` pin in Cargo.toml and the registry entry;
/// tests/witness_registry.rs fails the build if the three ever drift.
const WITNESS_VERSION: &str = "7.1.1";

fn sha256_of(path: &Path) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut reader = BufReader::new(fs::File::open(path)?);
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Canonical JSON per §7.3 / §8.4: sorted keys, no insignificant whitespace,
/// UTF-8. `serde_json` without the `preserve_order` feature stores objects in a
/// `BTreeMap`, so `to_string` already emits sorted keys with no whitespace —
/// byte-identical to Python's `json.dumps(..., sort_keys=True,
/// separators=(",", ":"))` for the integer/string payloads emitted here.
fn canonical_hash(value: &Value) -> String {
    let canonical = serde_json::to_string(value).expect("serialize canonical payload");
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

/// Value of `FILE_SCHEMA` from the STEP header, e.g. `IFC4`. IfcOpenShell
/// reports the same string as `model.schema`, so the two bridge witnesses'
/// `ifc_schema` fields are directly comparable.
fn file_schema(bytes: &[u8]) -> Option<String> {
    let head = &bytes[..bytes.len().min(64 * 1024)];
    let text = String::from_utf8_lossy(head).to_uppercase();
    let start = text.find("FILE_SCHEMA")? + "FILE_SCHEMA".len();
    let rest = &text[start..];
    let open = rest.find('\'')?;
    let tail = &rest[open + 1..];
    let close = tail.find('\'')?;
    Some(tail[..close].trim().to_string())
}

/// Exact-keyword instance counts for the whole data section, upper-cased so the
/// lookup matches the manifest's `source_ifc_type` spelling regardless of how
/// the exporter cased the keyword.
fn count_by_exact_type(bytes: &[u8]) -> BTreeMap<String, usize> {
    let mut scanner = EntityScanner::new(bytes);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (type_name, n) in scanner.count_by_type() {
        *counts.entry(type_name.trim().to_uppercase()).or_insert(0) += n;
    }
    counts
}

struct Args {
    manifest: PathBuf,
    corpus_dir: PathBuf,
    json: Option<PathBuf>,
    observation: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut positional: Vec<PathBuf> = Vec::new();
    let mut json = None;
    let mut observation = None;
    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--json" => {
                json = Some(PathBuf::from(
                    argv.next().ok_or("--json needs a path".to_string())?,
                ))
            }
            "--observation" => {
                observation = Some(PathBuf::from(
                    argv.next()
                        .ok_or("--observation needs a path".to_string())?,
                ))
            }
            "-h" | "--help" => {
                println!(
                    "usage: witness-ifc-lite <manifest.json> <corpus_dir> \
                     [--json OUT] [--observation OUT]"
                );
                std::process::exit(0);
            }
            other if other.starts_with('-') => return Err(format!("unknown flag {other}")),
            other => positional.push(PathBuf::from(other)),
        }
    }
    if positional.len() != 2 {
        return Err("usage: witness-ifc-lite <manifest.json> <corpus_dir>".to_string());
    }
    let corpus_dir = positional.pop().expect("two positionals");
    let manifest = positional.pop().expect("two positionals");
    Ok(Args {
        manifest,
        corpus_dir,
        json,
        observation,
    })
}

fn run() -> Result<i32, String> {
    let args = parse_args()?;
    let manifest_text =
        fs::read_to_string(&args.manifest).map_err(|e| format!("read manifest: {e}"))?;
    let manifest: Value =
        serde_json::from_str(&manifest_text).map_err(|e| format!("parse manifest: {e}"))?;

    let reference_name = match manifest.get("reference_ifc_file").and_then(Value::as_str) {
        Some(name) => name,
        None => {
            eprintln!(
                "{}: no reference_ifc_file — nothing to witness",
                args.manifest.display()
            );
            return Ok(0);
        }
    };
    let reference = args.corpus_dir.join(reference_name);
    if !reference.is_file() {
        return Err(format!("reference IFC missing at {}", reference.display()));
    }

    let actual_sha = sha256_of(&reference).map_err(|e| format!("hash reference IFC: {e}"))?;
    let expected_sha = manifest
        .get("source")
        .and_then(|s| s.get("reference_ifc_sha256"))
        .and_then(Value::as_str);
    if let Some(expected) = expected_sha {
        if expected != actual_sha {
            return Err(format!(
                "{}: sha256 {actual_sha} != manifest {expected}",
                reference.display()
            ));
        }
    }

    let bytes = fs::read(&reference).map_err(|e| format!("read reference IFC: {e}"))?;
    let schema = file_schema(&bytes).unwrap_or_else(|| "UNKNOWN".to_string());
    let counts = count_by_exact_type(&bytes);

    let empty = Map::new();
    let categories = manifest
        .get("counts")
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    let artifact_id = manifest.get("id").and_then(Value::as_str).unwrap_or("");
    println!(
        "{artifact_id}: {reference_name} ({schema}, sha256 {}…)",
        &actual_sha[..12]
    );
    println!(
        "{:<16} {:<22} {:>8} {:>8} {:>4}  result",
        "category", "ifc type", "expected", "ifc-lite", "tol"
    );

    let mut drift = 0usize;
    let mut records = Vec::new();
    let mut entity_counts = Map::new();
    for (category, spec) in categories {
        let ifc_type = match spec.get("source_ifc_type").and_then(Value::as_str) {
            Some(t) => t,
            None => continue,
        };
        let expected = spec.get("expected").and_then(Value::as_i64).unwrap_or(0);
        let tolerance = spec.get("tolerance").and_then(Value::as_i64).unwrap_or(0);
        let actual = counts.get(&ifc_type.to_uppercase()).copied().unwrap_or(0) as i64;
        let ok = (actual - expected).abs() <= tolerance;
        if !ok {
            drift += 1;
        }
        records.push(json!({
            "category": category,
            "ifc_type": ifc_type,
            "expected": expected,
            "tolerance": tolerance,
            "ifc_lite": actual,
            "agree": ok,
        }));
        entity_counts.insert(ifc_type.to_string(), json!(actual));
        println!(
            "{category:<16} {ifc_type:<22} {expected:>8} {actual:>8} {tolerance:>4}  {}",
            if ok { "ok" } else { "DRIFT" }
        );
    }

    if let Some(path) = args.json.as_ref() {
        let record = json!({
            "schema_version": 1,
            "manifest": manifest.get("id").cloned().unwrap_or(Value::Null),
            "reference_ifc": reference_name,
            "reference_ifc_sha256": actual_sha,
            "ifc_schema": schema,
            "witness": format!("{WITNESS_ID} {WITNESS_VERSION}"),
            "categories": records,
            "agree": drift == 0,
        });
        write_json(path, &record)?;
    }

    if let Some(path) = args.observation.as_ref() {
        let payload = json!({
            "entity_counts": Value::Object(entity_counts),
            "ifc_schema": schema,
        });
        let observation = json!({
            "schema_version": "1.0.0",
            "witness_id": WITNESS_ID,
            "witness_version": WITNESS_VERSION,
            "artifact_id": manifest.get("id").cloned().unwrap_or(Value::Null),
            "input_role": "bridge",
            "input_file": reference_name,
            "input_hash_sha256": actual_sha,
            "deterministic": true,
            "semantic_surface_covered": ["entity_counts"],
            "observation": payload,
            "observation_hash_sha256": canonical_hash(&payload),
            "unsupported_entities": [],
            "warnings": [],
        });
        write_json(path, &observation)?;
    }

    if drift > 0 {
        eprintln!(
            "error: {drift} categor{} drifted from the manifest",
            if drift == 1 { "y" } else { "ies" }
        );
        return Ok(1);
    }
    println!("cross-witness: IFClite agrees with the manifest for every source_ifc_type");
    Ok(0)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(|e| format!("serialize: {e}"))?;
    text.push('\n');
    fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))
}

fn main() -> ExitCode {
    match run() {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
