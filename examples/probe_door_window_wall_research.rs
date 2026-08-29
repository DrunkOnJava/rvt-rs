//! Hard-gap research probe for #30 / #32 / #23.
//!
//! Goals (fail-closed):
//! 1. Find any reliable Door vs Window discriminator on real corpus
//!    opening / partition data — or document a crisp negative.
//! 2. Find recoverable non-ArcWall / schema-field Wall records — or
//!    document a crisp negative (incl. 2024 ArcWall envelope status).
//!
//! Usage:
//!   RVT_PROJECT_CORPUS_DIR=_project_corpus/Revit \
//!     cargo run --release --example probe_door_window_wall_research

use rvt::arc_wall_record::{
    ARC_WALL_TAG, ARC_WALL_VARIANT_COMPOUND, ARC_WALL_VARIANT_STANDARD, SCHEMA_FAMILY_MARKER,
};
use rvt::rect_opening_index::{
    ARC_WALL_RECT_OPENING_TAG_2024, ArcWallRectOpeningIndex, OPENING_INDEX_FAMILY_MARKER,
    OPENING_INDEX_STRIDE,
};
use rvt::{RevitFile, compression, elem_table, formats, streams};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

const VWALL_RECT_OPENING_TAG_2024: u16 = 0x01a8;
const ARC_WALL_TAG_2024_CANDIDATE: u16 = 0x019c;

fn project_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("RVT_PROJECT_CORPUS_DIR").unwrap_or_else(|_| "_project_corpus/Revit".into()),
    )
}

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
fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    let bytes: [u8; 8] = buf.get(off..off + 8)?.try_into().ok()?;
    let v = f64::from_le_bytes(bytes);
    v.is_finite().then_some(v)
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

