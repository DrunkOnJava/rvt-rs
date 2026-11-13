//! `rvt-diff` — compare two Revit files stream-by-stream, or element by
//! element.
//!
//! Take two (or more)
//! versions of the *same logical* content and identify what's invariant
//! vs what changes. The invariant bytes are format structure; the varying
//! bytes are payload. This is the delta-analysis step that makes object
//! graph inference tractable without a spec.
//!
//! With `--elements`, two files are exported as `rvt-ifc` exports them and
//! compared by the GlobalId of each element the export writes
//! ([`rvt::ifc::element_diff`], #1843): the elements added, removed and
//! changed, and for each changed one the fields that differ. `--json` writes
//! it all (`docs/schemas/rvt-diff-elements.schema.json`).

use clap::Parser;
use rvt::ifc::element_diff::{ElementDiff, ElementState, diff_elements, element_states};
use rvt::{RevitFile, compression};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-diff",
    version,
    about = "Compare Revit files stream-by-stream, or two saves element by element (--elements)",
    after_help = "Examples:\n  \
        rvt-diff before.rvt after.rvt --decompress\n  \
        rvt-diff --elements monday.rvt tuesday.rvt --json changes.json"
)]
struct Cli {
    /// Two or more Revit files to compare.
    #[arg(required = true, num_args = 2..)]
    files: Vec<PathBuf>,

    /// Also decompress each stream and diff the inflated bytes.
    #[arg(long = "decompress")]
    decompress: bool,

    /// Bytes to show per differing region.
    #[arg(long = "preview", default_value_t = 32)]
    preview: usize,

    /// Compare exactly two files element by element: export each as rvt-ifc
    /// does and key the elements by the GlobalId the export writes. Lists the
    /// elements added, removed and changed, with the fields that changed.
    #[arg(long)]
    elements: bool,

    /// With --elements, write the whole comparison as JSON to this path
    /// (docs/schemas/rvt-diff-elements.schema.json).
    #[arg(long, value_name = "PATH", requires = "elements")]
    json: Option<PathBuf>,

    /// With --elements, the most elements of each kind the summary lists.
    #[arg(long, default_value_t = 20)]
    limit: usize,

    /// With --elements, scrub user names, paths and e-mail addresses from
    /// every name and text value printed or written (`rvt::redact`).
    #[arg(long, requires = "elements")]
    redact: bool,
}

