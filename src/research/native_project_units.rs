//! A project's display units, as its Project Units dialog sets them
//! (RE-445).
//!
//! A document holds one `UnitsElem` record. Its `m_units` field is an
//! `AUnits` object with one `FormatOptions` per spec. From Revit 2021 the
//! saved shape is `AUnits.m_formatOptionsMap`, a map from the spec's
//! `ForgeTypeId` (`autodesk.spec.aec:length-1.0.0`) to its `FormatOptions`,
//! whose `m_unitTypeId` is the display unit's `ForgeTypeId`
//! (`autodesk.unit.unit:feetFractionalInches-1.0.0`). Every spec is listed,
//! whatever discipline it belongs to: 133 to 150 specs on the corpus's
//! projects, of which an IFC unit assignment needs six.
//!
//! A family's document inside the project (an in-place family, a loaded
//! family kept with its document) has a `UnitsElem` of its own, whose
//! `m_famId` names the family: Snowdon Towers' architectural model holds two
//! beside the project's, with other units for power and area. Only the
//! project's, with `m_famId` unset, is read. A family file's own `UnitsElem`
//! names the family the file is (a 2025 beam family of the corpus): where
//! no record has `m_famId` unset, the file's one record is read.
//!
//! The shape is told by the field the schema gives `AUnits`, not by the
//! release. Revit 2015 to 2020 saved `AUnits.m_formatOptionsArr` instead: an
//! array of `FormatOptions` with an integer `m_displayUnits` and no spec,
//! its position standing for the spec. That shape is not read: the corpus
//! has later saves of four of its projects (Autodesk's `rac_basic`,
//! `rst_basic` 2019 and 2020 and `rac_advanced` 2019, each against its 2021
//! save), all metric, and they agree with length at position 0 and
//! millimetres as 2, but leave which position is which other spec, and
//! which integer is which unit, open. Such a file gives `None`, as before.
//!
//! Earlier, the exporter took every `autodesk.unit.*` string of the
//! partition and kept, for each IFC unit type, the identifier seen most
//! often. Those strings are the map's units and symbols, each spec's entry
//! once, so the count picked by name: Autodesk's 2024 Core Interior, whose
//! length is feet and fractional inches, came out in metres, square inches
//! and cubic inches, and the 130 units of other specs were reported as
//! unmapped.

use crate::RevitFile;
use crate::native_document;
use crate::native_json::identifier_i64;
use anyhow::{Result, anyhow, ensure};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// One spec's display format.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DisplayUnit {
    /// The display unit's `ForgeTypeId`, e.g.
    /// `autodesk.unit.unit:millimeters-1.0.1`.
    pub unit: String,
    /// The unit symbol's `ForgeTypeId`, empty where none is shown.
    pub symbol: String,
    /// The rounding increment, in the display unit; `None` where it was not
    /// read ([`string_record_maps`]).
    pub accuracy: Option<f64>,
}

/// The display unit of every spec the document lists, by the spec's
/// `ForgeTypeId`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ProjectUnits {
    pub specs: BTreeMap<String, DisplayUnit>,
}

impl ProjectUnits {
    /// The display unit of `spec`, named with or without its version
    /// (`autodesk.spec.aec:length`).
    pub fn unit_of(&self, spec: &str) -> Option<&DisplayUnit> {
        self.specs
            .iter()
            .find(|(id, _)| unversioned(id) == unversioned(spec))
            .map(|(_, unit)| unit)
    }
}

/// A `ForgeTypeId` without its `-X.Y.Z` version.
pub fn unversioned(id: &str) -> &str {
    match id.rsplit_once('-') {
        Some((name, version))
            if !version.is_empty()
                && version.split('.').all(|part| {
                    !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
                }) =>
        {
            name
        }
        _ => id,
    }
}

/// The document's display units, from its `UnitsElem`. `None` where the
/// native record path does not read the release, the file holds no
/// `UnitsElem` or its units are in the array of Revit 2015 to 2020; an
/// error where the record is read and does not decode, or two disagree.
pub fn read(rf: &mut RevitFile) -> Result<Option<ProjectUnits>> {
    if !rf
        .layout_release()
        .is_ok_and(|release| native_document::NATIVE_RELEASES.contains(&release))
    {
        return Ok(None);
    }
    let options = native_document::Options {
        selected_classes: BTreeSet::from(["UnitsElem".to_string()]),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    // Each UnitsElem's units, with whether `m_famId` names a family.
    let mut found: Vec<(bool, Option<ProjectUnits>)> = Vec::new();
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph
            .objects
            .first()
            .filter(|r| r.class_name == "UnitsElem")
        else {
            return Ok(());
        };
        let of_family = root
            .fields
            .get("m_famId")
            .and_then(identifier_i64)
            .is_some_and(|family| family != -1);
        let units = root
            .fields
            .get("m_units")
            .ok_or_else(|| anyhow!("UnitsElem {} has no m_units", record.identity.element_id))?;
        let read = if let Some(pairs) = units.get("m_formatOptionsMap") {
            Some(forge(pairs)?)
        } else if units.get("m_formatOptionsArr").is_some() {
            None
        } else {
            return Err(anyhow!("AUnits holds neither a format map nor an array"));
        };
        found.push((of_family, read));
        Ok(())
    })?;
    // The record with `m_famId` unset is the project's. A family file has
    // none such: its one UnitsElem names the family the file is.
    let project: Vec<_> = found.iter().filter(|(of_family, _)| !of_family).collect();
    let chosen = match (project.as_slice(), found.as_slice()) {
        ([], []) => None,
        ([], [only]) => only.1.clone(),
        ([], _) => {
            return Err(anyhow!(
                "{} UnitsElem records, each naming a family",
                found.len()
            ));
        }
        ([first, rest @ ..], _) => {
            ensure!(
                rest.iter().all(|other| other.1 == first.1),
                "two UnitsElem records disagree"
            );
            first.1.clone()
        }
    };
    Ok(chosen)
}

