#!/usr/bin/env python3
"""Score rules that would leave out the plumbing fixtures Revit's export lacks (RE-363, #1496).

Reads what `probes/examples/probe_re363_plumbing_fixtures.rs` prints for a model, joins each
`OST_PlumbingFixtures` element rvt-rs writes (`stage=written`) with the numeric `Tag`s of Revit's own
export, and says for every candidate rule how many of the fixtures Revit's export holds the rule would
drop (a false negative: an element Revit writes would be lost), how many of the fixtures Revit's export
lacks it would drop, and how many of those it leaves written. A rule lands only with no false negative
on every model that has an export, and `-f holdout=true` (the standing bar: never drop an element Revit
writes). A rule drops an element only on positive evidence: an unread or undecided reading keeps it.

Usage:

    python3 tools/re/tank_rules_vs_ifc.py <probe-output.txt> <revit-export.ifc> [--list]

`--list` is what Measure passes to a probe's scorer; it also lists the ids each rule drops that Revit's
export holds, the written fixtures it lacks with every reading the probe printed for them, and the
features (a scope, a field and its value) that no fixture Revit holds has and some fixture it lacks has.

The lines that start `re363 ` are counts, rule names and IFC schema names only: they are what a model
with no licence to redistribute it keeps (tools/ci/scrub-holdout-results.sh, RE-363).
"""

import collections
import sys

NONE = "none"


def tokens(line):
    """The `key=value` words of a line, as a dict."""
    return dict(word.split("=", 1) for word in line.split()[1:] if "=" in word)


def parse(path):
    """Everything the probe printed, by kind."""
    fixtures = {}
    fields = collections.defaultdict(dict)
    classes = {}
    forms = {}
    graphics = {}
    documents = {}
    for raw in open(path):
        line = raw.rstrip("\n")
        head = line.split(" ", 1)[0]
        if head == "fixture":
            word = tokens(line)
            fixtures[int(word["id"])] = word
        elif head == "forms":
            word = tokens(line)
            forms[int(word["id"])] = word
        elif head == "graphics":
            word = tokens(line)
            graphics[int(word["id"])] = word
        elif head == "famdoc":
            word = tokens(line)
            documents[int(word["family"])] = word
        elif head == "native":
            parts = line.split(" ")
            if len(parts) >= 4:
                classes[(parts[1], int(parts[2]))] = parts[3]
        elif head == "field":
            parts = line.split(" ", 4)
            if len(parts) == 5:
                fields[(parts[1], int(parts[2]))][parts[3]] = parts[4]
    return fixtures, fields, classes, forms, graphics, documents


def safe(text):
    """`text` in the alphabet tools/ci/scrub-holdout-results.sh keeps for `re363` lines."""
    return "".join(
        ch if ch.isascii() and (ch.isalnum() or ch in " _:.,;()=#<>/+-") else "_" for ch in text
    )


def export_tags(path):
    """(schema, set of numeric Tags) of the elements of Revit's export."""
    import ifcopenshell

    model = ifcopenshell.open(path)
    tags = set()
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or element.is_a("IfcVirtualElement"):
            continue
        tag = getattr(element, "Tag", None)
        if tag and tag.isdigit():
            tags.add(int(tag))
    return model.schema, tags


def integer(value):
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


class Reading:
    """What the probe printed for one written fixture, as the rules read it."""

    def __init__(self, ident, fixtures, fields, forms, graphics):
        self.ident = ident
        self.fixture = fixtures[ident]
        self.forms = forms.get(ident, {})
        self.graphics = graphics.get(ident, {})
        symbol = integer(self.forms.get("symbol"))
        family = integer(self.forms.get("family"))
        self.scopes = {
            "instance": fields.get(("instance", ident), {}),
            "symbol": fields.get(("symbol", symbol), {}) if symbol is not None else {},
            "family": fields.get(("family", family), {}) if family is not None else {},
        }

    def field(self, scope, key):
        return self.scopes[scope].get(key)

    def primitives(self):
        """The primitive counts at levels 1, 2 and 3, or None where graphics were not read."""
        text = self.graphics.get("prim")
        if not text:
            return None
        try:
            return [int(n) for n in text.split("/")]
        except ValueError:
            return None

    def features(self):
        """Every reading as `scope.key=value`, for the table of what separates the fixtures."""
        out = {}
        for scope, values in self.scopes.items():
            for key, value in values.items():
                out[f"{scope}.{key}={value}"] = None
        for name in ("stage", "frames", "container", "kind", "option", "enclosing"):
            if name in self.fixture:
                out[f"record.{name}={self.fixture[name]}"] = None
        out[f"record.mutual_same={'yes' if self.fixture.get('mutual_same', '-') != '-' else 'no'}"] = None
        out[f"record.mutual_any={'yes' if self.fixture.get('mutual_any', '-') != '-' else 'no'}"] = None
        for name in ("verdict", "doc", "forms", "cutting", "invisible", "bound", "incomplete"):
            if name in self.forms:
                out[f"forms.{name}={self.forms[name]}"] = None
        for name in ("outcome", "prim", "hidden", "visible", "diag", "blocking", "kinds",
                     "vis_branches", "filters", "nonsurface", "unbounded", "styles", "materials"):
            if name in self.graphics:
                out[f"graphics.{name}={self.graphics[name]}"] = None
        return out


