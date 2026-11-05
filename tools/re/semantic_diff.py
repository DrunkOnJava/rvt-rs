#!/usr/bin/env python3
"""The semantic diff of two MCP dumps (#1334): what moved in the meaning of an export, not in its bytes.

A dump is the answers of the MCP server's fixed query set on one model, as the deterministic
document `tools/re/mcp_client.py` writes (`mcp-dump.json`): every element's row, the levels, the
rooms, the property and quantity sets of a sample of elements, the summary and the diagnostics.
Measure writes one for each openly licensed model on both sides of a run with a base; this reads
the two and writes one diff. It is a pure function of the two JSON documents.

What the diff holds:

- Elements added and removed (keyed by ElementId, else GlobalId), and elements changed, with the
  value before and after at each path that moved.
- Offsets that shift for unrelated reasons are dropped: `offset` and `record_offset` inside a
  `provenance` object are byte positions in a stream, and a change that moves only those is counted
  (`coverage.position_only`) and not listed. The stream, the copy and the rule that chose a value
  stay, since a different source is a different statement.
- Order is not meaning: rows, rooms, property sets, properties and lists of strings are compared by
  key, not by position. An empty object or list is the same as no field (the server leaves out what
  is empty).
- Values are compared to the dump's precision, which is six decimals (tools/re/mcp_client.py rounds
  a float there before it writes the dump). 0.35 and 0.3500001 are different values when a dump
  holds both, an integer and the equal float are the same, and no tolerance is added to hide a
  change; whether an importer would notice is the scorer's question (tools/ci/jev_check.py), not
  this tool's. A change below the sixth decimal is not in a dump.
- Identical changes are collapsed into one finding, grouped by the whole triple (path, value
  before, value after) within a category: 400 walls that all gain the same property are one line
  with a count, and a wall that gains another value is a line of its own. Every group is kept in the
  JSON; only the text view cuts a long list, and says how many it left out.
- Per-category totals (elements by IFC entity, with rooms and levels) and a coverage block: how
  many elements' property sets both dumps hold (the dump keeps a sample of each entity's elements).

Usage:

    python3 tools/re/semantic_diff.py pair <base-dump.json> <head-dump.json> [--model NAME]
        [--json OUT] [--text OUT]
    python3 tools/re/semantic_diff.py measure <measure-base> <measure-head> <out-dir> [--requested-all]

`measure` is what Measure's compare job runs on its two merged result folders: for each openly
licensed model that has a dump on both sides it writes `<out-dir>/<model>.json` and `.txt`, and a
`manifest.json` naming the models, the models skipped and why, the open models no diff covers
(`uncovered`, against OPEN_MODEL_NAMES), whether the run was asked for every open model
(`--requested-all`: no model filter, the office and the Revit 2025 models on) and the measured
commits. A model on the restricted list (the held-out, spent, BIMcollab and rac_advanced models,
Autodesk's samples and families) must hold no dump; a dump there, or in a model on neither list,
fails the run with exit status 1 and no output.

Standard library only.
"""

import argparse
import collections
import fnmatch
import hashlib
import json
import os
import re
import sys

