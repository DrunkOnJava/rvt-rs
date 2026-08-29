//! Follow-up hard-gap probes: VWall instances, string↔id joins, minority columns.
use rvt::arc_wall_record::SCHEMA_FAMILY_MARKER;
use rvt::rect_opening_index::{
    ArcWallRectOpeningIndex, OPENING_INDEX_FAMILY_MARKER, OPENING_INDEX_STRIDE,
};
use rvt::{RevitFile, compression, elem_table, formats, streams};
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn read_u16(buf: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*buf.get(off)?, *buf.get(off + 1)?]))
}
fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *buf.get(off)?,
        *buf.get(off + 1)?,
        *buf.get(off + 2)?,
        *buf.get(off + 3)?,
    ]))
}

fn utf16_strings(buf: &[u8], min_chars: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 2 * min_chars <= buf.len() {
        let mut j = i;
        let mut s = String::new();
        while j + 1 < buf.len() {
            let lo = buf[j];
            let hi = buf[j + 1];
            if hi != 0 || !(0x20..=0x7e).contains(&lo) {
                break;
            }
            s.push(lo as char);
            j += 2;
        }
        if s.len() >= min_chars {
            out.push((i, s));
            i = j;
        } else {
            i += 2;
        }
    }
    out
}

fn main() {
    let dir =
        std::env::var("RVT_PROJECT_CORPUS_DIR").unwrap_or_else(|_| "_project_corpus/Revit".into());
    let mut rf = RevitFile::open(format!("{dir}/2024_Core_Interior.rvt")).expect("open");
    let part: Vec<u8> = compression::inflate_all_chunks(&rf.read_stream("Partitions/46").unwrap())
        .into_iter()
        .flatten()
        .collect();

    // Schema tags for VWall / ArcWall
    let formats_d =
        compression::inflate_at(&rf.read_stream(streams::FORMATS_LATEST).unwrap(), 0).unwrap();
    let schema = formats::parse_schema(&formats_d).unwrap();
    let vwall_tag = schema
        .classes
        .iter()
        .find(|c| c.name == "VWall")
        .and_then(|c| c.tag);
    let arc_tag = schema
        .classes
        .iter()
        .find(|c| c.name == "ArcWall")
        .and_then(|c| c.tag);
    println!("VWall tag={vwall_tag:?} ArcWall tag={arc_tag:?}");

    for (name, tag) in [("VWall", vwall_tag), ("ArcWall", arc_tag)] {
        let Some(tag) = tag else { continue };
        let mut hits = Vec::new();
        for i in 0..part.len().saturating_sub(3) {
            if read_u16(&part, i) == Some(tag)
                && part.get(i + 2) == Some(&0)
                && part.get(i + 3) == Some(&0)
            {
                hits.push(i);
            }
        }
        let mut marker4 = 0usize;
        let mut marker_alt = 0usize;
        let mut opening_marker = 0usize;
        for &off in hits.iter().take(2000) {
            if read_u32(&part, off + 4) == Some(SCHEMA_FAMILY_MARKER) {
                marker4 += 1;
            }
            // search +0..+64 for either family marker
            for rel in (0..64).step_by(4) {
                let v = read_u32(&part, off + rel);
                if v == Some(SCHEMA_FAMILY_MARKER) {
                    marker_alt += 1;
                    break;
                }
                if v == Some(OPENING_INDEX_FAMILY_MARKER) {
                    opening_marker += 1;
                    break;
                }
            }
        }
        println!(
            "{name} 0x{tag:04x}: filtered={} SCHEMA_MARKER@+4={marker4} any SCHEMA in +0..+64={marker_alt} OPENING_MARKER in +0..+64={opening_marker}",
            hits.len()
        );
    }

    // Minority column +0x30: split openings and see if related ids differ in pattern
    let offs = ArcWallRectOpeningIndex::find_all_for_revit_version(2024, &part);
    let mut by_col30: BTreeMap<u32, Vec<(u32, u32, u32)>> = BTreeMap::new();
    for &off in &offs {
        let Ok(rec) = ArcWallRectOpeningIndex::decode(&part, off) else {
            continue;
        };
        let col = read_u32(&part, off + 0x30).unwrap_or(0);
        by_col30
            .entry(col)
            .or_default()
            .push((rec.index, rec.related_id_a, rec.related_id_b));
    }
    println!("\n+0x30 column splits:");
    for (col, rows) in &by_col30 {
        println!("  0x{col:08x}: {} rows", rows.len());
        for (i, a, b) in rows.iter().take(5) {
            println!("    index={i} a={a} b={b}");
        }
    }

    // String→id join: for door/window family-like strings, collect u32s in ±64 bytes
    // and see intersection with opening related_id sets.
    let strings = utf16_strings(&part, 4);
    let mut related_a = BTreeSet::new();
    let mut related_b = BTreeSet::new();
    let mut related_all = BTreeSet::new();
    for &off in &offs {
        let Ok(rec) = ArcWallRectOpeningIndex::decode(&part, off) else {
            continue;
        };
        related_a.insert(rec.related_id_a);
        related_b.insert(rec.related_id_b);
        related_all.insert(rec.related_id_a);
        related_all.insert(rec.related_id_b);
    }
    println!(
        "\nrelated id sets: a={} b={} union={}",
        related_a.len(),
        related_b.len(),
        related_all.len()
    );

    let mut door_string_ids = BTreeSet::new();
    let mut window_string_ids = BTreeSet::new();
    let mut door_hits = 0usize;
    let mut window_hits = 0usize;
    for (soff, s) in &strings {
        let lower = s.to_ascii_lowercase();
        let is_door =
            (lower.contains("door") && !lower.contains("outdoor") && !lower.contains("indoor"))
                && !lower.starts_with("ifc");
        let is_window = lower.contains("window") && !lower.starts_with("ifc");
        if !is_door && !is_window {
            continue;
        }
        if is_door {
            door_hits += 1;
        }
        if is_window {
            window_hits += 1;
        }
        let start = soff.saturating_sub(64);
        let end = (*soff + s.len() * 2 + 64).min(part.len());
        for i in (start..end.saturating_sub(3)).step_by(1) {
            if let Some(id) = read_u32(&part, i) {
                if related_all.contains(&id) {
                    if is_door {
                        door_string_ids.insert(id);
                    }
                    if is_window {
                        window_string_ids.insert(id);
                    }
                }
            }
        }
    }
    println!("door-like non-Ifc strings={door_hits} window-like non-Ifc={window_hits}");
    println!(
        "opening-related ids within ±64B of door strings: {} {:?}",
        door_string_ids.len(),
        door_string_ids.iter().take(12).collect::<Vec<_>>()
    );
    println!(
        "opening-related ids within ±64B of window strings: {} {:?}",
        window_string_ids.len(),
        window_string_ids.iter().take(12).collect::<Vec<_>>()
    );
    let both: BTreeSet<_> = door_string_ids
        .intersection(&window_string_ids)
        .copied()
        .collect();
    let only_d: BTreeSet<_> = door_string_ids
        .difference(&window_string_ids)
        .copied()
        .collect();
    let only_w: BTreeSet<_> = window_string_ids
        .difference(&door_string_ids)
        .copied()
        .collect();
    println!(
        "split: door_only={} window_only={} both={}",
        only_d.len(),
        only_w.len(),
        both.len()
    );

    // Expand radius to ±2048 for any hit at all
    let mut door_far = BTreeSet::new();
    let mut window_far = BTreeSet::new();
    for (soff, s) in &strings {
        let lower = s.to_ascii_lowercase();
        let is_door =
            lower.contains("door") && !lower.contains("outdoor") && !lower.starts_with("ifc");
        let is_window = lower.contains("window") && !lower.starts_with("ifc");
        if !is_door && !is_window {
            continue;
        }
        let start = soff.saturating_sub(2048);
        let end = (*soff + s.len() * 2 + 2048).min(part.len());
        for i in (start..end.saturating_sub(3)).step_by(4) {
            if let Some(id) = read_u32(&part, i) {
                if related_all.contains(&id) {
                    if is_door {
                        door_far.insert(id);
                    }
                    if is_window {
                        window_far.insert(id);
                    }
                }
            }
        }
    }
    let both_f: BTreeSet<_> = door_far.intersection(&window_far).copied().collect();
    println!(
        "±2048B: door_ids={} window_ids={} both={}",
        door_far.len(),
        window_far.len(),
        both_f.len()
    );

    // ElemTable raw byte variance among related_a vs related_b records
    let records = elem_table::parse_records(&mut rf).unwrap();
    let by_id: HashMap<u32, &elem_table::ElemRecord> =
        records.iter().map(|r| (r.id_primary, r)).collect();
    println!("\nElemTable raw variance for related_a vs related_b:");
    for (label, set) in [("related_a", &related_a), ("related_b", &related_b)] {
        let mut lens = BTreeMap::<usize, usize>::new();
        let sample: Vec<_> = set.iter().copied().take(50).collect();
        let mut byte_uniques: Vec<BTreeSet<u8>> = Vec::new();
        for id in &sample {
            let Some(rec) = by_id.get(id) else { continue };
            *lens.entry(rec.raw.len()).or_insert(0) += 1;
            if byte_uniques.len() < rec.raw.len() {
                byte_uniques.resize(rec.raw.len(), BTreeSet::new());
            }
            for (i, b) in rec.raw.iter().enumerate() {
                byte_uniques[i].insert(*b);
            }
        }
        println!("  {label}: sample_len_hist={lens:?}");
        let variable: Vec<_> = byte_uniques
            .iter()
            .enumerate()
            .filter(|(_, s)| s.len() > 1)
            .map(|(i, s)| (i, s.len()))
            .collect();
        println!("    variable raw bytes: {variable:?}");
        // Show first raw hex samples
        for id in sample.iter().take(3) {
            if let Some(rec) = by_id.get(id) {
                let hex: String = rec
                    .raw
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("    id={id} secondary={} raw={hex}", rec.id_secondary);
            }
        }
    }

    // How many unique related pairs vs openings — multiplicity
    let mut pair_count = BTreeMap::<(u32, u32), usize>::new();
    for &off in &offs {
        let Ok(rec) = ArcWallRectOpeningIndex::decode(&part, off) else {
            continue;
        };
        *pair_count
            .entry((rec.related_id_a, rec.related_id_b))
            .or_insert(0) += 1;
    }
    let mut mult_hist = BTreeMap::<usize, usize>::new();
    for c in pair_count.values() {
        *mult_hist.entry(*c).or_insert(0) += 1;
    }
    println!("\nrelated-pair multiplicity histogram (count_of_openings → num_pairs):");
    for (k, v) in &mult_hist {
        println!("  multiplicity={k}: {v} pairs");
    }

    // Does related_a or related_b appear inside 2024 ArcWall-tag neighborhoods?
    if let Some(tag) = arc_tag {
        let mut wall_hits = Vec::new();
        for i in 0..part.len().saturating_sub(3) {
            if read_u16(&part, i) == Some(tag)
                && part.get(i + 2) == Some(&0)
                && part.get(i + 3) == Some(&0)
            {
                wall_hits.push(i);
            }
        }
        let mut a_near_wall = 0usize;
        let mut b_near_wall = 0usize;
        // Build set of u32s appearing in ±128 of wall hits (cap)
        let mut near_ids = BTreeSet::new();
        for &off in wall_hits.iter().take(500) {
            let start = off.saturating_sub(128);
            let end = (off + 256).min(part.len());
            for i in (start..end.saturating_sub(3)).step_by(4) {
                if let Some(id) = read_u32(&part, i) {
                    if related_all.contains(&id) {
                        near_ids.insert(id);
                    }
                }
            }
        }
        for id in &related_a {
            if near_ids.contains(id) {
                a_near_wall += 1;
            }
        }
        for id in &related_b {
            if near_ids.contains(id) {
                b_near_wall += 1;
            }
        }
        println!(
            "\nArcWall-tag neighborhoods (±128 of first 500 hits): related_ids present={}; a_hits={a_near_wall}/{} b_hits={b_near_wall}/{}",
            near_ids.len(),
            related_a.len(),
            related_b.len()
        );
    }

    // Opening index stride consistency: any non-60 gap that might be typed payload?
    let mut gaps = BTreeMap::<usize, usize>::new();
    for w in offs.windows(2) {
        *gaps.entry(w[1] - w[0]).or_insert(0) += 1;
    }
    println!("\nopening index inter-record gaps (top):");
    let mut g: Vec<_> = gaps.into_iter().collect();
    g.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (d, c) in g.iter().take(10) {
        println!("  gap={d}: {c}");
    }
    println!("OPENING_INDEX_STRIDE={OPENING_INDEX_STRIDE}");
}
