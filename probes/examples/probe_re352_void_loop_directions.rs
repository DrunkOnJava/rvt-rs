//! RE-352 (#1408): which way the sketch lines of a floor's void run, and
//! whether that tells which line Revit's export tags the void's opening with.
//!
//! FACT under test (RE-339): Revit tags a floor's void with the lowest
//! ElementId of its loop's lines when they were drawn counter-clockwise and
//! with the last line drawn when they were drawn clockwise, and the void
//! 3147725 of the BIM4LCA office, a loop of 6 lines with a notch drawn later,
//! is neither. This probe reads the direction two ways for every void in
//! Revit's export whose host is not a wall, and prints the line each reading
//! says Revit tags, beside the tag:
//!
//! - `ids`: the lowest ElementId `a` and the second lowest `b` of the loop's
//!   lines (the first two lines drawn); `b` next to `a` round the loop on the
//!   counter-clockwise side says it was drawn counter-clockwise, tag `a`; on
//!   the clockwise side, drawn clockwise, tag the line round the loop from `a`
//!   on the counter-clockwise side, which is the line drawn last when the loop
//!   was drawn in one go. Anywhere else `?`.
//! - `stored`: the direction each line is stored in (its `GLine`'s origin,
//!   direction and end parameters): where every line starts where another ends
//!   they make one oriented loop, and its direction is counter-clockwise or
//!   clockwise. Counter-clockwise gives the `ids` reading's clockwise tag, and
//!   clockwise gives `a`. Lines stored against the rest (a notch drawn later)
//!   leave `nochain`.
//!
//! A loop is the sketch lines of the void's host that meet end to end (their
//! `GLine` ends within 0.002 ft); it is read only where each line meets
//! exactly two others and they make one cycle of three lines or more. Each
//! void prints one line:
//!   void <tag> host <host> lines <n> tag <lowest|highest|other> ids <ccw|cw|?> \
//!     stored <ccw|cw|nochain> pred_ids <id|-> pred_stored <id|->
//!   void <tag> host <host> irregular <why>
//! and a last line of counts, `void tags:`, which holds no id.
//!
//! Reads Revit's export beside the model (`<model>.ifc`, `<model>_slim.ifc` or
//! `<model>_Revit.ifc`).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re352_void_loop_directions -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::partition_element_records::{self as per};
use serde_json::Value;

const SKETCH_LINES: i64 = -2_000_045;
const NEAR_FEET: f64 = 2e-3;

/// A straight line's start and end in plan, feet.
type Ends = ([f64; 2], [f64; 2]);

/// `#id -> (entity, args)` for every line of a STEP file.
fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

/// An element's `Tag`: the last quoted string of its arguments that is all
/// digits.
fn tag(args: &str) -> Option<u64> {
    args.split('\'')
        .rfind(|s| !s.is_empty() && s.len() <= 10 && s.chars().all(|c| c.is_ascii_digit()))?
        .parse()
        .ok()
}

struct Opening {
    tag: u64,
    host_class: String,
    host_tag: Option<u64>,
}

/// The openings of a STEP file that nothing fills.
fn read_export(step: &str) -> Vec<Opening> {
    let ents = entities(step);
    let mut host_of = BTreeMap::new();
    let mut filled = BTreeSet::new();
    for (entity, args) in ents.values() {
        let r = refs(args);
        match entity.as_str() {
            "IFCRELVOIDSELEMENT" if r.len() >= 2 => {
                host_of.insert(r[r.len() - 1], r[r.len() - 2]);
            }
            "IFCRELFILLSELEMENT" if r.len() >= 2 => {
                filled.insert(r[r.len() - 2]);
            }
            _ => {}
        }
    }
    let mut openings = Vec::new();
    for (id, (entity, args)) in &ents {
        if entity == "IFCOPENINGELEMENT" && !filled.contains(id) {
            let host = host_of.get(id).and_then(|h| ents.get(h));
            let Some(opening_tag) = tag(args) else {
                continue;
            };
            openings.push(Opening {
                tag: opening_tag,
                host_class: host.map_or_else(|| "none".to_string(), |(e, _)| e.clone()),
                host_tag: host.and_then(|(_, a)| tag(a)),
            });
        }
    }
    openings
}

