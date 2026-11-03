//! #1075: where `rvt-ifc`'s memory goes, stage by stage.
//!
//! FACT under test: the export's peak resident memory (10.3 GB on the
//! reporter's 202 MB model) is held by one stage. Candidates: the inflated
//! partitions `RevitFile` keeps for the handle's lifetime, the element walk
//! and export, the export's diagnostics, the saved-graphics pass, and the
//! STEP text built whole in memory before it is written.
//!
//! The probe runs `rvt-ifc`'s default pipeline in its order (open, export,
//! diagnostics, saved graphics for the default scope, STEP text) and, around
//! each stage, reads `/proc/self/status`. Before a stage it writes `5` to
//! `/proc/self/clear_refs`, which resets the peak (`VmHWM`) to the current
//! resident size, so each stage's `peak` is its own high-water mark. Linux
//! only; elsewhere the memory columns read 0.
//!
//! Output, one line per stage:
//!   stage <name> ms <elapsed> rss_kib <after> peak_kib <stage peak> start_kib <before> [<detail>]
//! then
//!   cache streams <n> bytes <inflated bytes the handle holds>
//!   peak_stage <name> peak_kib <largest stage peak>
//!
//! `cache` calls `RevitFile::inflated_partition` for every partition and
//! `Global/Latest`, so a stream the export never read is inflated there: the
//! `cache` stage's own peak says whether it was. RSS after a drop may not fall,
//! since the allocator keeps freed pages; the stage peaks are what to read.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1075_memory_stages -- MODEL.rvt [--large-model]

use std::time::Instant;

use rvt::RevitFile;
use rvt::ifc::step_writer::{StepOptions, write_step_with_options};
use rvt::ifc::{ExportDiagnosticsMode, ExportQualityMode, RvtDocExporter};

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

struct Stages {
    started: Instant,
    start_kib: u64,
    peak: (String, u64),
}

impl Stages {
    fn begin(&mut self) {
        let _ = std::fs::write("/proc/self/clear_refs", "5");
        self.start_kib = status_kib("VmRSS:");
        self.started = Instant::now();
    }

    fn end(&mut self, name: &str, detail: &str) {
        let ms = self.started.elapsed().as_millis();
        let rss = status_kib("VmRSS:");
        let peak = status_kib("VmHWM:");
        println!(
            "stage {name} ms {ms} rss_kib {rss} peak_kib {peak} start_kib {} {detail}",
            self.start_kib
        );
        if peak > self.peak.1 {
            self.peak = (name.to_string(), peak);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.iter().find(|a| !a.starts_with("--")).ok_or_else(|| {
        anyhow::anyhow!("usage: probe_1075_memory_stages MODEL.rvt [--large-model]")
    })?;
    let large_model = args.iter().any(|a| a == "--large-model");

    let mut stages = Stages {
        started: Instant::now(),
        start_kib: 0,
        peak: (String::new(), 0),
    };
    println!("start rss_kib {}", status_kib("VmRSS:"));

    stages.begin();
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, large_model))?;
    let release = rf.basic_file_info().map(|b| b.version).unwrap_or(0);
    stages.end("open", &format!("release {release}"));

    let mode = ExportQualityMode::Scaffold;
    let mut walker_limits = rvt::walker::WalkerLimits::default();
    if large_model {
        walker_limits.max_scan_bytes = rvt::cli::LARGE_MODEL_MAX_INFLATE_BYTES;
    }

    stages.begin();
    let mut model = RvtDocExporter.export_with_mode_and_limits(&mut rf, mode, walker_limits)?;
    stages.end("export", &format!("entities {}", model.entities.len()));

    stages.begin();
    let diagnostics = rvt::ifc::build_export_diagnostics_with_limits(
        &mut rf,
        &model,
        ExportDiagnosticsMode::Default,
        walker_limits,
    );
    stages.end("diagnostics", "");
    drop(diagnostics);

    stages.begin();
    let detail = match rvt::ifc::saved_meshes::default_scope(&mut rf) {
        Some(scope) => {
            let control = rvt::control::WalkerControl::default();
            match rvt::ifc::saved_meshes::attach_scoped_with_control(
                &mut rf, &mut model, scope, &control,
            ) {
                Ok(report) => format!(
                    "replaced {} of {} triangles {}",
                    report.replaced, report.candidates, report.triangles
                ),
                Err(e) => format!("error {e}"),
            }
        }
        None => "none".to_string(),
    };
    stages.end("saved_meshes", &detail);

    stages.begin();
    let step = write_step_with_options(&model, &StepOptions { timestamp: Some(0) });
    stages.end("step_text", &format!("bytes {}", step.len()));
    drop(step);

    stages.begin();
    let mut names = rf.partition_stream_names();
    names.push(rvt::streams::GLOBAL_LATEST.to_string());
    let (mut streams, mut bytes) = (0usize, 0usize);
    for name in &names {
        if let Ok(inflated) = rf.inflated_partition(name) {
            streams += 1;
            bytes += inflated.bytes().len();
        }
    }
    stages.end("cache", "");
    println!("cache streams {streams} bytes {bytes}");

    stages.begin();
    drop(rf);
    stages.end("drop_file", "");

    stages.begin();
    drop(model);
    stages.end("drop_model", "");

    println!("peak_stage {} peak_kib {}", stages.peak.0, stages.peak.1);
    Ok(())
}
