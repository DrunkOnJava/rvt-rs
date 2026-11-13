"""What a Revit 2026 or 2027 save's door and window cuts, ports and properties
are, held to the same project's 2025 save (RE-346).

Imported by `bodies_across_saves.py`, which pairs each 2026 and 2027 save with
the same Autodesk sample saved by earlier releases and calls `check` with the
2025 save (a pipe is held there by the same script's PIPES group). No Revit
export of a 2026 or 2027 file exists to score these against, so each is held
to the 2025 save of the same project, whose export is measured against
Revit's. Both saves are exported by the build under test, so a difference is
the reader's, or the project's own edit between saves; an error the 2025 save
shares with this one cannot be caught.

  cut openings  an opening that carries `Qto_OpeningElementBaseQuantities` is
                the extrusion of a door's or window's cut loop (RE-215, RE-244).
                Each is paired by `Tag` (its filler's) with the 2025 save's
                and fails on any difference `bodies_across_saves.differences`
                finds, and on its Depth. Its host is held to the 2025 save's
                with its openings and without them: a host that is the 2025
                save's without them and not with them is a wall the cut draws
                otherwise, and the volume the openings take out of a host must
                be the 2025 save's wherever the host itself differs. A cut the
                2025 save has and this save lacks fails.
  ports         an `IfcDistributionPort` is paired by its host's `Tag`, its
                `Name` and its `FlowDirection`; origin to 3 mm, axis to 1e-3.
                Fails on one that differs, one only this save places, and one
                only the 2025 save places.
  properties    the properties and quantities the widened release lists add
                (ADDED), element for element by `Tag` (rooms by `GlobalId`),
                floats to four places. Fails on one that differs and on one
                this save writes and the 2025 save does not. A value only
                the 2025 save writes is counted, not a failure: the PR does
                not add it. The other names are tallied and printed, not
                scored.
  doors and     each door's and window's OverallWidth, OverallHeight and the
  windows       type object it is typed by (its GlobalId, class and Name).
  census        every body of the save with the 2025 save's, as
                `save_pair_bodies.py` does it (world box to 3 mm, volume to
                0.1%). A body that differs fails. ROOFS_NOT_THE_2025_BODY, the
                roofs a 2026 or 2027 save may draw otherwise, is empty since
                RE-458 drew roof 243274 from its sketch's section on those saves
                as the 2025 save does, and no entry joins it. The PINNED_WALLS,
                16 walls of rac_basic pinned until #1471 healed them, are empty,
                and no entry joins them.

Every check prints how many it compared, and a check whose least number is
not reached fails (MINIMUMS), so one that compared nothing is never a pass.
"""

import collections
import re

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import numpy as np

import bodies_across_saves as saves
import save_pair_bodies as census

REFERENCE_YEAR = 2025
PORT_ORIGIN_TOLERANCE = 0.003
PORT_AXIS_TOLERANCE = 1e-3
OPENING_QTO = "Qto_OpeningElementBaseQuantities"
CENSUS_SHOW = 400

CUT_OPENINGS = "cut openings"
CUT_HOSTS = "hosts of cut openings"
PORTS = "ports"
FILLERS = "door and window sizes"
PROPERTIES = "properties"

# (class, property set, name), as regular expressions matched whole, of the
# properties and quantities the widened release lists write (RE-346). The set is
# "" for an attribute of the element itself.
ADDED = [
    (r"Ifc.*", r"Pset_.*Common", r"IsExternal"),
    (r"IfcRailing", r"Pset_RailingCommon", r"Height"),
    (r"IfcRailing", r"Qto_RailingBaseQuantities", r"Length"),
    (r"IfcSpace", r"Qto_SpaceBaseQuantities", r"Height|NetVolume"),
    (r"IfcPipeSegment", r"Qto_PipeSegmentBaseQuantities", r".*"),
    (r"IfcPipeSegment", r"Pset_PipeSegmentOccurrence", r"InvertElevation"),
    (r"IfcDuctSegment", r"Qto_DuctSegmentBaseQuantities", r"Length"),
    (r"IfcDoor", r"Qto_DoorBaseQuantities", r".*"),
    (r"IfcWindow", r"Qto_WindowBaseQuantities", r".*"),
    (r"IfcStair.*", r"", r"Name"),
    (r"IfcMember", r"", r"Name"),
    # The type and family a stair and its flights are read with on 2026 and 2027 as on
    # 2025, and what the type gives them. Not the `Family:Type` they would be written as
    # ObjectType: that is left out of every class from 2026 (#1472).
    (r"IfcStair(Flight)?", r"Pset_Stair(Flight)?Common|Pset_EnvironmentalImpactIndicators|Identity Data|Other", r".*"),
    (r"IfcStair(Flight)?", r"RvtElementRecordGeometry", r"FamilyName|FamilyNameSource|TypeName"),
]
ADDED = [tuple(re.compile(part) for part in rule) for rule in ADDED]