VERSION = 1
DUMP = "mcp-dump.json"
POSITION_KEYS = frozenset(("offset", "record_offset"))
# The models whose elements may be sent to a third party (Griffin, 2026-11-04): Core Interior,
# Einhoven and RE1 (MIT), the BIM4LCA office (CC BY-SA 4.0), puzzbobb's Revit 2025 models (Apache-2.0).
OPEN_MODELS = ("core-interior", "einhoven", "re1-*", "nordic-office-*", "r2025-*")
# Every open model a complete run measures: the reference models (tools/ci/measure-reference-models.sh),
# the BIM4LCA office (`nordic`) and puzzbobb's 24 Revit 2025 models (`r2025`). A check is written only
# for a run that covers all of them; a model added to a fetcher is added here, or a run is judged on
# fewer than it measured.
OPEN_MODEL_NAMES = (
    "core-interior",
    "einhoven",
    "nordic-office-as-built",
    "nordic-office-permit",
    "nordic-office-terrain-concrete",
    "nordic-office-terrain-timber",
    "r2025-00_Blank",
    "r2025-01_Walls",
    "r2025-02_1Slope",
    "r2025-02_1Slope_Rafter2PlumbCut",
    "r2025-02_1Slope_Rafter2SquareCut",
    "r2025-02_1Slope_Rafter2SquareCut_Overhang",
    "r2025-03_2Slope_Rafter2SquareCut",
    "r2025-04_Gable",
    "r2025-04_Gable_Swapped_Slopes",
    "r2025-05_1Slope_ArrowDefined",
    "r2025-06_1Slope_ArrowDefined_SetToSlope",
    "r2025-07_1Slope_ArrowDefined_2Cut50mmFascia",
    "r2025-08_LShapedHipRoof",
    "r2025-09_LShapedHipRoof_1Edge300Offset",
    "r2025-10_LShapedHipRoof_1Edge300Offset_Hole",
    "r2025-11_RoofByExtrusion",
    "r2025-12_Rooms_Door_Windows",
    "r2025-13_Rooms_Stairs",
    "r2025-14_Rooms_Stairs_endwithriserOFF",
    "r2025-15_Rooms_Stairs_NoRisers",
    "r2025-16_Rooms_Stairs_Monolithic",
    "r2025-17_Rooms_Stairs_Spiral",
    "r2025-18_WallJoins",
    "r2025-20_T_U_Roof",
    "re1-architecture",
    "re1-electrical",
    "re1-mechanical",
    "re1-plumbing",
)
RESTRICTED_PREFIXES = ("holdout-", "spent-", "bimcollab-", "racadv-", "sample-", "family-")
MAX_DUMP_BYTES = 400 * 1024 * 1024
MAX_FINDINGS = 20000
KEPT_IDS = 50
SHOWN_IDS = 6
TEXT_GROUPS_PER_PATH = 6
# What a detail repeats of its row (its storey is the row's `level`, and the levels section holds the
# elevations), or holds once for the file.
DETAIL_SKIPPED = frozenset(("file", "id", "name", "predefined_type", "level"))
# A room's own values; its row (an IfcSpace) carries the rest, and its existence.
ROOM_FIELDS = ("number", "area_square_feet", "level_elevation_feet")
SECTIONS = ("elements", "rooms", "levels", "summary", "diagnostics")
KIND_ORDER = {"element_removed": 0, "element_added": 1, "removed": 2, "added": 3, "changed": 4}
NOUNS = {"elements": "element", "rooms": "room", "levels": "level"}


class Absent:
    """The value of a path one side does not have."""

    def __repr__(self):
        return "ABSENT"


ABSENT = Absent()


