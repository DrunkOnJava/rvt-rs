//! RE-344 (#155, Discussion #1241): parameter ids, stored lengths and the
//! parameters of a family's forms, read against what mahach666 (RvtIO)
//! reported for them.
//!
//! FACTS under test, all from bytes. Arithmetic: `n / 25.4 / 12` and
//! `n / 304.8` differ in the last bits for 57,067 of the integers 1 to
//! 100,000 (54,471 by one unit in the last place, 2,596 by two). Sign rule: a
//! parameter id in an element's typed sets or in the slots of its
//! `FamilyParams` is negative for a built-in parameter, and a positive id is
//! the ElementId of a definition element (`ParamElem*`) of the same document.
//! Only the positive side can fail: definitions are keyed by positive
//! ElementIds, so a negative id never has one and `negative defined` is zero
//! by construction. The `sign` lines count the ids by store and by what a
//! positive id resolves to, and a `positive undefined` falsifies the rule for
//! the records read. Stored lengths: among the stored doubles that are whole or
//! tenth millimetres and not whole 1/256 inches (RE-310's selection), which of
//! seven conversions reproduce the stored bits, by store (the typed `Double`
//! sets, and the `m_value` of every `FamilyParams` slot) and by kind
//! (built-in, a declared definition, other), on Revit 2015 to 2027 (RE-310 read
//! the type value blocks of 2024 to 2027 only); for a value no conversion
//! reproduces, its distance from `div` and from the nearest of the seven.
//! A distance is in units of the last place of the magnitude: a value of either
//! sign whose magnitude is one unit smaller than the conversion's is -1.
//! Forms: the `Double` sets of a family's forms hold the ids -1001800 and
//! -1001801 on an `ExtrusionElem`, -1001802 and -1001803 on a
//! `RevolutionElem`, -1001804 and -1001805 on a `BlendElem` and a
//! `SweptBlendElem`, and none of them on a `SweepElem`. The probe calls them
//! the first and the second id and takes no meaning for them from the Revit
//! API. A revolution's values are tested as multiples of half of the
//! 14-decimal value 3.14159265358979 (a truncation of pi, 0x400921fb54442d11,
//! seven units in the last place below pi), and against pi by bits.
//! `PartAtom`: the stream of each of the 12 family-corpus files holds no
//! string shaped like a GUID, and a family with shared parameters (RE-327's
//! 2014 family) holds one for each.
//!
//! The seven conversions of a whole or tenth millimetre `mm` to feet: `chain`
//! `mm / 25.4 / 12`, `div` `mm / 304.8`, `mul` `mm * (1 / 304.8)`, `inch`
//! `mm * (1 / 25.4) / 12`, `inv12` `mm / 12 / 25.4`, `metre`
//! `mm / 1000 / 0.3048`, `cmchain` `mm / 10 * (1 / 2.54) * (1 / 12)`.
//!
//! Output, one record per line:
//!   file <name> release <layout release> kind project|family
//!   arith <pair> <ulp distance> <n> ...      the pure arithmetic over 1 to 100000
//!   partatom bytes <n> guid_like <n> | partatom absent
//!   walked groups <n> keyed <n> unframed <n> records <n>    (a family file)
//!   sample ...                               the first 12 stored lengths no
//!                                            conversion, or only some, reproduce
//!   tally <key> <n>                          sorted
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re344_parameter_ids_and_units -- FILE.rvt|FILE.rfa

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{self, Options};
use rvt::research::native_family_forms::family_records_in;
use rvt::research::native_form_geometry;
use rvt::research::native_metadata::{self, SavedMetadata};
use rvt::research::native_parameters::{self, GraphLimits, ObjectGraph};
use serde_json::Value;

type Tally = BTreeMap<String, usize>;
type Owner<'a> = &'a dyn Fn(i64) -> Option<String>;

