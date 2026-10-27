//! RE-207 (#35): an element's saved parameters, named by their definitions,
//! hold the values Revit's export writes from them.
//!
//! Revit's exporter writes `<set>.<property>` from the element's parameter of
//! that name before anything else (RE-194). For every element of Revit's own
//! IFC4 export, each parameter rvt-rs reads under such a name whose property
//! Revit writes on the same `Tag` must hold Revit's value: a Boolean, an
//! integer, a text, or a number written without a unit. A family instance
//! takes its type's type parameters: on the R2025 models (#356) a window's
//! `Pset_WindowCommon.FireRating` is its type's family parameter `Fire
//! Rating`, where the window has no parameter named for the property. A wall
//! takes its wall type's the same way: each R2025 wall lists its type's name
//! (Type Name, -1002001) from the type Revit's export relates it
//! to, whose `Tag` is the type's ElementId.
//!
//! Runs against `RVT_NORDIC_LCA_DIR` (the BIM4LCA housing on terrain, and the
//! as-built office where its export is there too) and
//! `RVT_PROJECT_CORPUS_DIR/r2025/`. Skips what is absent.

use crate::element_names::decode_step_string;
use crate::is_external::{entities, list, split_args};
use rvt::RevitFile;
use rvt::research::native_element_parameters::{ElementParameter, ParameterLevel};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A property value as Revit's export writes it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Written {
    Boolean(bool),
    Integer(i64),
    Text(String),
    Number(f64),
}

/// `Tag` -> `<set>.<property>` and plain property name -> value, for every
/// element's own sets then its type's; an opening sharing its door's or
/// window's `Tag` is left out. Measures with a unit are left out.
pub(crate) fn revit_values(step: &str) -> BTreeMap<u64, BTreeMap<String, Written>> {
    let ents = entities(step);
    let reference = |field: &str| field.trim().strip_prefix('#')?.parse::<u64>().ok();
    let set_values = |set: u64| -> Vec<(String, Written)> {
        let Some((entity, args)) = ents.get(&set) else {
            return Vec::new();
        };
        if entity != "IFCPROPERTYSET" {
            return Vec::new();
        }
        let f = split_args(args);
        let set_name = decode_step_string(f[2].trim().trim_matches('\''));
        let mut out = Vec::new();
        for property in list(&f[4]) {
            let Some(("IFCPROPERTYSINGLEVALUE", args)) =
                ents.get(&property).map(|(e, a)| (e.as_str(), a))
            else {
                continue;
            };
            let p = split_args(args);
            let name = decode_step_string(p[0].trim().trim_matches('\''));
            let Some((kind, raw)) = p[2].trim().split_once('(') else {
                continue;
            };
            let raw = raw.trim_end_matches(')');
            let value = match kind {
                "IFCBOOLEAN" | "IFCLOGICAL" => Written::Boolean(raw == ".T."),
                "IFCINTEGER" | "IFCCOUNTMEASURE" => {
                    let Ok(n) = raw.trim_end_matches('.').parse() else {
                        continue;
                    };
                    Written::Integer(n)
                }
                "IFCLABEL" | "IFCTEXT" | "IFCIDENTIFIER" => {
                    Written::Text(decode_step_string(raw.trim_matches('\'')))
                }
                "IFCTHERMALTRANSMITTANCEMEASURE" | "IFCREAL" => {
                    let Ok(n) = raw.parse() else {
                        continue;
                    };
                    Written::Number(n)
                }
                _ => continue,
            };
            out.push((format!("{set_name}.{name}"), value.clone()));
            out.push((name, value));
        }
        out
    };
    let mut by_object: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut by_type: BTreeMap<u64, u64> = BTreeMap::new();
    for (entity, args) in ents.values() {
        let f = split_args(args);
        match entity.as_str() {
            "IFCRELDEFINESBYPROPERTIES" => {
                if let Some(set) = f.get(5).and_then(|s| reference(s)) {
                    for object in list(&f[4]) {
                        by_object.entry(object).or_default().push(set);
                    }
                }
            }
            "IFCRELDEFINESBYTYPE" => {
                if let Some(ty) = f.get(5).and_then(|s| reference(s)) {
                    for object in list(&f[4]) {
                        by_type.insert(object, ty);
                    }
                }
            }
            _ => {}
        }
    }
    let mut out: BTreeMap<u64, BTreeMap<String, Written>> = BTreeMap::new();
    // Elements first, so an opening never takes its door's or window's Tag.
    let mut products: Vec<_> = ents.iter().collect();
    products.sort_by_key(|(_, (entity, _))| entity == "IFCOPENINGELEMENT");
    for (&id, (entity, args)) in products {
        // The eighth attribute is an element's Tag, but a spatial
        // element's LongName.
        if entity.ends_with("TYPE")
            || matches!(
                entity.as_str(),
                "IFCOWNERHISTORY" | "IFCSPACE" | "IFCBUILDINGSTOREY" | "IFCBUILDING" | "IFCSITE"
            )
        {
            continue;
        }
        let Some(tag) = split_args(args)
            .get(7)
            .and_then(|t| t.trim().trim_matches('\'').parse::<u64>().ok())
        else {
            continue;
        };
        if out.contains_key(&tag) {
            continue;
        }
        let mut values = BTreeMap::new();
        let type_sets = by_type
            .get(&id)
            .and_then(|ty| ents.get(ty))
            .and_then(|(_, args)| split_args(args).get(5).map(|s| list(s)))
            .unwrap_or_default();
        for set in by_object.get(&id).into_iter().flatten().chain(&type_sets) {
            for (name, value) in set_values(*set) {
                values.entry(name).or_insert(value);
            }
        }
        out.insert(tag, values);
    }
    out
}

