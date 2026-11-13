//! #1839: what stops a beam short of the end of its record box in Revit's export,
//! and does the near face of an element its list names say where?
//!
//! FACT under test: a structural framing record's box runs the beam's location
//! line to its ends, which sit inside the elements it frames into, and Revit's
//! export stops the beam at the near face of the element its reference list names
//! (RE-122 for columns). The rule ships for Revit 2023 (`element_record_beam_cuts::
//! beam_column_trims`). On Revit 2024 the naming holds, but most steel beams of
//! Snowdon Towers' structural sample stop a stored join cutback short of the
//! face, and concrete beams at it, so 2024 and later keep the line's ends. A
//! reporter's Revit 2027 concrete frame has 396 of 719 beams longer than Revit's
//! box (median 296 mm) and none shorter (#1839). The first run of this probe on the
//! BIM4LCA models (RE-455) found the record box ending where Revit's beam ends on
//! nearly every beam, and the beams Revit does shorten naming no column, so the
//! second question is what those beams name.
//!
//! The probe lists, for every placed structural framing record, its box along
//! the longer plan axis, the bounds the 2023 rule would give it, and for each kind
//! of element a beam's list can name (columns of both kinds, walls, other beams) how
//! many it names, how many hold each end and where the beam would stop at an end if
//! the one element of that kind that holds it cut it. It does not compare them with
//! Revit's export: `tools/re/beam_ends_vs_ifc.py` meshes Revit's beam in its own
//! frame and scores the box and each cut against its length. It prints ElementIds
//! and lengths in feet only.
//!
//! Output (one model per run):
//!   file revit=<release> beams=<n> columns=<n> walls=<n>
//!   beam tag=<ElementId> axis=<0|1|-> low=<ft> high=<ft> across=<ft> height=<ft> named=<n>
//!     held_low=<n> held_high=<n> trim_low=<ft|-> trim_high=<ft|->
//!     <kind>_named=<n> <kind>_held=<n>/<n> <kind>_low=<ft|-> <kind>_high=<ft|-> ...
//!     - `axis`: the plan axis the box is longer along (`-` for a square one), `low` and
//!       `high` its bounds on that axis;
//!     - `named`: the columns the beam's list names;
//!     - `held_low` and `held_high`: the named columns that hold each end in plan and
//!       overlap the beam in height (RE-122); one is a cut, two decline the beam;
//!     - `trim_low` and `trim_high`: the bounds on the axis that the 2023 rule gives;
//!     - `<kind>` is `column`, `wall` or `beam`: `_named` counts the elements of that kind
//!       the list names, `_held` the ones that hold each end (the same test as `held_*`),
//!       and `_low` and `_high` the bound on the axis at that end where exactly one holds
//!       it and its near face lies more than 1 mm inside the box, else `-`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1839_beam_column_ends -- MODEL.rvt

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rvt::RevitFile;
use rvt::research::element_record_beam_cuts::beam_column_trims;
use rvt::research::partition_element_records as per;

type Records = BTreeMap<u32, per::PartitionElementRecord>;

/// The kinds of element a beam's list can name, with the categories of each. The columns
/// come first and the walls second: `main` reads them by position.
static KINDS: [(&str, &[i64]); 3] = [
    ("column", &[per::OST_COLUMNS, per::OST_STRUCTURAL_COLUMNS]),
    ("wall", &[per::OST_WALLS]),
    ("beam", &[per::OST_STRUCTURAL_FRAMING]),
];

/// A cut shorter than this does not count as one, feet (1 mm).
const MIN_CUT_FEET: f64 = 0.0033;

/// The first placed-instance record of each ElementId among `records` in `categories`.
fn instances(records: &[per::PartitionElementRecord], categories: &[i64]) -> Records {
    let mut out = BTreeMap::new();
    for record in records {
        if categories.contains(&record.builtin_category) && record.is_exported_instance() {
            out.entry(record.element_id)
                .or_insert_with(|| record.clone());
        }
    }
    out
}

