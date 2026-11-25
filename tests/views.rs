//! #1117: views, sheets and the viewports placed on them are read from the
//! native records, and every placed viewport names a sheet and a view the
//! catalog holds.
//!
//! Runs against `RVT_NORDIC_LCA_DIR`'s as-built office, the model #1117's
//! probe measured 21 placed viewports on 19 sheets on. Skips when the model
//! is absent. What is checked is the catalog's own consistency and the
//! counts the probe published; the box-centre rule itself was measured on
//! one viewport against `Viewport.GetBoxCenter()` and is not re-measured
//! here, so a count is not a comparison with Revit.

use rvt::RevitFile;
use rvt::views::{ViewCatalog, ViewKind};
use std::path::PathBuf;

fn office() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_NORDIC_LCA_DIR").map(PathBuf::from)?;
    let path = dir.join("ARK_NordicLCA_Office_Concrete_As-Built.rvt");
    if !path.exists() {
        eprintln!("skipping: {} is absent", path.display());
        return None;
    }
    Some(path)
}

#[test]
fn office_views_sheets_and_viewports_are_read() {
    let Some(path) = office() else {
        return;
    };
    let mut file = RevitFile::open(&path).expect("open the office");
    let catalog = ViewCatalog::read(&mut file).expect("read views");
    eprintln!(
        "views {} sheets {} viewports {}",
        catalog.views.len(),
        catalog.sheets.len(),
        catalog.viewports.len()
    );

    let placed: Vec<_> = catalog
        .viewports
        .iter()
        .filter(|viewport| viewport.sheet_id.is_some())
        .collect();
    assert_eq!(placed.len(), 21, "the probe's 21 placed viewports (#1117)");
    // The probe counted the sheets those viewports are on, not every sheet
    // record: a sheet with no viewport is still a sheet.
    let sheets_with_viewports = catalog
        .sheets
        .iter()
        .filter(|sheet| catalog.viewports_on(sheet.id).next().is_some())
        .count();
    assert_eq!(
        sheets_with_viewports, 19,
        "the probe's 19 sheets with placed viewports (#1117)"
    );
    assert!(catalog.sheets.len() >= sheets_with_viewports);
    assert!(
        catalog
            .views
            .iter()
            .any(|view| view.kind == ViewKind::ThreeD),
        "the office has 3D views"
    );
    for viewport in placed {
        let sheet = viewport.sheet_id.expect("placed");
        let view = viewport.view_id.expect("a placed viewport names its view");
        assert!(catalog.sheet(sheet).is_some(), "sheet {sheet} is read");
        assert!(catalog.view(view).is_some(), "view {view} is read");
        assert!(
            catalog
                .viewports_on(sheet)
                .any(|candidate| candidate.id == viewport.id),
            "the sheet lists its viewport"
        );
        if let Some(center) = viewport.center {
            assert!(
                center.iter().all(|coordinate| coordinate.is_finite()),
                "viewport {} centre is finite",
                viewport.id
            );
        }
    }
    for sheet in &catalog.sheets {
        assert!(!sheet.number.is_empty(), "sheet {} has a number", sheet.id);
        assert!(!sheet.name.is_empty(), "sheet {} has a name", sheet.id);
    }
    let cameras = catalog
        .views
        .iter()
        .filter(|view| {
            view.viewer
                .as_ref()
                .and_then(|viewer| viewer.camera)
                .is_some()
        })
        .count();
    let targets = catalog
        .views
        .iter()
        .filter(|view| {
            view.viewer
                .as_ref()
                .and_then(|viewer| viewer.camera)
                .is_some_and(|camera| camera.target.is_some())
        })
        .count();
    eprintln!("3D cameras {cameras}, targets on the looking axis {targets}");
    assert!(cameras > 0, "the office has a 3D camera");
}

#[test]
fn the_same_file_gives_the_same_catalog_twice() {
    let Some(path) = office() else {
        return;
    };
    let read = || {
        let mut file = RevitFile::open(&path).expect("open the office");
        ViewCatalog::read(&mut file).expect("read views")
    };
    let first = read();
    let second = read();
    assert_eq!(first.views.len(), second.views.len());
    assert_eq!(first.sheets.len(), second.sheets.len());
    assert_eq!(first.viewports.len(), second.viewports.len());
    for (a, b) in first.viewports.iter().zip(&second.viewports) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.center, b.center);
    }
}