const LIMITS: GraphLimits = GraphLimits {
    max_values: 2_000_000,
    max_objects: 2_000_000,
    max_depth: 128,
};
const FORMS: [&str; 6] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "SweptBlendElem",
    "FreeFormElement",
];
const CONVERSIONS: [&str; 7] = ["chain", "div", "mul", "inch", "inv12", "metre", "cmchain"];
const EXTRUSION_FIRST: i64 = -1_001_800;
const EXTRUSION_SECOND: i64 = -1_001_801;
const REVOLUTION_FIRST: i64 = -1_001_802;
const REVOLUTION_SECOND: i64 = -1_001_803;
const BLEND_FIRST: i64 = -1_001_804;
const BLEND_SECOND: i64 = -1_001_805;
/// The 14-decimal value the revolutions' angles are multiples of.
#[allow(clippy::approx_constant)]
const TRUNCATED_PI: f64 = 3.14159265358979;
const SIGN: u64 = 1 << 63;

fn bump(tally: &mut Tally, key: impl Into<String>) {
    *tally.entry(key.into()).or_default() += 1;
}

fn conversions(mm: f64) -> [f64; 7] {
    [
        mm / 25.4 / 12.0,
        mm / 304.8,
        mm * (1.0 / 304.8),
        mm * (1.0 / 25.4) / 12.0,
        mm / 12.0 / 25.4,
        mm / 1000.0 / 0.3048,
        mm / 10.0 * (1.0 / 2.54) * (1.0 / 12.0),
    ]
}

/// `a`'s magnitude less `b`'s, in units of the last place: the same for a
/// negative value as for the positive one of the same magnitude.
fn ulps(a: f64, b: f64) -> i64 {
    ((a.to_bits() & !SIGN) as i64).wrapping_sub((b.to_bits() & !SIGN) as i64)
}

/// The `k` for which `angle` is `k` half turns of [`TRUNCATED_PI`], by bits.
fn half_turns(angle: f64) -> Option<u32> {
    (0..=8u32).find(|k| (f64::from(*k) * TRUNCATED_PI / 2.0).to_bits() == angle.to_bits())
}

fn bucket(distance: i64) -> String {
    if distance.abs() <= 3 {
        distance.to_string()
    } else {
        "over 3".to_string()
    }
}

fn identifier(value: &Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    inner.as_i64().or_else(|| {
        inner
            .get("m_id64")
            .or_else(|| inner.get("m_id"))
            .and_then(Value::as_i64)
    })
}

fn arithmetic() {
    let mut chain: BTreeMap<i64, usize> = BTreeMap::new();
    let mut mul: BTreeMap<i64, usize> = BTreeMap::new();
    let mut inch: BTreeMap<i64, usize> = BTreeMap::new();
    let mut cmchain: BTreeMap<i64, usize> = BTreeMap::new();
    for n in 1..=100_000u32 {
        let mm = f64::from(n);
        let all = conversions(mm);
        *chain.entry(ulps(all[0], all[1])).or_default() += 1;
        *mul.entry(ulps(all[2], all[1])).or_default() += 1;
        *inch.entry(ulps(all[3], all[1])).or_default() += 1;
        *cmchain.entry(ulps(all[6], all[1])).or_default() += 1;
    }
    for (name, counts) in [
        ("chain", &chain),
        ("mul", &mul),
        ("inch", &inch),
        ("cmchain", &cmchain),
    ] {
        let line: Vec<String> = counts.iter().map(|(d, n)| format!("{d}={n}")).collect();
        println!("arith {name}-minus-div {}", line.join(" "));
    }
    println!("arith 25.4*12 {:?}", 25.4f64 * 12.0);
}

fn guid_at(bytes: &[u8], at: usize) -> bool {
    let mut pos = at;
    for (group, len) in [8usize, 4, 4, 4, 12].into_iter().enumerate() {
        if group > 0 {
            if bytes.get(pos) != Some(&b'-') {
                return false;
            }
            pos += 1;
        }
        for _ in 0..len {
            if !bytes.get(pos).is_some_and(u8::is_ascii_hexdigit) {
                return false;
            }
            pos += 1;
        }
    }
    true
}