# No body may differ from the 2025 save's (ROOFS_NOT_THE_2025_BODY is empty). The 16
# walls of rac_basic pinned here on 2026-11-05 (Griffin; their source pair,
# volumes and world boxes, at most PINNED_MOST) all became the 2025 save's with
# #1471 (RE-351), and left the list, which no entry ever joins:
# tools/ci/check_same_project_projects.py holds PINNED_MOST at 0. A pinned wall
# whose source, volume or box moves by more than PIN_TOLERANCE fails, and so
# does one that heals.
PINNED_WALLS = {}
PINNED_MOST = 0
PIN_TOLERANCE = 1e-6
RECORD_BOX = "partition_element_record_bbox"
SAVED_MESH = "saved_graphics_mesh"
# The roofs a 2026 or 2027 save may draw otherwise than the 2025 save does, each as
# pinned and exactly these, in the saves of those two projects and none other. It is
# empty. 243274, rac_basic's roof by extrusion (the 2025 save's 35.4236 m3), was a saved
# mesh of 34.9741 m3 in the same box on 2026 and 2027, the default of every class since
# #1191 (Griffin, 2026-11-09), until RE-458 read the sketch's B-spline on those releases
# too and drew it from its section. 165755, 167036 and 167158 left the list with #1612
# and 201262 with #1706: its mesh is the 2025 save's body to the census's 0.1% and 3 mm
# (5.33205264 m3 against 5.33213143). A mesh would count only where its box contains the
# 2025 body's, to the census's 3 mm. The list only shrinks:
# tools/ci/check_same_project_projects.py holds it to what it held, and none is added.
ROOFS_NOT_THE_2025_BODY = {}

# A door's or window's Area, which a 2027 save leaves out because Revit 2027's own
# export gives it a value that is not Width times Height (RE-346): it must be
# absent there, and is not counted among the values compared.
AREA = (re.compile(r"Ifc(Door|Window)"), re.compile(r"Qto_(Door|Window)BaseQuantities"), re.compile(r"Area"))
AREA_LEFT_OUT = 2027  # the first release that leaves it out, as DOOR_WINDOW_AREA_REVIT_VERSIONS ends at 2026

# The least number each check compares on a sample of the project, whatever the
# release of the save unless a number is given by release: `PORTS` placed,
# `CUT_OPENINGS` cut, `PROPERTIES` values equal (not counting a door's or window's
# Area; a stair's and a flight's type-derived values are among them) and "areas" equal
# where the release writes them. From the 2026 and 2027 saves of Measure run
# 42119142486 (head 95fe042f), the numbers this script printed there: every number is
# the same on both releases. rac_basic's 2027 properties were 4 fewer (414) until
# #1489 (RE-373) gave its doors 431064 and 485679 their Width and Height (Measure run
# 42558908204, 418 on both saves).
MINIMUMS = {
    "racadvancedsampleproject": {PORTS: 30, CUT_OPENINGS: 108, PROPERTIES: 4733, "areas": 107, FILLERS: 124},
    "racbasicsampleproject": {PORTS: 60, CUT_OPENINGS: 25, PROPERTIES: 418, "areas": 13, FILLERS: 33},
    "rmeadvancedsampleproject": {PORTS: 6959, CUT_OPENINGS: 107, PROPERTIES: 5023, "areas": 107, FILLERS: 124},
    "rmebasicsampleproject": {PORTS: 7046, CUT_OPENINGS: 107, PROPERTIES: 5028, "areas": 107, FILLERS: 124},
    "rstbasicsampleproject": {PORTS: 0, CUT_OPENINGS: 0, PROPERTIES: 451, "areas": 0, FILLERS: 0},
}