fn main() -> ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    let cli = Cli::parse();
    let redact = cli.elements && cli.redact;
    let result = if cli.elements {
        run_elements(&cli)
    } else {
        run(cli)
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // An error opening a file names its path.
            let message = e.to_string();
            let message = if redact {
                rvt::redact::redact_sensitive(&message)
            } else {
                message
            };
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mut files: Vec<(PathBuf, RevitFile)> = cli
        .files
        .iter()
        .map(|p| RevitFile::open(p).map(|rf| (p.clone(), rf)))
        .collect::<Result<_, _>>()?;

    // Collect stream name union
    let all_streams: std::collections::BTreeSet<String> =
        files.iter().flat_map(|(_, rf)| rf.stream_names()).collect();

    println!("=== comparing {} files ===", files.len());
    for (p, rf) in &files {
        let names = rf.stream_names();
        println!(
            "  {} · {} streams · partitions={}",
            p.file_name().unwrap().to_string_lossy(),
            names.len(),
            rf.partition_stream_name().unwrap_or_else(|| "none".into())
        );
    }

    println!();
    println!(
        "{:<34} {}",
        "stream",
        files
            .iter()
            .map(|(p, _)| p
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .chars()
                .take(12)
                .collect::<String>())
            .collect::<Vec<_>>()
            .join("  ")
    );
    println!("{}", "-".repeat(34 + files.len() * 14));

    for name in &all_streams {
        let mut row = format!("{name:<34}");
        let mut sizes: Vec<Option<(usize, Option<usize>)>> = Vec::with_capacity(files.len());
        for (_, rf) in files.iter_mut() {
            match rf.read_stream(name) {
                Ok(bytes) => {
                    let decompressed_len = if cli.decompress {
                        compression::inflate_at(&bytes, 0).ok().map(|v| v.len())
                    } else {
                        None
                    };
                    sizes.push(Some((bytes.len(), decompressed_len)));
                }
                Err(_) => sizes.push(None),
            }
        }
        for s in &sizes {
            match s {
                None => row.push_str(&format!("{:>12}  ", "—")),
                Some((raw, Some(inflated))) => row.push_str(&format!("{raw:>5}→{inflated:<6}  ")),
                Some((raw, None)) => row.push_str(&format!("{raw:>12}  ")),
            }
        }
        println!("{row}");
    }

    // If exactly 2 files, do a byte-level diff on shared streams
    if files.len() == 2 {
        println!("\n=== byte-level diff (shared streams) ===");
        let shared: Vec<String> = all_streams
            .iter()
            .filter(|name| files.iter_mut().all(|(_, rf)| rf.read_stream(name).is_ok()))
            .cloned()
            .collect();
        for name in shared {
            let a = files[0].1.read_stream(&name)?;
            let b = files[1].1.read_stream(&name)?;
            let mut diffs = 0usize;
            let min_len = a.len().min(b.len());
            let mut first_diff: Option<usize> = None;
            for i in 0..min_len {
                if a[i] != b[i] {
                    diffs += 1;
                    if first_diff.is_none() {
                        first_diff = Some(i);
                    }
                }
            }
            let length_delta = (a.len() as isize) - (b.len() as isize);
            let pct = if min_len == 0 {
                100.0
            } else {
                100.0 * diffs as f64 / min_len as f64
            };
            if diffs == 0 && length_delta == 0 {
                println!("  {:<34} IDENTICAL", name);
            } else {
                println!(
                    "  {:<34} Δlen={:+}  Δbytes={}/{} ({:.1}%)  first@{}",
                    name,
                    length_delta,
                    diffs,
                    min_len,
                    pct,
                    first_diff
                        .map(|i| format!("0x{i:x}"))
                        .unwrap_or_else(|| "—".into())
                );
                if let Some(off) = first_diff {
                    let preview = cli.preview.min(min_len - off);
                    print!("         a[{off:>6x}..] ");
                    for b in &a[off..off + preview] {
                        print!("{:02x} ", b);
                    }
                    println!();
                    print!("         b[{off:>6x}..] ");
                    for bb in &b[off..off + preview] {
                        print!("{:02x} ", bb);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// What `--json` writes: the two files by name, and the comparison.
#[derive(Serialize)]
struct ElementReport<'a> {
    left: String,
    right: String,
    #[serde(flatten)]
    diff: &'a ElementDiff,
}

fn run_elements(cli: &Cli) -> anyhow::Result<()> {
    let [left, right] = cli.files.as_slice() else {
        anyhow::bail!("--elements compares exactly two files");
    };
    let label = |path: &Path| {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if cli.redact {
            rvt::redact::redact_sensitive(&name)
        } else {
            name
        }
    };
    let (left_label, right_label) = (label(left.as_path()), label(right.as_path()));
    let mut diff = diff_elements(
        &export_states(left, &left_label)?,
        &export_states(right, &right_label)?,
    );
    if cli.redact {
        redact_diff(&mut diff);
    }
    print_summary(&diff, &left_label, &right_label, cli.limit);
    if let Some(path) = &cli.json {
        let report = ElementReport {
            left: left_label,
            right: right_label,
            diff: &diff,
        };
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}

/// The elements `path`'s default IFC export writes. A stream a resource limit
/// kept out is said on stderr, since what it holds is then missing from the
/// comparison. The file is named by `label`, not its path, so `--redact`
/// leaves no user name on stderr.
fn export_states(path: &Path, label: &str) -> anyhow::Result<Vec<ElementState>> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let run = rvt::ifc::run_export(&mut rf, &rvt::ifc::ExportOptions::default())?;
    if !run.unread.is_empty() {
        eprintln!(
            "rvt-diff: {label}: {} stream(s) not read whole, so the comparison is missing what they hold:",
            run.unread.len()
        );
        for line in rvt::cli::unread_stream_report(&run.unread) {
            eprintln!("{line}");
        }
    }
    Ok(element_states(&run.model))
}

fn redact_diff(diff: &mut ElementDiff) {
    fn text(value: &mut String) {
        *value = rvt::redact::redact_sensitive(value);
    }
    fn state(state: &mut ElementState) {
        text(&mut state.name);
        for value in [&mut state.storey, &mut state.type_name, &mut state.material]
            .into_iter()
            .flatten()
        {
            text(value);
        }
        // A parameter's name is typed by the user as its value is.
        state.properties = std::mem::take(&mut state.properties)
            .into_iter()
            .map(|(key, mut value)| {
                json(&mut value);
                (rvt::redact::redact_sensitive(&key), value)
            })
            .collect();
    }
    fn json(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(inner) => text(inner),
            serde_json::Value::Array(items) => items.iter_mut().for_each(json),
            serde_json::Value::Object(fields) => fields.values_mut().for_each(json),
            _ => {}
        }
    }
    diff.added.iter_mut().for_each(state);
    diff.removed.iter_mut().for_each(state);
    for change in &mut diff.changed {
        text(&mut change.name);
        for field in &mut change.changes {
            text(&mut field.field);
            json(&mut field.left);
            json(&mut field.right);
        }
    }
}

fn print_summary(diff: &ElementDiff, left: &str, right: &str, limit: usize) {
    println!("=== elements: {left} -> {right} ===");
    println!(
        "  {} elements in {left}, {} in {right}: {} added, {} removed, {} changed, {} unchanged",
        diff.left_elements,
        diff.right_elements,
        diff.added.len(),
        diff.removed.len(),
        diff.changed.len(),
        diff.unchanged
    );
    let aside = diff.set_aside;
    if aside.left > 0 || aside.right > 0 {
        let twice = &diff.duplicate_global_ids;
        println!(
            "  not compared: {} elements of {left} and {} of {right}, whose GlobalId an export writes for more than one element ({} GlobalIds in {left}, {} in {right})",
            aside.left,
            aside.right,
            twice.left.len(),
            twice.right.len()
        );
        for (name, ids) in [(left, &twice.left), (right, &twice.right)] {
            for id in ids.iter().take(limit) {
                println!("    written twice by {name}: {id}");
            }
            if ids.len() > limit {
                println!("    ... {} more in {name}", ids.len() - limit);
            }
        }
    }
    let by_type = |states: &[ElementState]| {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for state in states {
            *counts.entry(state.ifc_type.as_str()).or_default() += 1;
        }
        counts
            .iter()
            .map(|(kind, count)| format!("{kind} {count}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    for (heading, states) in [("added", &diff.added), ("removed", &diff.removed)] {
        if states.is_empty() {
            continue;
        }
        println!("\n{heading}: {}", by_type(states.as_slice()));
        for state in states.iter().take(limit) {
            println!(
                "  {} {:<16} {} {}",
                state.global_id,
                state.ifc_type,
                state.tag.as_deref().unwrap_or("-"),
                state.name
            );
        }
        if states.len() > limit {
            println!("  ... {} more", states.len() - limit);
        }
    }
    if !diff.changed.is_empty() {
        let mut fields: BTreeMap<&str, usize> = BTreeMap::new();
        for change in &diff.changed {
            let mut seen = std::collections::BTreeSet::new();
            for field in &change.changes {
                let kind = if field.field.starts_with("property:") {
                    "properties"
                } else {
                    field.field.as_str()
                };
                if seen.insert(kind) {
                    *fields.entry(kind).or_default() += 1;
                }
            }
        }
        let fields = fields
            .iter()
            .map(|(field, count)| format!("{field} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("\nchanged, elements by field: {fields}");
        for change in diff.changed.iter().take(limit) {
            let names = change
                .changes
                .iter()
                .map(|field| field.field.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "  {} {:<16} {} {}: {names}",
                change.global_id,
                change.ifc_type,
                change.tag.as_deref().unwrap_or("-"),
                change.name
            );
        }
        if diff.changed.len() > limit {
            println!("  ... {} more", diff.changed.len() - limit);
        }
    }
}
