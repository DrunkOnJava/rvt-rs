//! How close the value each of #1825's near-zero geometric tests compares
//! comes to its threshold, on one model's default export.
//!
//! FACT under test: `threshold_census` counts, for each `note` call, the
//! values the test beside it compared, by sign and decade, with exact zeros
//! apart. A site whose values sit between 1e-12 and 1e-6 is one a drift of
//! about 5e-8 ft (RE-447) can tip; a site whose values are all exactly zero
//! or far from its threshold cannot. The export is the same with the census
//! on: `note` returns the value it is given, and the Measure run that reads
//! this compares every `ifc.sha256` with `main`'s.
//!
//! The model is exported as `rvt-ifc` exports it by default, and the STEP is
//! written (to nothing), so the writer's ring-closure tests run too. The
//! saved meshes are built on worker threads; the census counts across
//! threads.
//!
//! Output, one line per site that ran, then a total; counts only:
//!   site <file>:<line> n=<values> <bucket>=<count> ...   (non-zero buckets)
//!   census sites=<n> values=<n> export=<ok|failed>
//! Buckets: <-1e-6 -1e-6 -1e-7 -1e-8 -1e-9 -1e-12 0 +1e-12 +1e-9 +1e-8 +1e-7
//! +1e-6 >1e-6 nan; "-1e-9" holds values in [-1e-9, -1e-12), "+1e-9" those in
//! (1e-12, 1e-9], and so on.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1825_threshold_census -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::threshold_census::{self, BUCKET_LABELS};

fn export(rf: &mut RevitFile) -> anyhow::Result<()> {
    let run = rvt::ifc::run_export_with_limits(
        rf,
        &rvt::ifc::ExportOptions::default(),
        rvt::walker::WalkerLimits::default(),
    )?;
    let options = rvt::ifc::step_writer::StepOptions { timestamp: Some(0) };
    rvt::ifc::step_writer::write_step_to(&run.model, &options, std::io::sink())?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Measure passes the samples `--records` too.
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_1825_threshold_census MODEL.rvt");
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;

    threshold_census::record_thresholds();
    let exported = export(&mut rf);
    let sites = threshold_census::take_thresholds();

    let mut values = 0u64;
    for site in &sites {
        let n: u64 = site.buckets.iter().sum();
        values += n;
        let shown: Vec<String> = site
            .buckets
            .iter()
            .zip(BUCKET_LABELS)
            .filter(|(count, _)| **count > 0)
            .map(|(count, label)| format!("{label}={count}"))
            .collect();
        println!("site {}:{} n={n} {}", site.file, site.line, shown.join(" "));
    }
    println!(
        "census sites={} values={values} export={}",
        sites.len(),
        if exported.is_ok() { "ok" } else { "failed" }
    );
    if let Err(error) = exported {
        eprintln!("export failed: {error:#}");
    }
    Ok(())
}