/// Whether rvt-rs's value is the one Revit writes.
fn equal(ours: &ElementParameter, written: &Written) -> bool {
    match (written, &ours.value) {
        (Written::Boolean(b), serde_json::Value::Bool(v)) => b == v,
        (Written::Boolean(b), v) => v.as_i64().is_some_and(|n| (n != 0) == *b),
        (Written::Integer(n), v) => v.as_i64() == Some(*n),
        (Written::Text(t), v) => v.as_str() == Some(t.as_str()),
        (Written::Number(n), v) => v
            .as_f64()
            .is_some_and(|v| (v - n).abs() <= 1e-6 * v.abs().max(n.abs())),
    }
}

/// (pairs, different pairs as `id name ours revit`) over `names` (all named
/// parameters when empty).
fn pair(rvt: &Path, ifc: &Path, names: &[&str]) -> (usize, Vec<String>) {
    let step = std::fs::read_to_string(ifc).expect("Revit export");
    let revit = revit_values(&step);
    let mut file = RevitFile::open(rvt).expect("model");
    let ids = revit.keys().copied().collect();
    let ours = rvt::research::native_element_parameters::read(&mut file, &ids).expect("parameters");
    let (mut paired, mut differ) = (0, Vec::new());
    for (id, parameters) in &ours.elements {
        // Revit takes a parameter named for the property before `Fire Rating`.
        let named_fire_rating = parameters.iter().any(|p| {
            p.name
                .as_deref()
                .is_some_and(|n| n == "FireRating" || n.ends_with(".FireRating"))
        });
        for parameter in parameters {
            let Some(name) = parameter.name.as_deref() else {
                continue;
            };
            let written_as = match name {
                "Fire Rating" if named_fire_rating => continue,
                "Fire Rating" => "FireRating",
                n if n.starts_with("Pset_") => n,
                _ => continue,
            };
            if !names.is_empty() && !names.contains(&name) {
                continue;
            }
            let Some(written) = revit[id].get(written_as) else {
                continue;
            };
            paired += 1;
            if !equal(parameter, written) {
                differ.push(format!("{id} {name} {} {written:?}", parameter.value));
            }
        }
    }
    (paired, differ)
}

pub(crate) fn nordic_dir() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("RVT_NORDIC_LCA_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_NORDIC_LCA_DIR is not set");
        return None;
    };
    Some(dir)
}

#[test]
fn nordic_parameters_named_for_a_property_hold_revits_values() {
    let Some(dir) = nordic_dir() else {
        return;
    };
    // (model, its export, the fewest pairs measured).
    for (model, export, least) in [
        (
            "ARK_NordicLCA_Housing_Terrain-Concrete_BuildingPermit.rvt",
            "ARK_NordicLCA_Housing_Terrain-Concrete_BuildingPermit_Revit.ifc",
            27,
        ),
        (
            "ARK_NordicLCA_Office_Concrete_As-Built.rvt",
            "ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc",
            7_000,
        ),
    ] {
        let (rvt, ifc) = (dir.join(model), dir.join(export));
        if !rvt.exists() || !ifc.exists() {
            eprintln!("skipping {model}: model or export absent");
            continue;
        }
        let (paired, differ) = pair(&rvt, &ifc, &[]);
        assert!(
            differ.is_empty(),
            "{model}: {} of {paired} differ from Revit's: {:?}",
            differ.len(),
            &differ[..differ.len().min(10)]
        );
        assert!(paired >= least, "{model}: only {paired} values paired");
    }
}

