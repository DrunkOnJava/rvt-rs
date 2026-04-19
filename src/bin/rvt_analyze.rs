//! `rvt-analyze` — one-shot forensic analysis of any Autodesk Revit file.
//!
//! Combines every other rvt-rs probe into a single structured report:
//! file identity, document upgrade history, format anchors, schema table,
//! tagged-class linkage, content metadata, and a disclosure scan for
//! embedded names / paths.
//!
//! Usage:
//!
//! ```text
//!   rvt-analyze <file>             # full report, text output
//!   rvt-analyze <file> --json      # machine-readable JSON
//!   rvt-analyze <file> --section schema   # just one section
//! ```

use clap::{Parser, ValueEnum};
use rvt::{compression, object_graph, streams, RevitFile};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-analyze",
    version,
    about = "Single-shot forensic analysis of a Revit file — identity, history, \
             schema, Phase D link, content metadata, and disclosures."
)]
struct Cli {
    /// Path to a .rvt / .rfa / .rte / .rft file
    file: PathBuf,

    /// Emit machine-readable JSON instead of the human report
    #[arg(long)]
    json: bool,

    /// Print only a specific section (default: everything)
    #[arg(long, value_enum)]
    section: Option<Section>,

    /// Suppress the colored ANSI output (redirect-safe)
    #[arg(long)]
    no_color: bool,

    /// Redact PII — Windows usernames from paths, embedded creator names,
    /// and any Autodesk-employee identifiers — before rendering the report.
    /// Safe default for sharing output publicly.
    #[arg(long)]
    redact: bool,

