//! RE-19 honesty locks — Door/Window discriminator + schema-field Wall negatives.
//!
//! Corpus-backed. Skips cleanly without `RVT_PROJECT_CORPUS_DIR`.
//! See `reports/element-framing/RE-19-door-window-wall-negative.md`.

use rvt::rect_opening_index::ArcWallRectOpeningIndex;
use rvt::{RevitFile, compression, formats, streams, walker};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn project_dir() -> Option<PathBuf> {
    std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from)
}

fn inflate_partition(rf: &mut RevitFile, name: &str) -> Option<Vec<u8>> {
    let raw = rf.read_stream(name).ok()?;
    Some(
        compression::inflate_all_chunks(&raw)
            .into_iter()
            .flatten()
            .collect(),
    )
}

fn schema_names(rf: &mut RevitFile) -> Vec<String> {
    let formats_raw = rf
        .read_stream(streams::FORMATS_LATEST)
        .expect("Formats/Latest");
    let formats_d = compression::inflate_at(&formats_raw, 0).expect("inflate");
    let schema = formats::parse_schema(&formats_d).expect("schema");
    schema.classes.into_iter().map(|c| c.name).collect()
}

#[test]
fn re19_schema_lacks_literal_wall_door_window() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    for file in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        let path = project_dir.join(file);
        if !path.exists() {
            eprintln!("skipping: {} missing", path.display());
            continue;
        }
        let mut rf = RevitFile::open(&path).unwrap_or_else(|e| panic!("open {file}: {e}"));
        let names = schema_names(&mut rf);
        for forbidden in ["Wall", "Door", "Window", "FamilyInstance", "BasicWall"] {
            assert!(
                !names.iter().any(|n| n == forbidden),
                "{file}: schema must not declare literal `{forbidden}` (RE-18/RE-19)"
            );
        }
        assert!(
            names.iter().any(|n| n == "ArcWall"),
            "{file}: expected ArcWall concrete tag class"
        );
        assert!(
            names.iter().any(|n| n == "VWall"),
            "{file}: expected VWall concrete tag class"
        );
    }
}

#[test]
fn re19_opening_index_has_no_bimodal_discriminator_column() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    let mut rf = RevitFile::open(&path).expect("open");
    let part = inflate_partition(&mut rf, "Partitions/46").expect("Partitions/46");
    let offs = ArcWallRectOpeningIndex::find_all_for_revit_version(2024, &part);
    assert!(
        offs.len() >= 1000,
        "expected ≥1000 opening-index rows, got {}",
        offs.len()
    );

    // Mid-body columns between related_id_a (+0x14) and related_id_b (+0x36)
    // must not look like a Door/Window enum (two dominant values each ≥20%).
    let sample = offs.len().min(3000);
    for off in (0x1c..=0x30).step_by(4) {
        let mut hist: BTreeMap<u32, usize> = BTreeMap::new();
        for &row in offs.iter().take(sample) {
            let v = u32::from_le_bytes([
                part[row + off],
                part[row + off + 1],
                part[row + off + 2],
                part[row + off + 3],
            ]);
            *hist.entry(v).or_insert(0) += 1;
        }
        let mut ranked: Vec<_> = hist.into_iter().collect();
        ranked.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        if ranked.len() >= 2 {
            let total = sample as f64;
            let r0 = ranked[0].1 as f64 / total;
            let r1 = ranked[1].1 as f64 / total;
            assert!(
                !(r0 >= 0.20 && r1 >= 0.20 && r0 + r1 >= 0.85 && ranked.len() <= 4),
                "RE-19: opening-index +0x{off:02x} looks bimodal ({:?}) — re-evaluate Door/Window discriminator before inventing types",
                &ranked[..ranked.len().min(4)]
            );
        }
    }
}

#[test]
fn re19_2024_arcwall_tag_lacks_2023_family_marker() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    let mut rf = RevitFile::open(&path).expect("open");
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST).expect("formats");
    let formats_d = compression::inflate_at(&formats_raw, 0).expect("inflate");
    let schema = formats::parse_schema(&formats_d).expect("schema");
    let tag = schema
        .classes
        .iter()
        .find(|c| c.name == "ArcWall")
        .and_then(|c| c.tag)
        .expect("ArcWall tag");
    assert_eq!(tag, 0x019c, "2024 ArcWall tag drift");

    let part = inflate_partition(&mut rf, "Partitions/46").expect("Partitions/46");
    let mut filtered = 0usize;
    let mut family_at_4 = 0usize;
    let mut variant_07fa = 0usize;
    for i in 0..part.len().saturating_sub(0x12) {
        let t = u16::from_le_bytes([part[i], part[i + 1]]);
        if t != tag || part[i + 2] != 0 || part[i + 3] != 0 {
            continue;
        }
        filtered += 1;
        let marker = u32::from_le_bytes([part[i + 4], part[i + 5], part[i + 6], part[i + 7]]);
        if marker == rvt::arc_wall_record::SCHEMA_FAMILY_MARKER {
            family_at_4 += 1;
        }
        let variant = u16::from_le_bytes([part[i + 0x10], part[i + 0x11]]);
        if variant == rvt::arc_wall_record::ARC_WALL_VARIANT_STANDARD {
            variant_07fa += 1;
        }
    }
    assert!(
        filtered >= 100,
        "expected abundant 0x019c filtered hits, got {filtered}"
    );
    assert_eq!(
        family_at_4, 0,
        "RE-19/#23: 2024 ArcWall must not carry 2023 SCHEMA_FAMILY_MARKER at +4 (got {family_at_4})"
    );
    assert_eq!(
        variant_07fa, 0,
        "RE-19/#23: 2024 ArcWall must not carry 2023 variant 0x07fa (got {variant_07fa})"
    );

    // Version gate: production must not emit ArcWall on this 2024 file.
    let limits = walker::WalkerLimits {
        max_candidates: 2_000,
        ..walker::WalkerLimits::default()
    };
    let decoded: Vec<_> =
        walker::iter_elements_with_limits(&mut rf, walker::PRODUCTION_ELEMENT_MIN_SCORE, limits)
            .expect("iter")
            .collect();
    assert_eq!(
        decoded.iter().filter(|e| e.class == "ArcWall").count(),
        0,
        "RE-19: 2023 ArcWall decoder must stay version-gated off on 2024"
    );
}
