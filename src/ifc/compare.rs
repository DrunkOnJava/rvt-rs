//! Lightweight IFC4 STEP comparison for export QA (M5-05 / #41).
//!
//! Parses ISO-10303-21 text without a full EXPRESS engine and summarizes
//! the dimensions that matter when comparing an `rvt-rs` export against a
//! reference IFC (for example a Revit IFC export of the same model):
//!
//! - entity-type histograms
//! - building storey names + elevations
//! - axis-aligned bbox from `IFCCARTESIANPOINT` coordinates
//! - product object names/types (`IfcWall`, `IfcDoor`, …)
//! - material names
//! - property single-value keys
//!
//! This is deliberately approximate: STEP string escaping, multi-line
//! entities, and nested lists are handled well enough for rvt-rs and
//! common Revit exports, not as a general STEP validator.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Snapshot of structural facts extracted from one IFC STEP file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IfcFileSummary {
    /// Uppercase entity type → occurrence count (DATA section only).
    pub entity_counts: BTreeMap<String, usize>,
    /// Storeys in file order: `(name, elevation_metres_or_none)`.
    pub storeys: Vec<StoreySummary>,
    /// Axis-aligned bbox over all Cartesian points with ≥2 coords.
    pub bounding_box: Option<BoundingBox>,
    /// Product-like entities: type + Name attribute when present.
    pub objects: Vec<ObjectSummary>,
    /// Distinct `IfcMaterial` names (empty string if unnamed).
    pub materials: BTreeSet<String>,
    /// Distinct `IfcPropertySingleValue` Name keys.
    pub property_keys: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreySummary {
    pub name: String,
    pub elevation: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundingBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectSummary {
    pub ifc_type: String,
    pub name: String,
}

/// Side-by-side comparison of two [`IfcFileSummary`] values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IfcCompareReport {
    pub left_label: String,
    pub right_label: String,
    pub left: IfcFileSummary,
    pub right: IfcFileSummary,
    pub entity_count_deltas: BTreeMap<String, CountDelta>,
    pub storeys_only_left: Vec<StoreySummary>,
    pub storeys_only_right: Vec<StoreySummary>,
    pub objects_only_left: Vec<ObjectSummary>,
    pub objects_only_right: Vec<ObjectSummary>,
    pub materials_only_left: Vec<String>,
    pub materials_only_right: Vec<String>,
    pub property_keys_only_left: Vec<String>,
    pub property_keys_only_right: Vec<String>,
    pub bounding_box_delta: Option<BoundingBoxDelta>,
    /// Human-oriented notes (known catalogued divergences, empty sides, …).
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CountDelta {
    pub left: usize,
    pub right: usize,
    pub delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoundingBoxDelta {
    pub left: Option<BoundingBox>,
    pub right: Option<BoundingBox>,
    /// Absolute max-corner / min-corner differences when both present.
    pub max_extent_delta: Option<[f64; 3]>,
}

/// Parse an IFC STEP document into a comparable summary.
pub fn summarize_ifc_step(text: &str) -> IfcFileSummary {
    let mut summary = IfcFileSummary::default();
    let mut bbox_min = [f64::INFINITY; 3];
    let mut bbox_max = [f64::NEG_INFINITY; 3];
    let mut saw_point = false;

    for entity in iter_data_entities(text) {
        let (type_name, args) = match split_entity(&entity) {
            Some(parts) => parts,
            None => continue,
        };
        *summary.entity_counts.entry(type_name.clone()).or_insert(0) += 1;

        match type_name.as_str() {
            "IFCBUILDINGSTOREY" => {
                let name = nth_string_arg(&args, 2).unwrap_or_else(|| "(unnamed)".into());
                let elevation = trailing_numeric_arg(&args);
                summary.storeys.push(StoreySummary { name, elevation });
            }
            "IFCCARTESIANPOINT" => {
                if let Some(coords) = parse_coord_list(&args) {
                    let dims = coords.len().min(3);
                    for i in 0..dims {
                        saw_point = true;
                        bbox_min[i] = bbox_min[i].min(coords[i]);
                        bbox_max[i] = bbox_max[i].max(coords[i]);
                    }
                    // 2D points: treat Z as 0 for bbox completeness.
                    if dims == 2 {
                        saw_point = true;
                        bbox_min[2] = bbox_min[2].min(0.0);
                        bbox_max[2] = bbox_max[2].max(0.0);
                    }
                }
            }
            "IFCMATERIAL" => {
                let name = nth_string_arg(&args, 0).unwrap_or_default();
                summary.materials.insert(name);
            }
            "IFCPROPERTYSINGLEVALUE" => {
                if let Some(key) = nth_string_arg(&args, 0) {
                    summary.property_keys.insert(key);
                }
            }
            other if is_product_type(other) => {
                let name = nth_string_arg(&args, 2).unwrap_or_default();
                summary.objects.push(ObjectSummary {
                    ifc_type: type_name,
                    name,
                });
            }
            _ => {}
        }
    }

    if saw_point {
        summary.bounding_box = Some(BoundingBox {
            min: bbox_min,
            max: bbox_max,
        });
    }

    summary.objects.sort();
    summary
}

/// Compare two summaries. `left` is typically the rvt-rs export;
/// `right` is the reference (for example Revit IFC).
pub fn compare_summaries(
    left_label: impl Into<String>,
    left: IfcFileSummary,
    right_label: impl Into<String>,
    right: IfcFileSummary,
) -> IfcCompareReport {
    let left_label = left_label.into();
    let right_label = right_label.into();

    let mut types: BTreeSet<&str> = BTreeSet::new();
    types.extend(left.entity_counts.keys().map(String::as_str));
    types.extend(right.entity_counts.keys().map(String::as_str));

    let mut entity_count_deltas = BTreeMap::new();
    for ty in types {
        let l = *left.entity_counts.get(ty).unwrap_or(&0);
        let r = *right.entity_counts.get(ty).unwrap_or(&0);
        if l != r {
            entity_count_deltas.insert(
                ty.to_string(),
                CountDelta {
                    left: l,
                    right: r,
                    delta: l as i64 - r as i64,
                },
            );
        }
    }

    let left_storey_keys: BTreeSet<(String, Option<String>)> = left
        .storeys
        .iter()
        .map(|s| (s.name.clone(), s.elevation.map(|e| format!("{e:.6}"))))
        .collect();
    let right_storey_keys: BTreeSet<(String, Option<String>)> = right
        .storeys
        .iter()
        .map(|s| (s.name.clone(), s.elevation.map(|e| format!("{e:.6}"))))
        .collect();

    let storeys_only_left: Vec<StoreySummary> = left
        .storeys
        .iter()
        .filter(|s| {
            !right_storey_keys.contains(&(s.name.clone(), s.elevation.map(|e| format!("{e:.6}"))))
        })
        .cloned()
        .collect();
    let storeys_only_right: Vec<StoreySummary> = right
        .storeys
        .iter()
        .filter(|s| {
            !left_storey_keys.contains(&(s.name.clone(), s.elevation.map(|e| format!("{e:.6}"))))
        })
        .cloned()
        .collect();

    let left_objects: BTreeSet<_> = left.objects.iter().cloned().collect();
    let right_objects: BTreeSet<_> = right.objects.iter().cloned().collect();
    let objects_only_left: Vec<_> = left_objects.difference(&right_objects).cloned().collect();
    let objects_only_right: Vec<_> = right_objects.difference(&left_objects).cloned().collect();

    let materials_only_left: Vec<_> = left
        .materials
        .difference(&right.materials)
        .cloned()
        .collect();
    let materials_only_right: Vec<_> = right
        .materials
        .difference(&left.materials)
        .cloned()
        .collect();

    let property_keys_only_left: Vec<_> = left
        .property_keys
        .difference(&right.property_keys)
        .cloned()
        .collect();
    let property_keys_only_right: Vec<_> = right
        .property_keys
        .difference(&left.property_keys)
        .cloned()
        .collect();

    let bounding_box_delta = match (&left.bounding_box, &right.bounding_box) {
        (None, None) => None,
        (l, r) => {
            let max_extent_delta = match (l, r) {
                (Some(a), Some(b)) => Some([
                    (a.max[0] - a.min[0]) - (b.max[0] - b.min[0]),
                    (a.max[1] - a.min[1]) - (b.max[1] - b.min[1]),
                    (a.max[2] - a.min[2]) - (b.max[2] - b.min[2]),
                ]),
                _ => None,
            };
            Some(BoundingBoxDelta {
                left: l.clone(),
                right: r.clone(),
                max_extent_delta,
            })
        }
    };

    let mut notes = Vec::new();
    notes.extend(catalogued_divergence_notes(&entity_count_deltas));
    if left.objects.is_empty() && !right.objects.is_empty() {
        notes.push(
            "Left IFC has no product entities; rvt-rs may still be scaffold-only for this input."
                .into(),
        );
    }

    IfcCompareReport {
        left_label,
        right_label,
        left,
        right,
        entity_count_deltas,
        storeys_only_left,
        storeys_only_right,
        objects_only_left,
        objects_only_right,
        materials_only_left,
        materials_only_right,
        property_keys_only_left,
        property_keys_only_right,
        bounding_box_delta,
        notes,
    }
}

/// Render a concise human-readable summary for stdout.
pub fn format_human_report(report: &IfcCompareReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "=== IFC compare: {}  vs  {} ===\n",
        report.left_label, report.right_label
    ));

    let left_total: usize = report.left.entity_counts.values().sum();
    let right_total: usize = report.right.entity_counts.values().sum();
    out.push_str(&format!(
        "entities: left={left_total} right={right_total} differing_types={}\n",
        report.entity_count_deltas.len()
    ));

    if !report.entity_count_deltas.is_empty() {
        out.push_str("entity count deltas (left - right):\n");
        for (ty, d) in report.entity_count_deltas.iter().take(40) {
            out.push_str(&format!(
                "  {ty:<28} left={:<5} right={:<5} delta={:+}\n",
                d.left, d.right, d.delta
            ));
        }
        if report.entity_count_deltas.len() > 40 {
            out.push_str(&format!(
                "  … {} more types\n",
                report.entity_count_deltas.len() - 40
            ));
        }
    }

    out.push_str(&format!(
        "storeys: left={} right={} only_left={} only_right={}\n",
        report.left.storeys.len(),
        report.right.storeys.len(),
        report.storeys_only_left.len(),
        report.storeys_only_right.len()
    ));
    out.push_str(&format!(
        "objects: left={} right={} only_left={} only_right={}\n",
        report.left.objects.len(),
        report.right.objects.len(),
        report.objects_only_left.len(),
        report.objects_only_right.len()
    ));
    out.push_str(&format!(
        "materials: left={} right={} only_left={} only_right={}\n",
        report.left.materials.len(),
        report.right.materials.len(),
        report.materials_only_left.len(),
        report.materials_only_right.len()
    ));
    out.push_str(&format!(
        "property keys: left={} right={} only_left={} only_right={}\n",
        report.left.property_keys.len(),
        report.right.property_keys.len(),
        report.property_keys_only_left.len(),
        report.property_keys_only_right.len()
    ));

    match &report.bounding_box_delta {
        Some(bb) => match (&bb.left, &bb.right, bb.max_extent_delta) {
            (Some(l), Some(r), Some(d)) => {
                out.push_str(&format!(
                    "bbox extent delta (left-right): [{:.4}, {:.4}, {:.4}]\n",
                    d[0], d[1], d[2]
                ));
                out.push_str(&format!(
                    "  left  min {:?} max {:?}\n  right min {:?} max {:?}\n",
                    l.min, l.max, r.min, r.max
                ));
            }
            (Some(_), None, _) => out.push_str("bbox: present on left only\n"),
            (None, Some(_), _) => out.push_str("bbox: present on right only\n"),
            _ => out.push_str("bbox: unavailable on both sides\n"),
        },
        None => out.push_str("bbox: unavailable on both sides\n"),
    }

    if !report.notes.is_empty() {
        out.push_str("notes:\n");
        for note in &report.notes {
            out.push_str(&format!("  - {note}\n"));
        }
    }

    out
}

