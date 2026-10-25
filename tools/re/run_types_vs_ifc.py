#!/usr/bin/env python3
"""Which stair run type Revit's export gives each flight (#357).

Takes the output of `examples/probe_357_native_runs.rs`, whose `record`
lines give each run's own type (the one `OST_StairsRuns` type definition its
element record's reference list names, RE-28) and the type
`native_stairs::native_runs` takes through the run's stairs
(`StairsRun.m_stairsId` -> `StairsElement.m_typeId` ->
`StairsType.m_runTypeId`, from its `run` and `stairs` lines), and Revit's
IFC export. For every `IfcStairFlight` whose Tag is a run the probe printed, it
counts how often each of the two equals the Tag of the `IfcStairFlightType`
Revit relates the flight to, and lists every flight where they differ.

Needs IfcOpenShell.

Usage:

    python3 tools/re/run_types_vs_ifc.py <probe.txt> <revit.ifc>
"""

import re
import sys

import ifcopenshell


def read_probe(path):
    """Run Tag -> (own type, type through its stairs), from the `record`,
    `run` and `stairs` lines; the second is followed here, so it is there
    whether or not `native_runs` read the run's sketch."""
    own, run_stairs, stairs_type, run_type = {}, {}, {}, {}
    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(r"record (\d+) refs .* own-type (?:Some\((\d+)\)|None)", line)
        if m:
            own[m.group(1)] = m.group(2)
        m = re.match(r"run (\d+) .*\bm_stairsId=(\d+)", line)
        if m:
            run_stairs[m.group(1)] = m.group(2)
        m = re.match(r"stairs (\d+) StairsElement refs .*\bm_typeId=(\d+)", line)
        if m:
            stairs_type[m.group(1)] = m.group(2)
        m = re.match(r"stairs (\d+) StairsType refs .*\bm_runTypeId=(\d+)", line)
        if m:
            run_type[m.group(1)] = m.group(2)
    return {
        tag: (mine, run_type.get(stairs_type.get(run_stairs.get(tag))))
        for tag, mine in own.items()
    }


def flight_types(path):
    model = ifcopenshell.open(path)
    out = {}
    for rel in model.by_type("IfcRelDefinesByType"):
        kind = rel.RelatingType
        if not kind.is_a("IfcStairFlightType"):
            continue
        for flight in rel.RelatedObjects:
            if flight.is_a("IfcStairFlight") and flight.Tag:
                out[str(flight.Tag)] = (str(kind.Tag or ""), kind.Name or "")
    return out


def main():
    probe = read_probe(sys.argv[1])
    revit = flight_types(sys.argv[2])
    both = sorted(set(probe) & set(revit), key=int)
    print(f"runs in the probe: {len(probe)}; flights with a type in Revit's export: {len(revit)}; both: {len(both)}")
    counts = {"own": 0, "chain": 0, "own missing": 0, "chain missing": 0, "own differs from chain": 0}
    for tag in both:
        own, chain = probe[tag]
        theirs, name = revit[tag]
        counts["own"] += own == theirs
        counts["chain"] += chain == theirs
        counts["own missing"] += own is None
        counts["chain missing"] += chain is None
        if own is not None and chain is not None and own != chain:
            counts["own differs from chain"] += 1
        if own != theirs or chain != theirs:
            print(f"  {tag}: Revit {theirs} ({name}), own {own}, chain {chain}")
    print(", ".join(f"{k} {v}" for k, v in counts.items()) + f" (of {len(both)})")


if __name__ == "__main__":
    main()
