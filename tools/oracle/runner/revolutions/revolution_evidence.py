"""Shared saved-graph and revolution membership checks; no native API inputs."""

import math


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identifier(value):
    while isinstance(value, dict):
        value = value.get("m_id", value.get("m_id64"))
    return value


def one_object(record, name):
    found = [o for o in record["objects"] if o["class"] == name]
    require(len(found) == 1, "expected exactly one " + name)
    return found[0]


def linked_object(record, source, field):
    pointer = source["fields"][field]
    edges = [
        e
        for e in record["graph_edges"]
        if e["source_object_index"] == source["object_index"]
        and e["pointer_offset"] == pointer["offset"]
    ]
    require(len(edges) == 1, "missing or ambiguous graph edge: " + field)
    edge = edges[0]
    require(
        edge["pointer_token"] == pointer["pointer_token"]
        and edge["target_class_tag"] == pointer["class_tag"],
        "graph pointer mismatch",
    )
    targets = [
        o for o in record["objects"] if o["object_index"] == edge["target_object_index"]
    ]
    require(
        len(targets) == 1 and targets[0]["class_tag"] == pointer["class_tag"],
        "missing or mismatched graph target",
    )
    return targets[0]


def driven_curve(record):
    driver = linked_object(record, one_object(record, "CurveElem"), "m_pCurveDriver")
    require(driver["class"] == "CurveElemDriver", "unsupported curve driver")
    return linked_object(record, driver, "m_pCrv")


def sketch_evidence(doc):
    require(
        doc.get("schema") == "family-revolution-evidence-v1", "unsupported probe schema"
    )
    require(doc["status"] == "observed", "incomplete decode")
    records = {}
    for record in doc["records"]:
        require(
            record.get("decode_status") == "decoded"
            and record.get("graph_consumed_bytes") == record["body_bytes"],
            "incomplete record decode",
        )
        key = (record["class"], record["element_id"])
        require(
            key not in records, "repeated saved records require explicit resolution"
        )
        records[key] = record
    forms = [r for r in records.values() if r["class"] == "RevolutionElem"]
    require(len(forms) == 1, "expected exactly one revolution")
    form = forms[0]
    fields = one_object(form, "RevolutionElem")["fields"]
    require(fields["m_cutting"] is False, "void revolution unsupported")
    sketch_id = identifier(fields["m_sketchId"])
    sketch = records["VarSketch", sketch_id]
    sketch_fields = one_object(sketch, "VarSketch")["fields"]
    require(
        sketch_fields["m_flipXY"] is False and sketch_fields["m_flipZ"] is False,
        "flipped sketch unsupported",
    )
    plane_id = identifier(sketch_fields["m_sketchPlaneId"])
    plane = records["SketchPlane", plane_id]
    require(
        one_object(plane, "SketchPlane")["fields"]["m_flipZ"] is False,
        "flipped plane unsupported",
    )
    params = form["angle_parameter_values"]
    require(len(params) == 2, "ambiguous angle parameters")
    angles = {p["parameter_id"]: p["value"] for p in params}
    require(set(angles) == {-1001802, -1001803}, "missing or duplicate angle parameter")
    saved_angles = one_object(form, "ParamValueSetDouble")["fields"]["m_paramSet"]
    require(
        len(saved_angles) == 2
        and {identifier(p["m_paramId"]): p["m_value"] for p in saved_angles} == angles,
        "angle summary disagrees with decoded fields",
    )
    start, end = angles[-1001802], angles[-1001803]
    require(
        math.isfinite(start)
        and math.isfinite(end)
        and 0 <= start < end <= 2 * math.pi + 1e-12,
        "unsupported angular range",
    )
    curves, used = [], [form, sketch, plane]
    for r in records.values():
        if r["class"] != "CurveElem":
            continue
        membership = [o for o in r["objects"] if o["class"] == "SketchMembership"]
        if not any(
            identifier(o["fields"]["m_groupId"]) == sketch_id for o in membership
        ):
            continue
        require(len(membership) == 1, "ambiguous sketch membership")
        require(
            identifier(one_object(r, "CurveElem")["fields"]["m_sketchPlaneId"])
            == plane_id,
            "curve plane mismatch",
        )
        require(
            one_object(r, "CurveElemDriver")["fields"]["m_flip"] is False,
            "flipped curve unsupported",
        )
        data = one_object(r, "RevolutionCurveData")["fields"]
        require(type(data["m_axis"]) is bool, "invalid axis marker")
        require(data["m_bOldAxis"] is False, "old-axis flag unsupported")
        curves.append((data["m_axis"], r))
        used.append(r)
    return dict(
        curves=curves,
        used=used,
        start=start,
        end=end,
        form_id=form["element_id"],
        sketch_id=sketch_id,
    )
