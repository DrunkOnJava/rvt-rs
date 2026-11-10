# -*- coding: utf-8 -*-
"""Callable native fixture author. Does not launch Revit or run on import.

Invoke create_fixtures(uiapp, template_path, NEW_output_directory) only after
the user authorises the native session. Works with an existing Revit Python
script host. API creates files; byte interpretation belongs to the Rust probe.
The public NewRevolution API signature is the authoring reference, not an
on-disk format source. Exercised on Revit 2020.2.9 on 2026-11-09.
"""
import hashlib
import json
import math
import os

from rectangular_fixtures import definitions


def export_native_geometry(form, folder, name, DB, level_of_detail=None):
    """Public API oracle only. Never used as on-disk decoder input."""
    options = DB.Options()
    options.DetailLevel = DB.ViewDetailLevel.Fine
    geometry = form.get_Geometry(options)
    vertices, triangles, solids = [], [], []
    for obj in geometry:
        if not isinstance(obj, DB.Solid) or obj.Faces.Size == 0:
            continue
        centroid = obj.ComputeCentroid()
        solids.append(dict(volume_m3=obj.Volume * 0.3048**3,
                           centroid_m=[centroid.X*.3048, centroid.Y*.3048, centroid.Z*.3048],
                           faces=obj.Faces.Size))
        for face in obj.Faces:
            mesh = face.Triangulate() if level_of_detail is None else face.Triangulate(level_of_detail)
            for i in range(mesh.NumTriangles):
                triangle = mesh.get_Triangle(i)
                indices = []
                for j in range(3):
                    p = triangle.get_Vertex(j)
                    indices.append(len(vertices)+1)
                    vertices.append([p.X*304.8, p.Y*304.8, p.Z*304.8])
                triangles.append(indices)
    if not solids:
        raise RuntimeError("Native revolution yielded no solid geometry")
    path = os.path.join(folder, name + "-native.obj")
    with open(path, "w") as stream:
        stream.write("# Revit public API face triangulation; millimetres\n")
        for p in vertices:
            stream.write("v %.17g %.17g %.17g\n" % tuple(p))
        for t in triangles:
            stream.write("f %d %d %d\n" % tuple(t))
    options.Dispose()
    return dict(source="Revit public API; independent oracle, not format evidence",
                solids=solids, mesh_file=os.path.basename(path), mesh_sha256=sha256(path),
                triangles=len(triangles), level_of_detail=level_of_detail)


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def create_fixtures(uiapp, template_path, output_directory, fixture_definitions=None):
    import clr
    clr.AddReference("RevitAPI")
    from Autodesk.Revit import DB

    if not os.path.isfile(template_path) or not template_path.lower().endswith(".rft"):
        raise ValueError("Supply an installed, unhosted Generic Model family template")
    if os.path.exists(output_directory):
        raise ValueError("Output directory must be new; existing files are never overwritten")
    app = uiapp.Application
    original_docs = sorted((d.GetHashCode(), d.IsModified) for d in app.Documents)
    os.makedirs(output_directory)
    receipt = dict(status="running", revit_version=app.VersionNumber,
                   revit_build=app.VersionBuild, authoring="fresh in this release",
                   template_basename=os.path.basename(template_path),
                   template_sha256=sha256(template_path), files=[],
                   geometry_match=None, api_geometry_dump_used=True,
                   api_geometry_role="separate native oracle; never decoder source")
    def write_receipt():
        with open(os.path.join(output_directory, "native-authoring.json"), "w") as stream:
            json.dump(receipt, stream, indent=2, sort_keys=True)
    write_receipt()
    try:
        for f in (definitions() if fixture_definitions is None else fixture_definitions)["fixtures"]:
            doc = app.NewFamilyDocument(template_path)
            if doc is None:
                raise RuntimeError("Revit did not create a family document")
            try:
                transaction = DB.Transaction(doc, "Author synthetic revolution fixture")
                transaction.Start()
                try:
                    def point(values):
                        return DB.XYZ(*[v/304.8 for v in values])
                    curves = DB.CurveArray()
                    if "profile_arcs_mm" in f:
                        for arc in f["profile_arcs_mm"]:
                            curves.Append(DB.Arc.Create(point(arc["start"]),
                                point(arc["end"]), point(arc["midpoint"])))
                    else:
                        points = [point(p) for p in f["profile_points_mm"]]
                        for i in range(4):
                            curves.Append(DB.Line.CreateBound(points[i], points[(i+1)%4]))
                    loops = DB.CurveArrArray()
                    loops.Append(curves)
                    normal = DB.XYZ(*f["sketch_normal"])
                    plane = DB.Plane.CreateByNormalAndOrigin(normal, point(f["axis_points_mm"][0]))
                    sketch_plane = DB.SketchPlane.Create(doc, plane)
                    axis = DB.Line.CreateBound(*[point(p) for p in f["axis_points_mm"]])
                    form = doc.FamilyCreate.NewRevolution(True, loops, sketch_plane, axis,
                        math.radians(f["start_degrees"]), math.radians(f["end_degrees"]))
                    if doc.FamilyManager.CurrentType is None:
                        doc.FamilyManager.NewType("Reference")
                    doc.Regenerate()
                    if transaction.Commit() != DB.TransactionStatus.Committed:
                        raise RuntimeError("Fixture transaction was not committed")
                except Exception:
                    if transaction.GetStatus() == DB.TransactionStatus.Started:
                        transaction.RollBack()
                    raise
                finally:
                    transaction.Dispose()
                path = os.path.join(output_directory, f["id"]+".rfa")
                options = DB.SaveAsOptions()
                options.OverwriteExistingFile = False
                options.Compact = True
                try:
                    doc.SaveAs(path, options)
                finally:
                    options.Dispose()
                # Identity only: no API-derived geometry becomes a decoder rule.
                try:
                    form_id = int(form.Id.Value)
                except AttributeError:
                    form_id = int(form.Id.IntegerValue)
                native = export_native_geometry(form, output_directory, f["id"], DB)
                receipt["files"].append(dict(id=f["id"], file=os.path.basename(path),
                    sha256=sha256(path), bytes=os.path.getsize(path), element_id=form_id,
                    definition=f, native_geometry=native, status="saved_not_geometrically_validated"))
                write_receipt()
            finally:
                doc.Close(False)
        if original_docs != sorted((d.GetHashCode(), d.IsModified) for d in app.Documents):
            raise RuntimeError("Pre-existing document state changed during fixture creation")
        receipt["status"] = "saved_not_geometrically_validated"
    except Exception as error:
        receipt["status"] = "failed"
        receipt["error"] = str(error)
        raise
    finally:
        write_receipt()
    return receipt