/// The `N` numbers of a JSON array.
fn numbers<const N: usize>(value: Option<&Value>) -> Option<[f64; N]> {
    let items = value?.as_array()?;
    if items.len() != N {
        return None;
    }
    let mut out = [0.0; N];
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = item.as_f64()?;
    }
    Some(out)
}

/// A `GLine`'s start and end in plan.
fn line_ends(fields: &Value) -> Option<Ends> {
    let origin = numbers::<3>(fields.get("m_origin"))?;
    let direction = numbers::<3>(fields.get("m_dirVec"))?;
    let params = numbers::<2>(fields.get("m_endParams"))?;
    let at = |t: f64| [origin[0] + direction[0] * t, origin[1] + direction[1] * t];
    Some((at(params[0]), at(params[1])))
}

fn near(p: [f64; 2], q: [f64; 2]) -> bool {
    (p[0] - q[0]).hypot(p[1] - q[1]) <= NEAR_FEET
}

/// Whether two lines meet at an end of each.
fn touch(a: &Ends, b: &Ends) -> bool {
    [a.0, a.1]
        .iter()
        .any(|p| [b.0, b.1].iter().any(|q| near(*p, *q)))
}

/// The lines of `host` that meet `tag`'s, end to end, and those they meet.
fn loop_of(
    tag: u64,
    host: u64,
    owner_of: &BTreeMap<u64, u64>,
    geom: &BTreeMap<u64, Ends>,
) -> BTreeSet<u64> {
    let mut found = BTreeSet::from([tag]);
    let mut frontier = vec![tag];
    while let Some(current) = frontier.pop() {
        let Some(here) = geom.get(&current) else {
            continue;
        };
        for (id, there) in geom {
            if owner_of.get(id) == Some(&host) && !found.contains(id) && touch(here, there) {
                found.insert(*id);
                frontier.push(*id);
            }
        }
    }
    found
}

/// The lines of `ids` in the order they follow each other round one cycle,
/// from the lowest id, or `None` unless each meets exactly two others and
/// they make one cycle of at least three.
fn cycle(ids: &BTreeSet<u64>, geom: &BTreeMap<u64, Ends>) -> Option<Vec<u64>> {
    if ids.len() < 3 {
        return None;
    }
    let mut neighbours: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for a in ids {
        for b in ids.range(a + 1..) {
            if touch(geom.get(a)?, geom.get(b)?) {
                neighbours.entry(*a).or_default().push(*b);
                neighbours.entry(*b).or_default().push(*a);
            }
        }
    }
    if ids
        .iter()
        .any(|id| neighbours.get(id).is_none_or(|n| n.len() != 2))
    {
        return None;
    }
    let first = *ids.first()?;
    let mut order = vec![first];
    let mut previous = first;
    let mut current = *neighbours.get(&first)?.first()?;
    while current != first {
        order.push(current);
        if order.len() > ids.len() {
            return None;
        }
        let next = neighbours
            .get(&current)?
            .iter()
            .copied()
            .find(|id| *id != previous)?;
        previous = current;
        current = next;
    }
    (order.len() == ids.len()).then_some(order)
}

/// The vertices of the cycle `order` in the order it is walked: each line's
/// end that is not where the next line starts.
fn ring(order: &[u64], geom: &BTreeMap<u64, Ends>) -> Option<Vec<[f64; 2]>> {
    let mut points = Vec::with_capacity(order.len());
    for (at, id) in order.iter().enumerate() {
        let line = geom.get(id)?;
        let next = geom.get(order.get((at + 1) % order.len())?)?;
        let shared = [next.0, next.1].iter().any(|q| near(line.0, *q));
        points.push(if shared { line.1 } else { line.0 });
    }
    Some(points)
}