fn count_guid_like(bytes: &[u8]) -> usize {
    let mut count = 0;
    let mut at = 0;
    while at + 36 <= bytes.len() {
        if guid_at(bytes, at) {
            count += 1;
            at += 36;
        } else {
            at += 1;
        }
    }
    count
}

fn part_atom(rf: &mut RevitFile) {
    if !rf
        .stream_names()
        .iter()
        .any(|name| name == rvt::streams::PART_ATOM)
    {
        println!("partatom absent");
        return;
    }
    match rf.read_stream(rvt::streams::PART_ATOM) {
        Ok(bytes) => println!(
            "partatom bytes {} guid_like {}",
            bytes.len(),
            count_guid_like(&bytes)
        ),
        Err(error) => println!("partatom unread {error}"),
    }
}

fn sign(tally: &mut Tally, store: &str, storage: &str, id: i64, owner: Owner<'_>) {
    let what = if id == 0 {
        "zero".to_string()
    } else if id < 0 {
        match owner(id) {
            Some(class) => format!("negative defined {class}"),
            None => "negative".to_string(),
        }
    } else {
        match owner(id) {
            Some(class) => format!("positive {class}"),
            None => "positive undefined".to_string(),
        }
    };
    bump(tally, format!("sign {store} {storage} {what}"));
}

fn lengths(
    tally: &mut Tally,
    samples: &mut Vec<String>,
    store: &str,
    id: i64,
    value: f64,
    owner: Owner<'_>,
) {
    if value == 0.0 || !value.is_finite() {
        return;
    }
    let kind = if id < 0 {
        "builtin"
    } else if owner(id).is_some() {
        "declared"
    } else {
        "other"
    };
    let raw = value * 304.8;
    let mm = if (raw - raw.round()).abs() < 1e-6 {
        raw.round()
    } else if (raw * 10.0 - (raw * 10.0).round()).abs() < 1e-5 {
        (raw * 10.0).round() / 10.0
    } else {
        return;
    };
    let prefix = format!("ulp {store} {kind}");
    let steps = value * 3072.0;
    if (steps - steps.round()).abs() < 1e-6 {
        bump(tally, format!("{prefix} imperial"));
        return;
    }
    bump(tally, format!("{prefix} candidates"));
    let all = conversions(mm);
    if all.iter().all(|c| c.to_bits() == all[0].to_bits()) {
        bump(tally, format!("{prefix} agree"));
        return;
    }
    bump(tally, format!("{prefix} differ"));
    let matched: Vec<&str> = CONVERSIONS
        .iter()
        .zip(all.iter())
        .filter(|(_, c)| c.to_bits() == value.to_bits())
        .map(|(name, _)| *name)
        .collect();
    let set = if matched.is_empty() {
        "none".to_string()
    } else {
        matched.join("+")
    };
    bump(tally, format!("{prefix} match {set}"));
    let distance = ulps(value, all[1]);
    if value < 0.0 {
        bump(tally, format!("{prefix} negative values"));
    }
    if matched.is_empty() {
        bump(
            tally,
            format!("{prefix} none distance from div {}", bucket(distance)),
        );
        let nearest = all
            .iter()
            .map(|c| ulps(value, *c).abs())
            .min()
            .unwrap_or(i64::MAX);
        bump(
            tally,
            format!("{prefix} none nearest of seven {}", bucket(nearest)),
        );
    }
    if samples.len() < 12 {
        samples.push(format!(
            "sample store={store} parameter={id} mm={mm} bits={:016x} ulps_from_div={distance} matches={set}",
            value.to_bits()
        ));
    }
}

