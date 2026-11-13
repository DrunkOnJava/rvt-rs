//! #1865: which curtain wall holds the curtain members that get no
//! `IsExternal` on one save of a project where another save's export writes it.
//!
//! FACT under test: a curtain mullion or panel takes `IsExternal` from its
//! curtain wall, which takes it from its own parameter or its type's Function
//! (`attach_curtain_member_is_external`, RE-193, RE-269). On Autodesk's
//! `rac_advanced` saved by Revit 2027, 79 curtain members get none where the
//! 2025 save writes it, and the 2026 save writes all 3,192 (#1865). The
//! 2027 export's diagnostics differ from the 2026 export's in one line that
//! touches curtain walls (`curtain_wall_box_not_read` 3 on 2027), so the 79 may
//! be the members of three walls. This probe names them: per curtain wall of
//! each save, its own `IsExternal` and how many of its parts carry one, then
//! each part left without, then each element that lost its `IsExternal`
//! between the two saves with the curtain wall it is a part of, and what
//! differs in the curtain walls between the saves.
//!
//! The probe:
//! - with one file, reports that file's curtain walls;
//! - with two (Measure's `probe_pair_release`: a sample, then the same project
//!   saved by another release), reports both and compares their exports.
//!
//! Output (the samples are Autodesk's and redistributable; the held-out models
//! are not run with this probe, which prints names and ElementIds):
//!   curtain_wall side=<left|right> tag=<ElementId> name=<name> is_external=<true|false|absent>
//!     parts=<n> members=<n> plates=<n> other=<n> with_is_external=<n> without_is_external=<n>
//!   curtain_part_without side=<left|right> wall=<ElementId> tag=<ElementId> type=<IFC entity> name=<name>
//!   curtain_walls side=<left|right> n=<count> parts=<n> without_is_external=<n> walls_holding_them=<n>
//!   is_external_lost <IFC entity>=<n> ...   (written on the left, absent on the right; pairs only)
//!   is_external_lost_element tag=<ElementId> type=<IFC entity> wall=<ElementId or none> name=<name>
//!     (`wall=none`: the right export aggregates it into no curtain wall)
//!   curtain_wall_change tag=<ElementId> name=<name> <field> <left> -> <right>   (pairs only)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1865_curtain_is_external -- A.rvt [B.rvt]

use std::collections::{BTreeMap, HashMap};

use rvt::RevitFile;
use rvt::ifc::element_diff::{diff_elements, element_states};
use rvt::ifc::entities::{IfcEntity, PropertySet, PropertyValue};
use rvt::ifc::{ExportOptions, IfcModel, run_export};
use serde_json::Value;

/// The longest a printed property value is, characters.
const SHOWN: usize = 80;

/// The common property set `IsExternal` is written in on a curtain wall and
/// on each of its parts.
fn common_set(ifc_type: &str) -> Option<&'static str> {
    match ifc_type {
        "IFCCURTAINWALL" => Some("Pset_CurtainWallCommon"),
        "IFCMEMBER" => Some("Pset_MemberCommon"),
        "IFCPLATE" => Some("Pset_PlateCommon"),
        _ => None,
    }
}

fn export(path: &str) -> anyhow::Result<IfcModel> {
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    Ok(run_export(&mut rf, &ExportOptions::default())?.model)
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > SHOWN {
        let cut: String = text.chars().take(SHOWN).collect();
        format!("{cut}...")
    } else {
        text
    }
}

fn show(flag: Option<bool>) -> &'static str {
    match flag {
        Some(true) => "true",
        Some(false) => "false",
        None => "absent",
    }
}

/// A building element of the model: its IFC entity, name and Tag (the Revit
/// ElementId), and its `IsExternal` where its common set holds one.
struct Element {
    ifc_type: String,
    name: String,
    tag: String,
    external: Option<bool>,
}

/// A model with the property sets attached to its elements and the parts
/// each whole is aggregated from.
struct View<'a> {
    model: &'a IfcModel,
    attached: HashMap<usize, Vec<&'a PropertySet>>,
    parts_of: BTreeMap<usize, Vec<usize>>,
}

impl<'a> View<'a> {
    fn new(model: &'a IfcModel) -> Self {
        let mut attached: HashMap<usize, Vec<&PropertySet>> = HashMap::new();
        let mut parts_of: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for entity in &model.entities {
            match entity {
                IfcEntity::ElementPropertySet {
                    element: owner,
                    set,
                } => {
                    attached.entry(*owner).or_default().push(set);
                }
                IfcEntity::Aggregate { whole, parts } => {
                    parts_of
                        .entry(*whole)
                        .or_default()
                        .extend(parts.iter().copied());
                }
                _ => {}
            }
        }
        View {
            model,
            attached,
            parts_of,
        }
    }

