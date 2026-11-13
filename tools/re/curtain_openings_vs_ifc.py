#!/usr/bin/env python3
"""Doors and windows in curtain walls: what an opening's reference list, grid,
element data and record box say about the curtain wall Revit's export puts it
in (#1912, RE-452).

Reads `probes/examples/probe_1912_curtain_openings.rs`'s output and Revit's IFC
export. An opening is a door or window; Revit puts one in a curtain wall as a
part of it (`IfcRelAggregates` whose `RelatingObject` is an `IfcCurtainWall`),
matched by `Tag`, the Revit ElementId. For every opening the probe lists:

- the first table sets rvt-rs's export (`ours`) beside Revit's: right (the same
  wall), only Revit (Revit aggregates it and rvt-rs does not), only rvt-rs,
  different, and neither;
- the second table takes the openings only Revit aggregates and says why none
  of the evidence rvt-rs reads gave the wall: whether the wall is among the
  walls the opening's list names, one rvt-rs counts as a curtain wall, in the
  element data of an id of the list, and around the opening's record box;
- the third table scores candidate rules on every opening of the model, those
  Revit leaves in an ordinary wall included, against Revit's export: right,
  wrong wall, a wall where Revit has none (false positive) and a wall Revit
  has that the rule does not give (missed). `current` is the rule that
  `attach_curtain_walls` applies (RE-452) and must reproduce `ours`.

Usage:

    python3 tools/re/curtain_openings_vs_ifc.py <probe.txt> <revit.ifc> [--list]

`--list` also lists each opening only Revit aggregates and, for each candidate
rule, the openings it gets wrong or gives a wall Revit does not. Needs
IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell

# `ELEMENT_DATA_WALL_MIN_COPIES`: scan_walls_in_element_data counts a wall as
# named by an id's element data from this many places.
MIN_COPIES = 2
LISTED = 40


def ids_of(text):
    return [] if text == "-" else text.split(",")


class Opening:
    def __init__(self, fields):
        self.tag = fields["tag"]
        self.kind = fields["kind"]
        self.exported = fields["exported"] == "y"
        self.ours = None if fields["ours"] == "-" else fields["ours"]
        self.walls = ids_of(fields["walls"])
        self.curtain = ids_of(fields["curtain"])
        self.grid = {}
        for item in ids_of(fields["grid"]):
            other, wall = item.split(">")
            self.grid[other] = wall
        # id -> wall -> places the wall lies at, one per copy of the id's data.
        self.data = collections.defaultdict(dict)
        for item in ids_of(fields["data"]):
            head, copies = item.split("*")
            other, wall = head.split(">")
            self.data[other][wall] = [int(c) for c in copies.split("/")]
        self.box = ids_of(fields["box"])
        # The ids of the list that are no wall: (id, category, copies of data).
        self.ids = []
        for item in ids_of(fields.get("ids", "-")):
            head, places = item.rsplit("/d", 1)
            other, category = head.split(":", 1)
            self.ids.append((other, category, int(places)))

    def signature(self):
        """What the list holds besides walls: each id's category and whether it
        has element data, sorted."""
        return ", ".join(
            sorted(f"{category}{'+data' if places else ''}" for _, category, places in self.ids)
        ) or "no other id"


def state_of(opening, seen, wholes):
    """rvt-rs's wall for the opening against Revit's, or None where Revit's
    export has no such opening."""
    if opening.tag not in seen:
        return None
    theirs = wholes.get(opening.tag)
    if opening.ours is not None and theirs is not None:
        return "right" if opening.ours == theirs else "different"
    if theirs is not None:
        return "only Revit"
    return "only rvt-rs" if opening.ours is not None else "neither"


def read_probe(path):
    curtain, walls, openings = set(), set(), []
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if not parts:
            continue
        fields = dict(p.split("=", 1) for p in parts[1:] if "=" in p)
        if parts[0] == "curtain_walls":
            curtain = set(ids_of(fields.get("ids", "-")))
        elif parts[0] == "walls":
            walls = set(ids_of(fields.get("ids", "-")))
        elif parts[0] == "opening":
            openings.append(Opening(fields))
    return curtain, walls, openings


def revit_openings(model):
    """The tags of Revit's doors and windows, and for those aggregated into a
    curtain wall the wall's tag."""
    seen = set()
    for entity in ("IfcDoor", "IfcWindow"):
        for element in model.by_type(entity):
            if element.Tag:
                seen.add(str(element.Tag))
    wholes = {}
    for rel in model.by_type("IfcRelAggregates"):
        whole = rel.RelatingObject
        if not whole.is_a("IfcCurtainWall") or not whole.Tag:
            continue
        for part in rel.RelatedObjects:
            if (part.is_a("IfcDoor") or part.is_a("IfcWindow")) and part.Tag:
                wholes[str(part.Tag)] = str(whole.Tag)
    return seen, wholes


