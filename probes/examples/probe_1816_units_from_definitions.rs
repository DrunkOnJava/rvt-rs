//! #1816: the project's display units, taken from the definitions pass.
//!
//! FACT under test: the definitions pass decodes every `UnitsElem` record. A
//! pass made with the budgets of `native_project_units::read`'s own read sees
//! the records that read would, in the same order. So `read` takes the units
//! from that pass where the handle holds it, and walks the file for them only
//! where it does not. Both ways give the same result: the same units, `None`,
//! or the same error.
//!
//! The probe:
//! 1. Makes the export `rvt-ifc` makes with no flags and counts how its reads
//!    of the units went (`native_project_units::read_counts`).
//! 2. On a second handle, makes the definitions pass first (an `extract` that
//!    selects nothing) and then reads the units.
//! 3. On a third handle, reads them alone.
//!
//! It prints both results and whether they are equal.
//!
//! Output:
//!   export units_reads held=<n> walked=<n>
//!   fold source=<held|walked> folded=<kind> own=<kind> equal=<bool>
//!
//! `<kind>` is `specs:<n>`, `none` or `error:<message>`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1816_units_from_definitions -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::{native_document, native_project_units};

type Units = anyhow::Result<Option<native_project_units::ProjectUnits>>;

fn kind(units: &Units) -> String {
    match units {
        Ok(Some(units)) => format!("specs:{}", units.specs.len()),
        Ok(None) => "none".into(),
        Err(error) => format!("error:{error:#}"),
    }
}

fn same(a: &Units, b: &Units) -> bool {
    match (a, b) {
        (Ok(a), Ok(b)) => a == b,
        (Err(a), Err(b)) => format!("{a:#}") == format!("{b:#}"),
        _ => false,
    }
}

fn open(path: &str) -> anyhow::Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, false),
    )?)
}

fn main() -> anyhow::Result<()> {
    // Measure passes the samples `--records` too.
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_1816_units_from_definitions MODEL.rvt");

    let (held, walked) = native_project_units::read_counts();
    rvt::ifc::run_export_with_limits(
        &mut open(&path)?,
        &rvt::ifc::ExportOptions::default(),
        rvt::walker::WalkerLimits::default(),
    )?;
    let (held_after, walked_after) = native_project_units::read_counts();
    println!(
        "export units_reads held={} walked={}",
        held_after - held,
        walked_after - walked
    );

    let mut first = open(&path)?;
    let pass = native_document::Options {
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    if let Err(error) = native_document::extract(&mut first, &pass, |_| Ok(())) {
        println!("definitions pass failed: {error:#}");
    }
    let (held, _) = native_project_units::read_counts();
    let folded = native_project_units::read(&mut first);
    let source = if native_project_units::read_counts().0 > held {
        "held"
    } else {
        "walked"
    };
    let own = native_project_units::read(&mut open(&path)?);
    println!(
        "fold source={source} folded={} own={} equal={}",
        kind(&folded),
        kind(&own),
        same(&folded, &own)
    );
    Ok(())
}