    fn element(&self, index: usize) -> Option<Element> {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            property_set,
            ..
        } = self.model.entities.get(index)?
        else {
            return None;
        };
        let external = common_set(ifc_type).and_then(|wanted| {
            let own = property_set.iter();
            let more = self.attached.get(&index).into_iter().flatten().copied();
            own.chain(more)
                .filter(|set| set.name == wanted)
                .flat_map(|set| &set.properties)
                .find_map(|property| match (property.name.as_str(), &property.value) {
                    ("IsExternal", PropertyValue::Boolean(flag)) => Some(*flag),
                    _ => None,
                })
        });
        Some(Element {
            ifc_type: ifc_type.clone(),
            name: name.clone(),
            tag: type_guid.clone().unwrap_or_else(|| "-".to_string()),
            external,
        })
    }

    /// Every curtain wall that has parts, with its own element and its parts.
    fn curtain_walls(&self) -> Vec<(Element, Vec<Element>)> {
        let mut out = Vec::new();
        for (whole, parts) in &self.parts_of {
            let Some(wall) = self.element(*whole) else {
                continue;
            };
            if wall.ifc_type != "IFCCURTAINWALL" {
                continue;
            }
            let found: Vec<Element> = parts
                .iter()
                .filter_map(|part| self.element(*part))
                .collect();
            out.push((wall, found));
        }
        out
    }

    /// Print the curtain walls, each with its parts and how many carry
    /// `IsExternal`, then the parts left without.
    fn report(&self, side: &str) {
        let (mut walls, mut parts_total, mut without_total, mut holding) = (0usize, 0usize, 0, 0);
        for (wall, found) in self.curtain_walls() {
            walls += 1;
            let members = found.iter().filter(|p| p.ifc_type == "IFCMEMBER").count();
            let plates = found.iter().filter(|p| p.ifc_type == "IFCPLATE").count();
            let without: Vec<&Element> = found.iter().filter(|p| p.external.is_none()).collect();
            println!(
                "curtain_wall side={side} tag={} name={} is_external={} parts={} members={members} plates={plates} other={} with_is_external={} without_is_external={}",
                wall.tag,
                wall.name,
                show(wall.external),
                found.len(),
                found.len() - members - plates,
                found.len() - without.len(),
                without.len()
            );
            for part in &without {
                println!(
                    "curtain_part_without side={side} wall={} tag={} type={} name={}",
                    wall.tag, part.tag, part.ifc_type, part.name
                );
            }
            parts_total += found.len();
            without_total += without.len();
            holding += usize::from(!without.is_empty());
        }
        println!(
            "curtain_walls side={side} n={walls} parts={parts_total} without_is_external={without_total} walls_holding_them={holding}"
        );
    }

    /// The Tag of the curtain wall each part's Tag is aggregated into.
    fn wall_of_part(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for (wall, found) in self.curtain_walls() {
            for part in found {
                out.entry(part.tag).or_insert_with(|| wall.tag.clone());
            }
        }
        out
    }
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let files: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    let left_model = match files.first() {
        Some(path) => export(path)?,
        None => anyhow::bail!("usage: probe_1865_curtain_is_external A.rvt [B.rvt]"),
    };
    let left = View::new(&left_model);
    let Some(second) = files.get(1) else {
        left.report("left");
        return Ok(());
    };
    let right_model = export(second)?;
    let right = View::new(&right_model);
    left.report("left");
    right.report("right");
    let wall_of = right.wall_of_part();
    let diff = diff_elements(&element_states(&left_model), &element_states(&right_model));
    let mut lost: BTreeMap<&str, usize> = BTreeMap::new();
    for change in &diff.changed {
        for field in &change.changes {
            if field.field.ends_with(".IsExternal")
                && !field.left.is_null()
                && field.right.is_null()
            {
                *lost.entry(change.ifc_type.as_str()).or_default() += 1;
                let tag = change.tag.as_deref().unwrap_or("-");
                println!(
                    "is_external_lost_element tag={tag} type={} wall={} name={}",
                    change.ifc_type,
                    wall_of.get(tag).map_or("none", String::as_str),
                    change.name
                );
            }
            if change.ifc_type == "IFCCURTAINWALL" {
                println!(
                    "curtain_wall_change tag={} name={} {} {} -> {}",
                    change.tag.as_deref().unwrap_or("-"),
                    change.name,
                    field.field,
                    short(&field.left),
                    short(&field.right)
                );
            }
        }
    }
    println!(
        "is_external_lost {}",
        lost.iter()
            .map(|(kind, count)| format!("{kind}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    Ok(())
}
