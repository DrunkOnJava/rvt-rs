//! `rvt-info` — dump Revit file metadata as text or JSON. No Autodesk software required.
//!
//! One file: a detailed report (identity, document, PartAtom, schema,
//! streams). Several files or a folder: an inventory with one row per file
//! — Revit release, worksharing, last saved — as a table, CSV, JSON, or
//! JSON Lines. The inventory reads only the two small identity streams of
//! each file, so a share full of large projects scans quickly.

use clap::{Parser, ValueEnum};
use rvt::RevitFile;
use rvt::metadata::FileMetadata;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-info",
    version,
    about = "Read Revit (.rvt, .rfa, .rte, .rft) metadata without requiring Revit",
    after_help = "Examples:\n  \
        rvt-info model.rvt                          detailed report for one file\n  \
        rvt-info projects/                          inventory of every Revit file under a folder\n  \
        rvt-info projects/ -f csv > inventory.csv   the same inventory as a spreadsheet\n  \
        rvt-info a.rvt b.rfa --json                 one JSON record per file\n  \
        rvt-info model.rvt --extract-preview thumb.png"
)]
struct Cli {
    /// Revit files and/or folders. Folders are searched recursively for
    /// .rvt / .rfa / .rte / .rft files. More than one file (or any folder)
    /// switches to inventory output: one row per file.
    #[arg(required = true, value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Output format. `csv` and `jsonl` always produce inventory rows.
    #[arg(short = 'f', long = "format", value_enum)]
    format: Option<Format>,

    /// Shorthand for `--format json`.
    #[arg(long, conflicts_with = "format")]
    json: bool,

    /// Only look at the top level of folders given as PATH.
    #[arg(long = "no-recurse")]
    no_recurse: bool,

    /// Include a sample of class/schema names (single-file report).
    #[arg(long = "show-classes")]
    show_classes: bool,

    /// Include full raw class list (one per line after the summary;
    /// single-file report).
    #[arg(long = "all-classes")]
    all_classes: bool,

    /// Extract the PNG preview thumbnail to this path (single file only).
    #[arg(long = "extract-preview")]
    extract_preview: Option<PathBuf>,

    /// Redact PII — Windows usernames (including inside local-copy file
    /// names), the last-saved-by user, Autodesk-internal paths, and
    /// project-ID folder names — before rendering. Safe default for
    /// sharing output publicly.
    #[arg(long)]
    redact: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Text,
    Json,
    Csv,
    Jsonl,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let format = if cli.json {
        Format::Json
    } else {
        cli.format.unwrap_or(Format::Text)
    };
    let single = cli.paths.len() == 1 && !cli.paths[0].is_dir();
    if single && matches!(format, Format::Text | Format::Json) {
        report_one(&cli, format)?;
        return Ok(ExitCode::SUCCESS);
    }
    if cli.extract_preview.is_some() {
        anyhow::bail!("--extract-preview needs exactly one file");
    }
    let files = collect_files(&cli.paths, !cli.no_recurse)?;
    if files.is_empty() {
        anyhow::bail!("no .rvt / .rfa / .rte / .rft files found");
    }
    let rows = inventory(&files, cli.redact);
    match format {
        Format::Text => print_table(&rows),
        Format::Json => println!("{}", serde_json::to_string_pretty(&rows)?),
        Format::Jsonl => {
            for row in &rows {
                println!("{}", serde_json::to_string(row)?);
            }
        }
        Format::Csv => print_csv(&rows),
    }
    let failed = rows.iter().any(|r| r.error.is_some());
    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

// ---- single-file report --------------------------------------------------

#[derive(Serialize)]
struct Report<'a> {
    #[serde(flatten)]
    summary: &'a rvt::reader::Summary,
    /// Worksharing, save history and document identity
    /// (`rvt::metadata::FileMetadata`).
    document: Option<&'a FileMetadata>,
}