def is_added(cls, pset, name):
    return any(c.fullmatch(cls) and p.fullmatch(pset) and n.fullmatch(name) for c, p, n in ADDED)


def world(model_scale, element):
    """(origin in metres, Z axis) of the element's placement in the world."""
    matrix = np.array(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement), dtype=float)
    return matrix[:3, 3] * model_scale, matrix[:3, 2]


def collect_ports(model, scale):
    """(host Tag, Name, FlowDirection) -> [(origin, axis)] of the placed ports,
    and the number of ports with no placement or no host Tag."""
    placed = collections.defaultdict(list)
    unplaced = 0
    for rel in model.by_type("IfcRelNests"):
        tag = getattr(rel.RelatingObject, "Tag", None)
        for port in rel.RelatedObjects:
            if not port.is_a("IfcDistributionPort"):
                continue
            if not tag or port.ObjectPlacement is None:
                unplaced += 1
                continue
            placed[(tag, port.Name, port.FlowDirection)].append(world(scale, port))
    return placed, unplaced


def check_ports(candidate, reference, scales, fail, year, project):
    here, here_unplaced = collect_ports(candidate, scales[0])
    there, there_unplaced = collect_ports(reference, scales[1])
    same = differ = only_here = only_there = 0
    for key, ports in here.items():
        others = there.get(key, [])
        order = lambda port: tuple(np.round(port[0], 6))
        ports, others = sorted(ports, key=order), sorted(others, key=order)
        for (origin, axis), (origin2, axis2) in zip(ports, others):
            if np.abs(origin - origin2).max() <= PORT_ORIGIN_TOLERANCE and np.abs(axis - axis2).max() <= PORT_AXIS_TOLERANCE:
                same += 1
            else:
                differ += 1
                if differ <= 5:
                    fail(f"{PORTS}: {key[0]} {key[1]} differs: {np.abs(origin - origin2).max():.4f} m apart, axes differ by {np.abs(axis - axis2).max():.2g}")
        only_here += max(len(ports) - len(others), 0)
        only_there += max(len(others) - len(ports), 0)
    only_there += sum(len(ports) for key, ports in there.items() if key not in here)
    if differ:
        fail(f"{PORTS}: {differ} differ from the {year} save's")
    if only_here:
        fail(f"{PORTS}: {only_here} are placed here and not in the {year} save")
    if only_there:
        fail(f"{PORTS}: {only_there} are placed in the {year} save and not here")
    least = MINIMUMS.get(project, {}).get(PORTS, 0)
    if same < least:
        fail(f"{PORTS}: {same} compared equal, fewer than the {least} the sample holds")
    print(
        f"{PORTS}: compared {same + differ} against the {year} save (same {same}, differ {differ}, only here {only_here}, only there {only_there}; "
        f"not placed here {here_unplaced}, there {there_unplaced})"
    )
    return same + differ


def openings(model):
    """Tag -> [IfcOpeningElement] sorted by Name, each with whether it is a cut
    loop's extrusion (carries the opening quantities). An opening's GlobalId is
    derived from the export it is in, so it cannot order two saves alike."""
    out = collections.defaultdict(list)
    for element in model.by_type("IfcOpeningElement"):
        tag = getattr(element, "Tag", None)
        if tag and element.Representation:
            cut = OPENING_QTO in ifcopenshell.util.element.get_psets(element, qtos_only=True)
            out[tag].append((element, cut))
    for tag in out:
        out[tag].sort(key=lambda item: (item[0].Name or "", item[1]))
    return out


def opening_quantities(opening):
    """The opening's own quantities (its Depth), floats to six places."""
    quantities = ifcopenshell.util.element.get_psets(opening, qtos_only=True).get(OPENING_QTO, {})
    return {name: round(value, 6) if isinstance(value, float) else value for name, value in quantities.items() if name != "id"}


def host_of(opening):
    for rel in getattr(opening, "VoidsElements", None) or []:
        return rel.RelatingBuildingElement
    return None


def cut_volume(settings, settings_whole, host):
    """The volume the host's openings take out of it, in the file's cubic units, or why it did not mesh."""
    whole, with_cuts = saves.mesh(settings_whole, host), saves.mesh(settings, host)
    if isinstance(whole, str) or isinstance(with_cuts, str):
        return whole if isinstance(whole, str) else with_cuts
    return whole[0] - with_cuts[0]


