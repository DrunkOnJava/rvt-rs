//! Emit independently decoded unowned 2027 Mark / Type Mark occurrences.
//! cargo run --release --example oracle_parameter_probe -- MODEL.rvt > occurrences.json
use rvt::{RevitFile, compression, partition_parameter_records as parameters};
use serde_json::json;
#[path = "support/oracle_record_frames_2027.rs"]
mod record_frames;

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        (2..=3).contains(&args.len()),
        "expected MODEL.rvt [MAX_STRING_UNITS]"
    );
    let max_string_units = args
        .get(2)
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(parameters::MAX_STRING_UNITS);
    let mut rf = RevitFile::open(&args[1])?;
    let version = rf.basic_file_info()?.version;
    let mut observations = Vec::new();
    let mut failed_candidates = Vec::new();
    let mut scan_issues = Vec::new();
    let mut recognized_record_count = 0usize;
    for name in rf
        .stream_names()
        .iter()
        .filter(|name| name.starts_with("Partitions/"))
    {
        let stored = rf.read_stream(name)?;
        let prepared = compression::prepare_stream_for_inflate(name, &stored);
        for offset in compression::find_gzip_offsets(&prepared) {
            let bytes = match compression::inflate_at(&prepared, offset) {
                Ok(bytes) => bytes,
                Err(error) => {
                    failed_candidates.push(json!({"stream": name, "prepared_offset": offset, "error": error.to_string()}));
                    continue;
                }
            };
            for (record_offset, body) in record_frames::record_bodies(&bytes, version) {
                recognized_record_count += 1;
                for parameter_id in [
                    parameters::INSTANCE_MARK_PARAMETER_ID,
                    parameters::TYPE_MARK_PARAMETER_ID,
                ] {
                    let report =
                        parameters::scan_bounded_string_parameter_occurrences_2027_with_limit(
                            &bytes[body.clone()],
                            i64::from(parameter_id),
                            version,
                            max_string_units,
                        );
                    for issue in report.issues {
                        scan_issues.push(json!({"stream": name, "prepared_member_offset": offset, "record_offset": record_offset, "body_start": body.start, "parameter_id": parameter_id, "issue": issue}));
                    }
                    for mut occurrence in report.occurrences {
                        occurrence.offset += body.start;
                        observations.push(json!({"stream": name, "prepared_member_offset": offset, "record_offset": record_offset, "body_start": body.start, "body_end": body.end, "occurrence": occurrence}));
                    }
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"schema_version": 3, "scan_scope": "recognized_record_bodies_only", "scan_complete": scan_issues.is_empty() && failed_candidates.is_empty() && version == 2027, "scan_issues": scan_issues, "recognized_record_count": recognized_record_count, "max_string_units": max_string_units, "source": args[1], "revit_version": version, "ownership": "unresolved", "boundary_source": "research_2027_repeated_identity_and_dual_length_record_framing", "observations": observations, "failed_candidates": failed_candidates})
        )?
    );
    Ok(())
}