fn report_one(cli: &Cli, format: Format) -> anyhow::Result<()> {
    let path = &cli.paths[0];
    let mut rf = RevitFile::open(path)?;
    // Lossy summarise — the CLI tolerates partial parses (missing
    // PartAtom, unreadable class inventory) and prints whatever
    // identity we managed to extract.
    let mut summary = rf.summarize_lossy()?.value;
    let mut document = rf.metadata().ok();
    if cli.redact {
        redact_summary(&mut summary);
        if let Some(doc) = document.as_mut() {
            doc.redact();
        }
    }

    if let Some(preview_path) = &cli.extract_preview {
        let png = rf.preview_png()?;
        std::fs::write(preview_path, &png)?;
        eprintln!("preview PNG written to {}", preview_path.display());
    }

    if format == Format::Json {
        let report = Report {
            summary: &summary,
            document: document.as_ref(),
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    print_text(&name, &summary, document.as_ref(), cli.show_classes);
    if cli.all_classes {
        let names = rf.class_names()?;
        println!("\n=== all class names ({}) ===", names.len());
        for n in names {
            println!("{n}");
        }
    }
    Ok(())
}

fn redact_summary(s: &mut rvt::reader::Summary) {
    if let Some(p) = &s.original_path {
        s.original_path = Some(rvt::redact::redact_sensitive(p));
    }
}

fn print_text(
    name: &str,
    s: &rvt::reader::Summary,
    doc: Option<&FileMetadata>,
    show_classes: bool,
) {
    println!("Revit file · {name}");
    println!("  version:       {}", s.version);
    if let Some(b) = &s.build {
        println!("  build:         {}", b);
    }
    if let Some(l) = &s.locale {
        println!("  locale:        {}", l);
    }
    if let Some(g) = &s.guid {
        println!("  file GUID:     {}", g);
    }
    if let Some(p) = &s.original_path {
        println!("  original path: {}", p);
    }
    if let Some(p) = &s.partition_stream {
        println!("  partition:     {}", p);
    }
    println!(
        "  streams:       {} ({} total bytes)",
        s.streams.len(),
        s.file_size
    );

    if let Some(d) = doc {
        println!("\nDocument ·");
        if let Some(t) = &d.title {
            println!("  title:          {t}");
        }
        if let Some(saved) = &d.last_saved {
            println!("  last saved:     {}", display_timestamp(saved));
        }
        if let Some(w) = &d.worksharing {
            println!("  worksharing:    {w}");
        }
        if let Some(u) = &d.username {
            println!("  last saved by:  {u}");
        }
        if let Some(c) = &d.central_model_path {
            println!("  central model:  {c}");
        }
        if let Some(n) = d.document_increments {
            println!("  save counter:   {n}");
        }
        if let Some(cloud) = d.single_user_cloud_model {
            println!(
                "  cloud model:    {}",
                if cloud { "yes (single-user)" } else { "no" }
            );
        }
    }

    if let Some(pa) = &s.partatom {
        println!("\nPartAtom ·");
        if let Some(t) = &pa.title {
            println!("  title:     {}", t);
        }
        if let Some(i) = &pa.id {
            println!("  id:        {}", i);
        }
        if let Some(u) = &pa.updated {
            println!("  updated:   {}", u);
        }
        if let Some(oc) = &pa.omniclass {
            println!("  omniclass: {}", oc);
        }
        if !pa.categories.is_empty() {
            println!("  categories:");
            for c in &pa.categories {
                if let Some(sch) = &c.scheme {
                    println!("    - {} (scheme: {})", c.term, sch);
                } else {
                    println!("    - {}", c.term);
                }
            }
        }
        if !pa.taxonomies.is_empty() {
            println!("  taxonomies:");
            for t in &pa.taxonomies {
                println!("    - {} ({})", t.label, t.term);
            }
        }
    }

    println!("\nSchema ·");
    println!("  class names (inferred): {}", s.class_name_count);
    if show_classes && !s.class_name_sample.is_empty() {
        println!("  sample:");
        for n in &s.class_name_sample {
            println!("    - {}", n);
        }
    }

    println!("\nStreams ·");
    for name in &s.streams {
        println!("  {}", name);
    }
}

/// `2023-09-07T12:50:35Z` → `2023-09-07 12:50:35 UTC`; anything else as-is.
fn display_timestamp(iso: &str) -> String {
    match iso.strip_suffix('Z').and_then(|t| t.split_once('T')) {
        Some((date, time)) => format!("{date} {time} UTC"),
        None => iso.to_string(),
    }
}

// ---- inventory -------------------------------------------------------------

const REVIT_EXTENSIONS: [&str; 4] = ["rvt", "rfa", "rte", "rft"];

fn has_revit_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| REVIT_EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Explicit file arguments are kept whatever their extension (so a
/// mis-named file still gets a row); folders contribute only Revit files.
fn collect_files(paths: &[PathBuf], recurse: bool) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_dir() {
            walk(path, recurse, &mut out)
                .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        } else {
            out.push(path.clone());
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.clone()));
    Ok(out)
}

fn walk(dir: &Path, recurse: bool, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<std::io::Result<_>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        // `file_type` does not follow symlinks, so a symlinked folder is
        // never descended into (no cycles) while a symlinked file still is.
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if recurse {
                walk(&path, recurse, out)?;
            }
        } else if has_revit_extension(&path) && path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct Row {
    path: String,
    /// `project`, `family`, `project template`, `family template`, or
    /// `unknown`, from the file extension.
    file_type: &'static str,
    /// A Revit backup copy (`name.0001.rvt`).
    backup: bool,
    size_bytes: Option<u64>,
    error: Option<String>,
    #[serde(flatten)]
    metadata: Option<FileMetadata>,
}

fn file_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "rvt" => "project",
        "rfa" => "family",
        "rte" => "project template",
        "rft" => "family template",
        _ => "unknown",
    }
}

