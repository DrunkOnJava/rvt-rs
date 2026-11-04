#!/usr/bin/env python3
"""Check that the stage counts of a Revit 2008 to 2023 export add up (#1389).

Reads an export's `diagnostics.json` (`rvt-ifc --diagnostics`) and checks its
`decoded.element_record_stages`: the counts of what became of the frames at the
file's element-record markers, per partition stream and in total.

For the file and for each stream the steps must add up:

    markers = frame_outside_stream + category_out_of_range + box_not_valid
              + no_declared_id + records
    records = not_placed_instances + placed_instances
    placed_instances = without_volume + with_volume
    with_volume = superseded + nested_components + selected
    selected = in_non_primary_design_option + not_in_export_phase
               + nested_in_door_or_window + category_not_exported + handed_to_export

and the streams' counts must sum to the file's. Beside them:

- `declared_id` is no more than `markers`, `id_from_enclosing_record` no more
  than `records`;
- the elements handed to the export are the elements written plus those set aside,
  each counted for one cause: `handed_to_export = written_to_model +
  id_already_held + below_min_confidence + sum(not_written_by_class)`; and
  `written_to_model` is no more than the model's building elements
  (`exported.building_elements`);
- `stopped_at` is present exactly when `written_to_model` is 0, and names a stage;
- the `skipped` item `element_record_frame_stopped` has, by stage, the frames
  that stopped before they were records and the placed instances with no volume.

A Revit 2015 to 2023 project must have the block (Revit 2008 to 2014 files are not
exported by rvt-rs, so a file of those years is not required to; a family file,
which reads its elements from Global/Latest, has none). A file of a later release
has it only when its schema is an older release's (#421); either way it is
checked when present.

Usage:

    python3 tools/re/element_stage_counts.py <diagnostics.json>

Prints the counts and ends with `ok` or one line for each step that does not add
up; exits 1 on any, 0 otherwise.
"""

import json
import sys

STEPS = [
    (
        "markers",
        [
            "frame_outside_stream",
            "category_out_of_range",
            "box_not_valid",
            "no_declared_id",
            "records",
        ],
    ),
    ("records", ["not_placed_instances", "placed_instances"]),
    ("placed_instances", ["without_volume", "with_volume"]),
    ("with_volume", ["superseded", "nested_components", "selected"]),
    (
        "selected",
        [
            "in_non_primary_design_option",
            "not_in_export_phase",
            "nested_in_door_or_window",
            "category_not_exported",
            "handed_to_export",
        ],
    ),
]

FRAME_STOP_STAGES = [
    "frame_outside_stream",
    "category_out_of_range",
    "box_not_valid",
    "no_declared_id",
    "without_volume",
]

EMPTY_STAGES = {
    "elem_table_not_read",
    "schema_has_no_element_marker",
    "no_markers",
    "frame_outside_stream",
    "category_out_of_range",
    "box_not_valid",
    "no_declared_id",
    "stream_not_read",
    "no_placed_instances",
    "without_volume",
    "nested_components",
    "in_non_primary_design_option",
    "not_in_export_phase",
    "nested_in_door_or_window",
    "category_not_exported",
    "id_already_held",
    "below_min_confidence",
    "class_not_exported",
}


