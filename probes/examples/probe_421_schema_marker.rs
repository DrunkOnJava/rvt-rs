//! #421, item 1: the element-record marker each file's schema gives, against
//! the release's measured constant.
//!
//! One JSON object per file: `release`, `schema` (the marker from
//! `Outline` and `ElementParents`, RE-80), `constant` (`bbox_marker`),
//! `header_tag` (the schema's `ElementHeader`) and `prologue`
//! (`record_prologue_constant` of the schema's marker), and `scanned`, the
//! marker `file_bbox_marker` hands the scans, `schema_release` (the
//! release the schema belongs to, `schema_release`) and `layout` (the
//! release the readers take, `RevitFile::layout_release`), and
//! `elem_table`: the record count `Global/ElemTable` states, the record size
//! `detect_layout` takes (40, 28, or 12 for the fallback), and for 28 and 40
//! how many ids rise, where the first one does not, and whether the stated
//! count fits the stream. Counts and indices only, no ElementId, so a
//! reporter can post it for a private file.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_421_schema_marker -- FILE.rvt ...

use rvt::RevitFile;
use rvt::compression;
use rvt::research::partition_element_records as per;
use rvt::research::partition_element_records_2023::record_marker;

fn hex(marker: Option<[u8; 8]>) -> String {
    marker.map_or_else(
        || "null".into(),
        |m| format!("\"{}\"", m.map(|b| format!("{b:02x}")).join(" ")),
    )
}

/// `count`, `stride` and, per record size, `rises`, `first_drop` and `fits`.
fn elem_table(rf: &mut RevitFile) -> String {
    const NAME: &str = "Global/ElemTable";
    let Ok(raw) = rf.read_stream(NAME) else {
        return "null".into();
    };
    let Ok(d) = compression::inflate_stream_at(NAME, &raw, 8)
        .or_else(|_| compression::inflate_stream_at(NAME, &raw, 0))
    else {
        return "null".into();
    };
    let Some(count) = d
        .get(2..6)
        .and_then(|b| b.try_into().ok())
        .map(|b: [u8; 4]| u32::from_le_bytes(b) as usize)
    else {
        return "null".into();
    };
    let origin = rvt::elem_table::RECORDS_ORIGIN;
    let fit = |stride: usize, id_at: usize| {
        let fits = count
            .checked_mul(stride)
            .and_then(|n| n.checked_add(origin))
            .is_some_and(|end| end <= d.len());
        let (mut rises, mut first_drop, mut previous) = (0usize, None, None::<u32>);
        for k in 0..count {
            let at = origin + k * stride + id_at;
            let Some(id) = d
                .get(at..at + 4)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_le_bytes)
            else {
                break;
            };
            match previous {
                Some(p) if id > p => rises += 1,
                Some(_) if first_drop.is_none() => first_drop = Some(k),
                _ => {}
            }
            previous = Some(id);
        }
        format!(
            "{{\"rises\":{rises},\"first_drop\":{},\"fits\":{fits}}}",
            first_drop.map_or_else(|| "null".into(), |k: usize| k.to_string())
        )
    };
    format!(
        "{{\"count\":{count},\"stride\":{},\"fit28\":{},\"fit40\":{}}}",
        rvt::elem_table::detect_layout(&d).stride,
        fit(28, 4),
        fit(40, 20)
    )
}

fn main() {
    for path in std::env::args().skip(1) {
        let file = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut rf = match RevitFile::open(&path) {
            Ok(rf) => rf,
            Err(e) => {
                println!("{{\"file\":{file:?},\"error\":{:?}}}", e.to_string());
                continue;
            }
        };
        let release = rf.basic_file_info().map(|i| i.version).unwrap_or(0);
        let layout = rf.layout_release().unwrap_or(release);
        let schema = record_marker(&mut rf);
        let marker = schema.map(|(m, _)| m);
        println!(
            "{{\"file\":{file:?},\"release\":{release},\"schema\":{},\"constant\":{},\
             \"header_tag\":{},\"prologue\":{},\"scanned\":{},\"schema_release\":{},\"layout\":{layout},\
             \"elem_table\":{}}}",
            hex(marker),
            hex(per::bbox_marker(release)),
            schema.map_or_else(|| "null".into(), |(_, t)| t.to_string()),
            marker.map_or_else(
                || "null".into(),
                |m| per::record_prologue_constant(&m).to_string()
            ),
            hex(per::file_bbox_marker(&mut rf, layout)),
            per::schema_release(&mut rf).map_or_else(|| "null".into(), |r| r.to_string()),
            elem_table(&mut rf),
        );
    }
}