fn census(
    metadata: &SavedMetadata,
    owner: Owner<'_>,
    tally: &mut Tally,
    samples: &mut Vec<String>,
) {
    for set in &metadata.parameter_sets {
        for parameter in &set.parameters {
            let id = parameter.serialized_parameter_id;
            sign(tally, "typed", &parameter.storage_type, id, owner);
            let double = (parameter.storage_type == "Double")
                .then(|| parameter.raw_value.as_f64())
                .flatten();
            if let Some(value) = double {
                lengths(tally, samples, "typed", id, value, owner);
            }
        }
    }
    for slot in &metadata.family_parameter_slots {
        sign(tally, "slot", "-", slot.parameter_id, owner);
        if let Some(value) = slot.double_slot.as_f64() {
            lengths(tally, samples, "slot", slot.parameter_id, value, owner);
        }
    }
}

fn form(graph: &ObjectGraph, tally: &mut Tally) {
    let Some(root) = graph.objects.first() else {
        return;
    };
    let class = root.class_name.as_str();
    let mut doubles: BTreeMap<i64, f64> = BTreeMap::new();
    for object in &graph.objects {
        let storage = match object.class_name.as_str() {
            "ParamValueSetDouble" => "Double",
            "ParamValueSetInt" => "Integer",
            "ParamValueSetAString" => "String",
            "ParamValueSetElementId" => "ElementId",
            _ => continue,
        };
        let entries = object.fields.get("m_paramSet").and_then(Value::as_array);
        for entry in entries.into_iter().flatten() {
            let Some(id) = entry.get("m_paramId").and_then(identifier) else {
                continue;
            };
            bump(tally, format!("form {class} {storage} {id}"));
            let value = (storage == "Double")
                .then(|| entry.get("m_value").and_then(Value::as_f64))
                .flatten();
            if let Some(value) = value {
                doubles.insert(id, value);
            }
        }
    }
    bump(tally, format!("form {class} records"));
    let pair = |start: i64, end: i64| (doubles.get(&start).copied(), doubles.get(&end).copied());
    match class {
        "ExtrusionElem" => match pair(EXTRUSION_FIRST, EXTRUSION_SECOND) {
            (Some(first), Some(_)) => {
                bump(tally, "extrusion both ids");
                if first == 0.0 {
                    bump(tally, "extrusion first id is zero");
                }
            }
            _ => bump(tally, "extrusion without both ids"),
        },
        "RevolutionElem" => match pair(REVOLUTION_FIRST, REVOLUTION_SECOND) {
            (Some(first), Some(second)) => {
                bump(tally, "revolution both ids");
                bump(tally, format!("revolution first {:016x}", first.to_bits()));
                bump(
                    tally,
                    format!("revolution second {:016x}", second.to_bits()),
                );
                let order = if first < second {
                    "first below second"
                } else {
                    "first not below second"
                };
                bump(tally, format!("revolution {order}"));
                match (half_turns(first), half_turns(second)) {
                    (Some(a), Some(b)) => bump(
                        tally,
                        format!("revolution pair {a} to {b} half turns of 3.14159265358979"),
                    ),
                    _ => bump(tally, "revolution pair not half turns of 3.14159265358979"),
                }
                for (label, value) in [("first", first), ("second", second)] {
                    if value == 0.0 {
                        continue;
                    }
                    let by_bits =
                        (1..=8u32).any(|k| (f64::from(k) * PI / 2.0).to_bits() == value.to_bits());
                    let near = (1..=8u32).any(|k| (value - f64::from(k) * PI / 2.0).abs() < 1e-9);
                    bump(
                        tally,
                        format!("revolution {label} equals a multiple of pi/2 by bits {by_bits}"),
                    );
                    bump(
                        tally,
                        format!("revolution {label} within 1e-9 of a multiple of pi/2 {near}"),
                    );
                }
            }
            _ => bump(tally, "revolution without both ids"),
        },
        "BlendElem" | "SweptBlendElem" => match pair(BLEND_FIRST, BLEND_SECOND) {
            (Some(_), Some(_)) => bump(tally, format!("{class} both ids")),
            _ => bump(tally, format!("{class} without both ids")),
        },
        "SweepElem" => {
            let held = (BLEND_SECOND..=EXTRUSION_FIRST)
                .filter(|id| doubles.contains_key(id))
                .count();
            bump(tally, format!("sweep ids -1001800 to -1001805 held {held}"));
        }
        _ => {}
    }
}

