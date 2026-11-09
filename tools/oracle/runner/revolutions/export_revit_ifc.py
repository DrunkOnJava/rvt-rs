"""Export one new host project per fixture through Revit's public API."""
import json
import os

from create_revit_fixtures import sha256


def export_fixtures(uiapp, family_directory, output_directory):
    import clr
    clr.AddReference("RevitAPI")
    from Autodesk.Revit import DB

    if os.path.exists(output_directory):
        raise ValueError("Output directory must be new")
    os.makedirs(output_directory)
    app = uiapp.Application
    report = dict(status="running", revit_version=app.VersionNumber,
                  revit_build=app.VersionBuild, files=[],
                  settings=dict(schema="IFC4", model_view="Design Transfer View",
                                api_file_version="IFC4DTV", space_boundaries=0,
                                base_quantities=True, tessellation_level="1",
                                instance_location_feet=[0, 0, 0]))
    def save_report():
        with open(os.path.join(output_directory, "ifc-export.json"), "w") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
    save_report()
    try:
        for name in ["REV-A", "REV-B", "REV-BT"]:
            doc = app.NewProjectDocument(DB.UnitSystem.Metric)
            try:
                tx = DB.Transaction(doc, "Place synthetic reference family")
                tx.Start()
                try:
                    family_ref = clr.Reference[DB.Family]()
                    if not doc.LoadFamily(os.path.join(family_directory, name+".rfa"), family_ref):
                        raise RuntimeError("LoadFamily failed: " + name)
                    family = family_ref.Value
                    symbols = list(family.GetFamilySymbolIds())
                    if len(symbols) != 1:
                        raise RuntimeError("Expected exactly one fixture type")
                    symbol = doc.GetElement(symbols[0])
                    if not symbol.IsActive:
                        symbol.Activate()
                    DB.Level.Create(doc, 0)
                    doc.Regenerate()
                    instance = doc.Create.NewFamilyInstance(DB.XYZ.Zero, symbol, DB.Structure.StructuralType.NonStructural)
                    doc.Regenerate()
                    if tx.Commit() != DB.TransactionStatus.Committed:
                        raise RuntimeError("Placement transaction failed")
                finally:
                    if tx.GetStatus() == DB.TransactionStatus.Started:
                        tx.RollBack()
                    tx.Dispose()
                host = os.path.join(output_directory, name+"-host.rvt")
                opts = DB.SaveAsOptions()
                try:
                    opts.Compact = True
                    doc.SaveAs(host, opts)
                finally:
                    opts.Dispose()
                tx = DB.Transaction(doc, "Export native IFC reference")
                tx.Start()
                options = DB.IFCExportOptions()
                try:
                    # Generic IFC4 facets a full ring even at maximum detail in
                    # this release; explicit DTV preserves its advanced BRep.
                    options.FileVersion = DB.IFCVersion.IFC4DTV
                    options.SpaceBoundaryLevel = 0
                    options.ExportBaseQuantities = True
                    options.AddOption("TessellationLevelOfDetail", "1")
                    if not doc.Export(output_directory, name+"-native.ifc", options):
                        raise RuntimeError("IFC export returned false")
                finally:
                    options.Dispose()
                    if tx.GetStatus() == DB.TransactionStatus.Started:
                        tx.RollBack()
                    tx.Dispose()
                exported = os.path.join(output_directory, name+"-native.ifc")
                report["files"].append(dict(id=name, host_file=os.path.basename(host),
                    host_sha256=sha256(host), ifc_file=os.path.basename(exported),
                    ifc_sha256=sha256(exported), instance_id=instance.Id.IntegerValue))
                save_report()
            finally:
                doc.Close(False)
        report["status"] = "exported"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = str(error)
        raise
    finally:
        save_report()
    return report