/// The elements of `kind` that `beam`'s list names and that hold `end` on `axis`: they
/// contain it along the axis and the beam's middle across it, and overlap the beam in
/// height (RE-122).
fn holders<'a>(
    beam: &per::PartitionElementRecord,
    kind: &'a Records,
    axis: usize,
    end: f64,
) -> Vec<&'a per::PartitionElementRecord> {
    let b = beam.bbox_feet;
    let across = 1 - axis;
    let middle = (b[across] + b[across + 3]) / 2.0;
    kind.values()
        .filter(|member| {
            let c = member.bbox_feet;
            member.element_id != beam.element_id
                && beam.references.contains(&u64::from(member.element_id))
                && c[axis] <= end
                && end <= c[axis + 3]
                && c[across] <= middle
                && middle <= c[across + 3]
                && c[2] < b[5]
                && b[2] < c[5]
        })
        .collect()
}

/// Where `beam` would stop at its low or high end on `axis` if the one element of `kind`
/// that holds that end cut it: that element's near face, when it lies more than
/// [`MIN_CUT_FEET`] inside the beam's box.
fn near_face(
    beam: &per::PartitionElementRecord,
    kind: &Records,
    axis: usize,
    low_end: bool,
) -> Option<f64> {
    let b = beam.bbox_feet;
    let end = if low_end { b[axis] } else { b[axis + 3] };
    let held = holders(beam, kind, axis, end);
    let [member] = held.as_slice() else {
        return None;
    };
    let c = member.bbox_feet;
    let face = if low_end { c[axis + 3] } else { c[axis] };
    (b[axis] + MIN_CUT_FEET < face && face < b[axis + 3] - MIN_CUT_FEET).then_some(face)
}

fn feet(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| format!("{v:.4}"))
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1839_beam_column_ends MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories: Vec<i64> = KINDS
        .iter()
        .flat_map(|(_, kind)| kind.iter().copied())
        .collect();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let kinds: Vec<(&str, Records)> = KINDS
        .iter()
        .map(|(name, kind)| (*name, instances(&records, kind)))
        .collect();
    let beams = instances(&records, &[per::OST_STRUCTURAL_FRAMING]);
    let columns = &kinds[0].1;
    let beam_list: Vec<per::PartitionElementRecord> = beams.values().cloned().collect();
    let column_list: Vec<per::PartitionElementRecord> = columns.values().cloned().collect();
    let trims = beam_column_trims(&beam_list, &column_list);
    println!(
        "file revit={version} beams={} columns={} walls={}",
        beams.len(),
        columns.len(),
        kinds[1].1.len()
    );
    for (id, beam) in &beams {
        let b = beam.bbox_feet;
        let (along_x, along_y) = (b[3] - b[0], b[4] - b[1]);
        let named = columns
            .keys()
            .filter(|column| beam.references.contains(&u64::from(**column)))
            .count();
        if along_x == along_y {
            println!(
                "beam tag={id} axis=- low=0 high=0 across=0 height={:.4} named={named} held_low=0 held_high=0 trim_low=- trim_high=-",
                b[5] - b[2]
            );
            continue;
        }
        let axis = usize::from(along_y > along_x);
        let across = 1 - axis;
        let (trim_low, trim_high) = match trims.get(id) {
            Some(trim) => (
                format!("{:.4}", trim.low_feet),
                format!("{:.4}", trim.high_feet),
            ),
            None => ("-".to_string(), "-".to_string()),
        };
        let mut line = format!(
            "beam tag={id} axis={axis} low={:.4} high={:.4} across={:.4} height={:.4} named={named} held_low={} held_high={} trim_low={trim_low} trim_high={trim_high}",
            b[axis],
            b[axis + 3],
            b[across + 3] - b[across],
            b[5] - b[2],
            holders(beam, columns, axis, b[axis]).len(),
            holders(beam, columns, axis, b[axis + 3]).len(),
        );
        for (name, kind) in &kinds {
            let named = kind
                .values()
                .filter(|member| {
                    member.element_id != *id
                        && beam.references.contains(&u64::from(member.element_id))
                })
                .count();
            write!(
                line,
                " {name}_named={named} {name}_held={}/{} {name}_low={} {name}_high={}",
                holders(beam, kind, axis, b[axis]).len(),
                holders(beam, kind, axis, b[axis + 3]).len(),
                feet(near_face(beam, kind, axis, true)),
                feet(near_face(beam, kind, axis, false)),
            )?;
        }
        println!("{line}");
    }
    Ok(())
}
