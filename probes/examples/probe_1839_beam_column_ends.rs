//! #1839: where does Revit's export stop a beam that frames into a column, and
//! does the near face of the column the beam's record names say?
//!
//! FACT under test: a structural framing record's box runs the beam's location
//! line to its ends, which sit inside the columns it frames into, and Revit's
//! export stops the beam at the near face of the column its reference list names
//! (RE-122). The rule ships for Revit 2023 (`element_record_beam_cuts::
//! beam_column_trims`). On Revit 2024 the naming holds, but most steel beams of
//! Snowdon Towers' structural sample stop a stored join cutback short of the
//! face, and concrete beams at it, so 2024 and later keep the line's ends. A
//! reporter's Revit 2027 concrete frame has 396 of 719 beams longer than Revit's
//! box (median 296 mm) and none shorter (#1839).
//!
//! The probe lists, for every placed structural framing record, its box along
//! the longer plan axis, the columns (architectural and structural) its list
//! names that hold each end, and the bounds the 2023 rule would give it. It
//! does not compare them with Revit's export: `tools/re/beam_ends_vs_ifc.py`
//! meshes Revit's beam in its own frame and scores the box and the trim against
//! its length. It prints ElementIds and lengths in feet only.
//!
//! Output (one model per run):
//!   file revit=<release> beams=<n> columns=<n>
//!   beam tag=<ElementId> axis=<0|1|-> low=<ft> high=<ft> across=<ft> height=<ft> named=<n>
//!     held_low=<n> held_high=<n> trim_low=<ft|-> trim_high=<ft|->
//!     - `axis`: the plan axis the box is longer along (`-` for a square one), `low` and
//!       `high` its bounds on that axis;
//!     - `named`: the columns the beam's list names;
//!     - `held_low` and `held_high`: the named columns that hold each end in plan and
//!       overlap the beam in height (RE-122); one is a cut, two decline the beam;
//!     - `trim_low` and `trim_high`: the bounds on the axis that the 2023 rule gives.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1839_beam_column_ends -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::research::element_record_beam_cuts::beam_column_trims;
use rvt::research::partition_element_records as per;

/// The first placed-instance record of each ElementId among `records` in `category`.
fn instances(
    records: &[per::PartitionElementRecord],
    categories: &[i64],
) -> BTreeMap<u32, per::PartitionElementRecord> {
    let mut out = BTreeMap::new();
    for record in records {
        if categories.contains(&record.builtin_category) && record.is_exported_instance() {
            out.entry(record.element_id)
                .or_insert_with(|| record.clone());
        }
    }
    out
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1839_beam_column_ends MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = [
        per::OST_STRUCTURAL_FRAMING,
        per::OST_COLUMNS,
        per::OST_STRUCTURAL_COLUMNS,
    ];
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let beams = instances(&records, &[per::OST_STRUCTURAL_FRAMING]);
    let columns = instances(&records, &[per::OST_COLUMNS, per::OST_STRUCTURAL_COLUMNS]);
    let beam_list: Vec<per::PartitionElementRecord> = beams.values().cloned().collect();
    let column_list: Vec<per::PartitionElementRecord> = columns.values().cloned().collect();
    let trims = beam_column_trims(&beam_list, &column_list);
    println!(
        "file revit={version} beams={} columns={}",
        beams.len(),
        columns.len()
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
        let middle = (b[across] + b[across + 3]) / 2.0;
        let held = |end: f64| -> usize {
            columns
                .values()
                .filter(|column| {
                    let c = column.bbox_feet;
                    beam.references.contains(&u64::from(column.element_id))
                        && c[axis] <= end
                        && end <= c[axis + 3]
                        && c[across] <= middle
                        && middle <= c[across + 3]
                        && c[2] < b[5]
                        && b[2] < c[5]
                })
                .count()
        };
        let (trim_low, trim_high) = match trims.get(id) {
            Some(trim) => (
                format!("{:.4}", trim.low_feet),
                format!("{:.4}", trim.high_feet),
            ),
            None => ("-".to_string(), "-".to_string()),
        };
        println!(
            "beam tag={id} axis={axis} low={:.4} high={:.4} across={:.4} height={:.4} named={named} held_low={} held_high={} trim_low={trim_low} trim_high={trim_high}",
            b[axis],
            b[axis + 3],
            b[across + 3] - b[across],
            b[5] - b[2],
            held(b[axis]),
            held(b[axis + 3]),
        );
    }
    Ok(())
}
