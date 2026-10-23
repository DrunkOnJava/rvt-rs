//! RE-211 (#883): what a roof by extrusion's records hold of its profile and
//! its extent.
//!
//! FACT under test: a roof by extrusion (`ExtrusionRoof`) is sketched in a
//! vertical plane, so its profile is sketch lines whose records name the
//! roof (RE-25's owner reference), each a line or an arc in its element data
//! (RE-50, RE-96), and its extent along the plane's normal is in the roof's
//! own data. rvt-rs writes such a roof as a level block as tall as its
//! record box. This probe prints the raw facts;
//! `tools/re/extrusion_roofs_vs_ifc.py` measures them against the body of the
//! same roof in Revit's export.
//!
//! Output:
//!   roof <id> <class> box=<6 f64> refs=<first reference list>
//!   line <id> owner=<id> box=<6 f64> line=<start xyz>:<end xyz>
//!     arc=<centre xyz>,r=<radius>,x=<axis>,y=<axis>,a=<start>..<end>
//!     (each reading the data allows; "-" where it does not)
//!   list <sketched element> <sketch line ids>   each Sketch's curve list (RE-97)
//!   data <roof id> <stream>+<offset> <len> <hex of up to 1024 bytes>
//!   f64 <roof id> +<offset> <value>   each finite f64 in the roof's data
//!     with 1e-3 <= |value| <= 1e4, at every byte offset
//!
//! Usage:
//!   cargo run --profile ci --example probe_re211_extrusion_roofs -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::scan_sketch_curve_lists;
use rvt::research::partition_beam_axes::scan_sketch_curves;
use rvt::research::partition_element_records as per;
use rvt::research::partition_names::element_data_layout;
use std::collections::BTreeSet;

fn xyz(p: [f64; 3]) -> String {
    format!("{:.6},{:.6},{:.6}", p[0], p[1], p[2])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let roofs = per::scan_category_records_multi(&mut rf, version, &[per::OST_ROOFS], &declared)?;
    let roof_ids: BTreeSet<u32> = roofs.iter().map(|r| r.element_id).collect();
    for roof in &roofs {
        let class = classes
            .by_tag(roof.class_tag)
            .map_or("?", |c| c.name.as_str());
        println!(
            "roof {} {class} box={} refs={:?}",
            roof.element_id,
            roof.bbox_feet.map(|v| format!("{v:.6}")).join(","),
            roof.references
        );
    }
    let lines =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let line_ids: BTreeSet<u32> = lines.iter().map(|l| l.element_id).collect();
    let curves = scan_sketch_curves(&mut rf, version, &line_ids)?;
    for line in &lines {
        let curve = curves.get(&line.element_id);
        let as_line = curve.and_then(|c| c.line).map_or("-".to_string(), |l| {
            format!("{}:{}", xyz(l.start()), xyz(l.end()))
        });
        let as_arc = curve.and_then(|c| c.arc).map_or("-".to_string(), |a| {
            format!(
                "{},r={:.6},x={},y={},a={:.6}..{:.6}",
                xyz(a.centre),
                a.radius,
                xyz(a.x_axis),
                xyz(a.y_axis),
                a.start_angle,
                a.end_angle
            )
        });
        let owner = line
            .owner_reference
            .map_or("-".to_string(), |o| o.to_string());
        let mark = if line.owner_reference.is_some_and(|o| roof_ids.contains(&o)) {
            " roof"
        } else {
            ""
        };
        println!(
            "line {} owner={owner}{mark} box={} line={as_line} arc={as_arc}",
            line.element_id,
            line.bbox_feet.map(|v| format!("{v:.6}")).join(",")
        );
    }
    for (sketched, ids) in scan_sketch_curve_lists(&mut rf, version, &line_ids) {
        println!("list {sketched} {ids:?}");
    }
    let Some(layout) = element_data_layout(version) else {
        eprintln!("Revit {version}: no element data layout");
        return Ok(());
    };
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &layout.header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let at = hit + layout.header.len();
            let Some(id) = layout.id_at(buf, at) else {
                continue;
            };
            if !roof_ids.contains(&id) {
                continue;
            }
            let end = hits.get(index + 1).copied().unwrap_or(buf.len());
            let data = &buf[hit..end];
            println!(
                "data {id} {stream}+{hit} {} {}",
                data.len(),
                hex(&data[..data.len().min(1024)])
            );
            for offset in 0..data.len().saturating_sub(7) {
                let Some(bytes) = data.get(offset..offset + 8) else {
                    continue;
                };
                let Ok(bytes) = <[u8; 8]>::try_from(bytes) else {
                    continue;
                };
                let value = f64::from_le_bytes(bytes);
                if value.is_finite() && (1e-3..=1e4).contains(&value.abs()) {
                    println!("f64 {id} +{offset} {value:.9}");
                }
            }
        }
    }
    Ok(())
}