/// The units of an `AUnits.m_formatOptionsMap`: `first` is the spec,
/// `second` its `FormatOptions`.
fn forge(pairs: &crate::research::graph_value::Fv) -> Result<ProjectUnits> {
    let pairs = pairs
        .as_array()
        .ok_or_else(|| anyhow!("m_formatOptionsMap is not a list"))?;
    let mut specs = BTreeMap::new();
    for pair in pairs {
        let spec = pair["first"]["m_typeId"]
            .as_str()
            .ok_or_else(|| anyhow!("a format map entry has no spec"))?;
        let format = &pair["second"];
        let unit = format["m_unitTypeId"]["m_typeId"]
            .as_str()
            .ok_or_else(|| anyhow!("spec {spec} has no display unit"))?;
        let display = DisplayUnit {
            unit: unit.to_string(),
            symbol: format["m_symbolTypeId"]["m_typeId"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            accuracy: format["m_accuracy"].as_f64(),
        };
        ensure!(
            specs.insert(spec.to_string(), display).is_none(),
            "spec {spec} is listed twice"
        );
    }
    Ok(ProjectUnits { specs })
}

/// The format maps the partition's string records spell out, where the
/// native record walk does not read the file (it stops on Autodesk's sample
/// families before any record, with `invalid increment episode range`).
///
/// A map's entries are saved in the map's order, the spec's `ForgeTypeId`
/// and then its `FormatOptions`, whose symbol and unit are strings too. The
/// string scan sees each spec followed by its unit (it reads a symbol as the
/// unit's header, so the symbol is not seen): a map is a run of string
/// records that alternate spec, unit, spec, unit, with no other string
/// record between them and each spec after the one before. A run of one
/// pair is not taken: a lone spec beside a unit is a parameter's, not a
/// map's. A project's partition holds older copies of its map and the
/// maps of its families' documents, and the runs do not say which is
/// current: on 100 of the 102 corpus files the walk reads, one or more runs
/// equal the `UnitsElem` map, and on Autodesk's Japanese structural sample
/// none does (its runs are cut short). Each of Autodesk's sample families of
/// 2021 to 2027 holds one run. Only spec and unit are recovered: the
/// accuracy is not.
pub fn string_record_maps(records: &[crate::object_graph::StringRecord]) -> Vec<ProjectUnits> {
    let is_spec =
        |record: &crate::object_graph::StringRecord| record.value.starts_with("autodesk.spec.");
    let is_unit = |record: &crate::object_graph::StringRecord| {
        record.value.starts_with("autodesk.unit.unit:")
    };
    let mut maps = Vec::new();
    let mut run: Vec<(&str, &str)> = Vec::new();
    let mut close = |run: &mut Vec<(&str, &str)>| {
        if run.len() > 1 {
            maps.push(ProjectUnits {
                specs: run
                    .iter()
                    .map(|(spec, unit)| {
                        (
                            spec.to_string(),
                            DisplayUnit {
                                unit: unit.to_string(),
                                symbol: String::new(),
                                accuracy: None,
                            },
                        )
                    })
                    .collect(),
            });
        }
        run.clear();
    };
    let mut at = 0;
    while at + 1 < records.len() {
        let (spec, unit) = (&records[at], &records[at + 1]);
        if is_spec(spec) && is_unit(unit) {
            if run
                .last()
                .is_some_and(|(last, _)| *last >= spec.value.as_str())
            {
                close(&mut run);
            }
            run.push((spec.value.as_str(), unit.value.as_str()));
            at += 2;
            // The next pair continues the run only where it follows at once.
            if !records.get(at).is_some_and(is_spec) {
                close(&mut run);
            }
        } else {
            close(&mut run);
            at += 1;
        }
    }
    close(&mut run);
    maps
}

/// The project's display units: its `UnitsElem` ([`read`]), or, where the
/// native walk does not give them, the maps of its string records when they
/// all agree ([`string_record_maps`]). A project keeps its units' older
/// copies in the partition (Autodesk's 2024 Core Interior holds 47 runs of
/// 136 specs, 3 of them equal to its `UnitsElem`), so maps that disagree
/// give no units, which is current being the record walk's to say: that is
/// the one error, naming the walk's own where it failed. `Ok(None)` where
/// neither gives units: a file of Revit 2015 to 2020, whose units hold no
/// spec, or one the walk does not read that has no map either.
pub fn resolve(rf: &mut RevitFile) -> std::result::Result<Option<ProjectUnits>, String> {
    let walked = read(rf);
    if let Ok(Some(units)) = walked {
        return Ok(Some(units));
    }
    let maps = rf
        .partition_string_records()
        .map(|records| string_record_maps(&records))
        .unwrap_or_default();
    let agree = |a: &ProjectUnits, b: &ProjectUnits| {
        a.specs.len() == b.specs.len()
            && a.specs.iter().all(|(spec, unit)| {
                b.specs
                    .get(spec)
                    .is_some_and(|other| other.unit == unit.unit)
            })
    };
    match maps.split_first() {
        None => Ok(None),
        Some((first, rest)) if rest.iter().all(|map| agree(first, map)) => Ok(Some(first.clone())),
        Some(_) => Err(format!(
            "{}, and the partition's {} unit maps disagree",
            match walked {
                Err(error) => format!("its UnitsElem did not read ({error:#})"),
                _ => "its UnitsElem holds no spec names".to_string(),
            },
            maps.len()
        )),
    }
}
