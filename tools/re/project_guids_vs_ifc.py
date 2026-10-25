#!/usr/bin/env python3
"""Find the element whose RE-48 GlobalId Revit's export derives its
project's, site's and building's GlobalIds from (#878).

Reads `examples/probe_878_project_guids.rs`'s output (RE-48's GlobalId of
every declared element, and the project information records' ids). For
Revit's IfcProject, IfcSite and IfcBuilding, it expands each GlobalId to its
128 bits and, for each XOR of the low 8 bits from 0 to 255, looks for an
element whose RE-48 GlobalId it is.

Usage:

    python3 tools/re/project_guids_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import sys

import ifcopenshell

ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"


def expand(global_id):
    value = 0
    for ch in global_id:
        value = value * 64 + ALPHABET.index(ch)
    return value


def compress(value):
    chars = []
    for _ in range(22):
        chars.append(ALPHABET[value % 64])
        value //= 64
    return "".join(reversed(chars))


def main():
    by_guid, info, episodes = {}, [], {}
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) == 3 and parts[0] == "rgid":
            by_guid[parts[2]] = parts[1]
        elif len(parts) == 3 and parts[0] == "episode":
            episodes[expand(parts[2]) >> 32] = (parts[1], expand(parts[2]) & 0xFFFFFFFF)
        elif len(parts) == 3 and parts[0] == "category":
            info.append(parts[1])
    print(f"RE-48 GlobalIds {len(by_guid)}; project information records {sorted(set(info))}")
    model = ifcopenshell.open(sys.argv[2])
    for entity in ("IfcProject", "IfcSite", "IfcBuilding"):
        for element in model.by_type(entity):
            value = expand(element.GlobalId)
            hits = []
            for xor in range(256):
                owner = by_guid.get(compress(value ^ xor))
                if owner:
                    hits.append(f"xor {xor} -> element {owner}{' (project information)' if owner in info else ''}")
            print(f"{entity} {element.GlobalId}: {'; '.join(hits) or 'no element'}")
            prefix = episodes.get(value >> 32)
            if prefix:
                index, low = prefix
                print(f"  high 96 bits are episode {index}'s; low 32 bits XOR the episode's = {(value & 0xFFFFFFFF) ^ low}")
            else:
                closest = min(episodes, key=lambda h: bin(h ^ (value >> 32)).count("1"), default=None)
                if closest is not None:
                    print(f"  no episode's high 96 bits; the closest of {len(episodes)} differs in {bin(closest ^ (value >> 32)).count('1')} bits")


if __name__ == "__main__":
    main()