fn family_file(rf: &mut RevitFile, tally: &mut Tally, samples: &mut Vec<String>) -> Result<()> {
    let document = native_form_geometry::open(rf)?;
    let mut definitions: BTreeMap<i64, String> = BTreeMap::new();
    native_form_geometry::walk(rf, &document, &[102], |entry| {
        let class = entry.class.unwrap_or("-");
        if class.starts_with("ParamElem") {
            if let Ok(id) = i64::try_from(entry.id) {
                definitions.insert(id, class.to_string());
            }
            bump(tally, format!("definition {class}"));
        }
        Ok(())
    })?;
    let owner = |id: i64| definitions.get(&id).cloned();
    let walked = native_form_geometry::walk(rf, &document, &[102], |entry| {
        let Ok(graph) =
            native_parameters::decode_graph_with_limits(entry.body, &document.registry, &LIMITS)
        else {
            bump(tally, "record undecoded");
            return Ok(());
        };
        if entry.class.is_some_and(|class| FORMS.contains(&class)) {
            form(&graph, tally);
        }
        match native_metadata::project(&graph) {
            Ok(metadata) => census(&metadata, &owner, tally, samples),
            Err(_) => bump(tally, "metadata refused"),
        }
        Ok(())
    })?;
    println!(
        "walked groups {} keyed {} unframed {} records {}",
        walked.groups, walked.keyed_groups, walked.unframed_groups, walked.records
    );
    Ok(())
}

fn project_file(rf: &mut RevitFile, tally: &mut Tally, samples: &mut Vec<String>) -> Result<()> {
    let records = rvt::elem_table::parse_records(rf)?;
    let ids: BTreeSet<u64> = rvt::elem_table::declared_ids(&records)
        .into_iter()
        .map(u64::from)
        .collect();
    let options = Options {
        selected_ids: ids,
        releases: native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    native_document::extract(rf, &options, |record| {
        bump(tally, "records");
        let Some(metadata) = record.saved_metadata.as_ref() else {
            bump(tally, "records without metadata");
            return Ok(());
        };
        let owner = |id: i64| {
            metadata
                .parameter_definitions
                .iter()
                .find(|definition| definition.parameter_id == id)
                .map(|definition| definition.owner_class.clone())
        };
        census(metadata, &owner, tally, samples);
        Ok(())
    })?;
    let family_options = Options {
        selected_classes: BTreeSet::from(["Family".to_string()]),
        releases: native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    let mut families: BTreeSet<u64> = BTreeSet::new();
    native_document::extract_graphs(rf, &family_options, |record| {
        if record.graph.is_some() {
            families.insert(record.identity.element_id);
        }
        Ok(())
    })?;
    println!("families {}", families.len());
    let classes: BTreeSet<String> = FORMS.iter().map(|class| class.to_string()).collect();
    let forms = family_records_in(
        rf,
        &families,
        &classes,
        &native_document::NATIVE_RELEASES,
        None,
    )?;
    for items in forms.values() {
        for (_, graph) in items {
            form(graph, tally);
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re344_parameter_ids_and_units FILE.rvt|FILE.rfa");
    };
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let family = path.to_ascii_lowercase().ends_with(".rfa");
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, true))?;
    let release = rf.layout_release()?;
    println!(
        "file {name} release {release} kind {}",
        if family { "family" } else { "project" }
    );
    arithmetic();
    part_atom(&mut rf);
    let mut tally = Tally::new();
    let mut samples = Vec::new();
    let result = if family {
        family_file(&mut rf, &mut tally, &mut samples)
    } else {
        project_file(&mut rf, &mut tally, &mut samples)
    };
    if let Err(error) = result {
        println!("native: {error:#}");
    }
    for line in &samples {
        println!("{line}");
    }
    for (key, n) in &tally {
        println!("tally {key} {n}");
    }
    Ok(())
}
