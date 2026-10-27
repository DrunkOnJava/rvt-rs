//! `rvt-gltf --progress`: the conversion's progress on stderr, in the
//! `progress: <stage> <done>/<total>` lines of `rvt-elements --progress`,
//! and a .glb byte-identical to the one written without the flag. The tier-1
//! synthetic fixture proves the walker's stages everywhere; RE1-Architecture
//! (skipped unless `RVT_PROJECT_CORPUS_DIR` holds it) has saved graphics, so
//! it also proves the saved-graphics stages and that a cancelled token stops
//! the export and the saved-graphics read.

use std::path::{Path, PathBuf};

use rvt::control::{CancelToken, Stage, WalkerControl};

fn tier1() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("tier1")
        .join("architectural-2024")
        .join("architectural-2024.rvt");
    path.exists().then_some(path)
}

fn re1_architecture() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR")?;
    let path = PathBuf::from(dir).join("RE1-Architecture.rvt");
    path.exists().then_some(path)
}

/// The .glb and stderr of `rvt-gltf <input> --saved-meshes <extra>`.
fn gltf(input: &Path, tag: &str, extra: &[&str]) -> (Vec<u8>, String) {
    let dir = std::env::temp_dir().join(format!("rvt-gltf-progress-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let output = dir.join(format!("{tag}.glb"));
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_rvt-gltf"))
        .arg(input)
        .args(["--saved-meshes", "-o"])
        .arg(&output)
        .args(extra)
        .output()
        .expect("spawn rvt-gltf");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(out.status.success(), "{stderr}");
    let glb = std::fs::read(&output).expect("glb written");
    let _ = std::fs::remove_file(&output);
    (glb, stderr)
}

/// `(done, total)` of each line of `stage`, in order.
fn stage_lines(stderr: &str, stage: &str) -> Vec<(u64, Option<u64>)> {
    stderr
        .lines()
        .filter_map(|line| {
            line.strip_prefix("progress: ")?
                .strip_prefix(stage)?
                .strip_prefix(' ')
        })
        .map(|count| match count.split_once('/') {
            Some((done, total)) => (done.parse().unwrap(), Some(total.parse().unwrap())),
            None => (count.parse().unwrap(), None),
        })
        .collect()
}

#[test]
fn progress_goes_to_stderr_and_leaves_the_glb_unchanged() {
    let Some(input) = tier1() else {
        eprintln!("skipping: tier-1 fixture missing");
        return;
    };
    let (plain, quiet) = gltf(&input, "tier1-plain", &[]);
    let (glb, stderr) = gltf(&input, "tier1-progress", &["--progress"]);
    assert!(plain == glb, "--progress changed the .glb");
    assert!(quiet.is_empty(), "stderr without --progress: {quiet}");
    assert!(
        stderr.lines().all(|l| l.starts_with("progress: ")),
        "{stderr}"
    );
    assert_eq!(stderr.lines().next(), Some("progress: schema-parse 0"));
    assert_eq!(
        stage_lines(&stderr, "partition-scan").last(),
        Some(&(1, Some(1)))
    );
}

#[test]
fn saved_meshes_progress_counts_every_element_once() {
    let Some(input) = re1_architecture() else {
        eprintln!("skipping: RE1-Architecture.rvt absent");
        return;
    };
    let (plain, _) = gltf(&input, "re1-plain", &[]);
    let (glb, stderr) = gltf(&input, "re1-progress", &["--progress"]);
    assert!(plain == glb, "--progress changed the .glb");
    assert_eq!(
        stage_lines(&stderr, "saved-graphics"),
        [(0, None), (1, Some(1))]
    );
    let meshes = stage_lines(&stderr, "saved-meshes");
    let total = meshes.first().and_then(|(_, t)| *t).expect("saved-meshes");
    assert!(total > 0, "{stderr}");
    let expected: Vec<_> = (0..=total).map(|done| (done, Some(total))).collect();
    assert_eq!(meshes, expected, "one line per element, in order");
}

#[test]
fn a_cancelled_token_stops_the_export_and_the_saved_graphics_read() {
    let Some(input) = re1_architecture() else {
        eprintln!("skipping: RE1-Architecture.rvt absent");
        return;
    };
    let token = CancelToken::new();
    token.cancel();
    let cancelled = WalkerControl::new().with_cancel(token);
    let export = rvt::ifc::RvtDocExporter.export_with_mode_limits_and_control(
        &mut rvt::RevitFile::open(&input).unwrap(),
        rvt::ifc::ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
        &cancelled,
    );
    assert!(matches!(export, Err(rvt::Error::Cancelled)));

    // Cancelled once meshing starts: the read stops instead of finishing.
    let mut rf = rvt::RevitFile::open(&input).unwrap();
    let mut model = rvt::ifc::RvtDocExporter
        .export_with_mode_limits_and_control(
            &mut rf,
            rvt::ifc::ExportQualityMode::Scaffold,
            rvt::walker::WalkerLimits::default(),
            &WalkerControl::new(),
        )
        .unwrap();
    let token = CancelToken::new();
    let trip = token.clone();
    let control = WalkerControl::new()
        .with_cancel(token)
        .with_progress(move |event| {
            if event.stage == Stage::SavedMeshes {
                trip.cancel();
            }
        });
    let error = rvt::ifc::saved_meshes::attach_with_control(&mut rf, &mut model, &control)
        .expect_err("cancelled read fails");
    assert!(
        matches!(error.downcast_ref(), Some(rvt::Error::Cancelled)),
        "{error}"
    );
}
