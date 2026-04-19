//! Known OLE stream names in Revit files.

pub const BASIC_FILE_INFO: &str = "BasicFileInfo";
pub const CONTENTS: &str = "Contents";
pub const FORMATS_LATEST: &str = "Formats/Latest";
pub const GLOBAL_CONTENT_DOCUMENTS: &str = "Global/ContentDocuments";
pub const GLOBAL_DOC_INCREMENT_TABLE: &str = "Global/DocumentIncrementTable";
pub const GLOBAL_ELEM_TABLE: &str = "Global/ElemTable";
pub const GLOBAL_HISTORY: &str = "Global/History";
pub const GLOBAL_LATEST: &str = "Global/Latest";
pub const GLOBAL_PARTITION_TABLE: &str = "Global/PartitionTable";
pub const PART_ATOM: &str = "PartAtom";
pub const REVIT_PREVIEW_4_0: &str = "RevitPreview4.0";
pub const TRANSMISSION_DATA: &str = "TransmissionData";

/// Revit year → known Partitions/NN marker.
///
/// Observed empirically from the 11-version phi-ag corpus. 59 is skipped between
/// 2016 and 2017; afterwards the number monotonically increments.
pub fn partition_for_year(year: u32) -> Option<u32> {
    match year {
        2016 => Some(58),
        2017 => Some(60),
        2018 => Some(61),
        2019 => Some(62),
        2020 => Some(63),
        2021 => Some(64),
        2022 => Some(65),
        2023 => Some(66),
        2024 => Some(67),
        2025 => Some(68),
        2026 => Some(69),
        _ => None,
    }
}

/// Inverse of `partition_for_year`. Used for files with unexpected year
/// encodings when we can see the partition number but not the year.
pub fn year_for_partition(n: u32) -> Option<u32> {
    match n {
        58 => Some(2016),
        60 => Some(2017),
        61 => Some(2018),
        62 => Some(2019),
        63 => Some(2020),
        64 => Some(2021),
        65 => Some(2022),
        66 => Some(2023),
        67 => Some(2024),
        68 => Some(2025),
        69 => Some(2026),
        _ => None,
    }
}