def check_openings(candidate, reference, scales, settings, settings_whole, fail, year, project):
    here, there = openings(candidate), openings(reference)
    compared = beside = hosts_equal = hosts_cut_only = hosts_apart = hosts_cut_apart = 0
    host_tags = {}
    for tag in sorted(here, key=lambda t: (0, int(t)) if t.isdigit() else (1, t)):
        cut = [item for item in here[tag] if item[1]]
        if not cut:
            continue
        counterparts = there.get(tag, [])
        if len(counterparts) != len(here[tag]) or [c for _, c in counterparts] != [c for _, c in here[tag]]:
            fail(f"{CUT_OPENINGS}: {tag} is {len(here[tag])} opening(s) here, {len(counterparts)} in the {year} save, cut so in {sum(1 for _, c in counterparts if c)} of them")
            continue
        # Every opening a cut one shares its Tag with (a hole a face-hosted door cuts
        # in its host) is held too: its host moves with the door's.
        for (a, a_cut), (b, _) in zip(here[tag], counterparts):
            # An opening's GlobalId is derived from the export it is in, not from the
            # file: it differs between two saves of the same door.
            found = [d for d in saves.differences(settings, scales, (a, None, None), (b, None, None)) if d[0] != "GlobalId"]
            for field, mine, theirs in found:
                detail = f"{mine} here, {theirs} in the {year} save" if theirs != "" else mine
                fail(f"{CUT_OPENINGS}: {tag} differs in {field}: {detail}")
            if opening_quantities(a) != opening_quantities(b):
                fail(f"{CUT_OPENINGS}: {tag} quantities differ: {opening_quantities(a)} here, {opening_quantities(b)} in the {year} save")
            host, host2 = host_of(a), host_of(b)
            if getattr(host, "Tag", None) != getattr(host2, "Tag", None):
                fail(f"{CUT_OPENINGS}: {tag} is in host {getattr(host, 'Tag', None)} here and {getattr(host2, 'Tag', None)} in the {year} save")
            if not a_cut:
                beside += 1
                continue
            compared += 1
            if host is not None and host2 is not None and getattr(host, "Tag", None):
                host_tags[host.Tag] = (host, host2)
    for tag, (host, host2) in sorted(host_tags.items(), key=lambda item: (0, int(item[0])) if item[0].isdigit() else (1, item[0])):
        if getattr(host2, "Tag", None) != tag:
            fail(f"{CUT_HOSTS}: {tag} is {getattr(host2, 'Tag', None)} in the {year} save")
            continue
        whole = saves.differences(settings_whole, scales, (host, None, None), (host2, None, None))
        with_cuts = saves.differences(settings, scales, (host, None, None), (host2, None, None))
        if not with_cuts:
            hosts_equal += 1
            continue
        if whole:
            hosts_apart += 1
        else:
            hosts_cut_only += 1
            for field, mine, theirs in with_cuts:
                detail = f"{mine} here, {theirs} in the {year} save" if theirs != "" else mine
                fail(f"{CUT_HOSTS}: {tag} {host.is_a()} is the {year} save's without its openings and differs in {field} with them: {detail}")
        # What the openings take out of the host, wherever the host itself differs.
        cuts = [cut_volume(settings, settings_whole, item) for item in (host, host2)]
        if any(isinstance(volume, str) for volume in cuts):
            fail(f"{CUT_HOSTS}: {tag} {host.is_a()} did not mesh, so what its openings cut is not compared")
        elif abs(cuts[0] - cuts[1]) > saves.VOLUME_TOLERANCE * max(cuts[1], 1e-9):
            hosts_cut_apart += 1
            fail(f"{CUT_HOSTS}: {tag} {host.is_a()}: its openings cut {cuts[0]:.9g} here and {cuts[1]:.9g} in the {year} save")
    others = sum(1 for tag, items in there.items() if any(c for _, c in items) and not any(c for _, c in here.get(tag, [])))
    if others:
        fail(f"{CUT_OPENINGS}: {others} door or window cuts are cut in the {year} save and not here")
    least = MINIMUMS.get(project, {}).get(CUT_OPENINGS, 0)
    if compared < least:
        fail(f"{CUT_OPENINGS}: {compared} compared, fewer than the {least} the sample holds")
    print(
        f"{CUT_OPENINGS}: compared {compared} against the {year} save, and {beside} openings beside them; "
        f"their hosts {len(host_tags)}: equal with the openings {hosts_equal}, differ by the openings alone {hosts_cut_only}, "
        f"differ without them too {hosts_apart}, of which the openings cut another volume than the {year} save's {hosts_cut_apart}; "
        f"cut in the {year} save and not here: {others}"
    )
    return compared