fn catalogued_divergence_notes(deltas: &BTreeMap<String, CountDelta>) -> Vec<String> {
    let mut notes = Vec::new();
    // Keep these tied to open RE-15 / CLASS issues so QA reports stay honest.
    const CATALOGUE: &[(&str, &str)] = &[
        (
            "IFCWALL",
            "Wall recall gaps tracked in #81 (RE-15-01); expect left < right on real projects until ≥95%.",
        ),
        ("IFCDOOR", "Door recall gaps tracked in #82 (RE-15-02)."),
        (
            "IFCSLAB",
            "Slab recall / profile gaps tracked in #83 (RE-15-03) and #87 (RE-15-07).",
        ),
        (
            "IFCSPACE",
            "Space recall gaps tracked in #84 (RE-15-04) and #90 (RE-15-10).",
        ),
        (
            "IFCWINDOW",
            "Window decoder still open as #91 (CLASS-11); left often zero vs Revit reference.",
        ),
        (
            "IFCMATERIALLAYERSETUSAGE",
            "Compound layer thicknesses tracked in #88 (RE-15-08).",
        ),
        (
            "IFCOPENINGELEMENT",
            "Opening / void relationships tracked in #89 (RE-15-09).",
        ),
    ];
    for (ty, note) in CATALOGUE {
        if deltas.contains_key(*ty) {
            notes.push((*note).to_string());
        }
    }
    notes
}