fn utf16le_ascii_strings(buf: &[u8], min_chars: usize) -> Vec<(usize, String)> {
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

fn schema_wall_door_window_inventory(rf: &mut RevitFile, label: &str) {
    println!("\n=== SCHEMA inventory — {label} ===");
    let Ok(raw) = rf.read_stream(streams::FORMATS_LATEST) else {
        println!("  Formats/Latest unreadable");
        return;
    };
    let Ok(inflated) = compression::inflate_at(&raw, 0) else {
        println!("  inflate failed");
        return;
    };
    let Ok(schema) = formats::parse_schema(&inflated) else {
        println!("  schema parse failed");
        return;
    };

    let interesting = [
        "Wall",
        "ArcWall",
        "VWall",
        "Door",
        "Window",
        "Floor",
        "Level",
        "Room",
        "FamilyInstance",
        "HostObject",
        "HostObjAttr",
        "Opening",
        "WallFoundation",
        "CurtainWall",
        "StackedWall",
        "BasicWall",
    ];
    println!("  total classes: {}", schema.classes.len());
    for name in interesting {
        match schema.classes.iter().find(|c| c.name == name) {
            Some(c) => println!(
                "  {name:20} PRESENT tag={} fields={}",
                c.tag
                    .map(|t| format!("0x{t:04x}"))
                    .unwrap_or_else(|| "—".into()),
                c.fields.len()
            ),
            None => println!("  {name:20} ABSENT"),
        }
    }

    let mut wallish: Vec<_> = schema
        .classes
        .iter()
        .filter(|c| {
            let n = c.name.to_ascii_lowercase();
            n.contains("wall") || n.contains("door") || n.contains("window") || n.contains("open")
        })
        .map(|c| (c.name.clone(), c.tag))
        .collect();
    wallish.sort_by(|a, b| a.0.cmp(&b.0));
    println!("  wall/door/window/open* class names ({}):", wallish.len());
    for (name, tag) in wallish.iter().take(80) {
        println!(
            "    {name} tag={}",
            tag.map(|t| format!("0x{t:04x}"))
                .unwrap_or_else(|| "—".into())
        );
    }
    if wallish.len() > 80 {
        println!("    … {} more", wallish.len() - 80);
    }

    // Literal schema field names that look like Door/Window placement.
    let mut field_hits = BTreeMap::<String, Vec<String>>::new();
    for c in &schema.classes {
        for f in &c.fields {
            let n = f.name.to_ascii_lowercase();
            if n.contains("sill")
                || n.contains("flip_hand")
                || n.contains("fliphand")
                || n == "m_host_id"
                || n.contains("door")
                || n.contains("window")
            {
                field_hits
                    .entry(f.name.clone())
                    .or_default()
                    .push(c.name.clone());
            }
        }
    }
    println!("  schema fields with door/window/host/sill/flip hints:");
    if field_hits.is_empty() {
        println!("    (none)");
    } else {
        for (field, classes) in field_hits.iter().take(40) {
            let sample: Vec<_> = classes.iter().take(6).cloned().collect();
            println!(
                "    {field} on {} class(es) e.g. {:?}",
                classes.len(),
                sample
            );
        }
    }
}

fn opening_index_byte_variance(buf: &[u8], label: &str) {
    println!("\n=== Opening-index byte variance — {label} ===");
    let offs = ArcWallRectOpeningIndex::find_all_for_revit_version(2024, buf);
    println!("  decodable index rows: {}", offs.len());
    if offs.is_empty() {
        return;
    }

    // Per-byte unique-value count across first N records (discriminator hunt).
    let n = offs.len().min(3000);
    let mut uniques: Vec<BTreeSet<u8>> = vec![BTreeSet::new(); OPENING_INDEX_STRIDE];
    let mut related_pairs = BTreeMap::<(u32, u32), usize>::new();
    let mut delta_ab = BTreeMap::<i64, usize>::new();
    let mut even_even = 0usize;
    let mut const_cols = 0usize;

    for &off in offs.iter().take(n) {
        let Ok(rec) = ArcWallRectOpeningIndex::decode(buf, off) else {
            continue;
        };
        *related_pairs
            .entry((rec.related_id_a, rec.related_id_b))
            .or_insert(0) += 1;
        *delta_ab
            .entry(i64::from(rec.related_id_b) - i64::from(rec.related_id_a))
            .or_insert(0) += 1;
        if rec.related_id_a % 2 == 0 && rec.related_id_b % 2 == 0 {
            even_even += 1;
        }
        for (i, slot) in uniques.iter_mut().enumerate() {
            if let Some(&b) = buf.get(off + i) {
                slot.insert(b);
            }
        }
    }

    for set in &uniques {
        if set.len() <= 1 {
            const_cols += 1;
        }
    }
    println!("  sampled rows: {n}");
    println!("  constant columns (unique≤1): {const_cols}/{OPENING_INDEX_STRIDE}");
    println!("  even/even related pairs: {even_even}/{n}");

    let mut variable: Vec<(usize, usize)> = uniques
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.len()))
        .filter(|(_, u)| *u > 1)
        .collect();
    variable.sort_by_key(|(_, u)| std::cmp::Reverse(*u));
    println!("  top variable columns (offset → unique byte values):");
    for (off, u) in variable.iter().take(16) {
        println!("    +0x{off:02x}: {u} uniques");
    }

    // Known semantic columns should dominate variance; leftover mid-body
    // variance would be a discriminator candidate.
    let known_variable = BTreeSet::from([
        0x08usize, 0x09, 0x0a, 0x0b, // index
        0x14, 0x15, 0x16, 0x17, // related_a
        0x36, 0x37, 0x38, 0x39, // related_b
    ]);
    let leftover: Vec<_> = variable
        .iter()
        .copied()
        .filter(|(o, _)| !known_variable.contains(o))
        .collect();
    println!(
        "  variable columns OUTSIDE known index/related fields: {}",
        leftover.len()
    );
    for (off, u) in leftover.iter().take(12) {
        // Show value histogram for this column.
        let mut hist = BTreeMap::<u8, usize>::new();
        for &row in offs.iter().take(n) {
            if let Some(&b) = buf.get(row + off) {
                *hist.entry(b).or_insert(0) += 1;
            }
        }
        let top: Vec<_> = {
            let mut v: Vec<_> = hist.into_iter().collect();
            v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            v.into_iter().take(6).collect()
        };
        println!("    +0x{off:02x} uniques={u} top={top:?}");
    }

    let mut deltas: Vec<_> = delta_ab.into_iter().collect();
    deltas.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("  related_id_b - related_id_a top deltas:");
    for (d, c) in deltas.iter().take(8) {
        println!("    delta={d:>4} count={c}");
    }
    println!(
        "  unique related_id pairs: {} (of {n} sampled)",
        related_pairs.len()
    );
}