def collect_fillers(model, scale):
    """Tag -> [(GlobalId, class, OverallWidth and OverallHeight in metres, the type's GlobalId, class and Name)]
    for each door and window."""
    out = collections.defaultdict(list)
    for element in model.by_type("IfcDoor") + model.by_type("IfcWindow"):
        tag = getattr(element, "Tag", None)
        if not tag:
            continue
        kind = ifcopenshell.util.element.get_type(element)
        size = tuple(None if value is None else round(value * scale, 6) for value in (element.OverallWidth, element.OverallHeight))
        out[tag].append((element.GlobalId or "", element.is_a(), size, (kind.GlobalId, kind.is_a(), kind.Name) if kind else None))
    for tag in out:
        out[tag].sort(key=lambda item: item[0])
    return out


def check_fillers(candidate, reference, scales, fail, year, project):
    """Each door's and window's OverallWidth and OverallHeight, and the type object it is typed by
    (its GlobalId, class and Name), which the widened cut loop list moves."""
    here, there = collect_fillers(candidate, scales[0]), collect_fillers(reference, scales[1])
    same = differ = 0
    for tag, items in here.items():
        others = there.get(tag, [])
        if len(items) != len(others):
            differ += len(items)
            fail(f"{FILLERS}: {tag} is {len(items)} door or window here and {len(others)} in the {year} save")
            continue
        for mine, theirs in zip(items, others):
            if mine[1:] == theirs[1:]:
                same += 1
            else:
                differ += 1
                if differ <= 5:
                    fail(f"{FILLERS}: {tag} {mine[1]} differs: size {mine[2]} and type {mine[3]} here, size {theirs[2]} and type {theirs[3]} in the {year} save")
    only_there = sum(len(items) for tag, items in there.items() if tag not in here)
    if differ:
        fail(f"{FILLERS}: {differ} differ from the {year} save's")
    least = MINIMUMS.get(project, {}).get(FILLERS, 0)
    if same < least:
        fail(f"{FILLERS}: {same} compared equal, fewer than the {least} the sample holds")
    print(f"{FILLERS}: compared {same + differ} against the {year} save (same {same}, differ {differ}; only in the {year} save {only_there})")
    return same + differ


def collect_properties(model):
    """key -> (class, {(set, name): value}) for every product with a Tag (an
    IfcSpace by GlobalId), and the keys more than one product carries."""
    out, repeated = {}, set()
    for element in model.by_type("IfcProduct"):
        if element.is_a("IfcOpeningElement"):
            continue
        key = element.GlobalId if element.is_a("IfcSpace") else getattr(element, "Tag", None)
        if not key:
            continue
        if key in out or key in repeated:
            out.pop(key, None)
            repeated.add(key)
            continue
        values = {("", "Name"): element.Name}
        for pset, props in ifcopenshell.util.element.get_psets(element).items():
            for name, value in props.items():
                if name == "id":
                    continue
                values[(pset, name)] = round(value, 4) if isinstance(value, float) else value
        out[key] = (element.is_a(), values)
    return out, repeated


def is_area(cls, pset, name):
    return all(part.fullmatch(text) for part, text in zip(AREA, (cls, pset, name)))


