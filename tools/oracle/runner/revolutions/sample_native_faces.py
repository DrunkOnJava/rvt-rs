"""Export public-API face samples and projections as a separate native oracle.

Callable inside an authorised Revit host; never launches Revit. Input queries
are points in metres from the offline reconstruction, not fixture recipes.
The resulting native measurements must never feed the byte decoder.
"""
import json
import os

from create_revit_fixtures import sha256


def sample_faces(uiapp, family_path, query_path, output_path):
    from Autodesk.Revit import DB
    if os.path.exists(output_path):
        raise ValueError("Native oracle output must be new")
    with open(query_path) as stream:
        queries = json.load(stream)
    source_hash = sha256(family_path)
    if queries["source_sha256"] != source_hash:
        raise ValueError("Query source hash does not match family")
    app = uiapp.Application
    before = sorted((d.GetHashCode(), d.IsModified) for d in app.Documents)
    doc = app.OpenDocumentFile(family_path)
    try:
        forms = list(DB.FilteredElementCollector(doc).OfClass(DB.Revolution))
        if len(forms) != 1:
            raise ValueError("Expected one native revolution")
        options = DB.Options()
        options.DetailLevel = DB.ViewDetailLevel.Fine
        try:
            solids = [g for g in forms[0].get_Geometry(options)
                      if isinstance(g, DB.Solid) and g.Faces.Size > 0]
            faces = [f for solid in solids for f in solid.Faces]
            samples, counts = [], []
            for face in faces:
                bounds = face.GetBoundingBox()
                count = 0
                for i in range(21):
                    for j in range(21):
                        uv = DB.UV(bounds.Min.U+(bounds.Max.U-bounds.Min.U)*i/20.0,
                                   bounds.Min.V+(bounds.Max.V-bounds.Min.V)*j/20.0)
                        if face.IsInside(uv):
                            p = face.Evaluate(uv)
                            samples.append([p.X*.3048, p.Y*.3048, p.Z*.3048])
                            count += 1
                counts.append(count)
            projections = []
            for values in queries["points_m"]:
                point = DB.XYZ(*[v/.3048 for v in values])
                distances = []
                for face in faces:
                    hit = face.Project(point)
                    if hit is not None:
                        distances.append(hit.Distance*.3048)
                projections.append(min(distances) if distances else None)
            result = dict(schema="native-revolution-face-oracle-v1",
                          source_sha256=source_hash, query_sha256=sha256(query_path),
                          revit_version=app.VersionNumber, revit_build=app.VersionBuild,
                          native_volume_m3=sum(s.Volume for s in solids)*.3048**3,
                          points_m=samples, face_sample_counts=counts,
                          projection_distances_m=projections,
                          role="public API oracle only; never format interpretation")
        finally:
            options.Dispose()
    finally:
        doc.Close(False)
    if before != sorted((d.GetHashCode(), d.IsModified) for d in app.Documents):
        raise RuntimeError("Existing document state changed")
    if sha256(family_path) != source_hash:
        raise RuntimeError("Source family changed")
    with open(output_path, "w") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
    return result