fn search_category_strings_near_openings(buf: &[u8], label: &str) {
    println!("\n=== Category/family UTF-16 near openings — {label} ===");
    let offs = ArcWallRectOpeningIndex::find_all_for_revit_version(2024, buf);
    if offs.is_empty() {
        println!("  no openings");
        return;
    }

    // Global partition string inventory for Door/Window category hints.
    let strings = utf16le_ascii_strings(buf, 4);
    let mut door_like = 0usize;
    let mut window_like = 0usize;
    let mut ost_doors = 0usize;
    let mut ost_windows = 0usize;
    let mut samples_door = Vec::new();
    let mut samples_window = Vec::new();
    for (off, s) in &strings {
        let lower = s.to_ascii_lowercase();
        let is_door =
            lower.contains("door") && !lower.contains("outdoor") && !lower.contains("indoor");
        let is_window = lower.contains("window");
        if lower.contains("ost_doors") || s.contains("OST_Doors") {
            ost_doors += 1;
        }
        if lower.contains("ost_windows") || s.contains("OST_Windows") {
            ost_windows += 1;
        }
        if is_door {
            door_like += 1;
            if samples_door.len() < 12 {
                samples_door.push((*off, s.clone()));
            }
        }
        if is_window {
            window_like += 1;
            if samples_window.len() < 12 {
                samples_window.push((*off, s.clone()));
            }
        }
    }
    println!(
        "  UTF-16 ASCII strings ≥4 chars: {} · door-like={} window-like={} OST_Doors={} OST_Windows={}",
        strings.len(),
        door_like,
        window_like,
        ost_doors,
        ost_windows
    );
    println!("  door-like samples:");
    for (o, s) in &samples_door {
        println!("    @{o} {s:?}");
    }
    println!("  window-like samples:");
    for (o, s) in &samples_window {
        println!("    @{o} {s:?}");
    }

    // For each opening related_id, search if the id bytes appear within
    // ±512 B of a door/window string. Cap work.
    let mut id_near_door = 0usize;
    let mut id_near_window = 0usize;
    let mut id_near_both = 0usize;
    let mut neither = 0usize;
    let sample_n = offs.len().min(400);
    for &off in offs.iter().take(sample_n) {
        let Ok(rec) = ArcWallRectOpeningIndex::decode(buf, off) else {
            continue;
        };
        let mut near_d = false;
        let mut near_w = false;
        for id in [rec.related_id_a, rec.related_id_b] {
            let needle = id.to_le_bytes();
            // Search a window around the opening record itself first.
            let start = off.saturating_sub(256);
            let end = (off + OPENING_INDEX_STRIDE + 256).min(buf.len());
            let window = &buf[start..end];
            for (i, w) in window.windows(4).enumerate() {
                if w == needle {
                    let abs = start + i;
                    // Look for nearby strings within ±128 of this id occurrence.
                    for (so, s) in &strings {
                        if (*so as isize - abs as isize).unsigned_abs() < 128 {
                            let lower = s.to_ascii_lowercase();
                            if lower.contains("door") {
                                near_d = true;
                            }
                            if lower.contains("window") {
                                near_w = true;
                            }
                        }
                    }
                }
            }
        }
        match (near_d, near_w) {
            (true, true) => id_near_both += 1,
            (true, false) => id_near_door += 1,
            (false, true) => id_near_window += 1,
            (false, false) => neither += 1,
        }
    }
    println!(
        "  sampled {sample_n} openings: near_door_only={id_near_door} near_window_only={id_near_window} near_both={id_near_both} neither={neither}"
    );
    println!(
        "  NOTE: co-location ≠ discriminator unless near_door_only / near_window_only split cleanly."
    );
}