fn signed_area(points: &[[f64; 2]]) -> f64 {
    let mut sum = 0.0;
    for (at, p) in points.iter().enumerate() {
        let q = points[(at + 1) % points.len()];
        sum += p[0] * q[1] - q[0] * p[1];
    }
    sum / 2.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    Ccw,
    Cw,
}

impl Way {
    fn word(way: Option<Self>, none: &'static str) -> &'static str {
        match way {
            Some(Self::Ccw) => "ccw",
            Some(Self::Cw) => "cw",
            None => none,
        }
    }
}

/// The direction the lines of `ids` are stored in, where each line's end is
/// the start of exactly one other and they make one oriented cycle.
fn stored_way(ids: &BTreeSet<u64>, geom: &BTreeMap<u64, Ends>) -> Option<Way> {
    let mut next: BTreeMap<u64, u64> = BTreeMap::new();
    for a in ids {
        let end = geom.get(a)?.1;
        let mut followers = ids
            .iter()
            .filter(|b| *b != a && geom.get(*b).is_some_and(|line| near(end, line.0)));
        let first = *followers.next()?;
        if followers.next().is_some() {
            return None;
        }
        next.insert(*a, first);
    }
    let start = *ids.first()?;
    let mut order = vec![start];
    let mut current = *next.get(&start)?;
    while current != start {
        order.push(current);
        if order.len() > ids.len() {
            return None;
        }
        current = *next.get(&current)?;
    }
    if order.len() != ids.len() {
        return None;
    }
    let starts: Vec<[f64; 2]> = order
        .iter()
        .map(|id| geom.get(id).map(|line| line.0))
        .collect::<Option<_>>()?;
    Some(if signed_area(&starts) > 0.0 {
        Way::Ccw
    } else {
        Way::Cw
    })
}

/// What was read of one void's loop.
struct Reading {
    lines: usize,
    lowest: u64,
    highest: u64,
    ids_way: Option<Way>,
    stored: Option<Way>,
    pred_ids: Option<u64>,
    pred_stored: Option<u64>,
}

fn read_loop(
    tag: u64,
    host: u64,
    owner_of: &BTreeMap<u64, u64>,
    geom: &BTreeMap<u64, Ends>,
) -> Result<Reading, &'static str> {
    if !geom.contains_key(&tag) {
        return Err("the tag is not a straight sketch line of the host");
    }
    let ids = loop_of(tag, host, owner_of, geom);
    let Some(order) = cycle(&ids, geom) else {
        return Err("the lines do not make one cycle of lines that each meet two others");
    };
    let Some(points) = ring(&order, geom) else {
        return Err("a line has no ends");
    };
    let mut counter_clockwise = order;
    if signed_area(&points) < 0.0 {
        counter_clockwise[1..].reverse();
    }
    let mut sorted: Vec<u64> = ids.iter().copied().collect();
    sorted.sort_unstable();
    let (Some(&lowest), Some(&second), Some(&highest)) =
        (sorted.first(), sorted.get(1), sorted.last())
    else {
        return Err("fewer than two lines");
    };
    let n = counter_clockwise.len();
    let Some(at) = counter_clockwise.iter().position(|id| *id == lowest) else {
        return Err("the lowest line is not on the cycle");
    };
    let after = counter_clockwise[(at + 1) % n];
    let before = counter_clockwise[(at + n - 1) % n];
    let ids_way = if second == after && second != before {
        Some(Way::Ccw)
    } else if second == before && second != after {
        Some(Way::Cw)
    } else {
        None
    };
    let stored = stored_way(&ids, geom);
    Ok(Reading {
        lines: n,
        lowest,
        highest,
        ids_way,
        stored,
        pred_ids: ids_way.map(|way| if way == Way::Ccw { lowest } else { after }),
        pred_stored: stored.map(|way| if way == Way::Ccw { after } else { lowest }),
    })
}

#[derive(Default)]
struct Counts {
    voids: usize,
    irregular: usize,
    lowest: usize,
    highest: usize,
    other: usize,
    ids_read: usize,
    ids_right: usize,
    stored_read: usize,
    stored_right: usize,
    both_read: usize,
    both_agree: usize,
}

