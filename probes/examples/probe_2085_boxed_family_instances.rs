//! #2085: of the family instances an export leaves as a box because the file holds no saved
//! mesh for them, how many could be drawn from their family's own extrusions?
//!
//! FACT under test: `--family-extrusions` draws an instance from its family's extrusions only
//! where they reproduce the instance's own saved mesh, so an instance with no saved mesh never
//! enters it and keeps its record box. Rebuilt by the same rules (no void in the family, every
//! visible extrusion rebuilt, placed by the instance's transform), how many of those instances
//! would have a body, and how close is it to Revit's own (`tools/re/probe_2085_vs_ifc.py`
//! scores the second against Revit's export)?
//!
//! For each exported element whose `SavedMeshRefusal` is `no_saved_mesh` or
//! `no_saved_graphics` (a body still its record box, with nothing saved to draw it from), the
//! probe finds its symbol (`FamilyInstance.m_masterSymbolId`) and family
//! (`FamilySymbol.m_familyId`), reads the family's forms, rebuilds its extrusions as
//! `ifc::family_extrusions` does, places them by the instance's transform (RE-87) and prints:
//!   instance tag=<ElementId> class=<IFC entity> refusal=<..> family=<ElementId|-> extrusions=<n>
//!     voids=<n> others=<n> rebuilt=<yes|why not> transform=<yes|no> volume=<ft3|-> low=<x,y,z|->
//!     high=<x,y,z|->
//!   - `others`: the family's sweeps, blends, revolutions and swept blends, which the rebuild
//!     does not draw: a rebuilt body of a family that has any is short of them;
//!   - `volume`: the rebuilt solids' volumes summed (two that overlap are counted twice);
//!   - `low` and `high`: the corners of the rebuilt solids' box in the model's frame, feet.
//!   nested family=<ElementId> instances=<n> with_graphics=<n> whole=<n> triangles=<n>: for each
//!     family with no form the rebuild knows, its nested instances, those its document saved
//!     graphics for, and those whose graphics mesh whole. Where every one does, a boxed
//!     instance of the family is drawn from those meshes placed by its transform
//!     (`rebuilt=from_nested_graphics`), and its `volume`, `low` and `high` are that mesh's;
//!   nested_empty family=<ElementId> instance=<ElementId> objects=<class:n,...> detail1=<n>
//!     detail2=<n> master=<id:classes:triangles> referenced=<id:classes:triangles,...>: for a
//!     nested instance whose graphics mesh whole but draw no triangle, where its solids are:
//!     what its graphics hold, what they draw at the coarse and medium detail levels, and for
//!     its symbol and each symbol its graphics' instance records name, that id's record
//!     classes in the family's document (`absent` where it has none) and what its graphics
//!     draw (`-` where it has none);
//!   nested_family family=<ElementId> instances=<n> documents=<n> extrusions=<n> voids=<n>
//!     revolutions=<n> forms=<class:n,...> from_graphics=<n>
//!     rebuilt=<yes:solids+triangles|why not>: for a family with no form the rebuild knows
//!     that nests instances, their families' own documents found and what those hold
//!     (`forms`: every model form but extrusions, nested instances included). Each nested
//!     family is drawn from the saved graphics its own document holds for its visible solid
//!     forms where every one meshes whole (`from_graphics` counts those), else from its
//!     extrusions rebuilt by the same rules and its revolutions meshed by the shared
//!     revolution reader, with no other form; it is placed in the family by the transform
//!     its nested instance's saved graphics hold, refused where that is not a rotation, and
//!     a boxed instance is drawn from all of them placed by its own
//!     (`rebuilt=from_nested_family`, and `nested=` on its line says how its family's nested
//!     families went);
//!   formless family=<ElementId> records=<n> drawing=<class:n,...> classes=<class:n,...>: for
//!     each family with no
//!     extrusion and no other form the rebuild knows, every record class its document holds,
//!     most first (12 at most), so that what draws such a family (a nested instance, a
//!     free-form solid, an import) can be told;
//!   tally <key> <n>
//! The export follows no nested family; the probe does, to count what following one would draw.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2085_boxed_family_instances -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Options};
use rvt::research::native_family_forms::{
    FamilyRecords, family_records_in, nested_family_records_in,
};
use rvt::research::native_form_geometry::{ExtrusionSolid, FormDocument, Frame};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{
    graphics_including_filtered, graphics_with_resolver, graphics_with_resolver_at_detail,
};
use rvt::research::partition_instance_transforms::{InstanceTransform, scan_instance_transforms};

/// The refusals of a body the file holds nothing to draw from.
const NO_MESH: [&str; 2] = ["no_saved_mesh", "no_saved_graphics"];
/// Forms a family can hold that the rebuild does not draw.
const OTHER_FORMS: [&str; 4] = ["SweepElem", "BlendElem", "RevolutionElem", "SweptBlendElem"];