fn vwall_rect_opening_probe(buf: &[u8], label: &str) {
    println!("\n=== VWallRectOpening 0x01a8 — {label} ===");
    let mut hits = Vec::new();
    for i in 0..buf.len().saturating_sub(3) {
        if read_u16(buf, i) == Some(VWALL_RECT_OPENING_TAG_2024)
            && buf.get(i + 2) == Some(&0)
            && buf.get(i + 3) == Some(&0)
        {
            hits.push(i);
        }
    }
    println!("  filtered hits: {}", hits.len());
    if hits.is_empty() {
        return;
    }
    let mut deltas = BTreeMap::<usize, usize>::new();
    for w in hits.windows(2) {
        *deltas.entry(w[1] - w[0]).or_insert(0) += 1;
    }
    let mut top: Vec<_> = deltas.into_iter().collect();
    top.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("  top deltas:");
    for (d, c) in top.iter().take(8) {
        println!("    delta={d} count={c}");
    }

    // Door/window dim pairs + family marker presence.
    let mut marker_at = BTreeMap::<usize, usize>::new();
    let mut door_dims = 0usize;
    let mut window_dims = 0usize; // wider than tall sill-like: w 1.5-6, h 2-5
    for &off in hits.iter().take(800) {
        for moff in (4..64).step_by(4) {
            if read_u32(buf, off + moff) == Some(OPENING_INDEX_FAMILY_MARKER)
                || read_u32(buf, off + moff) == Some(SCHEMA_FAMILY_MARKER)
            {
                *marker_at.entry(moff).or_insert(0) += 1;
            }
        }
        let mut best_door = false;
        let mut best_win = false;
        for a in (0..160).step_by(8) {
            for b in (a + 8..168).step_by(8) {
                let (Some(w), Some(h)) = (read_f64(buf, off + a), read_f64(buf, off + b)) else {
                    continue;
                };
                if (1.5..5.0).contains(&w) && (5.0..9.0).contains(&h) {
                    best_door = true;
                }
                if (1.5..8.0).contains(&w) && (2.0..5.5).contains(&h) && h < w {
                    best_win = true;
                }
            }
        }
        if best_door {
            door_dims += 1;
        }
        if best_win {
            window_dims += 1;
        }
    }
    println!("  family/schema markers by relative offset (first 800):");
    for (o, c) in marker_at.iter().take(12) {
        println!("    +0x{o:02x}: {c}");
    }
    println!(
        "  door-plausible (w,h) pairs in first 168 B: {door_dims}/{}",
        hits.len().min(800)
    );
    println!(
        "  window-plausible (wider-than-tall) pairs: {window_dims}/{}",
        hits.len().min(800)
    );
    println!("  first 2 hex dumps (96 B):");
    for &off in hits.iter().take(2) {
        let end = (off + 96).min(buf.len());
        let hex: String = buf[off..end]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("    @{off}: {hex}");
    }
}

fn related_id_elemtable_cross(rf: &mut RevitFile, buf: &[u8], label: &str) {
    println!("\n=== related_id × ElemTable cross — {label} ===");
    let Ok(records) = elem_table::parse_records(rf) else {
        println!("  ElemTable unreadable");
        return;
    };
    let id_set: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    println!("  ElemTable primary ids: {}", id_set.len());

    let offs = ArcWallRectOpeningIndex::find_all_for_revit_version(2024, buf);
    let mut a_in = 0usize;
    let mut b_in = 0usize;
    let mut both = 0usize;
    let mut a_ids = BTreeSet::new();
    let mut b_ids = BTreeSet::new();
    for &off in &offs {
        let Ok(rec) = ArcWallRectOpeningIndex::decode(buf, off) else {
            continue;
        };
        let ai = id_set.contains(&rec.related_id_a);
        let bi = id_set.contains(&rec.related_id_b);
        if ai {
            a_in += 1;
            a_ids.insert(rec.related_id_a);
        }
        if bi {
            b_in += 1;
            b_ids.insert(rec.related_id_b);
        }
        if ai && bi {
            both += 1;
        }
    }
    println!(
        "  openings={} a_in_table={a_in} b_in_table={b_in} both={both}",
        offs.len()
    );
    println!(
        "  unique related_a in table={} unique related_b in table={}",
        a_ids.len(),
        b_ids.len()
    );
    let overlap: BTreeSet<_> = a_ids.intersection(&b_ids).copied().collect();
    println!("  overlap between a-set and b-set: {}", overlap.len());
    // Pair structure: is b always a+1?
    let mut plus_one = 0usize;
    for &off in offs.iter().take(5000) {
        let Ok(rec) = ArcWallRectOpeningIndex::decode(buf, off) else {
            continue;
        };
        if rec.related_id_b == rec.related_id_a.saturating_add(1) {
            plus_one += 1;
        }
    }
    println!(
        "  related_b == related_a+1 (first 5000): {plus_one}/{}",
        offs.len().min(5000)
    );
    println!(
        "  CONCLUSION seed: ElemTable membership confirms ids exist but does not type Door vs Window."
    );
}

