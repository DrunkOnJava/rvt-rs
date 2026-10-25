//! #878: which element's GlobalId Revit's export derives the project's,
//! site's and building's from.
//!
//! FACT under test: on Core Interior, the BIM4LCA office and the R2025
//! models, Revit's IfcSite and IfcBuilding GlobalIds are its IfcProject one
//! with the last 128-bit value's low bits XORed by 2 and 1, so all three
//! come from one GUID; that GUID is an element's RE-48 GlobalId (its
//! episode GUID XOR its ElementId), most likely the project information's.
//!
//! Output:
//!   category <id> <BuiltInCategory>   every record of OST_ProjectInformation
//!   rgid <id> <GlobalId>              RE-48's GlobalId of every declared element
//!   episode <index> <GlobalId>        each Global/History episode's GUID as a
//!                                     GlobalId with nothing XORed in
//!
//! `tools/re/project_guids_vs_ifc.py` finds Revit's three GlobalIds in it.
//!
//! Usage:
//!   cargo run --profile ci --example probe_878_project_guids -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records as per;

const OST_PROJECT_INFORMATION: i64 = -2_003_101;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records = if rvt::research::partition_element_records_2023::is_32bit_release(version) {
        rvt::research::partition_element_records_2023::scan_records(&mut rf, version)
    } else {
        per::scan_category_records_multi(&mut rf, version, &[OST_PROJECT_INFORMATION], &declared)?
    };
    for record in records
        .iter()
        .filter(|r| r.builtin_category == OST_PROJECT_INFORMATION)
    {
        println!("category {} {}", record.element_id, record.builtin_category);
    }
    for (id, global_id) in rvt::research::revit_global_ids::revit_global_ids(&mut rf)? {
        println!("rgid {id} {global_id}");
    }
    let stored = rf.read_stream(rvt::streams::GLOBAL_HISTORY)?;
    let prepared =
        rvt::compression::prepare_stream_for_inflate(rvt::streams::GLOBAL_HISTORY, &stored);
    let history = [0usize, 4, 8, 16]
        .iter()
        .filter(|&&at| rvt::compression::has_gzip_magic(prepared.as_ref(), at))
        .find_map(|&at| rvt::compression::inflate_at(prepared.as_ref(), at).ok());
    if let Some(episodes) = history
        .as_deref()
        .and_then(rvt::research::revit_global_ids::EpisodeHistory::parse)
    {
        for index in 0..episodes.len() as u32 {
            if let Some(guid) = episodes.episode_guid(index) {
                println!(
                    "episode {index} {}",
                    rvt::research::revit_global_ids::revit_ifc_global_id(guid, 0)
                );
            }
        }
    }
    Ok(())
}
