//! RE-376: the 2024 rows RE-15 read as an `ArcWallRectOpening` index name
//! system elements, not wall openings.
//!
//! FACT: every 60-byte row [`ArcWallRectOpeningIndex`] finds names two
//! elements whose native records are system elements
//! (`ConceptualConstructionType`, `ConceptualSurfaceType`, `GStyleElem`,
//! `FillPatternElem`, `LinePatternElem`, `MaterialElem`); no row names an
//! element a wall's record lists as cutting it, and a file whose walls hold
//! rectangular openings (`SWallRectOpening`) can give no row at all.
//!
//! Prints the rows' count, the classes of the ids they name, the native
//! records of each rectangular-opening class, and the elements each kind of
//! wall cut step names with how many of them a row names:
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re376_opening_index_ids -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::research::native_document;
use rvt::research::rect_opening_index::ArcWallRectOpeningIndex;

fn ids_named(pairs: &BTreeMap<(u32, u32), usize>, id: u64) -> bool {
    u32::try_from(id).is_ok_and(|id| pairs.keys().any(|(a, b)| *a == id || *b == id))
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re376_opening_index_ids FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    if !ArcWallRectOpeningIndex::supports_revit_version(version) {
        anyhow::bail!("Revit {version}: no opening index");
    }
    let mut rows = Vec::new();
    for stream in rf.partition_stream_names() {
        let buf = rf.inflated_partition(&stream)?;
        let bytes = buf.bytes();
        for off in ArcWallRectOpeningIndex::find_all_for_revit_version(version, bytes) {
            if let Ok(rec) = ArcWallRectOpeningIndex::decode(bytes, off) {
                rows.push((stream.clone(), off, rec));
            }
        }
    }
    let ids: BTreeSet<u64> = rows
        .iter()
        .flat_map(|(_, _, r)| [u64::from(r.related_id_a), u64::from(r.related_id_b)])
        .collect();
    let mut class_of: BTreeMap<u64, String> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: ids.clone(),
        graph_classes: Some(BTreeSet::new()),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(&mut rf, &options, |record| {
        class_of.insert(
            record.identity.element_id,
            record.class_name.unwrap_or_else(|| "?".into()),
        );
        Ok(())
    })?;
    let class = |id: u32| {
        class_of
            .get(&u64::from(id))
            .cloned()
            .unwrap_or_else(|| "<no record>".into())
    };
    let mut pairs: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut per_pair: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for (_, _, r) in &rows {
        *pairs
            .entry((class(r.related_id_a), class(r.related_id_b)))
            .or_default() += 1;
        *per_pair
            .entry((r.related_id_a, r.related_id_b))
            .or_default() += 1;
    }
    println!(
        "rows {}  distinct ids {}  distinct pairs {}",
        rows.len(),
        ids.len(),
        per_pair.len()
    );
    println!(
        "streams {:?}",
        rows.iter()
            .map(|(s, _, _)| s.as_str())
            .collect::<BTreeSet<_>>()
    );
    println!("class(a)\tclass(b)\trows");
    for ((a, b), n) in &pairs {
        println!("{a}\t{b}\t{n}");
    }
    println!("-- most repeated pairs");
    let mut top: Vec<_> = per_pair.iter().collect();
    top.sort_by(|x, y| y.1.cmp(x.1));
    for ((a, b), n) in top.iter().take(15) {
        println!("{a} {} | {b} {} | {n}", class(*a), class(*b));
    }
    let mut opening_records: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let classes: BTreeSet<String> = ["ArcWallRectOpening", "VWallRectOpening", "SWallRectOpening"]
        .into_iter()
        .map(String::from)
        .collect();
    let options = native_document::Options {
        selected_classes: classes,
        graph_classes: Some(BTreeSet::new()),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(&mut rf, &options, |record| {
        opening_records
            .entry(record.class_name.unwrap_or_else(|| "?".into()))
            .or_default()
            .push(record.identity.element_id);
        Ok(())
    })?;
    for class in ["ArcWallRectOpening", "VWallRectOpening", "SWallRectOpening"] {
        opening_records.entry(class.to_string()).or_default();
    }
    for (class, ids) in &opening_records {
        let named = ids.iter().filter(|id| ids_named(&per_pair, **id)).count();
        println!(
            "native {class}: {} records, {named} named by an index row; first {:?}",
            ids.len(),
            &ids[..ids.len().min(8)]
        );
    }
    let mut walls = BTreeSet::new();
    let options = native_document::Options {
        selected_classes: BTreeSet::from(["SWall".to_string(), "ArcWall".to_string()]),
        graph_classes: Some(BTreeSet::new()),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(&mut rf, &options, |record| {
        walls.insert(record.identity.element_id);
        Ok(())
    })?;
    let steps = rvt::research::native_wall_cuts::wall_cut_steps(&mut rf, &walls)?;
    let mut cutters: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
    for list in steps.values() {
        for step in list {
            cutters
                .entry(format!("{:?}", step.kind))
                .or_default()
                .insert(step.cutter);
        }
    }
    let all_cutters: BTreeSet<u64> = cutters.values().flatten().copied().collect();
    let mut cutter_class: BTreeMap<u64, String> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: all_cutters,
        graph_classes: Some(BTreeSet::new()),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(&mut rf, &options, |record| {
        cutter_class.insert(
            record.identity.element_id,
            record.class_name.unwrap_or_else(|| "?".into()),
        );
        Ok(())
    })?;
    println!(
        "walls {}  walls with cut steps {}",
        walls.len(),
        steps.values().filter(|s| !s.is_empty()).count()
    );
    for (kind, ids) in &cutters {
        let mut by_class: BTreeMap<&str, usize> = BTreeMap::new();
        for id in ids {
            *by_class
                .entry(cutter_class.get(id).map_or("<no record>", String::as_str))
                .or_default() += 1;
        }
        let named = ids.iter().filter(|id| ids_named(&per_pair, **id)).count();
        println!(
            "cutters {kind}: {} ids, {named} named by an index row, classes {by_class:?}",
            ids.len()
        );
    }
    println!("-- first 20 rows (stream, offset, index, a, b)");
    for (s, off, r) in rows.iter().take(20) {
        println!(
            "{s} {off:#x} {} {} {}",
            r.index, r.related_id_a, r.related_id_b
        );
    }
    Ok(())
}
