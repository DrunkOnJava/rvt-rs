//! #1429: which allocations hold `rvt-ifc`'s heap at its peak, by call site, on
//! the BIM4LCA office, for the default export.
//!
//! FACT under test: the 76 MiB (917 to 993 MiB as built) that the export's peak
//! resident memory rose by when #1401 (RE-340) drew the walls that embed a
//! curtain wall from their saved meshes is held by one call site, or a few
//! next to each other. dhat records the call stack of every allocation, so after
//! a run it can say how many bytes each stack held when the heap was at its
//! highest (`t-gmax`) and when the run ended (`t-end`). Run it at two refs and
//! compare the stacks at the peak.
//!
//! Two stages, each in a process of its own (dhat profiles one run per
//! process; the probe starts itself twice):
//!
//! - `export`: from before the file is opened to the end of the STEP text
//!   written to a sink, as `rvt-ifc` makes it with no flag: `ExportOptions::default()`,
//!   the handle dropped before the text is written. `peak_mib` is the heap's
//!   highest, `after_export_peak_mib` the same when the model was built.
//! - `meshes`: the saved-graphics pass alone (`saved_meshes::attach_scoped_with_control`
//!   for the default scope) on a model made with the saved meshes off, so
//!   `at_peak` is what that pass holds at its own highest.
//!
//! dhat slows a run by an order of magnitude; a stage is stopped after 9
//! minutes (Measure cuts a probe at 20). Each profile prints the largest call
//! sites by the innermost `rvt::` frame (`by function`), by the innermost
//! frames of the stack (`by stack`) and by the outermost `rvt::` frames (`by
//! pass`), in MiB and blocks. Frames are the symbols of the `ci` profile build,
//! so a function inlined into its caller shows as the caller.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1429_export_peak -- MODEL.rvt

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use rvt::RevitFile;
use rvt::ifc::step_writer::{StepOptions, write_step_to};
use rvt::ifc::{ExportOptions, SavedMeshChoice, run_export_with_limits};
use rvt::walker::WalkerLimits;

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

const STAGE_LIMIT: Duration = Duration::from_secs(540);
const FRAMES: usize = 30;
const TOP: usize = 30;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut path, mut stage) = (None, None);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--stage" {
            stage = rest.next().cloned();
        } else if !arg.starts_with("--") && path.is_none() {
            path = Some(arg.clone());
        }
    }
    let path = path.ok_or_else(|| anyhow::anyhow!("usage: probe_1429_export_peak MODEL.rvt"))?;
    match stage.as_deref() {
        None => parent(&path),
        Some("export") => export_stage(&path),
        Some("meshes") => meshes_stage(&path),
        Some(other) => anyhow::bail!("unknown stage {other}"),
    }
}

fn parent(path: &str) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    for stage in ["export", "meshes"] {
        println!("=== stage {stage}");
        let mut child = Command::new(&exe)
            .arg(path)
            .arg("--stage")
            .arg(stage)
            .spawn()?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                println!(
                    "=== stage {stage} {status} after {} s",
                    started.elapsed().as_secs()
                );
                break;
            }
            if started.elapsed() > STAGE_LIMIT {
                child.kill()?;
                let _ = child.wait();
                println!(
                    "=== stage {stage} stopped at the {} s limit",
                    STAGE_LIMIT.as_secs()
                );
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}

fn dump_path(stage: &str) -> PathBuf {
    std::env::temp_dir().join(format!("dhat-1429-{}-{stage}.json", std::process::id()))
}

fn profiler(file: &Path) -> dhat::Profiler {
    dhat::Profiler::builder()
        .file_name(file)
        .trim_backtraces(Some(FRAMES))
        .build()
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

fn status_kib(field: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix(field)?
                    .trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse()
                    .ok()
            })
        })
        .unwrap_or(0)
}

fn export_stage(path: &str) -> anyhow::Result<()> {
    let file = dump_path("export");
    let profile = profiler(&file);
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let run = run_export_with_limits(&mut rf, &ExportOptions::default(), WalkerLimits::default())?;
    let built = dhat::HeapStats::get();
    println!(
        "export built entities {} after_export_peak_mib {:.1} after_export_curr_mib {:.1} vm_hwm_mib {:.1}",
        run.model.entities.len(),
        mib(built.max_bytes as u64),
        mib(built.curr_bytes as u64),
        status_kib("VmHWM:") as f64 / 1024.0,
    );
    for note in &run.notes {
        println!("note {note}");
    }
    let model = run.model;
    let _diagnostics = run.diagnostics;
    drop(rf);
    let options = StepOptions { timestamp: Some(0) };
    let written = write_step_to(&model, &options, std::io::sink())?;
    let stats = dhat::HeapStats::get();
    println!(
        "export written {written} total_mib {:.1} total_blocks {} peak_mib {:.1} peak_blocks {} end_mib {:.1} end_blocks {} vm_hwm_mib {:.1}",
        mib(stats.total_bytes),
        stats.total_blocks,
        mib(stats.max_bytes as u64),
        stats.max_blocks,
        mib(stats.curr_bytes as u64),
        stats.curr_blocks,
        status_kib("VmHWM:") as f64 / 1024.0,
    );
    drop(profile);
    summarize(&file, "export")?;
    Ok(())
}

