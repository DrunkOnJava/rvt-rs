//! #1117: what a project's view records hold, and whether the catalog is
//! consistent with itself.
//!
//! QUESTION under test: can every view, sheet and placed viewport be listed,
//! and does every placed viewport name a sheet and a view the same read
//! holds? The box-centre rule itself is not re-measured here: it was compared
//! with `Viewport.GetBoxCenter()` on one viewport (#1117), and this probe
//! only reports the centres that rule gives on another model.
//!
//! Output:
//!   views <n> sheets <n> viewports <n> placed <n> centres <n>
//!   kinds <kind> <n> ...                       views by their class
//!   sheet <id> "<number>" "<name>" <n>         sheet and its viewport count
//!   viewport <id> sheet <id> view <id> centre <x> <y> | centre -
//!   camera <view id> target <x> <y> <z> | target -
//!   summary <kind> <records>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1117_views -- MODEL.rvt [--large-model]

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::views::ViewCatalog;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: MODEL.rvt [--large-model]");
    let large_model = std::env::args().any(|arg| arg == "--large-model");
    let mut file =
        RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, large_model))?;
    let catalog = ViewCatalog::read(&mut file)?;
    let placed: Vec<_> = catalog
        .viewports
        .iter()
        .filter(|viewport| viewport.sheet_id.is_some())
        .collect();
    let centres = placed
        .iter()
        .filter(|viewport| viewport.center.is_some())
        .count();
    println!(
        "views {} sheets {} viewports {} placed {} centres {}",
        catalog.views.len(),
        catalog.sheets.len(),
        catalog.viewports.len(),
        placed.len(),
        centres
    );

    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for view in &catalog.views {
        *kinds
            .entry(match view.kind {
                rvt::views::ViewKind::Plan => "plan",
                rvt::views::ViewKind::ThreeD => "three_d",
                rvt::views::ViewKind::Section => "section",
                rvt::views::ViewKind::Drafting => "drafting",
                rvt::views::ViewKind::Sheet => "sheet",
                rvt::views::ViewKind::Legend => "legend",
                rvt::views::ViewKind::Schedule => "schedule",
                rvt::views::ViewKind::Other => "other",
            })
            .or_default() += 1;
    }
    let line = kinds
        .iter()
        .map(|(kind, count)| format!("{kind} {count}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!("kinds {line}");

    for sheet in &catalog.sheets {
        let viewports = catalog.viewports_on(sheet.id).count();
        println!(
            "sheet {} \"{}\" \"{}\" {viewports}",
            sheet.id, sheet.number, sheet.name
        );
    }
    for viewport in &placed {
        let sheet = viewport.sheet_id.unwrap_or(0);
        let view = viewport.view_id.unwrap_or(0);
        assert!(catalog.sheet(sheet).is_some(), "sheet {sheet} is not read");
        assert!(catalog.view(view).is_some(), "view {view} is not read");
        match viewport.center {
            Some([x, y]) => println!(
                "viewport {} sheet {sheet} view {view} centre {x} {y}",
                viewport.id
            ),
            None => println!(
                "viewport {} sheet {sheet} view {view} centre -",
                viewport.id
            ),
        }
    }
    for view in &catalog.views {
        if let Some(camera) = view.viewer.as_ref().and_then(|viewer| viewer.camera) {
            match camera.target {
                Some([x, y, z]) => println!("camera {} target {x} {y} {z}", view.id),
                None => println!("camera {} target -", view.id),
            }
        }
    }
    for (kind, count) in &kinds {
        println!("summary {kind} {count}");
    }
    Ok(())
}
