//! Research probe (#90 / #219, RE-29): find the partition element record
//! that carries a Revit room, and dump everything the record family knows
//! about it.
//!
//! Stage 1 brute-scans every offset of every inflated `Partitions/*` stream
//! for a decodable element-record header (the same sweep
//! `probe_element_record_owner_lookup` uses) and histograms the
//! `BuiltInCategory` ids found, with the RE-21 instance-rule selection per
//! category. That is the "measure, do not assume" half: the room carrier is
//! whichever category's selected id set matches the reference export, not
//! whichever constant looks right.
//!
//! Stage 2, when `--ids <file>` names a newline-separated ElementId list
//! (the reference export's room ids), scores every category against it and
//! writes every frame of every wanted id to `--json <path>`: prologue
//! fields, bounding box, both counted reference lists and the `f64` run
//! that follows them.
//!
//! Stage 3, when `--verts <path>` names a `id x,y x,y …` file (the
//! reference export's room plan polygons), searches every inflated
//! partition byte for each polygon's vertices as packed `f64`, at every
//! stride from 2 to 8 doubles, and reports whether an ordered run exists —
//! the same negative-or-positive test RE-25 §2 ran for slab profiles.
//!
//! Not part of the shipped decode path.

use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct CategoryStats {
    records: usize,
    ids: BTreeSet<u32>,
    instance_ids: BTreeSet<u32>,
    symbol_ids: BTreeSet<u32>,
    member_ids: BTreeSet<u32>,
}

/// Polygon index, vertex index and winding an edge delta starts at.
type EdgeStart = (usize, usize, i8);

struct Frame {
    stream: String,
    offset: usize,
    record: per::PartitionElementRecord,
    refs2: Vec<u64>,
    tail_f64: Vec<f64>,
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4")))
}

/// Byte offset just past the second counted reference list.
fn after_reference_lists(buf: &[u8], record_at: usize) -> Option<usize> {
    let at = record_at + per::REFERENCE_LIST_OFFSET;
    let n1 = read_u32(buf, at)? as usize;
    let second = at + 4 + n1 * 8;
    let n2 = read_u32(buf, second)? as usize;
    Some(second + 4 + n2 * 8)
}