fn meshes_stage(path: &str) -> anyhow::Result<()> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let options = ExportOptions::default().with_saved_meshes(SavedMeshChoice::Off);
    let mut model = run_export_with_limits(&mut rf, &options, WalkerLimits::default())?.model;
    let Some(scope) = rvt::ifc::saved_meshes::default_scope(&mut rf) else {
        println!("meshes: no saved meshes are drawn for this release");
        return Ok(());
    };
    let file = dump_path("meshes");
    let profile = profiler(&file);
    let report = rvt::ifc::saved_meshes::attach_scoped_with_control(
        &mut rf,
        &mut model,
        scope,
        &rvt::control::WalkerControl::default(),
    )?;
    // The pass frees its decoded graphs on a thread of its own.
    std::thread::sleep(Duration::from_secs(5));
    let stats = dhat::HeapStats::get();
    println!("meshes report {report:?}");
    println!(
        "meshes total_mib {:.1} total_blocks {} peak_mib {:.1} peak_blocks {} end_mib {:.1} end_blocks {} vm_hwm_mib {:.1}",
        mib(stats.total_bytes),
        stats.total_blocks,
        mib(stats.max_bytes as u64),
        stats.max_blocks,
        mib(stats.curr_bytes as u64),
        stats.curr_blocks,
        status_kib("VmHWM:") as f64 / 1024.0,
    );
    drop(profile);
    summarize(&file, "meshes")?;
    drop(model);
    drop(rf);
    Ok(())
}

/// A frame as dhat prints it, `0x..: name (file:line:col)`, without the
/// address and the position.
fn frame_name(frame: &str) -> &str {
    let rest = frame.split_once(": ").map_or(frame, |(_, rest)| rest);
    rest.rsplit_once(" (").map_or(rest, |(name, _)| name)
}

fn short(name: &str) -> String {
    name.chars().take(80).collect()
}

/// bytes and blocks at the peak, then at the end
type Held = [u64; 4];

fn summarize(file: &Path, stage: &str) -> anyhow::Result<()> {
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(file)?)?;
    let _ = std::fs::remove_file(file);
    let frames: Vec<&str> = json["ftbl"]
        .as_array()
        .map(|all| all.iter().map(|v| v.as_str().unwrap_or("")).collect())
        .unwrap_or_default();
    let mut groups: [BTreeMap<String, Held>; 3] = Default::default();
    let mut total: Held = [0; 4];
    let mut sites = 0usize;
    for site in json["pps"].as_array().into_iter().flatten() {
        let held: Held = [
            site["gb"].as_u64().unwrap_or(0),
            site["gbk"].as_u64().unwrap_or(0),
            site["eb"].as_u64().unwrap_or(0),
            site["ebk"].as_u64().unwrap_or(0),
        ];
        sites += 1;
        for (sum, add) in total.iter_mut().zip(held) {
            *sum += add;
        }
        if held == [0; 4] {
            continue;
        }
        let names: Vec<&str> = site["fs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|index| index.as_u64())
            .filter_map(|index| frames.get(index as usize).copied())
            .map(frame_name)
            .collect();
        let ours: Vec<&str> = names
            .iter()
            .copied()
            .filter(|name| name.contains("rvt::"))
            .collect();
        let function = ours
            .first()
            .map_or_else(|| "(no rvt frame)".to_string(), |name| short(name));
        let stack = format!(
            "{} || {}",
            names
                .iter()
                .take(3)
                .map(|name| short(name))
                .collect::<Vec<_>>()
                .join(" < "),
            ours.iter()
                .take(4)
                .map(|name| short(name))
                .collect::<Vec<_>>()
                .join(" < ")
        );
        let pass = ours
            .iter()
            .rev()
            .take(5)
            .map(|name| short(name))
            .collect::<Vec<_>>()
            .join(" > ");
        for (map, key) in groups.iter_mut().zip([function, stack, pass]) {
            let slot = map.entry(key).or_insert([0; 4]);
            for (sum, add) in slot.iter_mut().zip(held) {
                *sum += add;
            }
        }
    }
    println!(
        "{stage} sites {sites} at_peak_mib {:.1} blocks {} at_end_mib {:.1} blocks {}",
        mib(total[0]),
        total[1],
        mib(total[2]),
        total[3],
    );
    let titles = [
        "by function",
        "by stack",
        "by pass (outermost rvt frames first)",
    ];
    for (title, map) in titles.iter().zip(&groups) {
        for (when, field) in [("at_peak", 0usize), ("at_end", 2usize)] {
            let mut rows: Vec<(&String, &Held)> =
                map.iter().filter(|(_, held)| held[field] > 0).collect();
            rows.sort_by_key(|(_, held)| std::cmp::Reverse(held[field]));
            println!("{stage} {when} {title} (MiB, blocks, call site):");
            for (key, held) in rows.into_iter().take(TOP) {
                println!("  {:9.1} {:9}  {key}", mib(held[field]), held[field + 1]);
            }
        }
    }
    Ok(())
}
