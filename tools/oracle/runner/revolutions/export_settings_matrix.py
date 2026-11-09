"""Compare explicit IFC formats and detail settings on the same native host.

Uses documented public Revit APIs. No native implementation code is used.
All changes are temporary: the source host is opened and closed without saving.
"""
import json
import os

from create_revit_fixtures import sha256


def export_matrix(uiapp, host_path, output_directory):
    import clr
    clr.AddReference("RevitAPI")
    from Autodesk.Revit import DB

    if os.path.exists(output_directory):
        raise ValueError("Output directory must be new")
    os.makedirs(output_directory)
    original_hash = sha256(host_path)
    app = uiapp.Application
    report = dict(status="running", revit_build=app.VersionBuild,
                  source_host_sha256=original_hash, exports=[])
    def save():
        with open(os.path.join(output_directory, "settings-matrix.json"), "w") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
    save()
    doc = app.OpenDocumentFile(host_path)
    try:
        tx = DB.Transaction(doc, "Temporary fine-detail oracle view")
        tx.Start()
        try:
            types = [t for t in DB.FilteredElementCollector(doc).OfClass(DB.ViewFamilyType)
                     if t.ViewFamily == DB.ViewFamily.ThreeDimensional]
            if not types:
                raise RuntimeError("Host has no 3D view family type")
            view = DB.View3D.CreateIsometric(doc, types[0].Id)
            view.DetailLevel = DB.ViewDetailLevel.Fine
            view.IsSectionBoxActive = False
            if tx.Commit() != DB.TransactionStatus.Committed:
                raise RuntimeError("View creation did not commit")
        finally:
            if tx.GetStatus() == DB.TransactionStatus.Started:
                tx.RollBack()
            tx.Dispose()
        cases = [("ifc4-low", "IFC4", "0.25", False),
                 ("ifc4-high", "IFC4", "1", False),
                 ("ifc4-high-fine-view", "IFC4", "1", True),
                 ("ifc4-dtv-high", "IFC4DTV", "1", False),
                 ("ifc4-dtv-high-fine-view", "IFC4DTV", "1", True),
                 ("ifc4-rv-high", "IFC4RV", "1", False)]
        for name, version, detail, fine_view in cases:
            tx = DB.Transaction(doc, "Compare IFC export configuration")
            tx.Start()
            options = DB.IFCExportOptions()
            entry = dict(name=name, file_version=version,
                         tessellation_level=detail, fine_view=fine_view)
            try:
                options.FileVersion = getattr(DB.IFCVersion, version)
                options.SpaceBoundaryLevel = 0
                options.ExportBaseQuantities = True
                options.AddOption("TessellationLevelOfDetail", detail)
                if fine_view:
                    options.FilterViewId = view.Id
                    options.AddOption("ActiveViewId", str(view.Id.IntegerValue))
                    options.AddOption("UseActiveViewGeometry", "true")
                if not doc.Export(output_directory, name+".ifc", options):
                    raise RuntimeError("Export returned false")
                entry["status"] = "exported"
                entry["sha256"] = sha256(os.path.join(output_directory, name+".ifc"))
            except Exception as error:
                entry["status"] = "failed"
                entry["error"] = str(error)
            finally:
                options.Dispose()
                if tx.GetStatus() == DB.TransactionStatus.Started:
                    tx.RollBack()
                tx.Dispose()
            report["exports"].append(entry)
            save()
        report["status"] = "completed"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = str(error)
        raise
    finally:
        doc.Close(False)
        report["source_unchanged"] = sha256(host_path) == original_hash
        save()
    return report