/// `Tower.0003.rvt` — the four-digit suffix Revit gives backup copies.
fn is_backup(path: &Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.rsplit_once('.'))
        .is_some_and(|(_, n)| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
}

fn inventory_row(path: &Path, redact: bool) -> Row {
    let size_bytes = std::fs::metadata(path).ok().map(|m| m.len());
    let (metadata, error) = match rvt::metadata::read_metadata(path) {
        Ok(mut m) => {
            if redact {
                m.redact();
            }
            (Some(m), None)
        }
        Err(rvt::Error::StreamNotFound(_)) => (
            None,
            Some("OLE/CFB file without a BasicFileInfo stream — not a Revit file".to_string()),
        ),
        Err(e) => (None, Some(e.to_string())),
    };
    let display = path.display().to_string();
    Row {
        path: if redact {
            rvt::redact::redact_sensitive(&display)
        } else {
            display
        },
        file_type: file_type(path),
        backup: is_backup(path),
        size_bytes,
        error,
        metadata,
    }
}

/// Read every file on a small worker pool (network shares are latency-bound)
/// and return rows in input order.
fn inventory(files: &[PathBuf], redact: bool) -> Vec<Row> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let workers = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 8)
        .min(files.len());
    let next = AtomicUsize::new(0);
    let slots: Mutex<Vec<Option<Row>>> = Mutex::new(files.iter().map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(path) = files.get(i) else { break };
                    let row = inventory_row(path, redact);
                    slots.lock().unwrap_or_else(|p| p.into_inner())[i] = Some(row);
                }
            });
        }
    });
    slots
        .into_inner()
        .unwrap_or_else(|p| p.into_inner())
        .into_iter()
        .flatten()
        .collect()
}