    /// Suppress the banner and empty-section spacing. Useful when piping
    /// output to other tools.
    #[arg(short = 'q', long)]
    quiet: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Section {
    Identity,
    History,
    Anchors,
    Schema,
    Link,
    Content,
    Disclosures,
}

#[derive(Serialize)]
struct Report {
    identity: Identity,
    history: Vec<String>,
    format_anchors: FormatAnchors,
    schema: SchemaSummary,
    link: LinkSummary,
    content_metadata: ContentMetadata,
    disclosures: Disclosures,
}

#[derive(Serialize)]
struct Identity {
    file_path: String,
    file_size_bytes: u64,
    revit_version: Option<u32>,
    build_tag: Option<String>,
    creator_path: Option<String>,
    file_guid: Option<String>,
    locale: Option<String>,
    format_version_counter: Option<u16>,
    partition_stream: Option<String>,
}

#[derive(Serialize)]
struct FormatAnchors {
    partition_table_bytes: usize,
    partition_table_invariant_bytes: Option<usize>,
    format_identifier_guid: Option<String>,
    partition_description: Option<String>,
    contents_wrapper_magic: Option<String>,
}

#[derive(Serialize)]
struct SchemaSummary {
    total_class_candidates: usize,
    tagged_class_count: usize,
    sample_tagged_classes: Vec<TaggedClass>,
}

#[derive(Serialize, Clone)]
struct TaggedClass {
    name: String,
    tag: u16,
}

#[derive(Serialize)]
struct LinkSummary {
    global_latest_bytes: usize,
    total_tag_hits: u32,
    expected_if_uniform_per_tag: f64,
    top_classes: Vec<LinkEntry>,
}

#[derive(Serialize)]
struct LinkEntry {
    name: String,
    tag: u16,
    hits: u32,
    ratio_vs_uniform: f64,
}

#[derive(Serialize)]
struct ContentMetadata {
    global_latest_string_records: usize,
    partitions_nn_string_records: usize,
    autodesk_units: usize,
    autodesk_specs: usize,
    autodesk_parameter_groups: usize,
    uniformat_codes: usize,
    omniclass_codes: usize,
    revit_categories_misc: usize,
    example_uniformat: Vec<String>,
    example_categories: Vec<String>,
}

#[derive(Serialize)]
struct Disclosures {
    creator_paths: Vec<String>,
    embedded_personal_names: Vec<String>,
    autodesk_internal_paths: Vec<String>,
    build_server_paths: Vec<String>,
}

const DIVIDER: &str = "─────────────────────────────────────────────────────────────────────────────";

struct Style {
    color: bool,
}

impl Style {
    fn bold(&self, s: &str) -> String {
        if self.color { format!("\x1b[1m{s}\x1b[0m") } else { s.into() }
    }
    fn dim(&self, s: &str) -> String {
        if self.color { format!("\x1b[2m{s}\x1b[0m") } else { s.into() }
    }
    fn cyan(&self, s: &str) -> String {
        if self.color { format!("\x1b[36m{s}\x1b[0m") } else { s.into() }
    }
    fn yellow(&self, s: &str) -> String {
        if self.color { format!("\x1b[33m{s}\x1b[0m") } else { s.into() }
    }
    fn red(&self, s: &str) -> String {
        if self.color { format!("\x1b[31m{s}\x1b[0m") } else { s.into() }
    }
    fn green(&self, s: &str) -> String {
        if self.color { format!("\x1b[32m{s}\x1b[0m") } else { s.into() }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let color = !cli.no_color && std::io::IsTerminal::is_terminal(&std::io::stdout());
    let s = Style { color };

    let file_size = std::fs::metadata(&cli.file)?.len();
    let mut rf = RevitFile::open(&cli.file)?;

    let identity = build_identity(&mut rf, &cli.file, file_size)?;
    let history = build_history(&mut rf).unwrap_or_default();
    let anchors = build_anchors(&mut rf)?;
    let schema_sum = build_schema(&mut rf)?;
    let link = build_link(&mut rf, &schema_sum.sample_tagged_classes)?;
    let content = build_content(&mut rf);
    let disclosures = build_disclosures(&identity, &content, &anchors);

    let mut report = Report {
        identity,
        history,
        format_anchors: anchors,
        schema: schema_sum,
        link,
        content_metadata: content,
        disclosures,
    };

    if cli.redact {
        redact_report(&mut report);
    }

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    print_report(&report, &s, cli.section, cli.quiet);
    Ok(())
}

fn build_identity(
    rf: &mut RevitFile,
    path: &std::path::Path,
    file_size: u64,
) -> anyhow::Result<Identity> {
    let bfi = rf.basic_file_info().ok();
    let partition_stream = rf.partition_stream_name();
    let format_version_counter = rf
        .read_stream(streams::GLOBAL_PARTITION_TABLE)
        .ok()
        .and_then(|b| compression::inflate_at(&b, 8).ok())
        .filter(|d| d.len() >= 2)
        .map(|d| u16::from_le_bytes([d[0], d[1]]));

    Ok(Identity {
        file_path: path.display().to_string(),
        file_size_bytes: file_size,
        revit_version: bfi.as_ref().map(|b| b.version),
        build_tag: bfi.as_ref().and_then(|b| b.build.clone()),
        creator_path: bfi.as_ref().and_then(|b| b.original_path.clone()),
        file_guid: bfi.as_ref().and_then(|b| b.guid.clone()),
        locale: bfi.as_ref().and_then(|b| b.locale.clone()),
        format_version_counter,
        partition_stream,
    })
}

fn build_history(rf: &mut RevitFile) -> anyhow::Result<Vec<String>> {
    let h = object_graph::DocumentHistory::from_revit_file(rf)?;
    Ok(h.entries)
}

fn build_anchors(rf: &mut RevitFile) -> anyhow::Result<FormatAnchors> {
    let raw = rf.read_stream(streams::GLOBAL_PARTITION_TABLE).ok();
    let decomp = raw
        .as_ref()
        .and_then(|b| compression::inflate_at(b, 8).ok());
    let partition_table_bytes = decomp.as_ref().map(|d| d.len()).unwrap_or(0);
    let guid = decomp.as_ref().and_then(|d| {
        if d.len() < 26 {
            return None;
        }
        let g = &d[10..26];
        Some(format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            g[3], g[2], g[1], g[0], g[5], g[4], g[7], g[6], g[8], g[9], g[10], g[11], g[12], g[13], g[14], g[15]
        ))
    });
    let partition_description = decomp.as_ref().and_then(|d| {
        // Scan forward from offset 0x30 for the first UTF-16LE ASCII-printable
        // character (pattern: <printable-ascii-byte> <zero-byte>), then decode
        // until we hit the trailing payload.  Length prefix alignment varies
        // slightly across releases so this is more robust than a hard offset.
        let scan_start = 0x30;
        if d.len() <= scan_start + 2 {
            return None;
        }
        let mut str_start = None;
        let limit = (scan_start + 16).min(d.len().saturating_sub(1));
        for i in scan_start..limit {
            let b0 = d[i];
            let b1 = d.get(i + 1).copied().unwrap_or(0);
            if (0x20..0x7f).contains(&b0) && b1 == 0 {
                str_start = Some(i);
                break;
            }
        }
        let start = str_start?;
        // Read until first NUL code-unit (00 00) or end of buffer; cap to
        // 512 bytes to avoid walking into the trailing footer.
        let mut end = start;
        let hard_end = (start + 512).min(d.len().saturating_sub(1));
        while end + 1 < hard_end {
            if d[end] == 0 && d[end + 1] == 0 {
                break;
            }
            end += 2;
        }
        if end > start {
            let (cow, _, _) = encoding_rs::UTF_16LE.decode(&d[start..end]);
            let s = cow.into_owned().trim().to_string();
            if !s.is_empty() {
                Some(s)
            } else {
                None
            }
        } else {
            None
        }
    });