def check_properties(candidate, reference, fail, year, project, own_year):
    here, repeated_here = collect_properties(candidate)
    there, repeated_there = collect_properties(reference)
    tally = collections.defaultdict(lambda: [0, 0, 0, 0])
    first = {}
    paired = 0
    for key, (cls, values) in here.items():
        if key not in there:
            continue
        paired += 1
        theirs = there[key][1]
        for name in set(values) | set(theirs):
            row = tally[(cls, *name)]
            if name in values and name in theirs:
                if values[name] == theirs[name]:
                    row[0] += 1
                else:
                    row[1] += 1
                    first.setdefault((cls, *name), f"{cls} {key}: {values[name]!r} here, {theirs[name]!r} in the {year} save")
            elif name in values:
                row[2] += 1
                first.setdefault((cls, *name), f"{cls} {key}: {values[name]!r} here, none in the {year} save")
            else:
                row[3] += 1
    equal = differ = only_here = only_there = areas = 0
    for (cls, pset, name), (same, diff, here_only, there_only) in sorted(tally.items()):
        if not is_added(cls, pset, name):
            continue
        if is_area(cls, pset, name):
            if own_year >= AREA_LEFT_OUT and same + diff + here_only:
                fail(f"{PROPERTIES}: {cls} {pset}.{name} is written on a {own_year} save, {same + diff + here_only} times")
            areas, same = areas + same, 0
        equal, differ, only_here, only_there = equal + same, differ + diff, only_here + here_only, only_there + there_only
        if diff or here_only:
            fail(f"{PROPERTIES}: {cls} {pset}.{name}: differ {diff}, only here {here_only} ({first[(cls, pset, name)]})")
    least = MINIMUMS.get(project, {}).get(PROPERTIES, 0)
    if isinstance(least, dict):
        least = least[own_year]
    if equal < least:
        fail(f"{PROPERTIES}: {equal} values equal, fewer than the {least} the sample holds")
    least = MINIMUMS.get(project, {}).get("areas", 0)
    if own_year < AREA_LEFT_OUT and areas < least:
        fail(f"{PROPERTIES}: {areas} doors' and windows' Areas equal, fewer than the {least} the sample holds")
    print(
        f"{PROPERTIES}: compared {equal + differ + only_here} values (not counting Areas) the release lists add on {paired} paired elements against the {year} save "
        f"(equal {equal}, differ {differ}, only here {only_here}, only there {only_there}; "
        f"doors' and windows' Areas, counted apart, equal: {areas}; "
        f"Tags shared by more than one product, not paired: {len(repeated_here)} here, {len(repeated_there)} there)"
    )
    for (cls, pset, name), (same, diff, here_only, there_only) in sorted(tally.items()):
        added = is_added(cls, pset, name)
        if added or diff or here_only or there_only:
            print(
                f"properties {'added by the lists' if added else 'not added by the lists, not scored'}: {cls} {pset}.{name}: "
                f"equal {same}, differ {diff}, only here {here_only}, only there {there_only}"
            )
    return equal + differ + only_here


def pin_off(body, reference, pin):
    """How far a pinned wall's two volumes and two world boxes are from its pin's,
    the largest, in m3 or m."""
    volume, other_volume, box, other_box = pin[2:]
    return max(
        abs(body[2] - volume),
        abs(reference[2] - other_volume),
        float(np.abs(np.concatenate([body[3], body[4]]) - box).max()),
        float(np.abs(np.concatenate([reference[3], reference[4]]) - other_box).max()),
    )


def roof_otherwise(body, reference, kind):
    """Whether a roof differs from the reference's as its pin says: its record box
    where the reference draws something else, or its saved mesh, whose box contains
    the reference body's (to the census's 3 mm)."""
    if body[1] != kind or reference[1] == kind:
        return False
    if kind == RECORD_BOX:
        return True
    slack = census.BOX_TOLERANCE_M
    return bool((body[3] <= reference[3] + slack).all() and (body[4] >= reference[4] - slack).all())