fn human_size(bytes: Option<u64>) -> String {
    let Some(b) = bytes else {
        return "-".to_string();
    };
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = b as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{b} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn print_table(rows: &[Row]) {
    let cells: Vec<[String; 5]> = rows
        .iter()
        .map(|r| match &r.metadata {
            Some(m) => [
                m.revit_version.to_string(),
                m.worksharing.clone().unwrap_or_else(|| "-".to_string()),
                m.last_saved
                    .as_deref()
                    .map(|t| display_timestamp(t).trim_end_matches(" UTC").to_string())
                    .unwrap_or_else(|| "-".to_string()),
                human_size(r.size_bytes),
                r.path.clone(),
            ],
            None => [
                "error".to_string(),
                "-".to_string(),
                "-".to_string(),
                human_size(r.size_bytes),
                format!(
                    "{}  ({})",
                    r.path,
                    r.error.as_deref().unwrap_or("unreadable")
                ),
            ],
        })
        .collect();
    let header = ["VERSION", "WORKSHARING", "LAST SAVED (UTC)", "SIZE", "FILE"];
    let mut widths = header.map(str::len);
    for row in &cells {
        for (w, cell) in widths.iter_mut().zip(row.iter()).take(4) {
            *w = (*w).max(cell.chars().count());
        }
    }
    let line = |cols: [&str; 5]| {
        format!(
            "{:<w0$}  {:<w1$}  {:<w2$}  {:>w3$}  {}",
            cols[0],
            cols[1],
            cols[2],
            cols[3],
            cols[4],
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
            w3 = widths[3],
        )
    };
    println!("{}", line(header));
    for row in &cells {
        println!("{}", line([&row[0], &row[1], &row[2], &row[3], &row[4]]));
    }

    let mut by_version: BTreeMap<u32, usize> = BTreeMap::new();
    let mut workshared = 0;
    let mut failed = 0;
    for r in rows {
        match &r.metadata {
            Some(m) => {
                *by_version.entry(m.revit_version).or_default() += 1;
                if m.workshared == Some(true) {
                    workshared += 1;
                }
            }
            None => failed += 1,
        }
    }
    let versions: Vec<String> = by_version
        .iter()
        .map(|(v, n)| format!("{v} ×{n}"))
        .collect();
    println!(
        "\n{} file{} · by release: {} · workshared: {workshared} · unreadable: {failed}",
        rows.len(),
        if rows.len() == 1 { "" } else { "s" },
        if versions.is_empty() {
            "-".to_string()
        } else {
            versions.join(", ")
        },
    );
}

const CSV_COLUMNS: [&str; 19] = [
    "path",
    "file_type",
    "backup",
    "size_bytes",
    "error",
    "revit_version",
    "build",
    "title",
    "last_saved",
    "worksharing",
    "workshared",
    "username",
    "central_model_path",
    "last_save_path",
    "document_guid",
    "document_increments",
    "locale",
    "single_user_cloud_model",
    "properties",
];

/// RFC 4180 field: quoted when it holds a comma, quote, or line break.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn print_csv(rows: &[Row]) {
    println!("{}", CSV_COLUMNS.join(","));
    for r in rows {
        let opt = |v: Option<&str>| v.unwrap_or_default().to_string();
        let num = |v: Option<u64>| v.map(|n| n.to_string()).unwrap_or_default();
        let flag = |v: Option<bool>| v.map(|b| b.to_string()).unwrap_or_default();
        let m = r.metadata.as_ref();
        let properties = m
            .map(|m| {
                m.properties
                    .iter()
                    .map(|p| format!("{}={}", p.key, p.value))
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        let fields = [
            r.path.clone(),
            r.file_type.to_string(),
            r.backup.to_string(),
            num(r.size_bytes),
            opt(r.error.as_deref()),
            num(m.map(|m| u64::from(m.revit_version))),
            opt(m.and_then(|m| m.build.as_deref())),
            opt(m.and_then(|m| m.title.as_deref())),
            opt(m.and_then(|m| m.last_saved.as_deref())),
            opt(m.and_then(|m| m.worksharing.as_deref())),
            flag(m.and_then(|m| m.workshared)),
            opt(m.and_then(|m| m.username.as_deref())),
            opt(m.and_then(|m| m.central_model_path.as_deref())),
            opt(m.and_then(|m| m.last_save_path.as_deref())),
            opt(m.and_then(|m| m.document_guid.as_deref())),
            num(m.and_then(|m| m.document_increments.map(u64::from))),
            opt(m.and_then(|m| m.locale.as_deref())),
            flag(m.and_then(|m| m.single_user_cloud_model)),
            properties,
        ];
        let line: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
        println!("{}", line.join(","));
    }
}