    // Contents stream wrapper magic
    let contents_raw = rf.read_stream(streams::CONTENTS).ok();
    let contents_magic = contents_raw.as_ref().filter(|d| d.len() >= 4).map(|d| {
        format!("{:02x} {:02x} {:02x} {:02x}", d[0], d[1], d[2], d[3])
    });

    Ok(FormatAnchors {
        partition_table_bytes,
        partition_table_invariant_bytes: Some(partition_table_bytes.saturating_sub(2)),
        format_identifier_guid: guid,
        partition_description,
        contents_wrapper_magic: contents_magic,
    })
}

fn build_schema(rf: &mut RevitFile) -> anyhow::Result<SchemaSummary> {
    let raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let decomp = compression::inflate_at(&raw, 0)?;
    let scan_limit = (64 * 1024).min(decomp.len());
    let data = &decomp[..scan_limit];

    let mut candidates = 0usize;
    let mut tagged: Vec<TaggedClass> = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() {
        let len = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if !(3..=60).contains(&len) || i + 2 + len + 2 > data.len() {
            i += 1;
            continue;
        }
        let name_bytes = &data[i + 2..i + 2 + len];
        if !looks_like_class_name(name_bytes) {
            i += 1;
            continue;
        }
        candidates += 1;
        let after = i + 2 + len;
        let u16v = u16::from_le_bytes([data[after], data[after + 1]]);
        if u16v & 0x8000 != 0 {
            tagged.push(TaggedClass {
                name: std::str::from_utf8(name_bytes).unwrap().to_string(),
                tag: u16v & 0x7fff,
            });
        }
        i += 2 + len;
    }
    tagged.sort_by_key(|t| t.tag);
    tagged.dedup_by(|a, b| a.name == b.name);

    Ok(SchemaSummary {
        total_class_candidates: candidates,
        tagged_class_count: tagged.len(),
        sample_tagged_classes: tagged,
    })
}

fn build_link(
    rf: &mut RevitFile,
    tagged: &[TaggedClass],
) -> anyhow::Result<LinkSummary> {
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let global = compression::inflate_at(&raw, 8)?;
    let total_positions = global.len().saturating_sub(1).max(1);

    let mut hits: HashMap<u16, u32> = HashMap::new();
    let mut idx = 0;
    while idx + 2 <= global.len() {
        let v = u16::from_le_bytes([global[idx], global[idx + 1]]);
        if v > 0 && v < 0x4000 {
            *hits.entry(v).or_insert(0) += 1;
        }
        idx += 1;
    }

    let expected_per_tag = total_positions as f64 / 0x4000 as f64;
    let mut entries: Vec<LinkEntry> = tagged
        .iter()
        .map(|t| {
            let h = *hits.get(&t.tag).unwrap_or(&0);
            LinkEntry {
                name: t.name.clone(),
                tag: t.tag,
                hits: h,
                ratio_vs_uniform: h as f64 / expected_per_tag.max(1.0),
            }
        })
        .collect();
    entries.sort_by(|a, b| b.hits.cmp(&a.hits));

    let total_hits: u32 = entries.iter().map(|e| e.hits).sum();
    Ok(LinkSummary {
        global_latest_bytes: global.len(),
        total_tag_hits: total_hits,
        expected_if_uniform_per_tag: expected_per_tag,
        top_classes: entries.into_iter().take(15).collect(),
    })
}

fn build_content(rf: &mut RevitFile) -> ContentMetadata {
    let global_records = object_graph::string_records_from_file(rf)
        .map(|r| r.len())
        .unwrap_or(0);
    let partition_records =
        object_graph::string_records_from_partitions(rf).unwrap_or_default();
    let total_partition = partition_records.len();

    let mut units = 0usize;
    let mut specs = 0usize;
    let mut groups = 0usize;
    let mut uniformat: Vec<String> = Vec::new();
    let mut omniclass = 0usize;
    let mut categories: Vec<String> = Vec::new();
    for r in &partition_records {
        let v = &r.value;
        if v.starts_with("autodesk.unit.") {
            units += 1;
        } else if v.starts_with("autodesk.spec.") {
            specs += 1;
        } else if v.starts_with("autodesk.parameter.group") {
            groups += 1;
        } else if v.chars().all(|c| c.is_ascii_digit() || c == '.') && v.contains('.') {
            omniclass += 1;
        } else if (v.starts_with('A')
            || v.starts_with('B')
            || v.starts_with('C')
            || v.starts_with('D')
            || v.starts_with('E')
            || v.starts_with('F')
            || v.starts_with('G')
            || v.starts_with('Z'))
            && v.len() >= 4
            && v[1..].chars().all(|c| c.is_ascii_digit())
        {
            uniformat.push(v.clone());
        } else if v.len() >= 4
            && !v.starts_with("autodesk.")
            && !v.starts_with('%')
            && v.chars().any(|c| c.is_ascii_alphabetic())
        {
            categories.push(v.clone());
        }
    }

    ContentMetadata {
        global_latest_string_records: global_records,
        partitions_nn_string_records: total_partition,
        autodesk_units: units,
        autodesk_specs: specs,
        autodesk_parameter_groups: groups,
        uniformat_codes: uniformat.len(),
        omniclass_codes: omniclass,
        revit_categories_misc: categories.len(),
        example_uniformat: uniformat.into_iter().take(6).collect(),
        example_categories: categories.into_iter().take(10).collect(),
    }
}

fn build_disclosures(
    identity: &Identity,
    _content: &ContentMetadata,
    _anchors: &FormatAnchors,
) -> Disclosures {
    // Creator path is the main source of PII / disclosure signal. We also
    // inspect raw_text of BasicFileInfo implicitly via creator_path.
    let mut creator_paths = Vec::new();
    let mut autodesk_internal_paths = Vec::new();
    let mut personal_names = Vec::new();

    if let Some(p) = &identity.creator_path {
        creator_paths.push(p.clone());
        // Flag if inside an Autodesk/OneDrive-Autodesk-owned folder
        if p.to_ascii_lowercase().contains("autodesk")
            || p.to_ascii_lowercase().contains("onedrive - autodesk")
        {
            autodesk_internal_paths.push(p.clone());
        }
        // Extract a Windows-style username slice if present
        if let Some(idx) = p.find("\\Users\\") {
            let tail = &p[idx + 7..];
            if let Some(end) = tail.find('\\') {
                let name = tail[..end].to_string();
                if !name.is_empty() && name.len() < 40 {
                    personal_names.push(name);
                }
            }
        }
    }

    Disclosures {
        creator_paths,
        embedded_personal_names: personal_names,
        autodesk_internal_paths,
        build_server_paths: Vec::new(),
    }
}

fn looks_like_class_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes[0].is_ascii_uppercase()
        && bytes[1..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn print_report(report: &Report, s: &Style, section: Option<Section>, quiet: bool) {
    let should_print = |want: Section| -> bool {
        match section {
            None => true,
            Some(sel) => std::mem::discriminant(&sel) == std::mem::discriminant(&want),
        }
    };

    if !quiet {
        println!("{}", s.bold(&banner()));
        println!();
    }

    if should_print(Section::Identity) {
        print_identity(&report.identity, s);
    }
    if should_print(Section::History) {
        print_history(&report.history, s);
    }
    if should_print(Section::Anchors) {
        print_anchors(&report.format_anchors, s);
    }
    if should_print(Section::Schema) {
        print_schema(&report.schema, s);
    }
    if should_print(Section::Link) {
        print_link(&report.link, s);
    }
    if should_print(Section::Content) {
        print_content(&report.content_metadata, s);
    }
    if should_print(Section::Disclosures) {
        print_disclosures(&report.disclosures, s);
    }
}

fn banner() -> String {
    let b = [
        "╭──────────────────────────────────────────────────────────────────────────╮",
        "│                                                                          │",
        "│   rvt-analyze  ·  open forensic reader for Autodesk Revit files          │",
        "│                                                                          │",
        "│   One command. Every moat layer. No Autodesk dependency.                 │",
        "│                                                                          │",
        "╰──────────────────────────────────────────────────────────────────────────╯",
    ];
    b.join("\n")
}

fn header(title: &str, s: &Style) {
    println!("{}", s.cyan(DIVIDER));
    println!("  {}", s.bold(title));
    println!("{}", s.cyan(DIVIDER));
}

fn row(label: &str, value: &str, s: &Style) {
    println!("  {:<30}  {}", s.dim(label), value);
}

fn print_identity(id: &Identity, s: &Style) {
    header("1 · File Identity", s);
    row("Path", &id.file_path, s);
    row("Size", &format!("{} bytes", id.file_size_bytes), s);
    row("Revit version", &fmt_opt_u32(&id.revit_version), s);
    row("Build tag", &fmt_opt(&id.build_tag), s);
    row("Creator path", &fmt_opt(&id.creator_path), s);
    row("File GUID", &fmt_opt(&id.file_guid), s);
    row("Locale", &fmt_opt(&id.locale), s);
    row("Partition stream", &fmt_opt(&id.partition_stream), s);
    row(
        "Internal format counter",
        &id.format_version_counter
            .map(|v| format!("{v} (Global/PartitionTable[0..2])"))
            .unwrap_or_else(|| "—".into()),
        s,
    );
    println!();
}

fn print_history(h: &[String], s: &Style) {
    header("2 · Document Upgrade History", s);
    if h.is_empty() {
        println!("  (no version-upgrade records found)");
    } else {
        println!(
            "  {}",
            s.dim(&format!("{} entries in Global/Latest, oldest first", h.len()))
        );
        for (i, entry) in h.iter().enumerate() {
            println!("  {:>2}.  {}", i + 1, entry);
        }
    }
    println!();
}

fn print_anchors(a: &FormatAnchors, s: &Style) {
    header("3 · Format Anchors", s);
    row(
        "Global/PartitionTable size",
        &format!("{} bytes decompressed", a.partition_table_bytes),
        s,
    );
    row(
        "Invariant region size",
        &a.partition_table_invariant_bytes
            .map(|b| format!("{b} bytes (byte-for-byte identical 2016→2026)"))
            .unwrap_or_else(|| "—".into()),
        s,
    );
    row(
        "Format identifier GUID",
        &fmt_opt(&a.format_identifier_guid),
        s,
    );
    row(
        "Partition description",
        &a.partition_description
            .as_ref()
            .map(|d| format!("\"{d}\""))
            .unwrap_or_else(|| "—".into()),
        s,
    );
    row(
        "Contents wrapper magic",
        &fmt_opt(&a.contents_wrapper_magic),
        s,
    );
    println!();
}

fn print_schema(ss: &SchemaSummary, s: &Style) {
    header("4 · Schema Table (Formats/Latest)", s);
    row(
        "Total class candidates",
        &ss.total_class_candidates.to_string(),
        s,
    );
    row(
        "With serialization tag (0x8000)",
        &format!(
            "{} ({:.1}%)",
            ss.tagged_class_count,
            100.0 * ss.tagged_class_count as f64 / ss.total_class_candidates.max(1) as f64
        ),
        s,
    );
    if !ss.sample_tagged_classes.is_empty() {
        println!();
        println!("  {}", s.dim("First 10 tagged classes (sorted by tag):"));
        for t in ss.sample_tagged_classes.iter().take(10) {
            println!("    0x{:04x}   {}", t.tag, t.name);
        }
    }
    println!();
}

fn print_link(link: &LinkSummary, s: &Style) {
    header("5 · Schema → Data Linkage (Phase D proof)", s);
    row(
        "Global/Latest size",
        &format!("{} bytes decompressed", link.global_latest_bytes),
        s,
    );
    row(
        "Expected hits per tag if uniform",
        &format!("{:.1}", link.expected_if_uniform_per_tag),
        s,
    );
    row(
        "Total tag hits observed",
        &link.total_tag_hits.to_string(),
        s,
    );
    println!();
    println!(
        "  {}",
        s.dim("Top tagged classes in this file's Global/Latest (ratio = hits / uniform):")
    );
    println!(
        "    {}",
        s.dim("tag    hits     ratio  class")
    );
    for e in &link.top_classes {
        let ratio_str = if e.ratio_vs_uniform >= 50.0 {
            s.red(&format!("{:>6.0}×", e.ratio_vs_uniform))
        } else if e.ratio_vs_uniform >= 5.0 {
            s.yellow(&format!("{:>6.0}×", e.ratio_vs_uniform))
        } else {
            format!("{:>6.1}×", e.ratio_vs_uniform)
        };
        println!(
            "    0x{:04x} {:>7}   {}  {}",
            e.tag, e.hits, ratio_str, e.name
        );
    }
    println!();
}

fn print_content(c: &ContentMetadata, s: &Style) {
    header("6 · Content Metadata (Partitions/NN + Global/Latest)", s);
    row(
        "Global/Latest string records",
        &c.global_latest_string_records.to_string(),
        s,
    );
    row(
        "Partitions/NN string records",
        &c.partitions_nn_string_records.to_string(),
        s,
    );
    println!();
    println!("  {}", s.dim("Namespace breakdown:"));
    println!("    autodesk.unit.*          {:>6}", c.autodesk_units);
    println!("    autodesk.spec.*          {:>6}", c.autodesk_specs);
    println!("    autodesk.parameter.group {:>6}", c.autodesk_parameter_groups);
    println!("    OmniClass codes          {:>6}", c.omniclass_codes);
    println!("    Uniformat codes          {:>6}", c.uniformat_codes);
    println!("    Revit categories / misc  {:>6}", c.revit_categories_misc);

    if !c.example_uniformat.is_empty() {
        println!();
        println!("  {}", s.dim("Example Uniformat codes:"));
        for v in c.example_uniformat.iter().take(6) {
            println!("    · {v}");
        }
    }
    if !c.example_categories.is_empty() {
        println!();
        println!("  {}", s.dim("Example Revit categories:"));
        for v in c.example_categories.iter().take(10) {
            println!("    · {v}");
        }
    }
    println!();
}

fn print_disclosures(d: &Disclosures, s: &Style) {
    header("7 · Disclosure Scan", s);
    if d.creator_paths.is_empty()
        && d.autodesk_internal_paths.is_empty()
        && d.embedded_personal_names.is_empty()
        && d.build_server_paths.is_empty()
    {
        println!(
            "  {} {}",
            s.green("✓"),
            "No creator-path or Autodesk-internal paths detected in BasicFileInfo."
        );
        println!();
        return;
    }
    if !d.autodesk_internal_paths.is_empty() {
        println!(
            "  {} {}",
            s.red("!"),
            s.bold("Autodesk-internal paths embedded in file:")
        );
        for p in &d.autodesk_internal_paths {
            println!("    {}", s.red(p));
        }
    }
    if !d.creator_paths.is_empty() {
        println!(
            "  {} {}",
            s.yellow("!"),
            "Creator-system paths (may leak user identity):"
        );
        for p in &d.creator_paths {
            println!("    {}", s.yellow(p));
        }
    }
    if !d.embedded_personal_names.is_empty() {
        println!(
            "  {} {}",
            s.yellow("!"),
            "Usernames extracted from embedded paths:"
        );
        for n in &d.embedded_personal_names {
            println!("    {}", s.yellow(n));
        }
    }
    println!();
    println!(
        "  {}",
        s.dim("Downstream tools should redact these before forwarding.")
    );
    println!();
}

fn fmt_opt(o: &Option<String>) -> String {
    o.clone().unwrap_or_else(|| "—".into())
}
fn fmt_opt_u32(o: &Option<u32>) -> String {
    o.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

/// Replace personally-identifying information with `<redacted>` markers so
/// the report can be shared or screenshotted without exposing user identity,
/// Autodesk employee names, or customer filesystem layout. Shape is
/// preserved (same hex / path structure) so the claims remain verifiable.
fn redact_report(r: &mut Report) {
    r.identity.file_path = redact_sensitive(&r.identity.file_path);
    r.identity.creator_path = r.identity.creator_path.as_ref().map(|p| redact_sensitive(p));
    r.identity.file_guid = r
        .identity
        .file_guid
        .as_ref()
        .map(|g| format!("{}-<redacted>", &g[..8.min(g.len())]));

    // Scrub leaked paths out of content-metadata examples. These come from
    // Partitions/NN where Autodesk embedded user / employee paths verbatim
    // while authoring the shipped reference family.
    r.content_metadata.example_uniformat = r
        .content_metadata
        .example_uniformat
        .iter()
        .map(|s| redact_sensitive(s))
        .collect();
    r.content_metadata.example_categories = r
        .content_metadata
        .example_categories
        .iter()
        .map(|s| redact_sensitive(s))
        .collect();

    // Disclosures: keep category counts, strip actual values so repeated
    // rendering cannot leak them.
    let n_creator = r.disclosures.creator_paths.len();
    let n_autodesk = r.disclosures.autodesk_internal_paths.len();
    let n_names = r.disclosures.embedded_personal_names.len();
    r.disclosures.creator_paths = (0..n_creator)
        .map(|_| "<redacted path>".to_string())
        .collect();
    r.disclosures.autodesk_internal_paths = (0..n_autodesk)
        .map(|_| "<redacted autodesk path>".to_string())
        .collect();
    r.disclosures.embedded_personal_names = (0..n_names)
        .map(|_| "<redacted>".to_string())
        .collect();
}

// Redaction helpers moved to `rvt::redact` for reuse by other CLIs.
// (rvt-info, rvt-history, and rvt-analyze all share them now.)
use rvt::redact::redact_sensitive;