type V3 = [f64; 3];

/// `class.field` of each record of `ids` whose root is `class`: the ElementId it names.
fn named_ids(
    rf: &mut RevitFile,
    ids: &BTreeSet<u64>,
    class: &str,
    field: &str,
) -> Result<BTreeMap<u64, u64>> {
    let mut out = BTreeMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let options = Options {
        selected_ids: ids.clone(),
        ..Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let target = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
            .filter(|root| root.class_name == class)
            .and_then(|root| root.fields.get(field))
            .and_then(|value| value.get("m_id"))
            .and_then(|id| id.get("m_id64").or_else(|| id.get("m_id")))
            .and_then(|id| id.as_i64())
            .and_then(|n| u64::try_from(n).ok());
        if let Some(target) = target {
            out.insert(record.identity.element_id, target);
        }
        Ok(())
    })?;
    Ok(out)
}

/// What the family's document holds and, where the rebuild draws it, its solids.
struct Family {
    extrusions: usize,
    voids: usize,
    others: usize,
    solids: Result<Vec<ExtrusionSolid>, String>,
}

fn family(document: &FormDocument, others: usize) -> Family {
    let ids = document.extrusion_ids();
    let mut voids = 0;
    let mut solids = Vec::new();
    let mut refused: Option<String> = None;
    for id in &ids {
        match document.extrusion(*id) {
            Ok(extrusion) if extrusion.cutting => voids += 1,
            Ok(extrusion) if extrusion.visible == Some(0) => {}
            Ok(extrusion) => match extrusion.default_solid() {
                Ok(solid) => solids.push(solid),
                Err(why) => {
                    refused.get_or_insert(why);
                }
            },
            Err(why) => {
                refused.get_or_insert(why);
            }
        }
    }
    let solids = if ids.is_empty() {
        Err("no extrusion".to_string())
    } else if voids > 0 {
        Err("a void".to_string())
    } else if let Some(why) = refused {
        Err(why)
    } else if solids.is_empty() {
        Err("no visible extrusion".to_string())
    } else {
        Ok(solids)
    };
    Family {
        extrusions: ids.len(),
        voids,
        others,
        solids,
    }
}

/// `solid`, in the family's coordinates, placed in the model by `transform`.
fn placed(solid: &ExtrusionSolid, transform: &InstanceTransform) -> ExtrusionSolid {
    let carry =
        |v: V3| [0usize, 1, 2].map(|i| (0..3).map(|j| transform.axes[j][i] * v[j]).sum::<f64>());
    let moved = carry(solid.frame.origin);
    ExtrusionSolid {
        profile: solid.profile.clone(),
        frame: Frame {
            origin: [0usize, 1, 2].map(|i| transform.origin[i] + moved[i]),
            axes: solid.frame.axes.map(carry),
        },
        low: solid.low,
        high: solid.high,
    }
}

/// An ElementId field's value, in either saved shape (`m_id` or `m_id64`).
fn element_id(value: &Fv) -> Option<u64> {
    if let Some(n) = value.as_i64() {
        return u64::try_from(n).ok();
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(element_id)
}

/// Triangles `graph` meshes to, drawn and not hidden or filtered, at `detail`.
fn drawn_triangles<'a>(
    graph: &'a ObjectGraph,
    graphics: &BTreeMap<u64, &'a ObjectGraph>,
    detail: i64,
) -> String {
    let meshes =
        graphics_with_resolver_at_detail(graph, &|other| graphics.get(&other).copied(), detail);
    if !meshes.diagnostics.is_empty() {
        return "refused".to_string();
    }
    meshes
        .primitives
        .iter()
        .filter(|p| !p.hidden && !p.filtered)
        .map(|p| p.triangles.len())
        .sum::<usize>()
        .to_string()
}

