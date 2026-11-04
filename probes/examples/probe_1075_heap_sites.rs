//! #1075: which code holds `rvt-ifc`'s heap at its peak and after the export,
//! by call site.
//!
//! FACT under test: the memory the export holds besides the handle's inflated
//! partitions is a few structures, each allocated in one place. dhat records
//! the call stack of every allocation, so after a run it can say how many
//! bytes each stack held when the heap was at its highest (`t-gmax`) and when
//! the run ended (`t-end`).
//!
//! Two stages, each run in a process of its own (dhat profiles one run per
//! process; the probe starts itself twice):
//!
//! - `export`: from before the file is opened to the end of
//!   `RvtDocExporter::export_with_mode_and_limits`, with the handle and the
//!   model still alive when the profile ends. `at_end` is what the export
//!   leaves held: the handle's caches and the model.
//! - `meshes`: the saved-graphics pass alone (`saved_meshes::attach_scoped_with_control`
//!   for the default scope) on an export made unprofiled, so `at_peak` is what
//!   that pass holds at its own highest, and `at_end` what it leaves.
//!
//! dhat slows a run by an order of magnitude, so a file over 48 MiB is skipped
//! and a stage is stopped after 9 minutes (the Measure step cuts a probe at 20).
//! It prints, for each profile, the largest call sites by the innermost `rvt::`
//! frame (`by function`) and by the innermost frames of the stack (`by stack`),
//! in MiB and blocks. Frames are the symbols of the `ci` profile build, so a
//! function inlined into its caller shows as the caller.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1075_heap_sites -- MODEL.rvt

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use rvt::RevitFile;
use rvt::ifc::{ExportQualityMode, RvtDocExporter};

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

const MAX_FILE_BYTES: u64 = 48 << 20;
const STAGE_LIMIT: Duration = Duration::from_secs(540);
const FRAMES: usize = 20;
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
    let path = path.ok_or_else(|| anyhow::anyhow!("usage: probe_1075_heap_sites MODEL.rvt"))?;
    match stage.as_deref() {
        None => parent(&path),
        Some("export") => export_stage(&path),
        Some("meshes") => meshes_stage(&path),
        Some(other) => anyhow::bail!("unknown stage {other}"),
    }
}

fn parent(path: &str) -> anyhow::Result<()> {
    let size = std::fs::metadata(path)?.len();
    if size > MAX_FILE_BYTES {
        println!(
            "skipped: {size} bytes is over the {MAX_FILE_BYTES} bytes dhat can follow in time"
        );
        return Ok(());
    }
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
    std::env::temp_dir().join(format!("dhat-1075-{}-{stage}.json", std::process::id()))
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

fn export_stage(path: &str) -> anyhow::Result<()> {
    let file = dump_path("export");
    let profile = profiler(&file);
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let model = RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    let stats = dhat::HeapStats::get();
    println!(
        "export entities {} total_mib {:.1} total_blocks {} peak_mib {:.1} peak_blocks {} end_mib {:.1} end_blocks {}",
        model.entities.len(),
        mib(stats.total_bytes),
        stats.total_blocks,
        mib(stats.max_bytes as u64),
        stats.max_blocks,
        mib(stats.curr_bytes as u64),
        stats.curr_blocks,
    );
    drop(profile);
    summarize(&file, "export")?;
    drop(model);
    drop(rf);
    Ok(())
}

fn meshes_stage(path: &str) -> anyhow::Result<()> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let mut model = RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
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
    println!(
        "meshes replaced {} of {} total_mib {:.1} total_blocks {} peak_mib {:.1} peak_blocks {} end_mib {:.1} end_blocks {}",
        report.replaced,
        report.candidates,
        mib(stats.total_bytes),
        stats.total_blocks,
        mib(stats.max_bytes as u64),
        stats.max_blocks,
        mib(stats.curr_bytes as u64),
        stats.curr_blocks,
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
    let (mut by_function, mut by_stack): (BTreeMap<String, Held>, BTreeMap<String, Held>) =
        (BTreeMap::new(), BTreeMap::new());
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
        for (map, key) in [(&mut by_function, function), (&mut by_stack, stack)] {
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
    for (title, map) in [("by function", &by_function), ("by stack", &by_stack)] {
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