def broken_steps(label, counts):
    """The lines for each step of `counts` that does not add up."""
    problems = []
    for name, parts in STEPS:
        total = sum(counts.get(part, 0) for part in parts)
        if counts.get(name, 0) != total:
            problems.append(
                f"{label}: {name} is {counts.get(name, 0)}, its parts sum to {total}"
            )
    if counts.get("declared_id", 0) > counts.get("markers", 0):
        problems.append(f"{label}: declared_id is more than markers")
    if counts.get("id_from_enclosing_record", 0) > counts.get("records", 0):
        problems.append(f"{label}: id_from_enclosing_record is more than records")
    return problems


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    with open(argv[1]) as handle:
        diagnostics = json.load(handle)
    release = diagnostics.get("input", {}).get("revit_version")
    stages = diagnostics.get("decoded", {}).get("element_record_stages")
    if stages is None:
        if isinstance(release, int) and 2015 <= release <= 2023:
            print(f"Revit {release}: no decoded.element_record_stages")
            print("MISSING")
            return 1
        print(f"Revit {release}: no decoded.element_record_stages (not this layout)")
        print("ok")
        return 0

    problems = []
    total = stages["total"]
    streams = stages["streams"]
    problems += broken_steps("file", total)
    summed = {}
    for name, counts in streams.items():
        problems += broken_steps(name, counts)
        for field, value in counts.items():
            summed[field] = summed.get(field, 0) + value
    for field in sorted(set(summed) | set(total)):
        if summed.get(field, 0) != total.get(field, 0):
            problems.append(
                f"file: {field} is {total.get(field, 0)}, the streams' counts sum to {summed.get(field, 0)}"
            )

    written = stages["written_to_model"]
    handed = total["handed_to_export"]
    building = diagnostics.get("exported", {}).get("building_elements")
    held = stages.get("id_already_held", 0)
    below = stages.get("below_min_confidence", 0)
    by_class = stages.get("not_written_by_class", {})
    set_aside = sum(by_class.values())
    if handed != written + held + below + set_aside:
        problems.append(
            f"file: {handed} elements handed to the export, {written} written + {held} id already held"
            f" + {below} below the confidence floor + {set_aside} not written by class"
        )
    if building is not None and written > building:
        problems.append(f"file: {written} elements written of {building} building elements")
    stopped_at = stages.get("stopped_at")
    if (written == 0) != (stopped_at is not None):
        problems.append(
            f"file: written_to_model is {written} and stopped_at is {stopped_at!r}"
        )
    if stopped_at is not None and stopped_at not in EMPTY_STAGES:
        problems.append(f"file: stopped_at {stopped_at!r} names no stage")

    item = next(
        (
            item
            for item in diagnostics.get("skipped", [])
            if item.get("reason") == "element_record_frame_stopped"
        ),
        None,
    )
    expected = {
        stage: total.get(stage, 0) for stage in FRAME_STOP_STAGES if total.get(stage, 0) > 0
    }
    if item is None:
        if expected:
            problems.append(f"skipped: no element_record_frame_stopped item, counts {expected}")
    else:
        if item.get("classes") != expected:
            problems.append(
                f"skipped: element_record_frame_stopped by stage {item.get('classes')}, counts {expected}"
            )
        if item.get("count") != sum(expected.values()):
            problems.append(
                f"skipped: element_record_frame_stopped count {item.get('count')}, counts {sum(expected.values())}"
            )

    print(f"Revit {release}, layout {stages['layout_release']}, {len(streams)} streams")
    for field in [
        "markers",
        "declared_id",
        "frame_outside_stream",
        "category_out_of_range",
        "box_not_valid",
        "no_declared_id",
        "records",
        "id_from_enclosing_record",
        "not_placed_instances",
        "placed_instances",
        "without_volume",
        "with_volume",
        "superseded",
        "nested_components",
        "selected",
        "in_non_primary_design_option",
        "not_in_export_phase",
        "nested_in_door_or_window",
        "category_not_exported",
        "handed_to_export",
    ]:
        print(f"  {field}: {total.get(field, 0)}")
    print(f"  stream_not_read: {total.get('stream_not_read', 0)}")
    print(f"  written_to_model: {written}")
    print(f"  id_already_held: {held}")
    print(f"  below_min_confidence: {below}")
    print(f"  not_written_by_class: {by_class}")
    print(f"  exported.building_elements: {building}")
    if stopped_at is not None:
        print(f"  stopped_at: {stopped_at}")
    for problem in problems:
        print(problem)
    if problems:
        return 1
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
