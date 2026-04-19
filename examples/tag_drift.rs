//! Build a per-class tag-drift table across every RVT release in the corpus.
//!
//! For each release, parses Formats/Latest and extracts every class whose
//! post-name u16 has the 0x8000 flag set (meaning it's a class definition,
//! not a reference). Then pivots into a table indexed by class name with
//! one column per release.
//!
//! Output formats: human-readable text table + CSV for reuse in other tools.

use rvt::{compression, streams::FORMATS_LATEST, RevitFile};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn looks_like_class_name(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes[0].is_ascii_uppercase()
        && bytes[1..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn extract_tagged_classes(bytes: &[u8]) -> Vec<(String, u16)> {
    let scan_limit = (64 * 1024).min(bytes.len());
    let data = &bytes[..scan_limit];
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() {
        let len = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if !(3..=60).contains(&len) || i + 2 + len + 2 > data.len() {
            i += 1;
            continue;
        }
        let name = &data[i + 2..i + 2 + len];
        if !looks_like_class_name(name) {
            i += 1;
            continue;
        }
        let after = i + 2 + len;
        let u16_after = u16::from_le_bytes([data[after], data[after + 1]]);
        if u16_after & 0x8000 != 0 {
            let tag = u16_after & 0x7fff;
            let name = std::str::from_utf8(name).unwrap().to_string();
            out.push((name, tag));
        }
        i += 2 + len;
    }
    out.sort_by_key(|c| c.1);
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

fn main() -> anyhow::Result<()> {
    let sample_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/_phiag/examples/Autodesk".to_string());
    let out_csv = std::env::args().nth(2);

    let mut per_release: Vec<(String, Vec<(String, u16)>)> = Vec::new();
    for v in 2016..=2026 {
        for filename in [
            format!("racbasicsamplefamily-{v}.rfa"),
            format!("rac_basic_sample_family-{v}.rfa"),
        ] {
            let path = PathBuf::from(&sample_dir).join(&filename);
            if !path.exists() {
                continue;
            }
            let mut rf = RevitFile::open(&path)?;
            let raw = rf.read_stream(FORMATS_LATEST)?;
            let decomp = compression::inflate_at(&raw, 0)?;
            let classes = extract_tagged_classes(&decomp);
            per_release.push((v.to_string(), classes));
            break;
        }
    }

    // Pivot: class_name -> map<release, tag>
    let mut grid: BTreeMap<String, BTreeMap<String, u16>> = BTreeMap::new();
    for (release, classes) in &per_release {
        for (name, tag) in classes {
            grid.entry(name.clone())
                .or_default()
                .insert(release.clone(), *tag);
        }
    }
    let releases: Vec<String> = per_release.iter().map(|(r, _)| r.clone()).collect();

    // Summary stats
    let total_classes = grid.len();
    let stable: Vec<_> = grid
        .iter()
        .filter(|(_, tags)| {
            tags.len() == releases.len()
                && tags.values().collect::<std::collections::HashSet<_>>().len() == 1
        })
        .map(|(name, _)| name.clone())
        .collect();
    let shifting: Vec<_> = grid
        .iter()
        .filter(|(_, tags)| {
            tags.values().collect::<std::collections::HashSet<_>>().len() > 1
        })
        .map(|(name, tags)| {
            let distinct = tags.values().collect::<std::collections::HashSet<_>>().len();
            (name.clone(), distinct)
        })
        .collect();

    println!("Releases analysed: {}", releases.len());
    println!("Distinct classes across all releases: {total_classes}");
    println!(
        "Tag-stable (same tag every release + present in all): {} ({:.1}%)",
        stable.len(),
        100.0 * stable.len() as f64 / total_classes as f64
    );
    println!(
        "Tag-shifting (at least two distinct tag values across releases): {}",
        shifting.len()
    );

    println!("\n== Tag-stable sample (first 10) ==");
    for name in stable.iter().take(10) {
        let tag = grid[name].values().next().unwrap();
        println!("  0x{:04x}  {}", tag, name);
    }

    println!("\n== Tag-shifting sample (first 10) ==");
    for (name, _) in shifting.iter().take(10) {
        let values: Vec<String> = releases
            .iter()
            .map(|r| {
                grid[name]
                    .get(r)
                    .map(|t| format!("0x{t:04x}"))
                    .unwrap_or_else(|| "-".into())
            })
            .collect();
        println!("  {name:<40}  {}", values.join(" "));
    }

    // Classes that disappear / appear
    let appeared: Vec<_> = grid
        .iter()
        .filter(|(_, tags)| !tags.contains_key(&releases[0]) && tags.contains_key(releases.last().unwrap()))
        .map(|(n, _)| n.clone())
        .collect();
    let disappeared: Vec<_> = grid
        .iter()
        .filter(|(_, tags)| tags.contains_key(&releases[0]) && !tags.contains_key(releases.last().unwrap()))
        .map(|(n, _)| n.clone())
        .collect();
    println!(
        "\nClasses introduced after {}: {} (e.g. {})",
        releases[0],
        appeared.len(),
        appeared.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
    );
    println!(
        "Classes removed by {}: {} (e.g. {})",
        releases.last().unwrap(),
        disappeared.len(),
        disappeared.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
    );

    // CSV output
    if let Some(csv_path) = out_csv {
        use std::io::Write;
        let mut f = std::fs::File::create(&csv_path)?;
        write!(f, "class_name")?;
        for r in &releases {
            write!(f, ",{r}")?;
        }
        writeln!(f)?;
        for (name, tags) in &grid {
            write!(f, "{name}")?;
            for r in &releases {
                match tags.get(r) {
                    Some(t) => write!(f, ",0x{t:04x}")?,
                    None => write!(f, ",")?,
                }
            }
            writeln!(f)?;
        }
        println!("\nCSV written to {csv_path}");
    } else {
        println!("\n(pass a second arg to write the full drift table as CSV)");
    }

    Ok(())
}
