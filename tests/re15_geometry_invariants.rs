//! RE-15 geometry-probe corpus invariants.
use rvt::object_graph;
use rvt::partition_name_candidates::{NameBucket, collect_name_candidates};
use rvt::rect_opening_index::{
    ARC_WALL_RECT_OPENING_TAG_2024, ArcWallRectOpeningIndex, OPENING_INDEX_FAMILY_MARKER,
    OPENING_INDEX_STRIDE,
};
use rvt::{RevitFile, compression, formats, streams};
use std::path::PathBuf;

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
}

fn skip_if_missing(path: &std::path::Path) -> bool {
    if path.exists() {
        return false;
    }
    eprintln!("skipping: {} not present", path.display());
    true
}

#[test]
fn einhoven_arcwall_tag_and_filtered_hits() {
    let path = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if skip_if_missing(&path) {
        return;
    }
    let mut rf = RevitFile::open(&path).unwrap();
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
    let formats_d = compression::inflate_at(&formats_raw, 0).unwrap();
    let schema = formats::parse_schema(&formats_d).unwrap();
    let tag = schema
        .classes
        .iter()
        .find(|c| c.name == "ArcWall")
        .and_then(|c| c.tag)
        .expect("ArcWall tag");
    assert_eq!(tag, 0x0191);
    let raw = rf.read_stream("Partitions/5").unwrap();
    let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
        .into_iter()
        .flatten()
        .collect();
    let mut n = 0usize;
    for i in 0..concat.len().saturating_sub(3) {
        let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if v == tag && concat[i + 2] == 0 && concat[i + 3] == 0 {
            n += 1;
        }
    }
    assert!(n >= 20, "expected >=20 ArcWall hits, got {n}");
}

#[test]
fn core2024_arcwall_tag_drifted_from_2023() {
    let path = project_dir().join("2024_Core_Interior.rvt");
    if skip_if_missing(&path) {
        return;
    }
    let mut rf = RevitFile::open(&path).unwrap();
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
    let formats_d = compression::inflate_at(&formats_raw, 0).unwrap();
    let schema = formats::parse_schema(&formats_d).unwrap();
    let arcwall = schema
        .classes
        .iter()
        .find(|c| c.name == "ArcWall")
        .and_then(|c| c.tag);
    let opening = schema
        .classes
        .iter()
        .find(|c| c.name == "ArcWallRectOpening")
        .and_then(|c| c.tag);
    assert_eq!(arcwall, Some(0x019c));
    assert_eq!(opening, Some(ARC_WALL_RECT_OPENING_TAG_2024));
    assert_ne!(arcwall, opening);
}

#[test]
fn core2024_opening_index_stride60_decodes() {
    let path = project_dir().join("2024_Core_Interior.rvt");
    if skip_if_missing(&path) {
        return;
    }
    let mut rf = RevitFile::open(&path).unwrap();
    let version = rf.basic_file_info().unwrap().version;
    assert_eq!(version, 2024);
    let raw = rf.read_stream("Partitions/46").unwrap();
    let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
        .into_iter()
        .flatten()
        .collect();
    let offsets = ArcWallRectOpeningIndex::find_all_for_revit_version(version, &concat);
    assert!(offsets.len() >= 1000, "got {}", offsets.len());
    let mut sequential_runs = 0usize;
    let mut prev_index = None;
    for &off in offsets.iter().take(500) {
        let rec = ArcWallRectOpeningIndex::decode(&concat, off).unwrap();
        assert_eq!(rec.tag, ARC_WALL_RECT_OPENING_TAG_2024);
        assert_eq!(rec.family_marker, OPENING_INDEX_FAMILY_MARKER);
        if let Some(prev) = prev_index {
            if rec.index == prev + 1 {
                sequential_runs += 1;
            }
        }
        prev_index = Some(rec.index);
        if off + OPENING_INDEX_STRIDE + 4 <= concat.len() {
            let next_tag = u16::from_le_bytes([
                concat[off + OPENING_INDEX_STRIDE],
                concat[off + OPENING_INDEX_STRIDE + 1],
            ]);
            if next_tag == ARC_WALL_RECT_OPENING_TAG_2024 {
                assert_eq!(concat[off + OPENING_INDEX_STRIDE + 2], 0);
            }
        }
    }
    assert!(sequential_runs >= 100, "got {sequential_runs}");
    assert!(ArcWallRectOpeningIndex::find_all_for_revit_version(2023, &concat).is_empty());
}

#[test]
fn partition_name_candidates_include_known_materials_and_levels() {
    for file in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let path = project_dir().join(file);
        if skip_if_missing(&path) {
            continue;
        }
        let mut rf = RevitFile::open(&path).unwrap();
        let records = object_graph::string_records_from_partitions(&mut rf).unwrap();
        let values: Vec<&str> = records.iter().map(|r| r.value.as_str()).collect();
        let candidates = collect_name_candidates(values.iter().copied());
        let materials: Vec<_> = candidates
            .iter()
            .filter(|(b, _)| *b == NameBucket::MaterialLike)
            .map(|(_, s)| s.as_str())
            .collect();
        let levels: Vec<_> = candidates
            .iter()
            .filter(|(b, _)| *b == NameBucket::LevelLike)
            .map(|(_, s)| s.as_str())
            .collect();
        assert!(
            materials.iter().any(|s| s.contains("Concrete")),
            "{file}: {materials:?}"
        );
        assert!(levels.contains(&"Level 1"), "{file}: {levels:?}");
        assert!(materials.len() >= 3, "{file}: {}", materials.len());
    }
}

#[test]
fn einhoven_has_no_filtered_arcwall_rect_opening_hits() {
    let path = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if skip_if_missing(&path) {
        return;
    }
    let mut rf = RevitFile::open(&path).unwrap();
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
    let formats_d = compression::inflate_at(&formats_raw, 0).unwrap();
    let schema = formats::parse_schema(&formats_d).unwrap();
    let tag = schema
        .classes
        .iter()
        .find(|c| c.name == "ArcWallRectOpening")
        .and_then(|c| c.tag)
        .expect("tag");
    assert_eq!(tag, 0x019c);
    let raw = rf.read_stream("Partitions/5").unwrap();
    let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
        .into_iter()
        .flatten()
        .collect();
    let mut n = 0usize;
    for i in 0..concat.len().saturating_sub(3) {
        let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if v == tag && concat[i + 2] == 0 && concat[i + 3] == 0 {
            n += 1;
        }
    }
    assert_eq!(n, 0, "unexpected hits {n}");
}