fn second_list(buf: &[u8], record_at: usize) -> Vec<u64> {
    let at = record_at + per::REFERENCE_LIST_OFFSET;
    let Some(n1) = read_u32(buf, at) else {
        return Vec::new();
    };
    let second = at + 4 + (n1 as usize) * 8;
    per::decode_reference_list(buf, second).unwrap_or_default()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut path = None;
    let mut ids_file = None;
    let mut json_out = None;
    let mut verts_file = None;
    let mut names_file = None;
    let mut name_dump_limit = 0usize;
    let mut tail_doubles = 64usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ids" => ids_file = args.next(),
            "--json" => json_out = args.next(),
            "--verts" => verts_file = args.next(),
            "--tail" => tail_doubles = args.next().and_then(|v| v.parse().ok()).unwrap_or(64),
            "--names" => names_file = args.next(),
            "--dump-names" => {
                name_dump_limit = args.next().and_then(|v| v.parse().ok()).unwrap_or(2)
            }
            other => path = Some(other.to_string()),
        }
    }
    let path = path.expect("usage: probe_re29_room_records <file.rvt> [--ids ids.txt]");

    let wanted: BTreeSet<u32> = match &ids_file {
        Some(file) => std::fs::read_to_string(file)?
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect(),
        None => BTreeSet::new(),
    };

    // `id x,y x,y …` per line.
    let mut polygons: Vec<(u32, Vec<(f64, f64)>)> = Vec::new();
    if let Some(file) = &verts_file {
        for line in std::fs::read_to_string(file)?.lines() {
            let mut parts = line.split_whitespace();
            let Some(id) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
                continue;
            };
            let pts: Vec<(f64, f64)> = parts
                .filter_map(|p| {
                    let (a, b) = p.split_once(',')?;
                    Some((a.parse().ok()?, b.parse().ok()?))
                })
                .collect();
            if !pts.is_empty() {
                polygons.push((id, pts));
            }
        }
    }

    // `id<TAB>name` per line.
    let mut names: Vec<(u32, String)> = Vec::new();
    if let Some(file) = &names_file {
        for line in std::fs::read_to_string(file)?.lines() {
            let Some((id, text)) = line.split_once('\t') else {
                continue;
            };
            let Ok(id) = id.trim().parse::<u32>() else {
                continue;
            };
            names.push((id, text.to_string()));
        }
    }

    let mut rf = RevitFile::open(&path)?;
    let declared: BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();
    println!("declared ElemTable ids: {}", declared.len());
    println!("wanted ids: {}", wanted.len());
    println!("reference polygons: {}", polygons.len());

    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();

    let mut stats: BTreeMap<i64, CategoryStats> = BTreeMap::new();
    let mut frames: BTreeMap<u32, Vec<Frame>> = BTreeMap::new();
    let mut total = 0usize;
    let want_vertex_search = !polygons.is_empty();
    // Every `OST_SketchLines` owner, so the RE-25 join can be scored
    // against the room id set without a second sweep.
    let mut sketch_owners: BTreeSet<u32> = BTreeSet::new();
    // Parameter value strings framed against a wanted ElementId.
    let mut param_strings: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    // Name-carrier search state.
    const NAME_WINDOW: usize = 1024;
    let mut name_hits = 0usize;
    let mut name_offsets: BTreeMap<i64, usize> = BTreeMap::new();
    let mut name_rooms_with_own_id: BTreeSet<u32> = BTreeSet::new();
    let mut name_dumps = name_dump_limit;
    // Recovered `(number, name, level id)` per room.
    let mut room_params: BTreeMap<u32, BTreeSet<(String, String, u64)>> = BTreeMap::new();
    // Declared ids that are NOT rooms but still accept the framing.
    let mut other_param_ids: BTreeSet<u32> = BTreeSet::new();
    // Longest ordered run of a polygon's vertices found as packed
    // `f64`, per `(polygon, stride, winding)`.
    let mut best_run: BTreeMap<(u32, usize, i8), (usize, String, usize)> = BTreeMap::new();
    // Quantised vertex index: pair -> (polygon index, vertex index).
    let mut vertex_index: BTreeMap<(i64, i64), Vec<(usize, usize)>> = BTreeMap::new();
    for (pi, (_, pts)) in polygons.iter().enumerate() {
        for (vi, (x, y)) in pts.iter().enumerate() {
            vertex_index
                .entry((quantise(*x), quantise(*y)))
                .or_default()
                .push((pi, vi));
        }
    }
    // Translation-invariant index: the delta of one edge -> the
    // polygons and vertices that edge starts at. Catches a polygon
    // stored in a local frame, which the absolute search cannot see.
    let mut edge_index: BTreeMap<(i64, i64), Vec<EdgeStart>> = BTreeMap::new();
    for (pi, (_, pts)) in polygons.iter().enumerate() {
        let n = pts.len();
        for vi in 0..n {
            for dir in [1i8, -1i8] {
                let next = if dir == 1 {
                    (vi + 1) % n
                } else {
                    (vi + n - 1) % n
                };
                let dx = quantise(pts[next].0 - pts[vi].0);
                let dy = quantise(pts[next].1 - pts[vi].1);
                edge_index.entry((dx, dy)).or_default().push((pi, vi, dir));
            }
        }
    }
    let mut best_delta_run: BTreeMap<u32, (usize, String, usize, usize)> = BTreeMap::new();
    // Strides tried, in doubles per vertex: 2 = packed (x,y),
    // 3 = (x,y,z) triples, 4 = (x,y,z,w) or a per-vertex record.
    const STRIDES: [usize; 4] = [2, 3, 4, 6];

    // The needle side: every coordinate the reference polygons use, and
    // every ordered `(x, y)` pair, quantised to 1e-6 ft so the float dust
    // of composing Revit's own placement chain cannot cause a miss.
    let mut needle_values: BTreeSet<i64> = BTreeSet::new();
    let mut needle_pairs: BTreeSet<(i64, i64)> = BTreeSet::new();
    for (_, pts) in &polygons {
        for (x, y) in pts {
            needle_values.insert(quantise(*x));
            needle_values.insert(quantise(*y));
            needle_pairs.insert((quantise(*x), quantise(*y)));
        }
    }
    // The haystack side, filled by the sweep below.
    let mut seen_values: BTreeSet<i64> = BTreeSet::new();
    let mut seen_pairs: BTreeSet<(i64, i64)> = BTreeSet::new();
    let mut seen_pairs_swapped: BTreeSet<(i64, i64)> = BTreeSet::new();

    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        if concat.len() < per::RECORD_MIN_LEN {
            continue;
        }
        for offset in 0..=(concat.len() - per::RECORD_MIN_LEN) {
            let Some(record) = per::decode_at(stream, &concat, offset, &declared) else {
                continue;
            };
            total += 1;
            let entry = stats.entry(record.builtin_category).or_default();
            entry.records += 1;
            entry.ids.insert(record.element_id);
            if record.is_exported_instance() {
                entry.instance_ids.insert(record.element_id);
            }
            if record.is_type_symbol() {
                entry.symbol_ids.insert(record.element_id);
            }
            if record.is_container_member() {
                entry.member_ids.insert(record.element_id);
            }
            if record.builtin_category == per::OST_SKETCH_LINES {
                if let Some(owner) = record.owner_reference {
                    sketch_owners.insert(owner);
                }
            }
            if wanted.contains(&record.element_id) {
                let refs2 = second_list(&concat, offset);
                let mut tail_f64 = Vec::new();
                if let Some(at) = after_reference_lists(&concat, offset) {
                    for index in 0..tail_doubles {
                        let start = at + index * 8;
                        let Some(slice) = concat.get(start..start + 8) else {
                            break;
                        };
                        tail_f64.push(f64::from_le_bytes(slice.try_into().expect("8")));
                    }
                }
                frames.entry(record.element_id).or_default().push(Frame {
                    stream: stream.clone(),
                    offset,
                    record,
                    refs2,
                    tail_f64,
                });
            }
        }

        // Room parameter-block scan, anchored on the owner ElementId:
        //
        //   +0x000  u64  owning room ElementId
        //   +0x03c  u64  owning room ElementId (confirmation)
        //   +0x044  u64  host Level ElementId
        //   +0x1f5  u32  number length in UTF-16 units
        //   +0x1f9  2*n  room Number, UTF-16LE
        //   +…+12   u32  name length in UTF-16 units
        //   +…+16   2*m  room Name, UTF-16LE
        if !wanted.is_empty() {
            let mut index = 0usize;
            while index + 8 <= concat.len() {
                let raw = u64::from_le_bytes(concat[index..index + 8].try_into().expect("8"));
                if raw != 0 && raw <= u64::from(u32::MAX) && declared.contains(&(raw as u32)) {
                    if let Some(block) = room_parameter_block(&concat, index, raw as u32) {
                        if wanted.contains(&(raw as u32)) {
                            room_params.entry(raw as u32).or_default().insert(block);
                        } else {
                            other_param_ids.insert(raw as u32);
                        }
                    }
                }
                index += 1;
            }
        }

        // Parameter-string scan, anchored on the owner rather than on
        // the value: at every offset whose `u64` is a wanted ElementId,
        // test the RE-22 `IFC Export As` framing (owner at value-220,
        // confirmation at value-286, `u32` length at value-4) and, when
        // both owner slots agree, read the UTF-16LE value.
        if !wanted.is_empty() {
            let mut index = 0usize;
            while index + 8 <= concat.len() {
                let raw = u64::from_le_bytes(concat[index..index + 8].try_into().expect("8"));
                if raw <= u64::from(u32::MAX) && wanted.contains(&(raw as u32)) {
                    if let Some(text) = owner_framed_string(&concat, index, raw as u32) {
                        param_strings.entry(raw as u32).or_default().insert(text);
                    }
                }
                index += 1;
            }
        }

        // Name-carrier search: find each room's name as a UTF-16LE run
        // and histogram the signed byte offset at which the room's own
        // ElementId appears in the surrounding window. A carrier shows
        // up as one offset that repeats across rooms; noise does not.
        for (id, text) in &names {
            let needle: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let mut from = 0usize;
            while let Some(found) = memchr_find(&concat[from..], &needle) {
                let at = from + found;
                from = at + 1;
                name_hits += 1;
                let lo = at.saturating_sub(NAME_WINDOW);
                let hi = (at + NAME_WINDOW).min(concat.len().saturating_sub(8));
                let mut own_at = Vec::new();
                for probe in lo..hi {
                    let raw = u64::from_le_bytes(concat[probe..probe + 8].try_into().expect("8"));
                    if raw == u64::from(*id) {
                        *name_offsets.entry(probe as i64 - at as i64).or_default() += 1;
                        name_rooms_with_own_id.insert(*id);
                        own_at.push(probe as i64 - at as i64);
                    }
                }
                if !own_at.is_empty() && name_dumps > 0 {
                    name_dumps -= 1;
                    println!(
                        "--- name hit: room {id} {text:?} in {stream}+0x{at:x}, own id at {own_at:?}"
                    );
                    let start = at.saturating_sub(600);
                    let mut probe = start;
                    while probe + 8 <= at + 80 {
                        let raw =
                            u64::from_le_bytes(concat[probe..probe + 8].try_into().expect("8"));
                        if raw != 0
                            && raw <= u64::from(u32::MAX)
                            && declared.contains(&(raw as u32))
                        {
                            println!(
                                "      {:>5}  u64 declared id {raw}",
                                probe as i64 - at as i64
                            );
                        }
                        let n = u32::from_le_bytes(concat[probe..probe + 4].try_into().expect("4"))
                            as usize;
                        if (1..=64).contains(&n) && probe + 4 + n * 2 <= concat.len() {
                            if let Some(s) = utf16_at(&concat, probe + 4, n) {
                                if s.chars().all(|c| !c.is_control())
                                    && s.chars().any(|c| c.is_alphanumeric())
                                {
                                    println!(
                                        "      {:>5}  u32 len {n} -> {s:?}",
                                        probe as i64 - at as i64
                                    );
                                }
                            }
                        }
                        probe += 1;
                    }
                }
            }
        }

        if want_vertex_search {
            // Every byte offset, not every 8-byte-aligned one: Revit's
            // runs are not guaranteed to sit on an 8-byte boundary of the
            // concatenated stream.
            let mut index = 0usize;
            while index + 16 <= concat.len() {
                let a = f64::from_le_bytes(concat[index..index + 8].try_into().expect("8"));
                if a.is_finite() && a.abs() < 1.0e4 {
                    let qa = quantise(a);
                    if needle_values.contains(&qa) {
                        seen_values.insert(qa);
                    }
                    let b =
                        f64::from_le_bytes(concat[index + 8..index + 16].try_into().expect("8"));
                    if b.is_finite() && b.abs() < 1.0e4 {
                        let qb = quantise(b);
                        if needle_pairs.contains(&(qa, qb)) {
                            seen_pairs.insert((qa, qb));
                        }
                        if needle_pairs.contains(&(qb, qa)) {
                            seen_pairs_swapped.insert((qb, qa));
                        }
                        // Ordered-run search: how far does this offset
                        // follow one polygon's vertex sequence, at each
                        // candidate stride, cyclically, in either
                        // winding?
                        if let Some(starts) = vertex_index.get(&(qa, qb)) {
                            for (pi, vi) in starts {
                                let pts = &polygons[*pi].1;
                                let n = pts.len();
                                let id = polygons[*pi].0;
                                for stride in STRIDES {
                                    for dir in [1i8, -1i8] {
                                        let mut run = 1usize;
                                        while run < n {
                                            let at = index + run * stride * 8;
                                            let Some(v) = read_pair(&concat, at) else {
                                                break;
                                            };
                                            let step = if dir == 1 {
                                                (vi + run) % n
                                            } else {
                                                (vi + n * n - run) % n
                                            };
                                            let (wx, wy) = pts[step];
                                            if quantise(v.0) != quantise(wx)
                                                || quantise(v.1) != quantise(wy)
                                            {
                                                break;
                                            }
                                            run += 1;
                                        }
                                        let entry = best_run.entry((id, stride, dir)).or_insert((
                                            0,
                                            String::new(),
                                            0,
                                        ));
                                        if run > entry.0 {
                                            *entry = (run, stream.clone(), index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Translation-invariant pass at stride 2: anchor on the
                // *delta* of one edge, so a polygon stored in a local
                // frame is still found.
                if let (Some(v0), Some(v1)) =
                    (read_pair(&concat, index), read_pair(&concat, index + 16))
                {
                    if v0.0.abs() < 1.0e6 && v1.0.abs() < 1.0e6 {
                        let key = (quantise(v1.0 - v0.0), quantise(v1.1 - v0.1));
                        if key != (0, 0) {
                            if let Some(starts) = edge_index.get(&key) {
                                for (pi, vi, dir) in starts {
                                    let pts = &polygons[*pi].1;
                                    let n = pts.len();
                                    let mut run = 2usize;
                                    while run < n {
                                        let Some(v) = read_pair(&concat, index + run * 16) else {
                                            break;
                                        };
                                        let step = if *dir == 1 {
                                            (vi + run) % n
                                        } else {
                                            (vi + n * n - run) % n
                                        };
                                        if quantise(v.0 - v0.0)
                                            != quantise(pts[step].0 - pts[*vi].0)
                                            || quantise(v.1 - v0.1)
                                                != quantise(pts[step].1 - pts[*vi].1)
                                        {
                                            break;
                                        }
                                        run += 1;
                                    }
                                    let id = polygons[*pi].0;
                                    let entry = best_delta_run.entry(id).or_insert((
                                        0,
                                        String::new(),
                                        0,
                                        0,
                                    ));
                                    if run > entry.0 {
                                        *entry = (run, stream.clone(), index, *pi);
                                    }
                                }
                            }
                        }
                    }
                }
                index += 1;
            }
        }
    }

    println!("total decodable element records: {total}");
    println!();
    println!(
        "category histogram (category, records, ids, instance-rule ids, symbols, members, wanted)"
    );
    let mut rows: Vec<(i64, &CategoryStats)> = stats.iter().map(|(k, v)| (*k, v)).collect();
    rows.sort_by_key(|(_, v)| std::cmp::Reverse(v.records));
    for (category, s) in &rows {
        let hit = s.ids.intersection(&wanted).count();
        let inst_hit = s.instance_ids.intersection(&wanted).count();
        println!(
            "  {category:>9}  records={:<6} ids={:<6} instances={:<6} symbols={:<5} members={:<5} wanted_ids={hit} wanted_instances={inst_hit}",
            s.records,
            s.ids.len(),
            s.instance_ids.len(),
            s.symbol_ids.len(),
            s.member_ids.len(),
        );
    }

    if !wanted.is_empty() {
        println!();
        println!("=== instance rule scored against the wanted id set ===");
        for (category, s) in &rows {
            let tp = s.instance_ids.intersection(&wanted).count();
            if tp == 0 {
                continue;
            }
            let fp = s.instance_ids.difference(&wanted).count();
            let fnn = wanted.difference(&s.instance_ids).count();
            println!(
                "  {category:>9}  TP={tp} FP={fp} FN={fnn} (selected {})",
                s.instance_ids.len()
            );
        }
        println!();
        println!(
            "wanted ids with at least one record: {} / {}",
            frames.len(),
            wanted.len()
        );
    }

    if let Some(out) = &json_out {
        let mut buf = String::from("[\n");
        for (index, (id, list)) in frames.iter().enumerate() {
            if index > 0 {
                buf.push_str(",\n");
            }
            buf.push_str(&format!("{{\"element_id\":{id},\"frames\":["));
            for (fi, f) in list.iter().enumerate() {
                if fi > 0 {
                    buf.push(',');
                }
                buf.push_str(&format!(
                    "{{\"stream\":\"{}\",\"offset\":{},\"category\":{},\"flags\":{},\"container\":{},\"placement_kind\":{},\"bbox\":{:?},\"refs1\":{:?},\"refs2\":{:?},\"preceding\":{},\"owner\":{},\"tail\":{:?}}}",
                    f.stream,
                    f.offset,
                    f.record.builtin_category,
                    f.record.flags,
                    f.record.container,
                    f.record.placement_kind,
                    f.record.bbox_feet,
                    f.record.references,
                    f.refs2,
                    f.record
                        .preceding_reference
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "null".into()),
                    f.record
                        .owner_reference
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "null".into()),
                    f.tail_f64,
                ));
            }
            buf.push_str("]}");
        }
        buf.push_str("\n]\n");
        std::fs::write(out, buf)?;
        println!("wrote {out}");
    }

    if want_vertex_search {
        println!();
        println!("=== vertex search (quantised to 1e-6 ft) ===");
        println!(
            "  distinct reference coordinates: {}, found as a double anywhere: {}",
            needle_values.len(),
            seen_values.len()
        );
        println!(
            "  distinct reference (x,y) vertices: {}, found as an adjacent double pair: {} (x,y) / {} (y,x)",
            needle_pairs.len(),
            seen_pairs.len(),
            seen_pairs_swapped.len()
        );
        let mut whole = 0usize;
        let mut any = 0usize;
        for (id, pts) in &polygons {
            let hits = pts
                .iter()
                .filter(|(x, y)| {
                    seen_pairs.contains(&(quantise(*x), quantise(*y)))
                        || seen_pairs_swapped.contains(&(quantise(*x), quantise(*y)))
                })
                .count();
            if hits == pts.len() {
                whole += 1;
            }
            if hits > 0 {
                any += 1;
                println!(
                    "  room {id}: {hits}/{} vertices framed as a double pair",
                    pts.len()
                );
            }
        }
        println!(
            "  polygons fully present as vertex pairs: {whole} / {}",
            polygons.len()
        );
        println!(
            "  polygons with at least one vertex pair: {any} / {}",
            polygons.len()
        );

        println!();
        println!("=== longest ordered vertex run per polygon ===");
        for stride in STRIDES {
            for dir in [1i8, -1i8] {
                let mut hist: BTreeMap<usize, usize> = BTreeMap::new();
                let mut closed = 0usize;
                for (id, pts) in &polygons {
                    let (run, stream, offset) = best_run
                        .get(&(*id, stride, dir))
                        .cloned()
                        .unwrap_or((0, String::new(), 0));
                    *hist.entry(run).or_default() += 1;
                    if run >= pts.len() {
                        closed += 1;
                    }
                    if run >= 3 {
                        println!(
                            "  stride {stride} dir {dir}: room {id} run {run}/{} at {stream}+0x{offset:x}",
                            pts.len()
                        );
                    }
                }
                println!(
                    "  stride {stride} dir {dir:>2}: run histogram {hist:?}, complete {closed}/{}",
                    polygons.len()
                );
            }
        }

        println!();
        println!("=== translation-invariant run (stride 2, edge-delta anchored) ===");
        let mut dhist: BTreeMap<usize, usize> = BTreeMap::new();
        let mut dclosed = 0usize;
        for (id, pts) in &polygons {
            let (run, stream, offset, _) =
                best_delta_run
                    .get(id)
                    .cloned()
                    .unwrap_or((0, String::new(), 0, 0));
            *dhist.entry(run).or_default() += 1;
            if run >= pts.len() {
                dclosed += 1;
            }
            if run >= 4 {
                println!(
                    "  room {id}: delta run {run}/{} at {stream}+0x{offset:x}",
                    pts.len()
                );
            }
        }
        println!(
            "  delta-run histogram: {dhist:?}, complete {dclosed}/{}",
            polygons.len()
        );

        println!();
        println!("=== name-carrier search ===");
        println!("  name string hits: {name_hits}");
        println!(
            "  rooms whose own id appears within {NAME_WINDOW} bytes of their name: {} / {}",
            name_rooms_with_own_id.len(),
            names.len()
        );
        let mut top: Vec<(i64, usize)> = name_offsets.into_iter().collect();
        top.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        println!(
            "  most repeated signed offsets (offset -> hits): {:?}",
            &top[..top.len().min(10)]
        );

        println!();
        println!("=== room parameter block (number / name / level), anchored on the room id ===");
        println!(
            "  rooms with at least one accepted block: {} / {}",
            room_params.len(),
            wanted.len()
        );
        let unique = room_params.values().filter(|v| v.len() == 1).count();
        println!(
            "  rooms whose blocks all agree: {unique} / {}",
            room_params.len()
        );
        let mut name_ok = 0usize;
        for (id, expected) in &names {
            if let Some(blocks) = room_params.get(id) {
                if blocks.len() == 1
                    && blocks.iter().next().map(|b| b.1.as_str()) == Some(expected.as_str())
                {
                    name_ok += 1;
                }
            }
        }
        println!(
            "  rooms whose single recovered name equals the reference: {name_ok} / {}",
            names.len()
        );
        println!(
            "  non-room declared ids that also accept the framing: {} (of {} declared)",
            other_param_ids.len(),
            declared.len()
        );
        for (id, blocks) in room_params.iter().take(6) {
            println!("  {id}: {blocks:?}");
        }

        println!();
        println!("=== owner-framed parameter strings (RE-22 framing, anchored on the room id) ===");
        println!(
            "  wanted ids with at least one framed string: {} / {}",
            param_strings.len(),
            wanted.len()
        );
        for (id, texts) in param_strings.iter().take(8) {
            println!("  {id}: {texts:?}");
        }

        println!();
        println!("=== OST_SketchLines owner join (RE-25) scored against the room ids ===");
        println!("  distinct sketch-line owners: {}", sketch_owners.len());
        println!(
            "  owners that are a wanted room id: {}",
            sketch_owners.intersection(&wanted).count()
        );
    }

    Ok(())
}

/// Offset from the owning ElementId to its confirmation copy.
const ROOM_PARAM_CONFIRM: usize = 0x3c;
/// Offset from the owning ElementId to the host Level ElementId.
const ROOM_PARAM_LEVEL: usize = 0x44;
/// Offset from the owning ElementId to the room number's length prefix.
const ROOM_PARAM_NUMBER_LEN: usize = 505;
/// Bytes between the end of the number string and the name's length prefix.
const ROOM_PARAM_NAME_GAP: usize = 8;

/// The room `(number, name, level id)` a parameter block anchored on
/// `owner_at` carries, fail-closed.
fn room_parameter_block(
    buf: &[u8],
    owner_at: usize,
    element_id: u32,
) -> Option<(String, String, u64)> {
    let confirm_at = owner_at.checked_add(ROOM_PARAM_CONFIRM)?;
    let confirm = u64::from_le_bytes(buf.get(confirm_at..confirm_at + 8)?.try_into().ok()?);
    if confirm != u64::from(element_id) {
        return None;
    }
    let level_at = owner_at.checked_add(ROOM_PARAM_LEVEL)?;
    let level = u64::from_le_bytes(buf.get(level_at..level_at + 8)?.try_into().ok()?);

    let number_len_at = owner_at.checked_add(ROOM_PARAM_NUMBER_LEN)?;
    let number_chars =
        u32::from_le_bytes(buf.get(number_len_at..number_len_at + 4)?.try_into().ok()?) as usize;
    if !(1..=64).contains(&number_chars) {
        return None;
    }
    let number = utf16_at(buf, number_len_at + 4, number_chars)?;

    let name_len_at = number_len_at + 4 + number_chars * 2 + ROOM_PARAM_NAME_GAP;
    let name_chars =
        u32::from_le_bytes(buf.get(name_len_at..name_len_at + 4)?.try_into().ok()?) as usize;
    if !(1..=64).contains(&name_chars) {
        return None;
    }
    let name = utf16_at(buf, name_len_at + 4, name_chars)?;

    if number.chars().any(|c| c.is_control()) || name.chars().any(|c| c.is_control()) {
        return None;
    }
    Some((number, name, level))
}

/// A UTF-16LE string of `chars` units at `at`.
fn utf16_at(buf: &[u8], at: usize, chars: usize) -> Option<String> {
    let bytes = buf.get(at..at + chars * 2)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&units).ok()
}

/// First occurrence of `needle` in `haystack`.
fn memchr_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    memchr::memmem::find(haystack, needle)
}

/// Quantise a coordinate to 1e-6 ft for tolerant byte matching.
fn quantise(v: f64) -> i64 {
    (v * 1.0e6).round() as i64
}

/// The UTF-16LE parameter value whose RE-22 owner slot sits at
/// `owner_at` and whose confirmation slot agrees, or `None`.
fn owner_framed_string(buf: &[u8], owner_at: usize, element_id: u32) -> Option<String> {
    use rvt::partition_ifc_export_overrides as ovr;
    let value_at = owner_at.checked_add(ovr::OWNER_OFFSET_BEFORE_VALUE)?;
    let confirm_at = value_at.checked_sub(ovr::OWNER_CONFIRM_OFFSET_BEFORE_VALUE)?;
    let confirm = u64::from_le_bytes(buf.get(confirm_at..confirm_at + 8)?.try_into().ok()?);
    if confirm != u64::from(element_id) {
        return None;
    }
    let len_at = value_at.checked_sub(ovr::LENGTH_PREFIX_OFFSET_BEFORE_VALUE)?;
    let chars = u32::from_le_bytes(buf.get(len_at..len_at + 4)?.try_into().ok()?) as usize;
    if !(1..=ovr::MAX_VALUE_CHARS).contains(&chars) {
        return None;
    }
    let bytes = buf.get(value_at..value_at + chars * 2)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    if text.chars().any(|c| c.is_control()) {
        return None;
    }
    Some(text)
}

/// Two adjacent finite doubles at `at`, plan-plausible.
fn read_pair(buf: &[u8], at: usize) -> Option<(f64, f64)> {
    let x = f64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?);
    let y = f64::from_le_bytes(buf.get(at + 8..at + 16)?.try_into().ok()?);
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some((x, y))
}