fn arcwall_2024_envelope(buf: &[u8], label: &str) {
    println!("\n=== 2024 ArcWall envelope hunt — {label} ===");
    // Tag from schema is expected 0x019c; also check 0x0191 leftover.
    for tag in [ARC_WALL_TAG_2024_CANDIDATE, ARC_WALL_TAG] {
        let mut filtered = Vec::new();
        for i in 0..buf.len().saturating_sub(3) {
            if read_u16(buf, i) == Some(tag)
                && buf.get(i + 2) == Some(&0)
                && buf.get(i + 3) == Some(&0)
            {
                filtered.push(i);
            }
        }
        println!("  tag 0x{tag:04x} filtered hits: {}", filtered.len());
        if filtered.is_empty() {
            continue;
        }

        let mut variant_hist = BTreeMap::<u16, usize>::new();
        let mut family_marker = 0usize;
        let mut std_2023 = 0usize;
        let mut compound_2023 = 0usize;
        let mut coord_plausible = 0usize;
        for &off in filtered.iter().take(2000) {
            if read_u32(buf, off + 4) == Some(SCHEMA_FAMILY_MARKER) {
                family_marker += 1;
            }
            if let Some(v) = read_u16(buf, off + 0x10) {
                *variant_hist.entry(v).or_insert(0) += 1;
                if v == ARC_WALL_VARIANT_STANDARD {
                    std_2023 += 1;
                }
                if v == ARC_WALL_VARIANT_COMPOUND {
                    compound_2023 += 1;
                }
            }
            // 2023-style 6 f64 coords starting +0x12
            let mut ok = true;
            for k in 0..6 {
                match read_f64(buf, off + 0x12 + k * 8) {
                    Some(v) if v.abs() < 5000.0 => {}
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                coord_plausible += 1;
            }
        }
        let mut variants: Vec<_> = variant_hist.into_iter().collect();
        variants.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        println!(
            "    SCHEMA_FAMILY_MARKER@+4: {family_marker}/{}",
            filtered.len().min(2000)
        );
        println!(
            "    2023 variant 0x07fa: {std_2023} · 0x0821: {compound_2023} · coord6@+0x12 plausible: {coord_plausible}"
        );
        println!("    top variants @+0x10:");
        for (v, c) in variants.iter().take(10) {
            println!("      0x{v:04x}: {c}");
        }

        // Try alternate coord bases if 2023 layout fails.
        if coord_plausible == 0 && !filtered.is_empty() {
            println!(
                "    alternate f64-pair bases (first 200 hits, need ≥4 building-scale pairs):"
            );
            let mut base_hits = BTreeMap::<usize, usize>::new();
            for &off in filtered.iter().take(200) {
                for base in (0x08..0x40).step_by(2) {
                    let mut pairs = 0;
                    for k in 0..4 {
                        let x = read_f64(buf, off + base + k * 16);
                        let y = read_f64(buf, off + base + k * 16 + 8);
                        if let (Some(x), Some(y)) = (x, y) {
                            if x.abs() < 500.0
                                && y.abs() < 500.0
                                && (x.abs() > 0.5 || y.abs() > 0.5)
                            {
                                pairs += 1;
                            }
                        }
                    }
                    if pairs >= 2 {
                        *base_hits.entry(base).or_insert(0) += 1;
                    }
                }
            }
            let mut ranked: Vec<_> = base_hits.into_iter().collect();
            ranked.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            for (b, c) in ranked.iter().take(8) {
                println!("      base=+0x{b:02x}: {c}/200");
            }
            if ranked.is_empty() {
                println!("      (no alternate plan-coord bases cleared threshold)");
            }
        }

        println!("    sample hex (64 B) first hit:");
        if let Some(&off) = filtered.first() {
            let end = (off + 64).min(buf.len());
            let hex: String = buf[off..end]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            println!("      @{off}: {hex}");
        }
    }
}

fn schema_field_wall_on_global(rf: &mut RevitFile, label: &str) {
    println!("\n=== Global/Latest schema-field Wall/Door/Window — {label} ===");
    let limits = rvt::walker::WalkerLimits {
        max_candidates: 50_000,
        ..rvt::walker::WalkerLimits::default()
    };
    match rvt::walker::iter_elements_with_limits(
        rf,
        rvt::walker::PRODUCTION_ELEMENT_MIN_SCORE,
        limits,
    ) {
        Ok(iter) => {
            let mut counts = BTreeMap::<String, usize>::new();
            for e in iter {
                *counts.entry(e.class.clone()).or_insert(0) += 1;
            }
            println!("  production class counts:");
            for (k, v) in &counts {
                println!("    {k}: {v}");
            }
            for need in ["Wall", "Door", "Window", "ArcWall", "VWall"] {
                let n = counts.get(need).copied().unwrap_or(0);
                println!("  typed `{need}` rows: {n}");
            }
        }
        Err(e) => println!("  iter_elements failed: {e}"),
    }

    // Diagnostic: inflate Global/Latest and scan with min_score floor.
    let Ok(formats_raw) = rf.read_stream(streams::FORMATS_LATEST) else {
        println!("  Formats/Latest unreadable for diagnostic scan");
        return;
    };
    let Ok(formats_d) = compression::inflate_at(&formats_raw, 0) else {
        return;
    };
    let Ok(schema) = formats::parse_schema(&formats_d) else {
        return;
    };
    let Ok(raw) = rf.read_stream(streams::GLOBAL_LATEST) else {
        println!("  Global/Latest unreadable");
        return;
    };
    let Ok((_, d)) = compression::inflate_at_auto(&raw) else {
        return;
    };
    let cands =
        rvt::walker::scan_candidates_with_limits(&schema, &d, i64::MIN + 1, limits).candidates;
    let mut counts = BTreeMap::<String, usize>::new();
    for c in &cands {
        *counts.entry(c.class_name.clone()).or_insert(0) += 1;
    }
    println!("  diagnostic candidate top classes:");
    let mut ranked: Vec<_> = counts.into_iter().collect();
    ranked.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (k, v) in ranked.iter().take(15) {
        println!("    {k}: {v}");
    }
    for need in ["Wall", "Door", "Window"] {
        let n = ranked
            .iter()
            .find(|(k, _)| k == need)
            .map(|(_, v)| *v)
            .unwrap_or(0);
        println!("  diagnostic `{need}`: {n}");
    }
}

fn einhoven_2023_opening_absence(buf: &[u8]) {
    println!("\n=== Einhoven 2023 opening / door tags ===");
    for (name, tag) in [
        ("ArcWallRectOpening-2023-guess", 0x019cu16),
        ("ArcWallRectOpening-2024", ARC_WALL_RECT_OPENING_TAG_2024),
        ("VWallRectOpening-2023-guess", 0x019du16),
        ("VWallRectOpening-2024", VWALL_RECT_OPENING_TAG_2024),
        ("ArcWall-2023", ARC_WALL_TAG),
        ("VWall-2023-guess", 0x0192u16),
    ] {
        let mut n = 0usize;
        for i in 0..buf.len().saturating_sub(3) {
            if read_u16(buf, i) == Some(tag)
                && buf.get(i + 2) == Some(&0)
                && buf.get(i + 3) == Some(&0)
            {
                n += 1;
            }
        }
        println!("  {name} 0x{tag:04x}: {n} filtered");
    }
    let strings = utf16le_ascii_strings(buf, 4);
    let door = strings
        .iter()
        .filter(|(_, s)| s.to_ascii_lowercase().contains("door"))
        .count();
    let window = strings
        .iter()
        .filter(|(_, s)| s.to_ascii_lowercase().contains("window"))
        .count();
    println!(
        "  UTF-16 door-like strings={} window-like={} (of {} total)",
        door,
        window,
        strings.len()
    );
}

fn opening_index_column_bit_patterns(buf: &[u8]) {
    println!("\n=== Opening-index mid-body u16/u32 histograms (discriminator hunt) ===");
    let offs = ArcWallRectOpeningIndex::find_all_for_revit_version(2024, buf);
    if offs.is_empty() {
        return;
    }
    // Columns between known fields: +0x1c .. +0x31 and +0x3a .. +0x3b already known 0x0248
    let sample = offs.len().min(4000);
    for width in [2usize, 4usize] {
        println!("  width={width}:");
        for off in (0x1c..=0x30).step_by(width) {
            let mut hist: HashMap<u64, usize> = HashMap::new();
            for &row in offs.iter().take(sample) {
                let v = if width == 2 {
                    read_u16(buf, row + off).map(u64::from)
                } else {
                    read_u32(buf, row + off).map(u64::from)
                };
                if let Some(v) = v {
                    *hist.entry(v).or_insert(0) += 1;
                }
            }
            let mut ranked: Vec<_> = hist.into_iter().collect();
            ranked.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            let distinct = ranked.len();
            // Interesting if bimodal ~50/50 (door vs window) or small enum.
            if distinct <= 8 {
                let top: Vec<_> = ranked
                    .iter()
                    .take(6)
                    .map(|(v, c)| format!("0x{v:x}:{c}"))
                    .collect();
                println!("    +0x{off:02x}: distinct={distinct} {top:?}");
            } else if distinct == 2
                || ranked
                    .get(0)
                    .zip(ranked.get(1))
                    .is_some_and(|((_, a), (_, b))| {
                        let total = sample as f64;
                        let ra = *a as f64 / total;
                        let rb = *b as f64 / total;
                        ra > 0.2 && rb > 0.2 && ra + rb > 0.85
                    })
            {
                let top: Vec<_> = ranked
                    .iter()
                    .take(4)
                    .map(|(v, c)| format!("0x{v:x}:{c}"))
                    .collect();
                println!("    +0x{off:02x}: BIMODAL? distinct={distinct} {top:?}");
            }
        }
    }
}

fn main() {
    let dir = project_dir();
    println!("Door/Window/Wall hard-gap research");
    println!("corpus: {}", dir.display());

    // --- 2024 Core Interior ---
    let path_2024 = dir.join("2024_Core_Interior.rvt");
    let mut rf24 = RevitFile::open(&path_2024).expect("open 2024");
    schema_wall_door_window_inventory(&mut rf24, "2024 Core Interior");
    let part46 = inflate_partition(&mut rf24, "Partitions/46").expect("Partitions/46");
    println!("\nPartitions/46 inflated: {} B", part46.len());
    opening_index_byte_variance(&part46, "2024 Partitions/46");
    opening_index_column_bit_patterns(&part46);
    search_category_strings_near_openings(&part46, "2024 Partitions/46");
    vwall_rect_opening_probe(&part46, "2024 Partitions/46");
    related_id_elemtable_cross(&mut rf24, &part46, "2024 Core Interior");
    arcwall_2024_envelope(&part46, "2024 Partitions/46");
    schema_field_wall_on_global(&mut rf24, "2024 Core Interior");

    // --- 2023 Einhoven ---
    let path_23 = dir.join("Revit_IFC5_Einhoven.rvt");
    let mut rf23 = RevitFile::open(&path_23).expect("open 2023");
    schema_wall_door_window_inventory(&mut rf23, "2023 Einhoven");
    let part5 = inflate_partition(&mut rf23, "Partitions/5").expect("Partitions/5");
    println!("\nPartitions/5 inflated: {} B", part5.len());
    einhoven_2023_opening_absence(&part5);
    arcwall_2024_envelope(&part5, "2023 Partitions/5 (control)");
    schema_field_wall_on_global(&mut rf23, "2023 Einhoven");

    println!("\n=== END probe_door_window_wall_research ===");
}