def canon(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def short_hash(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:8]


def model_scope(name):
    """"restricted" for a model that is never sent anywhere, "open" for one that may be, else "unknown"."""
    if any(name.startswith(prefix) for prefix in RESTRICTED_PREFIXES):
        return "restricted"
    if any(fnmatch.fnmatchcase(name, pattern) for pattern in OPEN_MODELS):
        return "open"
    return "unknown"


# ---- flattening -------------------------------------------------------------------------------


def is_container(value):
    return isinstance(value, (dict, list))


def same(a, b):
    """Equal values: exact, an integer equal to a float, a boolean only a boolean."""
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(same(a[key], b[key]) for key in a)
    return type(a) is type(b) and a == b


def scrub(value, positions=False, provenance=False):
    """The value without the byte offsets inside its `provenance` objects (all of them with `positions`)."""
    if isinstance(value, dict):
        return {
            key: scrub(item, positions, provenance or key == "provenance")
            for key, item in value.items()
            if positions or not (provenance and key in POSITION_KEYS)
        }
    if isinstance(value, list):
        return [scrub(item, positions, provenance) for item in value]
    return value


def item_key(item):
    if isinstance(item, dict):
        if isinstance(item.get("name"), str):
            return "name", item["name"]
        if isinstance(item.get("id"), (str, int)) and not isinstance(item.get("id"), bool):
            return "id", str(item["id"])
    return None


def flatten(value, path, out):
    """Every leaf of `value` at its path, in `out`: dictionaries by key, lists of dictionaries by their name or id,
    lists of strings sorted, other lists in order, a dictionary of plain fields (a property) as one leaf."""
    if isinstance(value, dict):
        for key in sorted(value):
            flatten(value[key], f"{path}.{key}" if path else key, out)
    elif isinstance(value, list):
        flatten_list(value, path, out)
    else:
        out[path or "(root)"] = value


def flatten_list(items, path, out):
    if not items:
        return
    if not any(is_container(item) for item in items):
        out[path] = sorted(items) if all(isinstance(item, str) for item in items) else list(items)
        return
    keys = [item_key(item) for item in items]
    if all(key is not None for key in keys) and len({field for field, _ in keys}) == 1:
        field = keys[0][0]
        seen = collections.Counter()
        for item in sorted(items, key=canon):
            name = str(item[field])
            seen[name] += 1
            label = name if seen[name] == 1 else f"{name}#{seen[name]}"
            rest = {key: value for key, value in item.items() if key != field}
            child = f"{path}[{label}]"
            if any(is_container(value) for value in rest.values()):
                flatten(rest, child, out)
            else:
                out[child] = rest
        return
    seen = collections.Counter()
    for text in sorted(canon(item) for item in items):
        seen[text] += 1
        label = short_hash(text) if seen[text] == 1 else f"{short_hash(text)}#{seen[text]}"
        out[f"{path}[#{label}]"] = text


def flat(value, positions=False):
    out = {}
    flatten(scrub(value, positions), "", out)
    return out


def changes_between(before, after):
    """(path, before, after) for every path whose leaf differs, with ABSENT for a side that lacks it."""
    out = []
    for path in sorted(before.keys() | after.keys()):
        if path in before and path in after:
            if not same(before[path], after[path]):
                out.append((path, before[path], after[path]))
        elif path in before:
            out.append((path, before[path], ABSENT))
        else:
            out.append((path, ABSENT, after[path]))
    return out


# ---- reading a dump ---------------------------------------------------------------------------


def row_key(row):
    if row.get("id"):
        return str(row["id"])
    provenance = row.get("provenance") or {}
    if provenance.get("global_id"):
        return "gid:" + str(provenance["global_id"])
    return "row:" + short_hash(canon(scrub(row)))


def keyed(rows, key_of=row_key):
    """The items by key. A key two items share gets `#2`, `#3` in the order of their content."""
    out = {}
    seen = collections.Counter()
    for row in sorted(rows, key=lambda row: canon(scrub(row))):
        key = key_of(row)
        seen[key] += 1
        out[key if seen[key] == 1 else f"{key}#{seen[key]}"] = row
    return out


def category_of(row):
    return (row.get("provenance") or {}).get("category") or "(none)"


def detail_of(row, details):
    return details.get(str(row["id"])) if row.get("id") else None


def element_flat(row, detail, positions):
    out = flat({key: value for key, value in row.items() if key != "id"}, positions)
    if detail is not None:
        out.update(flat({key: value for key, value in detail.items() if key not in DETAIL_SKIPPED}, positions))
    return out


# ---- findings ---------------------------------------------------------------------------------


def leaf_number(leaf):
    if isinstance(leaf, dict):
        leaf = leaf.get("value")
    if isinstance(leaf, (int, float)) and not isinstance(leaf, bool):
        return leaf
    return None


def significant(number):
    return float(f"{number:.6g}")


def id_order(key):
    return (0, int(key), "") if key.isdigit() else (1, 0, key)


class Groups:
    """Identical changes, collapsed: one group per (section, category, path, value before, value after)."""

    def __init__(self):
        self.leaf = {}
        self.whole = {}

    def change(self, section, category, key, path, before, after):
        token = (
            section,
            category,
            path,
            "\0" if before is ABSENT else canon(before),
            "\0" if after is ABSENT else canon(after),
        )
        group = self.leaf.get(token)
        if group is None:
            kind = "added" if before is ABSENT else "removed" if after is ABSENT else "changed"
            group = {"section": section, "category": category, "kind": kind, "path": path, "count": 0, "ids": []}
            if before is not ABSENT:
                group["before"] = before
            if after is not ABSENT:
                group["after"] = after
            left, right = leaf_number(before), leaf_number(after)
            if kind == "changed" and left is not None and right is not None:
                group["delta"] = significant(right - left)
                if left != 0:
                    group["relative"] = significant((right - left) / abs(left))
            self.leaf[token] = group
        group["count"] += 1
        group["ids"].append(key)

    def element(self, section, category, kind, key, row):
        provenance = row.get("provenance") or {}
        token = (
            section,
            category,
            kind,
            str(provenance.get("class_name") or provenance.get("class") or ""),
            str(row.get("predefined_type") or ""),
            str(provenance.get("body_source") or ""),
            str(provenance.get("profile_source") or provenance.get("space_outline_source") or ""),
        )
        group = self.whole.get(token)
        if group is None:
            group = {
                "section": section,
                "category": category,
                "kind": kind,
                "path": "",
                "count": 0,
                "ids": [],
                "names": [],
                "class": token[3],
                "predefined_type": token[4],
                "body_source": token[5],
                "profile_source": token[6],
            }
            self.whole[token] = group
        group["count"] += 1
        group["ids"].append(key)
        if len(group["names"]) < 3 and row.get("name"):
            group["names"].append(str(row["name"])[:80])

    def findings(self):
        groups = list(self.leaf.values()) + list(self.whole.values())
        for group in groups:
            group["ids"] = sorted(group["ids"], key=id_order)[:KEPT_IDS]
        groups.sort(key=group_order)
        return groups


def group_order(group):
    return (
        SECTIONS.index(group["section"]),
        group["category"],
        KIND_ORDER[group["kind"]],
        group["path"],
        -group["count"],
        canon(group.get("before", "")),
        canon(group.get("after", "")),
        group.get("class", ""),
        group.get("predefined_type", ""),
        group.get("body_source", ""),
        group.get("profile_source", ""),
    )


def compare(section, base_items, head_items, flat_of, category_of_item, groups, totals, membership=True):
    """Added, removed and changed items of one collection: the number of items that moved only by a
    byte offset. `flat_of(before, after, positions)` is the pair of flattened leaves to compare;
    `membership` is false where another collection already says which items exist."""
    position_only = 0
    for key in sorted(base_items.keys() | head_items.keys(), key=id_order):
        before, after = base_items.get(key), head_items.get(key)
        if before is None or after is None:
            if membership:
                present = after if before is None else before
                category = category_of_item(present)
                totals[category]["added" if before is None else "removed"] += 1
                groups.element(section, category, "element_added" if before is None else "element_removed", key, present)
            continue
        category = category_of_item(after)
        found = changes_between(*flat_of(before, after, False))
        if found:
            totals[category]["changed"] += 1
            for path, left, right in found:
                groups.change(section, category, key, path, left, right)
        elif changes_between(*flat_of(before, after, True)):
            totals[category]["position_only"] += 1
            position_only += 1
    return position_only


def summary_part(summary):
    """The summary without what the rows already say: the levels, and the counts per entity, class and total."""
    out = {key: value for key, value in summary.items() if key != "levels"}
    elements = out.get("elements")
    if isinstance(elements, dict):
        out["elements"] = {
            key: value for key, value in elements.items() if key not in ("by_ifc_type", "by_revit_class", "total")
        }
    return out


def room_flat(room, positions):
    return flat({key: room[key] for key in ROOM_FIELDS if key in room}, positions)


def level_flat(level, positions):
    return flat({key: value for key, value in level.items() if key not in ("name", "element_count")}, positions)


def diff_dumps(base, head):
    """The semantic diff of two dumps, as a dict of sorted, deterministic content."""
    groups = Groups()
    totals = collections.defaultdict(collections.Counter)

    base_rows, head_rows = keyed(base.get("rows", [])), keyed(head.get("rows", []))
    for rows, side in ((base_rows, "base"), (head_rows, "head")):
        for row in rows.values():
            totals[category_of(row)][side] += 1
    base_details, head_details = base.get("details", {}), head.get("details", {})
    both = base_rows.keys() & head_rows.keys()
    compared = sum(1 for key in both if detail_of(base_rows[key], base_details) is not None and detail_of(head_rows[key], head_details) is not None)
    one_side = sum(1 for key in both if (detail_of(base_rows[key], base_details) is None) != (detail_of(head_rows[key], head_details) is None))

    def element_pair(before, after, positions):
        left, right = detail_of(before, base_details), detail_of(after, head_details)
        if left is None or right is None:
            left = right = None
        return element_flat(before, left, positions), element_flat(after, right, positions)

    position_only = compare("elements", base_rows, head_rows, element_pair, category_of, groups, totals)

    base_rooms, head_rooms = keyed(base.get("rooms", [])), keyed(head.get("rooms", []))
    for rooms, side in ((base_rooms, "base"), (head_rooms, "head")):
        totals["(rooms)"][side] += len(rooms)
    position_only += compare(
        "rooms",
        base_rooms,
        head_rooms,
        lambda before, after, positions: (room_flat(before, positions), room_flat(after, positions)),
        lambda room: "(rooms)",
        groups,
        totals,
        membership=False,
    )

    base_levels = keyed(base.get("levels", []), lambda level: str(level.get("name")))
    head_levels = keyed(head.get("levels", []), lambda level: str(level.get("name")))
    for levels, side in ((base_levels, "base"), (head_levels, "head")):
        totals["(levels)"][side] += len(levels)
    compare(
        "levels",
        base_levels,
        head_levels,
        lambda before, after, positions: (level_flat(before, positions), level_flat(after, positions)),
        lambda level: "(levels)",
        groups,
        totals,
    )

    for section, left, right in (
        ("summary", summary_part(base.get("summary", {})), summary_part(head.get("summary", {}))),
        ("diagnostics", base.get("diagnostics", {}), head.get("diagnostics", {})),
    ):
        for path, before, after in changes_between(flat(left), flat(right)):
            groups.change(section, f"({section})", section, path, before, after)

    found = groups.findings()
    dropped = 0
    if len(found) > MAX_FINDINGS:
        dropped = len(found) - MAX_FINDINGS
        found = sorted(sorted(found, key=lambda group: -group["count"])[:MAX_FINDINGS], key=group_order)
    for number, group in enumerate(found, 1):
        group["id"] = f"f{number}"

    categories = {}
    for name in sorted(totals):
        counts = totals[name]
        categories[name] = {
            key: counts[key] for key in ("base", "head", "added", "removed", "changed", "position_only")
        }
    result = {
        "version": VERSION,
        "elements": {
            "base": len(base_rows),
            "head": len(head_rows),
            "added": sum(1 for key in head_rows if key not in base_rows),
            "removed": sum(1 for key in base_rows if key not in head_rows),
            "changed": sum(counts["changed"] for name, counts in categories.items() if not name.startswith("(")),
        },
        "categories": categories,
        "coverage": {"details_compared": compared, "details_one_side": one_side, "position_only": position_only},
        "findings": found,
        "dropped_groups": dropped,
    }
    result["summary"] = render_text(result)
    return result


# ---- the text view ----------------------------------------------------------------------------

PSET_PATH = re.compile(r"property_sets\[([^\]]*)\]\.properties\[([^\]]*)\]")
PSET_KIND = re.compile(r"property_sets\[([^\]]*)\]\.kind$")


def show_path(path):
    return PSET_PATH.sub(r"\1.\2", PSET_KIND.sub(r"\1 (kind)", path))


def show_value(leaf):
    if leaf is ABSENT:
        return "absent"
    if isinstance(leaf, dict):
        leaf = {key: value for key, value in leaf.items() if key != "global_id"}
    if isinstance(leaf, dict) and "value" in leaf:
        text = json.dumps(leaf["value"], ensure_ascii=True)
        kind = leaf.get("ifc_type") or leaf.get("quantity")
        text = text if kind is None else f"{text} ({kind})"
    else:
        text = json.dumps(leaf, ensure_ascii=True, sort_keys=True)
    return text if len(text) <= 100 else text[:97] + "..."


def plural(count, word):
    return f"{count} {word}" if count == 1 else f"{count} {word}s"


def describe(group):
    """One finding as a line, without its id."""
    count, section = group["count"], group["section"]
    if group["kind"] in ("element_added", "element_removed"):
        side = "only in the head" if group["kind"] == "element_added" else "only in the base"
        parts = [part for part in (group["class"], group["predefined_type"], group["body_source"], group["profile_source"]) if part]
        names = "; e.g. " + ", ".join(group["names"]) if group["names"] else ""
        noun = f"{group['category']} {NOUNS.get(section, 'item')}"
        return f"{plural(count, noun)} {side} ({', '.join(parts) or 'no class'}){names}"
    change = f"{show_path(group['path'])} {show_value(group.get('before', ABSENT))} -> {show_value(group.get('after', ABSENT))}"
    if "delta" in group:
        change += f" (delta {group['delta']:+g}" + (f", relative {group['relative']:+g})" if "relative" in group else ")")
    if section in NOUNS:
        return f"{plural(count, group['category'] + ' ' + NOUNS[section])}: {change}"
    return f"{section}: {change}"


def id_note(group):
    ids = group["ids"]
    if not ids or group["section"] not in NOUNS:
        return ""
    more = group["count"] - min(len(ids), SHOWN_IDS)
    return f" [{', '.join(ids[:SHOWN_IDS])}{', +' + str(more) if more > 0 else ''}]"


def render_text(diff, model=None):
    """The human summary of a diff, deterministic: totals per category, then each finding."""
    elements, coverage = diff["elements"], diff["coverage"]
    lines = [f"semantic diff{' of ' + model if model else ''}"]
    lines.append(
        f"elements: base {elements['base']}, head {elements['head']}; {elements['added']} added, "
        f"{elements['removed']} removed, {elements['changed']} changed, "
        f"{coverage['position_only']} moved by byte offset only (not listed)"
    )
    lines.append(
        f"coverage: property sets compared on {coverage['details_compared']} elements the dump holds on both sides "
        f"({coverage['details_one_side']} held on one side only, not compared)"
    )
    lines.append("by category (base -> head, added, removed, changed):")
    for name, counts in diff["categories"].items():
        lines.append(f"  {name}: {counts['base']} -> {counts['head']}, +{counts['added']} -{counts['removed']} ~{counts['changed']}")
    findings = diff["findings"]
    if not findings:
        lines.append("no semantic difference")
        return "\n".join(lines) + "\n"
    lines.append(f"findings: {len(findings)} groups of identical changes")
    shown = collections.Counter()
    hidden = collections.defaultdict(lambda: [0, 0])
    for group in findings:
        key = (group["section"], group["category"], group["path"])
        shown[key] += 1
        if shown[key] > TEXT_GROUPS_PER_PATH:
            hidden[key][0] += 1
            hidden[key][1] += group["count"]
            continue
        lines.append(f"  {group['id']} [{group['section']}] {describe(group)}{id_note(group)}")
    for (section, category, path), (count, changes) in sorted(hidden.items()):
        lines.append(
            f"  ... {plural(count, 'further value pair')} of {show_path(path) or category} in {category}, "
            f"on {changes} changes (all in the JSON)"
        )
    if diff.get("dropped_groups"):
        lines.append(f"  ... {diff['dropped_groups']} smaller groups are left out of the JSON as well (over {MAX_FINDINGS})")
    return "\n".join(lines) + "\n"


# ---- Measure's use ----------------------------------------------------------------------------


def load_dump(path):
    if os.path.getsize(path) > MAX_DUMP_BYTES:
        raise ValueError("over the size limit")
    with open(path, encoding="utf-8") as handle:
        dump = json.load(handle)
    if not isinstance(dump, dict) or not isinstance(dump.get("rows"), list):
        raise ValueError("not a dump")
    return dump


def commit_of(folder):
    try:
        with open(os.path.join(folder, "commit.txt"), encoding="utf-8") as handle:
            token = handle.read().split()[0]
    except (OSError, IndexError):
        return None
    return token if re.fullmatch(r"[0-9a-f]{40}", token) else None


def model_folders(folder):
    if not os.path.isdir(folder):
        return []
    return sorted(name for name in os.listdir(folder) if os.path.isdir(os.path.join(folder, name)))


def write(path, text):
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


def uncovered_models(diffed, skipped, expected=OPEN_MODEL_NAMES):
    """(model, reason) for each open model that no diff covers, in order: a skipped model with the reason
    it was skipped, any other with the fact that neither side held a result folder for it."""
    reasons = {item["model"]: item["reason"] for item in skipped}
    return [(name, reasons.get(name, "no result folder on either side")) for name in expected if name not in set(diffed)]


def measure(base_dir, head_dir, out_dir, requested_all=False):
    """Diff every openly licensed model that has a dump on both sides. Returns (manifest, violations);
    a violation is a dump in a model that is restricted or on neither list."""
    names = sorted(set(model_folders(base_dir)) | set(model_folders(head_dir)))
    violations, models, skipped = [], [], []
    for name in names:
        scope = model_scope(name)
        sides = {"base": base_dir, "head": head_dir}
        held = [side for side, folder in sides.items() if os.path.exists(os.path.join(folder, name, DUMP))]
        if scope != "open":
            if held:
                violations.append(f"{name}: a dump on the {' and '.join(held)} side of a {scope} model")
            continue
        if len(held) < 2:
            missing = [side for side in sides if side not in held]
            skipped.append({"model": name, "reason": f"no dump on the {' or '.join(missing)} side"})
            continue
        try:
            before = load_dump(os.path.join(base_dir, name, DUMP))
            after = load_dump(os.path.join(head_dir, name, DUMP))
        except (OSError, ValueError) as error:
            skipped.append({"model": name, "reason": f"a dump could not be read ({type(error).__name__})"})
            continue
        try:
            diff = diff_dumps(before, after)
        except Exception as error:  # a diff that cannot be made is recorded, not a red run
            skipped.append({"model": name, "reason": f"the diff failed ({type(error).__name__})"})
            continue
        models.append(
            {
                "model": name,
                "file": f"{name}.json",
                "findings": len(diff["findings"]),
                "dropped_groups": diff["dropped_groups"],
                "elements": diff["elements"],
            }
        )
        os.makedirs(out_dir, exist_ok=True)
        write(os.path.join(out_dir, f"{name}.json"), json.dumps({"model": name, "diff": diff}, sort_keys=True, indent=1) + "\n")
        write(os.path.join(out_dir, f"{name}.txt"), render_text(diff, name))
    manifest = {
        "version": VERSION,
        "base_sha": commit_of(base_dir),
        "head_sha": commit_of(head_dir),
        "models": models,
        "skipped": skipped,
        "uncovered": [{"model": name, "reason": reason} for name, reason in uncovered_models([m["model"] for m in models], skipped)],
        "requested_all": bool(requested_all),
        "folders_checked": len(names),
    }
    return manifest, violations


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    pair = sub.add_parser("pair")
    pair.add_argument("base")
    pair.add_argument("head")
    pair.add_argument("--model")
    pair.add_argument("--json")
    pair.add_argument("--text")
    run = sub.add_parser("measure")
    run.add_argument("base_dir")
    run.add_argument("head_dir")
    run.add_argument("out_dir")
    run.add_argument("--requested-all", action="store_true")
    args = parser.parse_args(argv)
    if args.command == "pair":
        diff = diff_dumps(load_dump(args.base), load_dump(args.head))
        if args.json:
            write(args.json, json.dumps({"model": args.model, "diff": diff}, sort_keys=True, indent=1) + "\n")
        text = render_text(diff, args.model)
        if args.text:
            write(args.text, text)
        else:
            sys.stdout.write(text)
        return 0
    manifest, violations = measure(args.base_dir, args.head_dir, args.out_dir, args.requested_all)
    if violations:
        for line in violations:
            print(f"::error::semantic diff: {line}")
        print("a dump may exist only for an openly licensed model (OPEN_MODELS in tools/re/semantic_diff.py)")
        return 1
    if not manifest["models"] and not manifest["skipped"]:
        print(f"semantic diff: no dump on either side in {manifest['folders_checked']} model folders, nothing to compare")
        return 0
    os.makedirs(args.out_dir, exist_ok=True)
    write(os.path.join(args.out_dir, "manifest.json"), json.dumps(manifest, sort_keys=True, indent=1) + "\n")
    print(
        f"semantic diff: {len(manifest['models'])} models diffed, {len(manifest['skipped'])} skipped, "
        f"{manifest['folders_checked']} model folders checked against the restricted list"
    )
    for item in manifest["skipped"]:
        print(f"  skipped {item['model']}: {item['reason']}")
    if manifest["uncovered"]:
        print(
            f"  {len(manifest['uncovered'])} of {len(OPEN_MODEL_NAMES)} open models are not covered by a diff, so the semantic check "
            + ("is NOT MEASURED at best" if manifest["requested_all"] else "writes no check run (this run was not asked for every open model)")
            + ": "
            + ", ".join(item["model"] for item in manifest["uncovered"][:12])
            + (f", and {len(manifest['uncovered']) - 12} more" if len(manifest["uncovered"]) > 12 else "")
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