/// Where the solids of a nested instance whose saved graphics mesh whole but draw no
/// triangle are: its graphics' object classes, what it draws at the coarse and medium
/// detail levels, and, for its symbol (`m_masterSymbolId`) and every symbol its graphics'
/// instance records name (`InstanceInfo.m_symbolId`), the classes of that id's records in
/// the family's document and the triangles of its graphics there, at fine detail.
fn empty_nested<'a>(
    family: u64,
    instance: u64,
    graph: &'a ObjectGraph,
    items: &'a [(u64, ObjectGraph)],
    graphics: &BTreeMap<u64, &'a ObjectGraph>,
) -> String {
    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
    for object in &graph.objects {
        *classes.entry(object.class_name.as_str()).or_default() += 1;
    }
    let listed: Vec<String> = classes.iter().map(|(c, n)| format!("{c}:{n}")).collect();
    let record_classes = |id: u64| -> String {
        let found: Vec<&str> = items
            .iter()
            .filter(|(record, _)| *record == id)
            .filter_map(|(_, g)| g.objects.first().map(|o| o.class_name.as_str()))
            .collect();
        if found.is_empty() {
            "absent".to_string()
        } else {
            found.join("+")
        }
    };
    let describe = |id: u64| -> String {
        let triangles = graphics
            .get(&id)
            .copied()
            .map(|g| drawn_triangles(g, graphics, 3))
            .unwrap_or_else(|| "-".to_string());
        format!("{id}:{}:{triangles}", record_classes(id))
    };
    let master = items
        .iter()
        .filter(|(record, _)| *record == instance)
        .filter_map(|(_, g)| g.objects.first())
        .find(|root| root.class_name == "FamilyInstance")
        .and_then(|root| root.fields.get("m_masterSymbolId"))
        .and_then(element_id);
    let referenced: Vec<String> = graph
        .objects
        .iter()
        .filter(|o| o.class_name == "InstanceInfo")
        .map(|o| match o.fields.get("m_symbolId").and_then(element_id) {
            Some(id) => describe(id),
            None => "unread".to_string(),
        })
        .collect();
    format!(
        "nested_empty family={family} instance={instance} objects={} detail1={} detail2={} master={} referenced={}",
        listed.join(","),
        drawn_triangles(graph, graphics, 1),
        drawn_triangles(graph, graphics, 2),
        master.map_or_else(|| "unread".to_string(), describe),
        if referenced.is_empty() {
            "-".to_string()
        } else {
            referenced.join(",")
        }
    )
}

/// The transform a nested instance's saved graphics place its symbol by
/// (`InstanceInfo.m_Trf`): the 3x3 matrix and the origin, in the family's coordinates.
fn nested_transform(graph: &ObjectGraph) -> Option<([V3; 3], V3)> {
    let trf = graph
        .objects
        .iter()
        .find(|o| o.class_name == "InstanceInfo")?
        .fields
        .get("m_Trf")?;
    let triple = |v: &Fv| -> Option<V3> {
        let items = v.as_array().filter(|items| items.len() == 3)?;
        let mut out = [0.; 3];
        for (k, item) in items.iter().enumerate() {
            out[k] = item.as_f64().filter(|x| x.is_finite())?;
        }
        Some(out)
    };
    let rows = trf
        .get("m_3x3")?
        .as_array()
        .filter(|rows| rows.len() == 3)?;
    let matrix = [triple(&rows[0])?, triple(&rows[1])?, triple(&rows[2])?];
    Some((matrix, triple(trf.get("m_or")?)?))
}

/// Whether a 3x3 matrix is a rotation: orthonormal rows and a determinant of 1, within
/// 1e-9, so no scale, shear or mirror (the saved graphics refuse a scaled instance too).
fn is_rotation(m: &[V3; 3]) -> bool {
    let dot = |a: V3, b: V3| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let orthonormal = (0..3)
        .all(|a| (0..3).all(|b| (dot(m[a], m[b]) - if a == b { 1.0 } else { 0.0 }).abs() < 1e-9));
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    orthonormal && (det - 1.0).abs() < 1e-9
}

/// The model form classes whose saved graphics draw a family's geometry.
const FORM_CLASSES: [&str; 8] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "SweptBlendElem",
    "FreeFormElement",
    "Text3dElem",
    "ImportInstance",
];

/// A form's stored `Visible` (`IS_VISIBLE_PARAM`, -1006205), where it holds one.
fn stored_visible(graph: &ObjectGraph) -> Option<i64> {
    graph
        .objects
        .iter()
        .filter(|o| o.class_name == "ParamValueSetInt")
        .filter_map(|o| o.fields.get("m_paramSet")?.as_array())
        .flatten()
        .find(|entry| entry.get("m_paramId").and_then(element_id_signed) == Some(-1_006_205))
        .and_then(|entry| entry.get("m_value")?.as_i64())
}

/// [`element_id`], keeping the sign (a built-in parameter's id is negative).
fn element_id_signed(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(element_id_signed)
}

