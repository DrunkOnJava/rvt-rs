#!/usr/bin/env python3
"""Cross-witness gate: IfcOpenShell parses a Revit-authored reference IFC and
must agree with the project-count manifest that the rvt-rs decoder is also
gated against (docs/verification-protocol.md).

Usage: witness-ifcopenshell.py <manifest.json> <corpus_dir> [--json OUT]
                               [--observation OUT]

`--observation` additionally writes an OctetProof observation
(docs/octetproof-spec-draft.md §6.2): entity counts for every manifest
`source_ifc_type` plus, for every manifest `relations` category, the relation's
`[host Tag, filling Tag]` pair set, canonicalized (sorted keys, no whitespace)
and hashed so a replay can prove the witness saw the same thing.

The manifest's `reference_ifc_file` is resolved under <corpus_dir>, its
SHA-256 is checked against `source.reference_ifc_sha256` (a golden artifact
must be the exact bytes the registry names), and every category carrying a
`source_ifc_type` is counted with IfcOpenShell (exact type, no subtypes — the
same semantics as the manifest's STEP-constructor counts) and compared to
`expected` within `tolerance`. Exit 1 on any drift or hash mismatch.

This validates the exporter ↔ independent-reader edge. It says nothing about
rvt-rs decode fidelity on its own; that is the sibling gate in
tests/project_count_fixtures.rs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

import ifcopenshell


def fills_element_pairs(model) -> list[list[str]]:
    """`IfcRelFillsElement` host/filling `Tag` pairs, canonically sorted.

    The chain is Revit's own: `IfcRelVoidsElement` binds an opening to the
    element it voids, `IfcRelFillsElement` binds that opening to the element
    that fills it, so the pair `[host Tag, filling Tag]` is the door/window
    to host-wall relation as an IFC reader sees it (OctetProof 1.1.0 §7.2,
    field class *relation pair sets*).

    An unset `Tag`, or an opening with no `IfcRelVoidsElement`, contributes an
    empty string rather than dropping the pair: a missing half must surface as
    a disagreement, never as a silent omission. Duplicates are kept, so the
    value is a sorted multiset.
    """
    voided_by = {}
    for rel in model.by_type("IfcRelVoidsElement", include_subtypes=False):
        voided_by[rel.RelatedOpeningElement.id()] = rel.RelatingBuildingElement

    def tag_of(entity) -> str:
        return "" if entity is None else (getattr(entity, "Tag", None) or "")

    pairs = []
    for rel in model.by_type("IfcRelFillsElement", include_subtypes=False):
        host = voided_by.get(rel.RelatingOpeningElement.id())
        pairs.append([tag_of(host), tag_of(rel.RelatedBuildingElement)])
    return sorted(pairs)


RELATION_READERS = {"IFCRELFILLSELEMENT": fills_element_pairs}


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("corpus_dir", type=Path)
    parser.add_argument("--json", type=Path, help="write the agreement record here")
    parser.add_argument("--observation", type=Path, help="write an OctetProof observation here")
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text())
    reference_name = manifest.get("reference_ifc_file")
    if not reference_name:
        print(f"{args.manifest}: no reference_ifc_file — nothing to witness", file=sys.stderr)
        return 0
    reference = args.corpus_dir / reference_name
    if not reference.is_file():
        print(f"error: reference IFC missing at {reference}", file=sys.stderr)
        return 1

    expected_sha = manifest.get("source", {}).get("reference_ifc_sha256")
    actual_sha = sha256_of(reference)
    if expected_sha and actual_sha != expected_sha:
        print(f"error: {reference.name} sha256 {actual_sha} != manifest {expected_sha}", file=sys.stderr)
        return 1

    model = ifcopenshell.open(str(reference))
    record = {
        "schema_version": 1,
        "manifest": manifest.get("id"),
        "reference_ifc": reference_name,
        "reference_ifc_sha256": actual_sha,
        "ifc_schema": model.schema,
        "witness": f"ifcopenshell {getattr(ifcopenshell, 'version', '?')}",
        "categories": [],
    }
    drift = 0
    print(f"{manifest.get('id')}: {reference_name} ({model.schema}, sha256 {actual_sha[:12]}…)")
    print(f"{'category':<16} {'ifc type':<22} {'expected':>8} {'ifcos':>6} {'tol':>4}  result")
    for category, spec in manifest.get("counts", {}).items():
        ifc_type = spec.get("source_ifc_type")
        if not ifc_type:
            continue
        expected = int(spec.get("expected", 0))
        tolerance = int(spec.get("tolerance", 0))
        try:
            actual = len(model.by_type(ifc_type, include_subtypes=False))
        except RuntimeError:
            actual = 0  # type not in this schema → zero instances
        ok = abs(actual - expected) <= tolerance
        drift += 0 if ok else 1
        record["categories"].append({
            "category": category,
            "ifc_type": ifc_type,
            "expected": expected,
            "tolerance": tolerance,
            "ifcopenshell": actual,
            "agree": ok,
        })
        print(f"{category:<16} {ifc_type:<22} {expected:>8} {actual:>6} {tolerance:>4}  {'ok' if ok else 'DRIFT'}")

    relations: dict[str, list[list[str]]] = {}
    for category, spec in manifest.get("relations", {}).items():
        relation_type = spec.get("relation_ifc_type")
        reader = RELATION_READERS.get(relation_type)
        if reader is None:
            print(f"error: {category}: no reader for relation type {relation_type}", file=sys.stderr)
            return 1
        pairs = reader(model)
        relations[relation_type] = pairs
        expected = int(spec.get("expected_pairs", 0))
        ok = len(pairs) == expected
        drift += 0 if ok else 1
        record["relations"] = record.get("relations", [])
        record["relations"].append({
            "category": category,
            "relation_ifc_type": relation_type,
            "expected_pairs": expected,
            "ifcopenshell_pairs": len(pairs),
            "agree": ok,
        })
        print(f"{category:<16} {relation_type:<22} {expected:>8} {len(pairs):>6} {0:>4}  {'ok' if ok else 'DRIFT'}")

    record["agree"] = drift == 0
    if args.json:
        args.json.write_text(json.dumps(record, indent=2))
    if args.observation:
        payload = {
            "entity_counts": {c["ifc_type"]: c["ifcopenshell"] for c in record["categories"]},
            "relations": relations,
            "ifc_schema": model.schema,
        }
        canonical = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        observation = {
            "schema_version": "1.1.0",
            "witness_id": "ifcopenshell",
            "witness_version": str(getattr(ifcopenshell, "version", "?")),
            "artifact_id": manifest.get("id"),
            "input_role": "bridge",
            "input_file": reference_name,
            "input_hash_sha256": actual_sha,
            "deterministic": True,
            "semantic_surface_covered": ["entity_counts", "relations"],
            "observation": payload,
            "observation_hash_sha256": hashlib.sha256(canonical).hexdigest(),
            "unsupported_entities": [],
            "warnings": [],
        }
        args.observation.parent.mkdir(parents=True, exist_ok=True)
        args.observation.write_text(json.dumps(observation, indent=2, sort_keys=True) + "\n")
    if drift:
        print(f"error: {drift} categor{'y' if drift == 1 else 'ies'} drifted from the manifest", file=sys.stderr)
        return 1
    print("cross-witness: IfcOpenShell agrees with the manifest for every source_ifc_type and relation")
    return 0


if __name__ == "__main__":
    sys.exit(main())
