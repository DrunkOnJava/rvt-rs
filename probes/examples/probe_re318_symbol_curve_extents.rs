//! RE-318 (#810): the extent along X of the curves in a door's or window's
//! symbol graphics, by category and flags.
//!
//! FACT under test: the tree dump of `probe_re318_symbol_graphics_tree`
//! (Measure 41721882540) shows each symbol's saved graphics hold, besides its
//! surfaces, the 12 edges of several boxes as `GLine`s of category -1. The
//! boxes of the O-16 door's symbol 1727541 run from x -6.6109 to 2.4278 ft,
//! 9.0387 ft, Revit's `OverallWidth` where its surfaces give 9.0224 ft; those
//! of the hatch symbols 2058852 (90 degree type) and 1766487 run from -1.5584
//! to 4.3045 ft, 5.8629 ft, Revit's width on both, though the 90 degree
//! type's visible surfaces span 3.1168 ft. This prints, for the symbol
//! behind every door and window the export writes, the extent along X of its
//! `GLine`s and `GArc`s grouped by class, category and `m_GInfo.m_flags`, to
//! find which group is Revit's `OverallWidth` on every one.
//!
//! Output, one record per line:
//!   inst <id> <class> symbol=<drawn by>
//!   cext <symbol> <GLine|GArc> cat=<category> flags=<m_GInfo.m_flags> n=<count> x0=<min> x1=<max>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re318_symbol_curve_extents -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, extract_graphs};

fn element_id(value: &Fv) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")?
        .as_i64()
        .and_then(|n| u64::try_from(n).ok())
}

fn triple(value: Option<&Fv>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    let [a, b, c] = items else {
        return None;
    };
    Some([a.as_f64()?, b.as_f64()?, c.as_f64()?])
}

fn pair(value: Option<&Fv>) -> Option<[f64; 2]> {
    let items = value?.as_array()?;
    let [a, b] = items else {
        return None;
    };
    Some([a.as_f64()?, b.as_f64()?])
}

/// The range along X of a `GLine`: its start and end.
fn line_range(fields: &Fv) -> Option<(f64, f64)> {
    let origin = triple(fields.get("m_origin"))?;
    let direction = triple(fields.get("m_dirVec"))?;
    let [t0, t1] = pair(fields.get("m_endParams"))?;
    let (a, b) = (origin[0] + direction[0] * t0, origin[0] + direction[0] * t1);
    Some((a.min(b), a.max(b)))
}

/// The range along X of a `GArc`: x(t) = cx + r (cos t xv0 + sin t yv0) for t
/// between its end parameters, at its ends and wherever x'(t) is zero.
fn arc_range(fields: &Fv) -> Option<(f64, f64)> {
    let center = triple(fields.get("m_center"))?;
    let radius = fields.get("m_radius")?.as_f64()?;
    let x_vec = triple(fields.get("m_xVec"))?;
    let y_vec = triple(fields.get("m_yVec"))?;
    let [t0, t1] = pair(fields.get("m_endParams"))?;
    let (lo_t, hi_t) = (t0.min(t1), t0.max(t1));
    let at = |t: f64| center[0] + radius * (t.cos() * x_vec[0] + t.sin() * y_vec[0]);
    let mut candidates = vec![lo_t, hi_t];
    let peak = y_vec[0].atan2(x_vec[0]);
    let mut k = ((lo_t - peak) / std::f64::consts::PI).floor() as i64 - 1;
    while (peak + k as f64 * std::f64::consts::PI) <= hi_t {
        let t = peak + k as f64 * std::f64::consts::PI;
        if t >= lo_t {
            candidates.push(t);
        }
        k += 1;
    }
    let values: Vec<f64> = candidates.into_iter().map(at).collect();
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
}

#[derive(Default)]
struct Group {
    count: usize,
    lo: f64,
    hi: f64,
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if ifc_type.eq_ignore_ascii_case("IFCDOOR")
                || ifc_type.eq_ignore_ascii_case("IFCWINDOW")
            {
                if let Ok(id) = tag.parse() {
                    classes.insert(id, ifc_type.to_ascii_uppercase());
                }
            }
        }
    }
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    let options = Options {
        selected_ids: ids,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        if let Some(symbol) = graph
            .objects
            .iter()
            .find(|object| object.class_name == "InstanceInfo")
            .and_then(|object| object.fields.get("m_symbolId"))
            .and_then(element_id)
        {
            drawn_by.insert(record.identity.element_id, symbol);
        }
        Ok(())
    })?;
    for (id, class) in &classes {
        match drawn_by.get(id) {
            Some(symbol) => println!("inst {id} {class} symbol={symbol}"),
            None => println!("inst {id} {class} -"),
        }
    }
    let symbols: BTreeSet<u64> = drawn_by.values().copied().collect();
    let options = Options {
        selected_ids: symbols,
        channels: BTreeSet::from([103]),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let mut groups: BTreeMap<(String, i64, u64), Group> = BTreeMap::new();
        for object in &graph.objects {
            let range = match object.class_name.as_str() {
                "GLine" => line_range(&object.fields),
                "GArc" => arc_range(&object.fields),
                _ => continue,
            };
            let Some((lo, hi)) = range else {
                continue;
            };
            let category = object
                .fields
                .pointer("/m_GInfo/m_categoryId/m_id/m_id64")
                .and_then(Fv::as_i64)
                .unwrap_or(i64::MIN);
            let flags = object
                .fields
                .pointer("/m_GInfo/m_flags")
                .and_then(Fv::as_u64)
                .unwrap_or(0);
            let group = groups
                .entry((object.class_name.clone(), category, flags))
                .or_insert_with(|| Group {
                    count: 0,
                    lo: f64::INFINITY,
                    hi: f64::NEG_INFINITY,
                });
            group.count += 1;
            group.lo = group.lo.min(lo);
            group.hi = group.hi.max(hi);
        }
        for ((class, category, flags), group) in &groups {
            println!(
                "cext {id} {class} cat={category} flags={flags} n={} x0={:.6} x1={:.6}",
                group.count, group.lo, group.hi
            );
        }
        Ok(())
    })?;
    Ok(())
}