def data_named(opening, curtain, min_copies):
    """Per id of the opening's list, the curtain walls its element data names, as
    scan_walls_in_element_data reads it: a copy names the walls lying at
    `min_copies` places or more, a copy that names none is skipped, and an id
    whose copies name different sets is left out."""
    named = {}
    for other, walls in opening.data.items():
        walls = {wall: copies for wall, copies in walls.items() if wall in curtain}
        if not walls:
            continue
        sets = []
        for copy in range(max(len(copies) for copies in walls.values())):
            names = frozenset(
                wall for wall, copies in walls.items() if copies[copy] >= min_copies
            )
            if names:
                sets.append(names)
        if sets and all(names == sets[0] for names in sets):
            named[other] = set(sets[0])
    return named


def the_one(walls):
    walls = set(walls)
    return next(iter(walls)) if len(walls) == 1 else None


def grid_wall(opening):
    """(1, _) of attach_curtain_walls: the wall of the one grid the list names;
    `False` when the list names several walls' grids."""
    walls = set(opening.grid.values())
    if len(walls) == 1:
        return next(iter(walls))
    return False if walls else None


def with_grid(rest):
    def rule(opening, curtain):
        grid = grid_wall(opening)
        if grid is not None:
            return grid or None
        return rest(opening, curtain)

    return rule


def named_curtain(opening, curtain):
    return [wall for wall in opening.walls if wall in curtain]


def confirmed_by_data(opening, curtain, min_copies):
    """RE-452: the one wall of those the list names that the element data of an
    id of the list names too."""
    named = set(named_curtain(opening, curtain))
    if len(named) != 1:
        return None
    walls = set()
    for names in data_named(opening, curtain, min_copies).values():
        walls |= names & named
    return the_one(walls)


def rule_named1(opening, curtain):
    return the_one(named_curtain(opening, curtain))


def rule_named1_box(opening, curtain):
    wall = rule_named1(opening, curtain)
    return wall if wall in opening.box else None


def rule_named1_data_or_box(opening, curtain):
    wall = rule_named1(opening, curtain)
    if wall is None:
        return None
    return wall if wall in opening.box or confirmed_by_data(opening, curtain, MIN_COPIES) else None


def rule_data_only(opening, curtain):
    walls = set()
    for names in data_named(opening, curtain, MIN_COPIES).values():
        walls |= names
    return the_one(walls)


def rule_box_only(opening, curtain):
    return the_one(wall for wall in opening.box if wall in curtain)


def rule_only_curtain(opening, curtain):
    """The list names one wall in all, and it is a curtain wall."""
    if len(opening.walls) == 1 and opening.walls[0] in curtain:
        return opening.walls[0]
    return None


RULES = (
    ("current", with_grid(lambda o, c: confirmed_by_data(o, c, MIN_COPIES))),
    ("named1", with_grid(rule_named1)),
    ("only curtain", with_grid(rule_only_curtain)),
    ("named1+data1", with_grid(lambda o, c: confirmed_by_data(o, c, 1))),
    ("named1+box", with_grid(rule_named1_box)),
    ("named1+(data|box)", with_grid(rule_named1_data_or_box)),
    ("data only", with_grid(rule_data_only)),
    ("box only", with_grid(rule_box_only)),
)