/// What a nested family's own document saved as graphics for its visible solid forms,
/// meshed, in its own coordinates; else why not. Every form that is not a void and not
/// stored hidden must have graphics that mesh whole with a triangle.
fn nested_form_graphics(records: &[(u64, ObjectGraph)]) -> Result<NestedBodies, String> {
    let graphics: BTreeMap<u64, &ObjectGraph> = records
        .iter()
        .filter(|(_, g)| {
            g.objects
                .first()
                .is_some_and(|r| r.class_name == "GElement")
        })
        .map(|(id, g)| (*id, g))
        .collect();
    let mut bodies = NestedBodies::default();
    let mut forms = 0;
    for (id, graph) in records {
        let Some(root) = graph
            .objects
            .first()
            .filter(|r| FORM_CLASSES.contains(&r.class_name.as_str()))
        else {
            continue;
        };
        if root.fields.get("m_cutting").and_then(Fv::as_bool) == Some(true)
            || stored_visible(graph) == Some(0)
        {
            continue;
        }
        forms += 1;
        let Some(own) = graphics.get(id) else {
            return Err(format!("{} has no graphics", root.class_name));
        };
        let meshes = graphics_with_resolver(own, &|other| graphics.get(&other).copied());
        if let Some(why) = meshes.diagnostics.first() {
            return Err(format!("{} graphics {why}", root.class_name));
        }
        if meshes.unbounded_faces != 0 {
            return Err(format!("{} graphics unbounded faces", root.class_name));
        }
        let before = bodies.triangles.len();
        for primitive in meshes
            .primitives
            .iter()
            .filter(|p| !p.hidden && !p.filtered)
        {
            let base = bodies.vertices.len();
            bodies.vertices.extend(primitive.vertices.iter().copied());
            bodies
                .triangles
                .extend(primitive.triangles.iter().filter_map(|t| {
                    let corner = |i: u32| usize::try_from(i).ok().map(|i| base + i);
                    Some([corner(t[0])?, corner(t[1])?, corner(t[2])?])
                }));
        }
        if bodies.triangles.len() == before {
            return Err(format!("{} graphics draw no triangle", root.class_name));
        }
    }
    if forms == 0 {
        return Err("no visible solid form".to_string());
    }
    Ok(bodies)
}

/// What a formless family's nested families draw, in the formless family's coordinates:
/// their extrusions as solids and their revolutions as meshes.
#[derive(Default)]
struct NestedBodies {
    solids: Vec<ExtrusionSolid>,
    vertices: Vec<V3>,
    triangles: Vec<[usize; 3]>,
}

/// What one nested family's own document draws, in its own coordinates, or why not:
/// its extrusions by the rules of [`family`] and its revolutions by the shared revolution
/// reader (`FormDocument::revolution`, the one `--family-revolutions` uses). A sweep, blend
/// or swept blend refuses it, as the rebuild draws none of them.
struct NestedFamily {
    extrusions: usize,
    voids: usize,
    revolutions: usize,
    /// Every model form class its document holds but extrusions, counted.
    forms: BTreeMap<String, usize>,
    bodies: Result<NestedBodies, String>,
    /// Its visible solid forms' own saved graphics, meshed ([`nested_form_graphics`]).
    graphics: Result<NestedBodies, String>,
}

fn nested_family(records: Vec<(u64, ObjectGraph)>) -> NestedFamily {
    let mut forms: BTreeMap<String, usize> = BTreeMap::new();
    for (_, graph) in &records {
        if let Some(root) = graph.objects.first().filter(|root| {
            OTHER_FORMS.contains(&root.class_name.as_str())
                || ["FreeFormElement", "ImportInstance", "FamilyInstance"]
                    .contains(&root.class_name.as_str())
        }) {
            *forms.entry(root.class_name.clone()).or_default() += 1;
        }
    }
    let graphics = nested_form_graphics(&records);
    let mut document = FormDocument::default();
    for (record, graph) in records {
        document.insert(record, graph);
    }
    let own = family(&document, 0);
    let revolution_ids = document.revolution_ids();
    let undrawn: Vec<&String> = forms
        .keys()
        .filter(|class| class.as_str() != "RevolutionElem")
        .collect();
    let bodies = (|| -> Result<NestedBodies, String> {
        if let Some(class) = undrawn.first() {
            return Err(format!("{class} not drawn"));
        }
        let mut bodies = NestedBodies::default();
        if own.extrusions > 0 {
            bodies.solids = own.solids.clone()?;
        }
        for id in &revolution_ids {
            let mesh = document
                .revolution(*id)
                .and_then(|r| r.mesh())
                .map_err(|why| format!("revolution {why}"))?;
            let base = bodies.vertices.len();
            bodies.vertices.extend(mesh.vertices);
            bodies
                .triangles
                .extend(mesh.triangles.iter().map(|t| t.map(|i| base + i)));
        }
        if bodies.solids.is_empty() && bodies.triangles.is_empty() {
            return Err("no form drawn".to_string());
        }
        Ok(bodies)
    })();
    NestedFamily {
        extrusions: own.extrusions,
        voids: own.voids,
        revolutions: revolution_ids.len(),
        forms,
        bodies,
        graphics,
    }
}

