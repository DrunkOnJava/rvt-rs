//! `rvt-dump` writes what the parsers see: page-stripped, member-joined
//! bytes identical to `RevitFile::inflated_partition` and to the
//! `Global/ElemTable` decode. The tier-1 fixture runs everywhere; the 2024
//! magnetar project (skipped unless `RVT_PROJECT_CORPUS_DIR` holds it) has
//! the multi-page streams where the old dump dropped members.

use rvt::RevitFile;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tier1() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("tier1")
        .join("architectural-2024")
        .join("architectural-2024.rvt")
}

fn core_interior() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR")?;
    let path = PathBuf::from(dir).join("2024_Core_Interior.rvt");
    path.exists().then_some(path)
}

fn out_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvt-dump-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn dump(input: &Path, out: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_rvt-dump"))
        .arg(input)
        .arg("-o")
        .arg(out)
        .output()
        .expect("run rvt-dump");
    assert!(
        output.status.success(),
        "rvt-dump failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// Every partition's `.decomp` is byte-identical to the library's own
/// inflated view; a partition the library inflates to nothing has none.
fn assert_partitions_match_library(rf: &mut RevitFile, out: &Path) {
    let partitions = rf.partition_stream_names();
    assert!(!partitions.is_empty());
    for name in partitions {
        let inflated = rf.inflated_partition(&name).expect("inflate");
        let path = out.join(format!("{}.decomp", name.replace('/', "_")));
        if inflated.bytes().is_empty() {
            assert!(
                !path.exists(),
                "{name}: dumped bytes the library has none of"
            );
            continue;
        }
        let dumped = fs::read(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            dumped == inflated.bytes(),
            "{name}: dump is {} bytes, library view {}",
            dumped.len(),
            inflated.bytes().len()
        );
    }
}

#[test]
fn tier1_dump_matches_the_parsers() {
    let out = out_dir("tier1");
    let stdout = dump(&tier1(), &out);
    assert_eq!(
        stdout.lines().filter(|l| l.starts_with("  ")).count(),
        RevitFile::open(tier1()).expect("open").stream_names().len(),
        "one line per stream:\n{stdout}"
    );
    assert!(!stdout.contains("did not inflate"), "{stdout}");
    let mut rf = RevitFile::open(tier1()).expect("open");
    assert_partitions_match_library(&mut rf, &out);
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn multi_page_streams_are_page_stripped() {
    let Some(input) = core_interior() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt not present");
        return;
    };
    let out = out_dir("core-interior");
    let stdout = dump(&input, &out);
    assert!(!stdout.contains("did not inflate"), "{stdout}");
    assert!(stdout.contains("gzip members"), "{stdout}");
    let mut rf = RevitFile::open(&input).expect("open");
    assert_partitions_match_library(&mut rf, &out);
    let header = rvt::elem_table::parse_header(&mut rf).expect("ElemTable header");
    let elem_table = fs::read(out.join("Global_ElemTable.decomp")).expect("ElemTable dumped");
    assert_eq!(elem_table.len(), header.decompressed_bytes);
    assert_eq!(elem_table.len(), 1_057_030);
    let _ = fs::remove_dir_all(&out);
}