def why(opening, wall, curtain, walls):
    """The evidence for Revit's `wall` that the opening carries, as words."""
    if not opening.exported:
        return "rvt-rs writes no such opening"
    bits = []
    bits.append("list names it" if wall in opening.walls else "list does not name it")
    if wall in curtain:
        bits.append("curtain wall of rvt-rs")
    elif wall in walls:
        bits.append("wall rvt-rs does not count as a curtain wall")
    else:
        bits.append("no wall record in rvt-rs")
    most = max(
        (
            max(places)
            for named in opening.data.values()
            for named_wall, places in named.items()
            if named_wall == wall
        ),
        default=0,
    )
    if most >= MIN_COPIES:
        bits.append(f"data names it at {MIN_COPIES} places or more")
    elif most:
        bits.append("data names it at one place")
    else:
        bits.append("data does not name it")
    if opening.grid:
        theirs = set(opening.grid.values())
        bits.append("grid names it" if theirs == {wall} else "grid names another wall")
    bits.append("box has it" if wall in opening.box else "box does not have it")
    bits.append(f"{len(named_curtain(opening, curtain))} curtain walls named")
    return "; ".join(bits)


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    curtain, walls, openings = read_probe(args[0])
    revit = ifcopenshell.open(args[1])
    seen, wholes = revit_openings(revit)
    print(
        f"probe: {len(openings)} openings, {len(curtain)} curtain walls, {len(walls)} walls;"
        f" Revit's export: {len(seen)} doors and windows, {len(wholes)} aggregated into a curtain wall"
    )

    table = collections.Counter()
    only_revit = []
    for opening in openings:
        state = state_of(opening, seen, wholes)
        if state is None:
            table[(opening.kind, "not in Revit's export")] += 1
            continue
        if state == "only Revit":
            only_revit.append((opening, wholes[opening.tag]))
        table[(opening.kind, state)] += 1
    print("rvt-rs against Revit (kind, state): count")
    for (kind, state), n in sorted(table.items()):
        print(f"  {kind:<7} {state:<22} {n}")
    missing = len(wholes) - sum(
        1 for opening in openings if opening.tag in wholes
    )
    print(f"aggregated by Revit with no opening record in the probe: {missing}")

    print("openings only Revit aggregates: why no wall was found (kind, evidence): count")
    classes = collections.Counter()
    for opening, theirs in only_revit:
        classes[(opening.kind, why(opening, theirs, curtain, walls))] += 1
    for (kind, text), n in sorted(classes.items(), key=lambda kv: -kv[1]):
        print(f"  {kind:<7} {n:>4}  {text}")

    print("openings whose list names a curtain wall, by state, walls named and what else the list holds (category, +data where it has element data): count")
    holding = collections.Counter()
    for opening in openings:
        state = state_of(opening, seen, wholes)
        if state is not None and opening.curtain:
            holding[(state, len(opening.walls), opening.kind, opening.signature())] += 1
    for (state, walls_named, kind, signature), n in sorted(holding.items(), key=lambda kv: (kv[0][0], -kv[1])):
        print(f"  {state:<11} {walls_named} walls {kind:<7} {n:>4}  {signature}")

    print("candidate rules against Revit on every opening in its export (kind, rule): right / wrong / false positive / missed")
    wrong_rows = collections.defaultdict(list)
    for name, rule in RULES + (("ours", lambda o, c: o.ours),):
        for kind in ("door", "window"):
            tally = collections.Counter()
            for opening in openings:
                if opening.kind != kind or opening.tag not in seen:
                    continue
                predicted = rule(opening, curtain)
                theirs = wholes.get(opening.tag)
                if predicted is None and theirs is None:
                    continue
                if predicted == theirs:
                    tally["right"] += 1
                elif predicted is None:
                    tally["missed"] += 1
                elif theirs is None:
                    tally["false positive"] += 1
                    wrong_rows[name].append((opening, predicted, theirs))
                else:
                    tally["wrong"] += 1
                    wrong_rows[name].append((opening, predicted, theirs))
            print(
                f"  {kind:<7} {name:<18} {tally['right']:>4} / {tally['wrong']:>3} /"
                f" {tally['false positive']:>3} / {tally['missed']:>4}"
            )

    current = dict(RULES)["current"]
    disagree = [
        (opening, current(opening, curtain))
        for opening in openings
        if current(opening, curtain) != opening.ours
    ]
    print(f"`current` reproduces rvt-rs's own wall on {len(openings) - len(disagree)} of {len(openings)} openings")
    for opening, predicted in disagree[:LISTED]:
        print(f"  differs {opening.tag} {opening.kind}: ours={opening.ours} current={predicted}")

    if "--list" in sys.argv:
        for opening, theirs in sorted(only_revit, key=lambda row: (len(row[0].tag), row[0].tag)):
            print(f"only Revit {opening.tag} {opening.kind} wall={theirs}: {why(opening, theirs, curtain, walls)}")
        for name, rows in wrong_rows.items():
            for opening, predicted, theirs in rows[:LISTED]:
                print(f"{name} gives {opening.tag} {opening.kind} wall={predicted}, Revit {theirs or 'none'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