fn is_product_type(ty: &str) -> bool {
    matches!(
        ty,
        "IFCWALL"
            | "IFCWALLSTANDARDCASE"
            | "IFCSLAB"
            | "IFCROOF"
            | "IFCCEILING"
            | "IFCCOVERING"
            | "IFCDOOR"
            | "IFCWINDOW"
            | "IFCCOLUMN"
            | "IFCBEAM"
            | "IFCMEMBER"
            | "IFCSTAIR"
            | "IFCRAILING"
            | "IFCSPACE"
            | "IFCBUILDINGELEMENTPROXY"
            | "IFCFURNISHINGELEMENT"
            | "IFCFLOWTERMINAL"
            | "IFCFLOWSEGMENT"
            | "IFCOPENINGELEMENT"
    )
}

fn iter_data_entities(text: &str) -> impl Iterator<Item = String> + '_ {
    let data = extract_data_section(text);
    let mut out = Vec::new();
    let mut rest = data;
    while let Some(hash) = rest.find('#') {
        rest = &rest[hash..];
        let Some(eq) = rest.find('=') else { break };
        let after_eq = &rest[eq + 1..];
        let Some(end) = find_entity_end(after_eq) else {
            break;
        };
        let entity = after_eq[..end].trim();
        if !entity.is_empty() {
            out.push(entity.to_string());
        }
        rest = &after_eq[end + 1..];
    }
    out.into_iter()
}