def check_census(candidate_path, reference_path, year, project, fail, own_year):
    """Every body of the candidate against the reference's. A body that differs
    fails, unless it is one of the ROOFS_NOT_THE_2025_BODY (a roof the candidate
    draws as its record box or its saved mesh, as pinned, where the reference draws
    another body, counted, exactly those), or one of the PINNED_WALLS, which must
    differ exactly as pinned: one whose source, volume or box moves, or that heals,
    fails."""
    pinned = PINNED_WALLS.get(project, {}).get(own_year, {})
    if len(pinned) > PINNED_MOST:
        fail(f"census: {len(pinned)} pinned walls, more than the {PINNED_MOST} the list may hold")
    here, here_lost, _ = census.bodies(candidate_path)
    there, there_lost, _ = census.bodies(reference_path)
    same = differ = absent = pinned_held = 0
    expected_roofs = ROOFS_NOT_THE_2025_BODY.get(project, {}) if own_year > REFERENCE_YEAR else {}
    roofs_otherwise = set()
    by_kind = collections.Counter()
    lines = []
    for tag in sorted(here, key=census.tag_order):
        body = here[tag]
        if tag not in there:
            absent += 1
            by_kind[(body[0], body[1], "not in the reference")] += 1
            fail(f"census: {body[0]} {tag} ({body[1]}) has a body here and none in the {year} save")
            continue
        agrees, box = census.same(body, there[tag])
        if tag in pinned:
            continue
        if agrees:
            same += 1
            continue
        differ += 1
        by_kind[(body[0], body[1], there[tag][1])] += 1
        lines.append(f"  {body[0]} {tag} {body[1]} here, {there[tag][1]} in the {year} save: box off {box:.4f} m, volume {body[2]:.6f} here, {there[tag][2]:.6f} there")
        if body[0] == "IfcRoof" and tag in expected_roofs and roof_otherwise(body, there[tag], expected_roofs[tag]):
            roofs_otherwise.add(tag)
        else:
            fail(
                f"census: {body[0]} {tag} ({body[1]} here, {there[tag][1]} in the {year} save) differs from the {year} save's "
                "and is neither a pinned wall nor a roof pinned as drawn otherwise"
            )
    if roofs_otherwise != set(expected_roofs):
        pinned_as = ", ".join(f"{tag} {expected_roofs[tag]}" for tag in sorted(expected_roofs, key=census.tag_order))
        fail(
            f"census: roofs the {year} save draws otherwise: {sorted(roofs_otherwise, key=census.tag_order)} here, "
            f"{sorted(expected_roofs, key=census.tag_order)} expected as pinned ({pinned_as}), and exactly those"
        )
    for tag, pin in sorted(pinned.items(), key=lambda item: census.tag_order(item[0])):
        body, reference = here.get(tag), there.get(tag)
        if body is None or reference is None:
            fail(f"census: pinned wall {tag} has no body {'here' if body is None else 'in the ' + str(year) + ' save'}")
        elif census.same(body, reference)[0]:
            fail(f"census: pinned wall {tag} is now the {year} save's: remove its pin")
        elif (body[1], reference[1]) != pin[:2] or pin_off(body, reference, pin) > PIN_TOLERANCE:
            fail(
                f"census: pinned wall {tag} moved: {body[1]} here, {reference[1]} in the {year} save, pinned {pin[0]} and {pin[1]}; "
                f"its volumes and boxes are off the pin by up to {pin_off(body, reference, pin):.6f} (m3 or m)"
            )
        else:
            pinned_held += 1
            differ += 1
    print(
        f"census: {len(here)} bodies paired by Tag with the {year} save's: same {same}, differ {differ} "
        f"(pinned walls as pinned, source, volume and box, {pinned_held} of {len(pinned)}; roofs drawn otherwise as pinned {len(roofs_otherwise)} of {len(expected_roofs)} expected), "
        f"not in the {year} save {absent}; did not mesh here {len(here_lost)}, there {len(there_lost)}"
    )
    for (cls, mine, theirs), count in sorted(by_kind.items(), key=lambda item: (-item[1], item[0])):
        print(f"census by kind: {cls}: {count} ({mine} here, {theirs} in the {year} save)")
    for line in lines[:CENSUS_SHOW]:
        print(f"census differing{line}")
    if len(lines) > CENSUS_SHOW:
        print(f"census differing ... and {len(lines) - CENSUS_SHOW} more")
    return same + differ


def check(project, own_year, candidate_path, candidate, reference_path, reference, scales, settings, fail):
    """Run every check of this module on a candidate and the 2025 save's
    model; returns the number of things compared."""
    settings_whole = ifcopenshell.geom.settings()
    settings_whole.set("use-world-coords", True)
    settings_whole.set("disable-opening-subtractions", True)
    total = check_ports(candidate, reference, scales, fail, REFERENCE_YEAR, project)
    total += check_openings(candidate, reference, scales, settings, settings_whole, fail, REFERENCE_YEAR, project)
    total += check_fillers(candidate, reference, scales, fail, REFERENCE_YEAR, project)
    total += check_properties(candidate, reference, fail, REFERENCE_YEAR, project, own_year)
    total += check_census(candidate_path, reference_path, REFERENCE_YEAR, project, fail, own_year)
    return total