def rules_for(readings):
    """Rule name -> predicate on a Reading, True where the rule drops the fixture."""
    rules = {
        "nested_mutual_same_category": lambda r: r.fixture.get("mutual_same", "-") != "-",
        "nested_mutual_any_category": lambda r: r.fixture.get("mutual_any", "-") != "-",
        "container_slot_set": lambda r: any(
            part != NONE for part in r.fixture.get("container", NONE).split("+")
        ),
        "design_option_set": lambda r: any(
            part != NONE for part in r.fixture.get("option", NONE).split("+")
        ),
        "host_unset": lambda r: r.field("instance", "m_hostId.m_id.m_id64") == "-1",
        "work_plane_unset": lambda r: r.field("instance", "m_workPlaneBased") == "false",
        "marked_invisible": lambda r: r.field("instance", "m_invisible") == "true",
        "forms_bodiless": lambda r: r.forms.get("verdict") == "bodiless",
        "forms_family_has_no_form": lambda r: r.forms.get("verdict") == "undecided_no_forms",
        "forms_family_nests_instance_or_unread": lambda r: r.forms.get("verdict")
        in ("undecided_incomplete", "undecided_no_document"),
        "graphics_no_surface_and_no_diagnostic": lambda r: r.graphics.get("outcome")
        in ("empty_none", "empty_view_only"),
        "graphics_no_surface_at_all": lambda r: r.primitives() is not None
        and r.primitives()[2] == 0,
        "graphics_all_hidden": lambda r: integer(r.graphics.get("hidden")) is not None
        and integer(r.graphics.get("hidden")) > 0
        and integer(r.graphics.get("visible")) == 0,
    }
    supers = sorted(
        {
            key
            for reading in readings
            for key in reading.scopes["instance"]
            if "super" in key.lower()
        }
    )
    for key in supers:
        rules[f"instance_{key}_set"] = lambda r, key=key: (
            integer(r.field("instance", key)) is not None and integer(r.field("instance", key)) > 0
        )
    rules["bodiless_by_forms_or_graphics"] = lambda r: rules["forms_bodiless"](r) or rules[
        "graphics_no_surface_and_no_diagnostic"
    ](r)
    return rules, supers


def main(argv):
    listing = "--list" in argv
    argv = [arg for arg in argv if arg != "--list"]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    fixtures, fields, classes, forms, graphics, documents = parse(argv[1])
    schema, tags = export_tags(argv[2])
    written = sorted(i for i, f in fixtures.items() if f.get("stage") == "written")
    in_export = [i for i in written if i in tags]
    extras = [i for i in written if i not in tags]
    stages = collections.Counter(
        (f.get("stage"), i in tags) for i, f in fixtures.items()
    )
    print(
        f"export schema {schema}; elements with a numeric Tag: {len(tags)}; plumbing fixture "
        f"elements {len(fixtures)}; written {len(written)} (in export {len(in_export)}, not in "
        f"it {len(extras)})"
    )
    print(
        safe(
            f"re363 fixtures: schema {schema}; elements {len(fixtures)}; written {len(written)}; "
            f"written in export {len(in_export)}; written not in export {len(extras)}"
        )
    )
    for (stage, held), count in sorted(stages.items(), key=lambda item: (str(item[0][0]), item[0][1])):
        print(safe(f"re363 stage: {stage}; in export {'yes' if held else 'no'}; {count}"))
    readings = {i: Reading(i, fixtures, fields, forms, graphics) for i in written}
    for name, count in (
        ("native instance records", sum(1 for i in written if readings[i].scopes["instance"])),
        ("forms lines", sum(1 for i in written if readings[i].forms)),
        ("graphics lines", sum(1 for i in written if readings[i].graphics)),
    ):
        print(safe(f"re363 read: {name} {count} of {len(written)}"))
    rules, supers = rules_for(list(readings.values()))
    if not supers:
        print("re363 rule instance_super_field: no instance field is named super")
    for name, rule in rules.items():
        dropped = [i for i in written if rule(readings[i])]
        wrong = [i for i in dropped if i in tags]
        right = [i for i in dropped if i not in tags]
        print(
            safe(
                f"re363 rule {name}: schema {schema}; written {len(written)}; in export "
                f"{len(in_export)}; not in export {len(extras)}; rule drops {len(dropped)}; "
                f"dropped in export {len(wrong)}; dropped not in export {len(right)}; "
                f"not in export and kept {len(extras) - len(right)}"
            )
        )
        if listing and wrong:
            print(f"  {name} would drop, in Revit's export: {','.join(map(str, wrong[:30]))}")
    if listing:
        for ident in extras:
            reading = readings[ident]
            print(f"written, not in Revit's export: {ident}")
            print(f"  record: {reading.fixture}")
            print(f"  forms: {reading.forms}")
            print(f"  graphics: {reading.graphics}")
            print(
                "  dropped by: "
                + (", ".join(name for name, rule in rules.items() if rule(reading)) or "no rule")
            )
        held = collections.defaultdict(int)
        lacking = collections.defaultdict(int)
        for ident, reading in readings.items():
            for feature in reading.features():
                (held if ident in tags else lacking)[feature] += 1
        print(
            "features some fixture Revit's export lacks has and no fixture it holds has "
            "(scope.reading=value: lacking count):"
        )
        for feature in sorted(lacking):
            if held.get(feature, 0) == 0:
                print(f"  {feature}: {lacking[feature]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