fn extract_data_section(text: &str) -> &str {
    let upper = text; // STEP keywords are conventionally uppercase
    if let Some(start) = find_ci(upper, "DATA;") {
        let body = &upper[start + 5..];
        if let Some(end) = find_ci(body, "ENDSEC;") {
            return &body[..end];
        }
        return body;
    }
    text
}

fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

fn find_entity_end(s: &str) -> Option<usize> {
    let mut in_string = false;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if in_string {
            if c == '\'' {
                // STEP doubled quote ''
                if matches!(chars.peek(), Some((_, '\''))) {
                    chars.next();
                    continue;
                }
                in_string = false;
            }
            continue;
        }
        match c {
            '\'' => in_string = true,
            ';' => return Some(i),
            _ => {}
        }
    }
    None
}

fn split_entity(entity: &str) -> Option<(String, String)> {
    let entity = entity.trim().trim_end_matches(';').trim();
    let paren = entity.find('(')?;
    let type_name = entity[..paren].trim().to_ascii_uppercase();
    if type_name.is_empty() {
        return None;
    }
    // TYPE(args) — strip the matching outer call parentheses only.
    let inner = entity[paren + 1..].trim_end();
    let args = inner.strip_suffix(')').unwrap_or(inner).to_string();
    Some((type_name, args))
}

fn nth_string_arg(args: &str, n: usize) -> Option<String> {
    let parts = split_top_level_args(args);
    let raw = parts.get(n)?;
    parse_step_string(raw.trim())
}

fn trailing_numeric_arg(args: &str) -> Option<f64> {
    let parts = split_top_level_args(args);
    let last = parts.last()?.trim();
    if last == "$" || last.is_empty() {
        return None;
    }
    last.parse::<f64>().ok()
}