#[test]
fn r2025_windows_take_their_types_fire_rating() {
    let Some(dir) =
        std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(|d| PathBuf::from(d).join("r2025"))
    else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let model = dir.join("12_Rooms_Door_Windows.rvt");
    if !model.exists() {
        eprintln!("skipping: no R2025 model 12");
        return;
    }
    let (paired, differ) = pair(&model, &model.with_extension("ifc"), &["Fire Rating"]);
    assert!(differ.is_empty(), "differ from Revit's: {differ:?}");
    assert_eq!(paired, 3, "the three windows' Fire Rating");
    // The value is the type's: the window instances hold none of their own.
    let mut file = RevitFile::open(&model).expect("model");
    let ours = rvt::research::native_element_parameters::read(&mut file, &BTreeSet::from([12538]))
        .expect("parameters");
    let fire = ours.elements[&12538]
        .iter()
        .find(|p| p.name.as_deref() == Some("Fire Rating"))
        .expect("Fire Rating");
    assert_eq!(fire.level, ParameterLevel::Type);
    assert_eq!(fire.source_element_id, 12499);
}

#[test]
fn r2025_walls_take_their_wall_types_name() {
    let Some(dir) =
        std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(|d| PathBuf::from(d).join("r2025"))
    else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut checked = 0;
    for name in ["01_Walls", "08_LShapedHipRoof", "18_WallJoins"] {
        let model = dir.join(format!("{name}.rvt"));
        let Ok(step) = std::fs::read_to_string(model.with_extension("ifc")) else {
            eprintln!("skipping: no R2025 model {name}");
            continue;
        };
        // Wall Tag -> its type's Tag and name, as Revit's export relates them.
        let ents = entities(&step);
        let field = |args: &str, i: usize| {
            decode_step_string(split_args(args)[i].trim().trim_matches('\''))
        };
        let mut types: BTreeMap<u64, (i64, String)> = BTreeMap::new();
        for (entity, args) in ents.values() {
            if entity != "IFCRELDEFINESBYTYPE" {
                continue;
            }
            let f = split_args(args);
            let Some(("IFCWALLTYPE", type_args)) = f[5]
                .trim()
                .strip_prefix('#')
                .and_then(|id| id.parse::<u64>().ok())
                .and_then(|id| ents.get(&id))
                .map(|(e, a)| (e.as_str(), a.as_str()))
            else {
                continue;
            };
            let type_tag: i64 = field(type_args, 7).parse().expect("type Tag");
            let type_name = field(type_args, 2);
            let type_name = type_name
                .split_once(':')
                .map_or(type_name.as_str(), |(_, n)| n);
            for wall in list(&f[4]) {
                if let Some(("IFCWALL", wall_args)) = ents.get(&wall).map(|(e, a)| (e.as_str(), a))
                {
                    let tag = field(wall_args, 7).parse().expect("wall Tag");
                    types.insert(tag, (type_tag, type_name.to_string()));
                }
            }
        }
        assert!(!types.is_empty(), "{name}: Revit's export types no wall");
        let mut file = RevitFile::open(&model).expect("model");
        let ids: BTreeSet<u64> = types.keys().copied().collect();
        let ours =
            rvt::research::native_element_parameters::read(&mut file, &ids).expect("parameters");
        for (tag, (type_tag, type_name)) in &types {
            let symbol_name = ours.elements[tag]
                .iter()
                .find(|p| p.parameter_id == -1_002_001)
                .unwrap_or_else(|| panic!("{name}: wall {tag} lists no type name"));
            assert_eq!(
                symbol_name.level,
                ParameterLevel::Type,
                "{name}: wall {tag}"
            );
            assert_eq!(
                symbol_name.source_element_id, *type_tag,
                "{name}: wall {tag}"
            );
            assert_eq!(symbol_name.value, type_name.as_str(), "{name}: wall {tag}");
            checked += 1;
        }
    }
    if checked > 0 {
        assert_eq!(checked, 5 + 7 + 9, "the walls of models 01, 08 and 18");
    }
}