/// What a formless family's nested families draw, each rebuilt from its own document by
/// [`nested_family`] and placed in the formless family's coordinates by its nested
/// instance's saved transform, where every nested instance's are; else why not. With a
/// summary line's fields: nested instances, their families' documents found, and those
/// documents' extrusions, voids, revolutions and other forms.
fn nested_family_solids(
    nested: &[u64],
    items: &[(u64, ObjectGraph)],
    graphics: &BTreeMap<u64, &ObjectGraph>,
    documents: FamilyRecords,
) -> (String, Result<NestedBodies, String>) {
    let root = |id: u64, class: &str| {
        items
            .iter()
            .filter(|(record, _)| *record == id)
            .filter_map(|(_, graph)| graph.objects.first())
            .find(|root| root.class_name == class)
    };
    let read: BTreeMap<u64, NestedFamily> = documents
        .into_iter()
        .map(|(family_id, records)| (family_id, nested_family(records)))
        .collect();
    let (mut found, mut extrusions, mut voids, mut revolutions) = (0, 0, 0, 0);
    let mut from_graphics = 0usize;
    let mut forms: BTreeMap<String, usize> = BTreeMap::new();
    let mut placed = NestedBodies::default();
    let mut refused: Option<String> = None;
    for instance in nested {
        let family_id = root(*instance, "FamilyInstance")
            .and_then(|r| r.fields.get("m_masterSymbolId"))
            .and_then(element_id)
            .and_then(|symbol| root(symbol, "FamilySymbol"))
            .and_then(|r| r.fields.get("m_familyId"))
            .and_then(element_id);
        let Some(nested_family) = family_id.and_then(|f| read.get(&f)) else {
            refused.get_or_insert("nested family document not found".to_string());
            continue;
        };
        found += 1;
        extrusions += nested_family.extrusions;
        voids += nested_family.voids;
        revolutions += nested_family.revolutions;
        for (class, n) in &nested_family.forms {
            *forms.entry(class.clone()).or_default() += n;
        }
        let Some((matrix, origin)) = graphics.get(instance).copied().and_then(nested_transform)
        else {
            refused.get_or_insert("no nested transform".to_string());
            continue;
        };
        if !is_rotation(&matrix) {
            refused.get_or_insert("nested transform not a rotation".to_string());
            continue;
        }
        let carry =
            |v: V3| [0usize, 1, 2].map(|i| (0..3).map(|j| matrix[i][j] * v[j]).sum::<f64>());
        let place = |v: V3| {
            let moved = carry(v);
            [0usize, 1, 2].map(|i| origin[i] + moved[i])
        };
        // The forms' own saved graphics first, as Revit tessellated them; else the
        // rebuilt extrusions and revolutions.
        match nested_family
            .graphics
            .as_ref()
            .or(nested_family.bodies.as_ref())
        {
            Ok(own) => {
                if nested_family.graphics.is_ok() {
                    from_graphics += 1;
                }
                for solid in &own.solids {
                    placed.solids.push(ExtrusionSolid {
                        profile: solid.profile.clone(),
                        frame: Frame {
                            origin: place(solid.frame.origin),
                            axes: solid.frame.axes.map(carry),
                        },
                        low: solid.low,
                        high: solid.high,
                    });
                }
                let base = placed.vertices.len();
                placed
                    .vertices
                    .extend(own.vertices.iter().map(|v| place(*v)));
                placed
                    .triangles
                    .extend(own.triangles.iter().map(|t| t.map(|i| base + i)));
            }
            Err(why) => {
                let graphics_why = nested_family
                    .graphics
                    .as_ref()
                    .err()
                    .map_or("-", |w| w.as_str());
                refused.get_or_insert(format!("nested {why}; graphics {graphics_why}"));
            }
        }
    }
    let listed: Vec<String> = forms.iter().map(|(c, n)| format!("{c}:{n}")).collect();
    let summary = format!(
        "instances={} documents={found} extrusions={extrusions} voids={voids} revolutions={revolutions} forms={} from_graphics={from_graphics}",
        nested.len(),
        if listed.is_empty() {
            "-".to_string()
        } else {
            listed.join(",")
        }
    );
    let result = match refused {
        Some(why) => Err(why),
        None if placed.solids.is_empty() && placed.triangles.is_empty() => {
            Err("nested no form drawn".to_string())
        }
        None => Ok(placed),
    };
    (summary, result)
}

/// The enclosed volume of a closed mesh about its own centre, and its box.
fn mesh_volume_box(vertices: &[V3], triangles: &[[usize; 3]]) -> (f64, V3, V3) {
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for v in vertices {
        for k in 0..3 {
            low[k] = low[k].min(v[k]);
            high[k] = high[k].max(v[k]);
        }
    }
    let n = vertices.len().max(1) as f64;
    let centre: V3 = [0usize, 1, 2].map(|k| vertices.iter().map(|v| v[k]).sum::<f64>() / n);
    let volume = triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| {
                let v = vertices[i];
                [v[0] - centre[0], v[1] - centre[1], v[2] - centre[2]]
            });
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0
        })
        .sum::<f64>()
        .abs();
    (volume, low, high)
}