fn parse_coord_list(args: &str) -> Option<Vec<f64>> {
    let trimmed = args.trim();
    let list_src = if trimmed.starts_with('(') {
        trimmed.to_string()
    } else {
        split_top_level_args(trimmed).into_iter().next()?
    };
    let t = list_src.trim();
    let inner = t
        .strip_prefix('(')?
        .strip_suffix(')')
        .unwrap_or(t.strip_prefix('(')?);
    let mut coords = Vec::new();
    for part in inner.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        coords.push(p.parse::<f64>().ok()?);
    }
    if coords.len() >= 2 {
        Some(coords)
    } else {
        None
    }
}

fn split_top_level_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut chars = args.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            cur.push(c);
            if c == '\'' {
                if matches!(chars.peek(), Some('\'')) {
                    cur.push(chars.next().unwrap());
                    continue;
                }
                in_string = false;
            }
            continue;
        }
        match c {
            '\'' => {
                in_string = true;
                cur.push(c);
            }
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_step_string(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw == "$" {
        return Some(String::new());
    }
    let inner = raw.strip_prefix('\'')?.strip_suffix('\'')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' && matches!(chars.peek(), Some('\'')) {
            chars.next();
            out.push('\'');
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('gid',$,'Demo',$,$,$,$,$,$);
#2=IFCBUILDINGSTOREY('g',$,'Level 1',$,$,$,$,'Level 1',.ELEMENT.,0.);
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCCARTESIANPOINT((10.,5.,3.));
#5=IFCMATERIAL('Concrete',$,$);
#6=IFCWALL('g',$,'Wall A',$,$,$,$,$);
#7=IFCPROPERTYSINGLEVALUE('Height',$,IFCLENGTHMEASURE(3.),$);
ENDSEC;
END-ISO-10303-21;
"#;

    #[test]
    fn summarize_counts_storeys_bbox_materials_props() {
        let s = summarize_ifc_step(MINI);
        assert_eq!(s.entity_counts.get("IFCWALL"), Some(&1));
        assert_eq!(s.storeys.len(), 1);
        assert_eq!(s.storeys[0].name, "Level 1");
        assert_eq!(s.storeys[0].elevation, Some(0.0));
        let bb = s.bounding_box.expect("bbox");
        assert_eq!(bb.min, [0.0, 0.0, 0.0]);
        assert_eq!(bb.max, [10.0, 5.0, 3.0]);
        assert!(s.materials.contains("Concrete"));
        assert!(s.property_keys.contains("Height"));
        assert_eq!(s.objects[0].name, "Wall A");
    }

    #[test]
    fn compare_identical_is_empty_deltas() {
        let a = summarize_ifc_step(MINI);
        let report = compare_summaries("a", a.clone(), "b", a);
        assert!(report.entity_count_deltas.is_empty());
        assert!(report.objects_only_left.is_empty());
        assert!(report.objects_only_right.is_empty());
    }

    #[test]
    fn compare_detects_missing_wall_and_catalog_note() {
        let left = summarize_ifc_step(MINI);
        let right = summarize_ifc_step(&MINI.replace(
            "#6=IFCWALL('g',$,'Wall A',$,$,$,$,$);",
            "#6=IFCWALL('g',$,'Wall A',$,$,$,$,$);\n#8=IFCWALL('g',$,'Wall B',$,$,$,$,$);",
        ));
        let report = compare_summaries("rvt", left, "revit", right);
        let wall = report.entity_count_deltas.get("IFCWALL").expect("delta");
        assert_eq!(wall.left, 1);
        assert_eq!(wall.right, 2);
        assert!(
            report
                .notes
                .iter()
                .any(|n| n.contains("#81") || n.contains("RE-15-01"))
        );
        let human = format_human_report(&report);
        assert!(human.contains("IFCWALL"));
    }

    #[test]
    fn fixture_synthetic_project_parses() {
        let text = include_str!("../../tests/fixtures/synthetic-project.ifc");
        let s = summarize_ifc_step(text);
        assert!(s.entity_counts.get("IFCWALL").copied().unwrap_or(0) >= 1);
        assert!(s.storeys.len() >= 3);
        assert!(!s.materials.is_empty());
        assert!(s.bounding_box.is_some());
    }
}