fn probe(path: &str) -> Result<()> {
    let model = PathBuf::from(path);
    let stem = model.with_extension("");
    let export = [
        model.with_extension("ifc"),
        PathBuf::from(format!("{}_slim.ifc", stem.display())),
        PathBuf::from(format!("{}_Revit.ifc", stem.display())),
    ]
    .into_iter()
    .find(|candidate| candidate.exists());
    let Some(export) = export else {
        println!("no Revit export beside the model");
        return Ok(());
    };
    let voids: Vec<Opening> = read_export(&std::fs::read_to_string(&export)?)
        .into_iter()
        .filter(|opening| opening.host_class != "IFCWALL")
        .collect();
    let hosts: BTreeSet<u64> = voids
        .iter()
        .filter_map(|opening| opening.host_tag)
        .collect();

    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let mut owner_of: BTreeMap<u64, u64> = BTreeMap::new();
    if let Some(marker) = per::file_bbox_marker(&mut rf, revit) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for span in per::partition_record_chain(buf, &marker) {
                let Some(frame) =
                    per::decode_frame_as(&stream, buf, span.start, span.element_id as u32, &marker)
                else {
                    continue;
                };
                let Some(owner) = frame
                    .owner_reference
                    .map(u64::from)
                    .filter(|owner| hosts.contains(owner))
                else {
                    continue;
                };
                if frame.builtin_category == SKETCH_LINES {
                    owner_of.entry(span.element_id).or_insert(owner);
                }
            }
        }
    }
    let options = Options {
        selected_ids: owner_of.keys().copied().collect(),
        ..Options::default()
    };
    let mut geom: BTreeMap<u64, Ends> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record: Record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for object in &graph.objects {
            if object.class_name != "GLine" {
                continue;
            }
            if let Some(ends) = line_ends(&object.fields) {
                geom.insert(id, ends);
            }
        }
        Ok(())
    })?;

    let mut counts = Counts::default();
    for opening in &voids {
        let Some(host) = opening.host_tag else {
            continue;
        };
        counts.voids += 1;
        let reading = match read_loop(opening.tag, host, &owner_of, &geom) {
            Ok(reading) => reading,
            Err(why) => {
                counts.irregular += 1;
                println!("void {} host {host} irregular: {why}", opening.tag);
                continue;
            }
        };
        let which = if opening.tag == reading.lowest {
            counts.lowest += 1;
            "lowest"
        } else if opening.tag == reading.highest {
            counts.highest += 1;
            "highest"
        } else {
            counts.other += 1;
            "other"
        };
        if reading.pred_ids.is_some() {
            counts.ids_read += 1;
            counts.ids_right += usize::from(reading.pred_ids == Some(opening.tag));
        }
        if reading.pred_stored.is_some() {
            counts.stored_read += 1;
            counts.stored_right += usize::from(reading.pred_stored == Some(opening.tag));
        }
        if reading.pred_ids.is_some() && reading.pred_stored.is_some() {
            counts.both_read += 1;
            counts.both_agree += usize::from(reading.pred_ids == reading.pred_stored);
        }
        let show = |id: Option<u64>| id.map_or_else(|| "-".to_string(), |id| id.to_string());
        println!(
            "void {} host {host} lines {} tag {which} ids {} stored {} pred_ids {} pred_stored {}",
            opening.tag,
            reading.lines,
            Way::word(reading.ids_way, "?"),
            Way::word(reading.stored, "nochain"),
            show(reading.pred_ids),
            show(reading.pred_stored),
        );
    }
    println!(
        "void tags: voids {}, irregular {}, tag is the lowest id {}, the highest {}, another {}, ids reading {} (right {}), stored reading {} (right {}), both read {} (agree {})",
        counts.voids,
        counts.irregular,
        counts.lowest,
        counts.highest,
        counts.other,
        counts.ids_read,
        counts.ids_right,
        counts.stored_read,
        counts.stored_right,
        counts.both_read,
        counts.both_agree,
    );
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("error {path}: {error:#}");
        }
    }
}