/// A why-not with its digits written as `#`, so that reasons that differ only in a number are
/// one tally, and its spaces as `_`, so that it is one field.
fn reason(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '0'..='9' => '#',
            ' ' => '_',
            other => other,
        })
        .take(80)
        .collect()
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2085_boxed_family_instances MODEL.rvt"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.basic_file_info()?.version;
    let export = run_export(&mut rf, &ExportOptions::default())?;
    // ElementId -> (IFC entity, refusal) of every body the file holds nothing to draw from.
    let mut boxed: BTreeMap<u64, (String, String)> = BTreeMap::new();
    let mut refusals = 0usize;
    for entity in &export.model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        let refusal = set.properties.iter().find_map(|p| match &p.value {
            PropertyValue::Text(text) if p.name == "SavedMeshRefusal" => Some(text.clone()),
            _ => None,
        });
        let Some(refusal) = refusal else {
            continue;
        };
        refusals += 1;
        if let (Ok(id), true) = (tag.parse::<u64>(), NO_MESH.contains(&refusal.as_str())) {
            boxed.insert(id, (ifc_type.clone(), refusal));
        }
    }
    let ids: BTreeSet<u64> = boxed.keys().copied().collect();
    let symbol_of = named_ids(&mut rf, &ids, "FamilyInstance", "m_masterSymbolId")?;
    let symbols: BTreeSet<u64> = symbol_of.values().copied().collect();
    let family_of_symbol = named_ids(&mut rf, &symbols, "FamilySymbol", "m_familyId")?;
    let families: BTreeSet<u64> = family_of_symbol.values().copied().collect();
    let classes: BTreeSet<String> = ["ExtrusionElem", "CurveElem", "SketchPlane"]
        .iter()
        .chain(OTHER_FORMS.iter())
        .map(|class| class.to_string())
        .collect();
    let records = family_records_in(
        &mut rf,
        &families,
        &classes,
        &Options::default().releases,
        None,
    )?;
    let mut read: BTreeMap<u64, Family> = BTreeMap::new();
    for (id, items) in records {
        let others = items
            .iter()
            .filter(|(_, graph)| {
                graph
                    .objects
                    .first()
                    .is_some_and(|root| OTHER_FORMS.contains(&root.class_name.as_str()))
            })
            .count();
        let mut document = FormDocument::default();
        for (record, graph) in items {
            document.insert(record, graph);
        }
        read.insert(id, family(&document, others));
    }
    // What a family with no form the rebuild reads holds instead: every record
    // class of its document, counted (nested instances are `FamilyInstance`).
    let formless: BTreeSet<u64> = families
        .iter()
        .filter(|f| {
            read.get(f)
                .is_none_or(|family| family.extrusions == 0 && family.others == 0)
        })
        .copied()
        .collect();
    let everything = family_records_in(
        &mut rf,
        &formless,
        &BTreeSet::new(),
        &Options::default().releases,
        None,
    )?;
    // A formless family's nested instances, drawn from the graphics the family's own
    // document saved for them, in the family's coordinates: (vertices, triangles),
    // where every nested instance has graphics that mesh whole.
    let mut nested_meshes: BTreeMap<u64, (Vec<V3>, Vec<[usize; 3]>)> = BTreeMap::new();
    // A formless family's nested families' own extrusions, each read from its own
    // document and placed by its nested instance's saved transform.
    let mut nested_records = nested_family_records_in(
        &mut rf,
        &formless,
        &BTreeSet::new(),
        &Options::default().releases,
    )?;
    let mut nested_solids: BTreeMap<u64, Result<NestedBodies, String>> = BTreeMap::new();
    for (id, items) in &everything {
        let graphics: BTreeMap<u64, &ObjectGraph> = items
            .iter()
            .filter(|(_, graph)| {
                graph
                    .objects
                    .first()
                    .is_some_and(|root| root.class_name == "GElement")
            })
            .map(|(record, graph)| (*record, graph))
            .collect();
        let nested: Vec<u64> = items
            .iter()
            .filter(|(_, graph)| {
                graph
                    .objects
                    .first()
                    .is_some_and(|root| root.class_name == "FamilyInstance")
            })
            .map(|(record, _)| *record)
            .collect();
        let (mut with_graphics, mut whole) = (0usize, 0usize);
        // Why a nested instance's graphics do not mesh whole, digits as `#`, counted.
        let mut why: BTreeMap<String, usize> = BTreeMap::new();
        let mut vertices: Vec<V3> = Vec::new();
        let mut triangles: Vec<[usize; 3]> = Vec::new();
        // Where a whole mesh holds no triangle: what the walk left out, and how many
        // triangles it holds with hidden and filtered branches read too.
        let (mut visibility, mut filters, mut non_surface, mut primitives) = (0, 0, 0, 0);
        let (mut hidden_triangles, mut instance_refs) = (0usize, 0usize);
        for instance in &nested {
            let Some(graph) = graphics.get(instance) else {
                continue;
            };
            with_graphics += 1;
            let meshes = graphics_with_resolver(graph, &|other| graphics.get(&other).copied());
            if !meshes.diagnostics.is_empty() || meshes.unbounded_faces != 0 {
                for diagnostic in &meshes.diagnostics {
                    *why.entry(reason(diagnostic)).or_default() += 1;
                }
                if meshes.unbounded_faces != 0 {
                    *why.entry("unbounded_faces".to_string()).or_default() += 1;
                }
                continue;
            }
            whole += 1;
            let own: usize = meshes
                .primitives
                .iter()
                .filter(|p| !p.hidden && !p.filtered)
                .map(|p| p.triangles.len())
                .sum();
            if own == 0 {
                println!("{}", empty_nested(*id, *instance, graph, items, &graphics));
            }
            visibility += meshes.excluded_visibility_branches;
            filters += meshes.rejected_filters;
            non_surface += meshes.excluded_non_surface_branches;
            primitives += meshes.primitives.len();
            let wide =
                graphics_including_filtered(graph, &|other| graphics.get(&other).copied(), 3);
            hidden_triangles += wide
                .primitives
                .iter()
                .map(|p| p.triangles.len())
                .sum::<usize>();
            // Instance references in the nested instance's graphics, which point to
            // the nested family's own symbol graphics in another document.
            instance_refs += graph
                .objects
                .iter()
                .filter(|o| o.class_name == "InstanceInfo" || o.class_name == "GInstance")
                .count();
            for primitive in meshes
                .primitives
                .iter()
                .filter(|p| !p.hidden && !p.filtered)
            {
                let base = vertices.len();
                vertices.extend(primitive.vertices.iter().copied());
                for t in &primitive.triangles {
                    let corner = |i: u32| usize::try_from(i).ok().map(|i| base + i);
                    if let (Some(a), Some(b), Some(c)) = (corner(t[0]), corner(t[1]), corner(t[2]))
                    {
                        triangles.push([a, b, c]);
                    }
                }
            }
        }
        let refused: Vec<String> = why
            .iter()
            .take(6)
            .map(|(text, n)| format!("{text}:{n}"))
            .collect();
        println!(
            "nested family={id} instances={} with_graphics={with_graphics} whole={whole} triangles={} primitives={primitives} excluded_visibility={visibility} rejected_filters={filters} non_surface={non_surface} triangles_with_hidden_and_filtered={hidden_triangles} instance_refs={instance_refs} refused={}",
            nested.len(),
            triangles.len(),
            if refused.is_empty() {
                "-".to_string()
            } else {
                refused.join("|")
            }
        );
        if !nested.is_empty() && whole == nested.len() && !triangles.is_empty() {
            nested_meshes.insert(*id, (vertices, triangles));
        }
        if !nested.is_empty() {
            let (summary, result) = nested_family_solids(
                &nested,
                items,
                &graphics,
                nested_records.remove(id).unwrap_or_default(),
            );
            println!(
                "nested_family family={id} {summary} rebuilt={}",
                match &result {
                    Ok(bodies) => format!("yes:{}+{}", bodies.solids.len(), bodies.triangles.len()),
                    Err(why) => reason(why),
                }
            );
            nested_solids.insert(*id, result);
        }
        let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, graph) in items {
            if let Some(root) = graph.objects.first() {
                *classes.entry(root.class_name.as_str()).or_default() += 1;
            }
        }
        let mut ranked: Vec<(&str, usize)> = classes.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let listed: Vec<String> = ranked
            .iter()
            .take(12)
            .map(|(class, n)| format!("{class}:{n}"))
            .collect();
        // The classes that would draw such a family, counted whatever their rank.
        let drawing: Vec<String> = [
            "FamilyInstance",
            "FreeFormElement",
            "ImportInstance",
            "Text3dElem",
            "ExtrusionElem",
            "SweepElem",
            "BlendElem",
            "RevolutionElem",
            "SweptBlendElem",
        ]
        .iter()
        .map(|class| {
            format!(
                "{class}:{}",
                ranked
                    .iter()
                    .find(|(c, _)| c == class)
                    .map_or(0, |(_, n)| *n)
            )
        })
        .collect();
        println!(
            "formless family={id} records={} drawing={} classes={}",
            items.len(),
            drawing.join(","),
            listed.join(",")
        );
    }
    for id in formless.difference(&everything.keys().copied().collect()) {
        println!("formless family={id} records=0 drawing=- classes=-");
    }
    let wanted: BTreeSet<u32> = ids
        .iter()
        .filter_map(|id| u32::try_from(*id).ok())
        .collect();
    let transforms = scan_instance_transforms(&mut rf, release, &wanted)?;

    println!(
        "file revit={release} refused_bodies={refusals} no_mesh={} with_family={} families={}",
        boxed.len(),
        boxed
            .keys()
            .filter(|id| symbol_of
                .get(id)
                .is_some_and(|s| family_of_symbol.contains_key(s)))
            .count(),
        families.len(),
    );
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for (id, (class, refusal)) in &boxed {
        let family_id = symbol_of
            .get(id)
            .and_then(|symbol| family_of_symbol.get(symbol))
            .copied();
        let family = family_id.and_then(|f| read.get(&f));
        let transform = u32::try_from(*id).ok().and_then(|key| transforms.get(&key));
        let (mut rebuilt, solids) = match (family_id, family) {
            (None, _) => ("no_family".to_string(), None),
            (Some(_), None) => ("family_document_not_read".to_string(), None),
            (Some(_), Some(family)) => match &family.solids {
                Ok(solids) => ("yes".to_string(), Some(solids)),
                Err(why) => (reason(why), None),
            },
        };
        // A formless family whose nested instances' saved graphics mesh whole: that
        // mesh, placed by the project instance's transform.
        let nested_mesh = family_id
            .filter(|_| solids.is_none())
            .and_then(|f| nested_meshes.get(&f))
            .zip(transform)
            .map(|((vertices, triangles), transform)| {
                let carry = |v: V3| {
                    [0usize, 1, 2].map(|i| {
                        transform.origin[i]
                            + (0..3).map(|j| transform.axes[j][i] * v[j]).sum::<f64>()
                    })
                };
                let placed: Vec<V3> = vertices.iter().map(|v| carry(*v)).collect();
                (placed, triangles)
            });
        if nested_mesh.is_some() {
            rebuilt = "from_nested_graphics".to_string();
        }
        // A formless family whose nested families' extrusions and revolutions all rebuild:
        // those, placed by the project instance's transform.
        let nested_state = family_id.and_then(|f| nested_solids.get(&f));
        let from_nested = nested_state
            .filter(|_| solids.is_none() && nested_mesh.is_none())
            .and_then(|r| r.as_ref().ok());
        if from_nested.is_some() {
            rebuilt = "from_nested_family".to_string();
        }
        *tally
            .entry(format!("{class} rebuilt {rebuilt}"))
            .or_default() += 1;
        let placed_solids: Option<Vec<ExtrusionSolid>> = solids
            .or(from_nested.map(|bodies| &bodies.solids))
            .zip(transform)
            .map(|(solids, transform)| solids.iter().map(|s| placed(s, transform)).collect());
        if solids.is_some() {
            let key = match (&placed_solids, family.map(|f| f.others)) {
                (None, _) => "rebuilt, no transform read",
                (Some(_), Some(0)) => "rebuilt and placed, extrusions only",
                (Some(_), _) => "rebuilt and placed, with forms not drawn",
            };
            *tally.entry(key.to_string()).or_default() += 1;
        }
        // The nested families' revolution meshes, placed by the project instance's transform.
        let nested_family_mesh = from_nested
            .filter(|bodies| !bodies.triangles.is_empty())
            .zip(transform)
            .map(|(bodies, transform)| {
                let carry = |v: V3| {
                    [0usize, 1, 2].map(|i| {
                        transform.origin[i]
                            + (0..3).map(|j| transform.axes[j][i] * v[j]).sum::<f64>()
                    })
                };
                let placed: Vec<V3> = bodies.vertices.iter().map(|v| carry(*v)).collect();
                (placed, &bodies.triangles)
            });
        // Every part drawn (solids, then meshes): its volume and box, summed and joined.
        let mut parts: Vec<(f64, V3, V3)> = Vec::new();
        for solid in placed_solids.iter().flatten() {
            let (a, b) = solid.bounding_box();
            parts.push((solid.volume(), a, b));
        }
        for (vertices, triangles) in nested_mesh.iter().chain(nested_family_mesh.iter()) {
            parts.push(mesh_volume_box(vertices, triangles));
        }
        let (volume, low, high) = if parts.is_empty() {
            ("-".to_string(), "-".to_string(), "-".to_string())
        } else {
            let mut low = [f64::INFINITY; 3];
            let mut high = [f64::NEG_INFINITY; 3];
            for (_, a, b) in &parts {
                for k in 0..3 {
                    low[k] = low[k].min(a[k]);
                    high[k] = high[k].max(b[k]);
                }
            }
            let volume: f64 = parts.iter().map(|(v, _, _)| v).sum();
            let point = |p: V3| format!("{:.6},{:.6},{:.6}", p[0], p[1], p[2]);
            (format!("{volume:.6}"), point(low), point(high))
        };
        println!(
            "instance tag={id} class={class} refusal={refusal} family={} extrusions={} voids={} others={} rebuilt={rebuilt} transform={} nested={} volume={volume} low={low} high={high}",
            family_id.map_or_else(|| "-".to_string(), |f| f.to_string()),
            family.map_or(0, |f| f.extrusions),
            family.map_or(0, |f| f.voids),
            family.map_or(0, |f| f.others),
            if transform.is_some() { "yes" } else { "no" },
            match nested_state {
                None => "-".to_string(),
                Some(Ok(bodies)) =>
                    format!("yes:{}+{}", bodies.solids.len(), bodies.triangles.len()),
                Some(Err(why)) => reason(why),
            },
        );
    }
    for (key, n) in &tally {
        println!("tally {key} {n}");
    }
    Ok(())
}
