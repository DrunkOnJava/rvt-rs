//! RE-210 (#35): `rvt-ifc --revit-property-sets` writes an element's named
//! parameters in one set per parameter group, as Revit's exporter does with
//! "Export Revit property sets", with the values Revit writes.
//!
//! The BIM4LCA as-built office's doors and windows hold a Yes/No parameter
//! `FireExit` in the group Identity Data. Revit's export (common sets only)
//! writes the same parameter as `Pset_DoorCommon.FireExit` and
//! `Pset_WindowCommon.FireExit`; rvt-rs's export with the option writes it
//! in the element's set `Identity Data`, as a Boolean, and on every door and
//! window Revit writes it for (266 and 232) the two are equal.
//!
//! The RE1 MEP models hold air flow, pressure, power, voltage and current
//! parameters, which Revit writes as those measures in SI units. rvt-rs's export with
//! the option writes them so, and declares each one's unit; every voltage
//! is a nominal one.
//!
//! Runs against `RVT_NORDIC_LCA_DIR` and `RVT_PROJECT_CORPUS_DIR`; skips what
//! is absent.

use crate::element_parameters::{Written, nordic_dir, revit_values};
use crate::is_external::{entities, list, split_args};
use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::path::PathBuf;

#[test]
fn nordic_office_fire_exit_is_revits_in_identity_data() {
    let Some(dir) = nordic_dir() else {
        return;
    };
    let rvt = dir.join("ARK_NordicLCA_Office_Concrete_As-Built.rvt");
    let ifc = dir.join("ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc");
    if !rvt.exists() || !ifc.exists() {
        eprintln!("skipping: the as-built office or its export is absent");
        return;
    }
    let mut file = RevitFile::open(&rvt).expect("model");
    let mut model = RvtDocExporter
        .export_with_diagnostics(&mut file)
        .expect("export")
        .model;
    let report =
        rvt::ifc::revit_property_sets::attach(&mut file, &mut model).expect("property sets");
    assert!(report.properties > 0, "no property written: {report:?}");
    let ours = revit_values(&write_step(&model));
    let revit = revit_values(&std::fs::read_to_string(&ifc).expect("Revit export"));

    let (mut paired, mut differ) = (0, Vec::new());
    for (tag, theirs) in &revit {
        let Some(written) = ["Pset_DoorCommon", "Pset_WindowCommon"]
            .iter()
            .find_map(|set| theirs.get(&format!("{set}.FireExit")))
        else {
            continue;
        };
        paired += 1;
        let mine = ours.get(tag).and_then(|v| v.get("Identity Data.FireExit"));
        if !matches!(mine, Some(Written::Boolean(_))) || mine != Some(written) {
            differ.push(format!("{tag}: ours {mine:?}, Revit's {written:?}"));
        }
    }
    assert!(
        differ.is_empty(),
        "{} of {paired} doors and windows differ: {:?}",
        differ.len(),
        &differ[..differ.len().min(10)]
    );
    assert!(paired >= 490, "only {paired} doors and windows paired");
}

#[test]
fn re1_mep_flows_pressures_powers_and_voltages_are_in_si_units() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    // Each measure and the unit its values are in.
    let measures = [
        ("IFCVOLUMETRICFLOWRATEMEASURE", "VOLUMETRICFLOWRATEUNIT"),
        ("IFCPRESSUREMEASURE", "PRESSUREUNIT"),
        ("IFCPOWERMEASURE", "POWERUNIT"),
        ("IFCELECTRICVOLTAGEMEASURE", "ELECTRICVOLTAGEUNIT"),
        ("IFCELECTRICCURRENTMEASURE", "ELECTRICCURRENTUNIT"),
    ];
    let mut written = 0;
    for model in ["RE1-Electrical", "RE1-Mechanical", "RE1-Plumbing"] {
        let rvt = dir.join(format!("{model}.rvt"));
        if !rvt.exists() {
            eprintln!("skipping {model}: absent");
            continue;
        }
        let mut file = RevitFile::open(&rvt).expect("model");
        let mut model_ifc = RvtDocExporter
            .export_with_diagnostics(&mut file)
            .expect("export")
            .model;
        rvt::ifc::revit_property_sets::attach(&mut file, &mut model_ifc).expect("property sets");
        let step = write_step(&model_ifc);
        let ents = entities(&step);
        // The unit type of each unit the IfcUnitAssignment lists: the second
        // field of an IfcSIUnit and of an IfcDerivedUnit.
        let declared: Vec<String> = ents
            .values()
            .filter(|(entity, _)| entity == "IFCUNITASSIGNMENT")
            .flat_map(|(_, args)| list(&split_args(args)[0]))
            .filter_map(|unit| ents.get(&unit))
            .filter_map(|(_, args)| Some(split_args(args).get(1)?.trim_matches('.').to_string()))
            .collect();
        for (measure, unit) in measures {
            let values: Vec<f64> = step
                .split(&format!("{measure}("))
                .skip(1)
                .filter_map(|rest| rest.split(')').next()?.parse().ok())
                .collect();
            if values.is_empty() {
                continue;
            }
            written += values.len();
            assert!(
                declared.iter().any(|declared| declared == unit),
                "{model}: {} values of {measure}, no {unit} declared",
                values.len()
            );
            if measure == "IFCELECTRICVOLTAGEMEASURE" {
                let odd: Vec<&f64> = values
                    .iter()
                    .filter(|v| ![120.0, 208.0, 240.0, 277.0, 480.0].contains(*v))
                    .collect();
                assert!(odd.is_empty(), "{model}: voltages not nominal: {odd:?}");
            }
        }
    }
    if dir.join("RE1-Electrical.rvt").exists() {
        assert!(written >= 56, "only {written} values of the five measures");
    }
}
