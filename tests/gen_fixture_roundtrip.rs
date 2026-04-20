//! Round-trip test for the `gen-fixture` binary.
//!
//! Shells out to `cargo run --bin gen-fixture` with deterministic
//! flags, then feeds the output back into the library's reader,
//! schema parser, and walker to verify the synthetic bytes survive
//! the full decode pipeline.

use rvt::RevitFile;
use std::path::PathBuf;
use std::process::Command;

/// Path to the workspace root. Resolved via `CARGO_MANIFEST_DIR`,
/// which Cargo sets for every `cargo test` invocation.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Unique scratch path per test, placed inside `target/` so `cargo
/// clean` sweeps them and `tests/fixtures/` stays tidy.
fn scratch_path(tag: &str) -> PathBuf {
    let mut p = workspace_root();
    p.push("target");
    p.push("gen-fixture-tests");
    std::fs::create_dir_all(&p).expect("create scratch dir");
    p.push(format!("synthetic-{tag}.rvt"));
    p
}

fn run_gen_fixture(name: &str, extra_args: &[&str]) -> PathBuf {
    let out = scratch_path(name);
    // Always rebuild against the current source so tests stay in sync
    // with the binary; `cargo run` handles that.
    let status = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--bin", "gen-fixture", "--"])
        .arg(name)
        .arg("--output")
        .arg(&out)
        .args(extra_args)
        .current_dir(workspace_root())
        .status()
        .expect("cargo run gen-fixture");
    assert!(status.success(), "gen-fixture exited non-zero: {status:?}");
    assert!(out.exists(), "gen-fixture didn't create {}", out.display());
    out
}

#[test]
fn minimal_fixture_opens_and_parses() {
    let path = run_gen_fixture("roundtrip-minimal", &[]);
    let mut rf = RevitFile::open(&path).expect("open synthetic fixture");
    let missing = rf.missing_required_streams();
    assert!(
        missing.is_empty(),
        "synthetic fixture is missing required streams: {missing:?}"
    );
    assert!(rf.has_revit_signature(), "Revit signature check failed");

    let bfi = rf.basic_file_info().expect("parse BasicFileInfo");
    assert_eq!(bfi.version, 2024);

    let schema = rf.schema().expect("parse schema");
    let names: Vec<&str> = schema.classes.iter().map(|c| c.name.as_str()).collect();
    for expected in ["Wall", "Level", "Project"] {
        assert!(
            names.contains(&expected),
            "expected class {expected} missing from parsed schema; got {names:?}"
        );
    }
}

#[test]
fn walker_read_field_by_type_consumes_synthetic_instance() {
    // Use a seeded fixture so the synthetic payloads are stable.
    let path = run_gen_fixture(
        "roundtrip-walker",
        &[
            "--seed",
            "42",
            "--classes",
            "Wall,Project",
            "--element-count",
            "4",
        ],
    );
    let mut rf = RevitFile::open(&path).expect("open walker fixture");
    let schema = rf.schema().expect("parse schema");

    // Find the Wall class and decode its first synthesized instance
    // via the generic field walker.
    let wall = schema
        .classes
        .iter()
        .find(|c| c.name == "Wall")
        .expect("Wall class present in synthetic schema");
    assert!(
        !wall.fields.is_empty(),
        "Wall class in synthetic schema has no fields: {wall:?}"
    );

    // Pull decompressed Global/Latest (skip the 8-byte custom prefix,
    // same way RevitFile::read_adocument does).
    let raw = rf.read_stream(rvt::streams::GLOBAL_LATEST).unwrap();
    let decomp = rvt::compression::inflate_at(&raw, 8).expect("inflate global/latest");
    assert!(!decomp.is_empty(), "decompressed Global/Latest is empty");

    // Step past the lead-in padding + the first element's 8-byte
    // record header to point at the first payload field.
    // Lead-in = 0x20 zero bytes (see gen_fixture::build_global_latest).
    let payload_start = 0x20 + 8;
    let mut cursor = payload_start;
    let mut decoded_any = false;
    for f in &wall.fields {
        if let Some(ft) = &f.field_type {
            let before = cursor;
            let value = rvt::walker::read_field_by_type(&decomp, &mut cursor, ft);
            assert!(
                cursor >= before,
                "walker cursor moved backwards on field {}",
                f.name
            );
            // Make sure at least one field decoded to a non-Bytes
            // variant — if every field falls through to raw Bytes
            // we've probably misaligned the payload.
            if !matches!(value, rvt::walker::InstanceField::Bytes(_)) {
                decoded_any = true;
            }
        }
    }
    assert!(
        decoded_any,
        "walker fell through to Bytes for every field — payload likely misaligned"
    );
}

#[test]
fn fixture_output_is_deterministic() {
    // Same seed + flags → byte-identical output across runs.
    let a = run_gen_fixture("roundtrip-det-a", &["--seed", "7"]);
    let b = run_gen_fixture("roundtrip-det-b", &["--seed", "7"]);
    let a_bytes = std::fs::read(&a).unwrap();
    let b_bytes = std::fs::read(&b).unwrap();
    assert_eq!(
        a_bytes.len(),
        b_bytes.len(),
        "same-seed fixtures have different sizes: {} vs {}",
        a_bytes.len(),
        b_bytes.len()
    );
    // Compare decompressed Formats/Latest rather than the CFB raw
    // bytes — CFB sector allocation isn't deterministic across runs,
    // but the stream contents should be.
    let mut a_rf = RevitFile::open(&a).unwrap();
    let mut b_rf = RevitFile::open(&b).unwrap();
    let a_raw = a_rf.read_stream(rvt::streams::FORMATS_LATEST).unwrap();
    let b_raw = b_rf.read_stream(rvt::streams::FORMATS_LATEST).unwrap();
    let a_decomp = rvt::compression::inflate_at(&a_raw, 0).unwrap();
    let b_decomp = rvt::compression::inflate_at(&b_raw, 0).unwrap();
    assert_eq!(
        a_decomp, b_decomp,
        "same-seed fixtures produce different Formats/Latest payloads"
    );
}
