#!/usr/bin/env python3
"""Where a file keeps each element's editing episode, found from Revit's own
GlobalIds (RE-208, #878).

Revit's GlobalId for an element is its episode's GUID with the ElementId
XORed into the last 32 bits (RE-48). So every GlobalId in Revit's export,
with its element's Tag XORed back out, gives that element's episode GUID.
This scorer takes the output of `examples/probe_re208_episodes_2015.rs` (the
GUID runs it found in `Global/History` and every `Global/ElemTable` record)
and Revit's export, and reports:

- how many of the export's elements have an episode GUID that one of the
  history's runs holds, and at which index of which run;
- for those, every offset of the element's ElemTable record where a u32
  equals that index counted from the first entry or from the last, ranked
  by how many elements agree.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/episodes_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import argparse
import collections
import struct

import ifcopenshell

ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"


def expand(global_id):
    n = 0
    for ch in global_id:
        n = n * 64 + ALPHABET.index(ch)
    return n.to_bytes(16, "big")


def stored_order(canonical):
    b = bytearray(canonical)
    b[0:4] = b[0:4][::-1]
    b[4:6] = b[4:6][::-1]
    b[6:8] = b[6:8][::-1]
    return bytes(b)


def read_probe(path):
    runs, records, header = collections.defaultdict(list), {}, []
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "guid":
            runs[int(parts[1])].append(bytes.fromhex(parts[3]))
        elif parts[0] == "rec":
            raw = bytes.fromhex(parts[3])
            for ident in (int(parts[1]), int(parts[2])):
                records.setdefault(ident, raw)
        elif parts[0] in ("history", "elemtable", "run"):
            header.append(line.rstrip())
    return runs, records, header


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true")
    args = ap.parse_args()
    runs, records, header = read_probe(args.probe)
    for line in header[:12]:
        print(line)
    where = {}
    for start, guids in runs.items():
        for index, guid in enumerate(guids):
            where.setdefault(guid, (start, index, len(guids)))

    model = ifcopenshell.open(args.reference)
    elements = {}
    for product in model.by_type("IfcProduct"):
        # An opening carries its host's Tag.
        if product.is_a("IfcOpeningElement"):
            continue
        tag = getattr(product, "Tag", None)
        if tag and str(tag).isdigit():
            elements[int(tag)] = product.GlobalId

    found, missing, offsets = collections.Counter(), [], collections.Counter()
    episodes = collections.Counter()
    no_record = 0
    for tag, global_id in sorted(elements.items()):
        value = bytearray(expand(global_id))
        low = struct.unpack(">I", value[12:16])[0] ^ tag
        value[12:16] = struct.pack(">I", low)
        guid = stored_order(bytes(value))
        if guid not in where:
            missing.append((tag, global_id, guid.hex()))
            continue
        start, index, n = where[guid]
        found[start] += 1
        episodes[(start, index)] += 1
        raw = records.get(tag)
        if raw is None:
            no_record += 1
            continue
        for at in range(0, len(raw) - 3):
            v = struct.unpack("<I", raw[at:at + 4])[0]
            if v == index:
                offsets[(at, "index from the first entry")] += 1
            if v == n - 1 - index:
                offsets[(at, "index from the last entry")] += 1

    total = len(elements)
    print(f"elements with a numeric Tag in Revit's export: {total}")
    print(f"episode GUID in a history run: {sum(found.values())} ({', '.join(f'run at {s}: {c}' for s, c in sorted(found.items()))})")
    print(f"distinct episodes they use: {len(episodes)}; no ElemTable record for the Tag: {no_record}")
    print("record offsets holding the episode number, by elements agreeing:")
    for (at, form), count in offsets.most_common(8):
        print(f"  +{at} ({form}): {count}")
    print(f"episode GUID in no run: {len(missing)}")
    for tag, global_id, guid in missing[: 20 if args.list else 10]:
        print(f"  {tag} {global_id} episode {guid}")

    # A GlobalId taken from a stored IfcGUID parameter is in the file as text.
    stored = collections.defaultdict(list)
    for line in open(args.probe, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) >= 5 and parts[0] == "gid":
            stored[parts[4]].append((parts[1], int(parts[2]), parts[3], parts[5] if len(parts) > 5 else ""))
    by_tag = {gid: tag for tag, gid in elements.items()}
    hits = {gid: stored[gid] for gid in by_tag if gid in stored}
    print(f"GlobalId-shaped strings in the file: {len(stored)}; Revit's element GlobalIds among them: {len(hits)} of {len(by_tag)}")
    places = collections.Counter((s, enc) for occ in hits.values() for s, _, enc, _ in occ)
    for (stream, enc), n in places.most_common(10):
        print(f"  {stream} {enc}: {n}")
    for gid, occ in list(hits.items())[: 12 if args.list else 4]:
        for stream, at, enc, before in occ[:2]:
            print(f"  {by_tag[gid]} {gid} {stream}+{at} {enc} before={before}")
    # The same 21 trailing characters with another first one: the stored value
    # differs from Revit's in the top two bits only.
    by_suffix = collections.defaultdict(list)
    for s, occ in stored.items():
        by_suffix[s[1:]].append((s, occ))
    firsts, matched, prefixes = collections.Counter(), 0, collections.Counter()
    for gid, tag in by_tag.items():
        found = by_suffix.get(gid[1:])
        if not found:
            continue
        matched += 1
        for s, occ in found:
            firsts[(s[0], gid[0])] += 1
            for _, _, _, before in occ:
                prefixes[before[-16:]] += 1
    print(f"Revit's GlobalIds whose last 21 characters a stored string has: {matched} of {len(by_tag)}")
    # The stored value as a UniqueId-based GlobalId: XOR the Tag out of it and
    # look its episode up; and how its top two bits relate to the export's.
    in_history, xor_top = 0, collections.Counter()
    relation = collections.Counter()
    for gid, tag in by_tag.items():
        for s, _ in by_suffix.get(gid[1:], []):
            value = bytearray(expand(s))
            low = struct.unpack(">I", value[12:16])[0] ^ tag
            value[12:16] = struct.pack(">I", low)
            if stored_order(bytes(value)) in where:
                in_history += 1
            a, b = ALPHABET.index(s[0]), ALPHABET.index(gid[0])
            xor_top[a ^ b] += 1
            e = expand(gid)
            relation[("export byte 15 low 2 bits", e[15] & 3, "xor", a ^ b)] += 1
            relation[("tag low 2 bits", tag & 3, "xor", a ^ b)] += 1
    print(f"  stored values whose episode (Tag XORed out) is in the history: {in_history}")
    # An episode mixed with the id some other way would still share bytes 1 to
    # 11 (canonical order) with the GlobalId.
    middles = {}
    for start, guids in runs.items():
        for index, g in enumerate(guids):
            middles.setdefault(stored_order(g)[1:12], (start, index))
    share_export = sum(1 for gid in by_tag if expand(gid)[1:12] in middles)
    share_stored = sum(1 for s in stored if expand(s)[1:12] in middles)
    print(f"  GlobalIds sharing bytes 1-11 with a history episode: Revit's {share_export} of {len(by_tag)}, stored {share_stored} of {len(stored)}")
    shared8 = sum(1 for gid in by_tag if any(expand(gid)[4:12] == stored_order(g)[4:12] for gs in runs.values() for g in gs))
    print(f"  Revit's GlobalIds sharing bytes 4-11 with a history episode: {shared8}")
    print("  stored top two bits XOR Revit's: " + ", ".join(f"{k} {v}" for k, v in sorted(xor_top.items())))
    for key, n in sorted(relation.items()):
        print(f"  {key}: {n}")
    print("  stored first character -> Revit's: " + ", ".join(f"{a}->{b} {n}" for (a, b), n in sorted(firsts.items())))
    print("  the 8 bytes before those strings: " + ", ".join(f"{p} {n}" for p, n in prefixes.most_common(4)))

    # Elements made in one episode share its first 12 bytes once their Tag is
    # XORed out of the last 4: count the groups.
    for label, values in (("Revit's element GlobalIds", list(by_tag.items())),
                          ("stored strings matched to them", [(s, by_tag[g]) for g in by_tag for s, _ in by_suffix.get(g[1:], [])])):
        groups = collections.Counter()
        for gid, tag in values:
            if gid[0] not in ALPHABET[:4]:
                continue
            value = bytearray(expand(gid))
            value[12:16] = struct.pack(">I", struct.unpack(">I", value[12:16])[0] ^ tag)
            groups[bytes(value[:12])] += 1
        print(f"{label}: {len(values)} in {len(groups)} episode prefixes, largest {max(groups.values(), default=0)}")

    # Every GlobalId whose bytes 1 to 11 are a history episode's: what its
    # first byte and its last 32 bits hold against the episode's.
    print("GlobalIds sharing bytes 1-11 with a history episode:")
    shared = collections.Counter()
    for root in model.by_type("IfcRoot"):
        value = expand(root.GlobalId) if root.GlobalId[0] in ALPHABET[:4] else None
        hit = middles.get(value[1:12]) if value else None
        if hit is None:
            continue
        episode = stored_order(runs[hit[0]][hit[1]])
        low = struct.unpack(">I", value[12:16])[0] ^ struct.unpack(">I", episode[12:16])[0]
        shared[(root.is_a(), hit, value[0] ^ episode[0])] += 1
        if args.list:
            print(f"  {root.is_a()} {root.GlobalId} Tag {getattr(root, 'Tag', None)} id XORed in {low}")
    for (cls, hit, top), n in sorted(shared.items()):
        print(f"  {cls}: {n}, episode {hit[1]} of the run at {hit[0]}, first byte XOR {top:#04x}")

    # The project, site, building and storeys have no Tag: is a stored string
    # their GlobalId, whole or on its last 21 characters?
    spatial = ("IfcProject", "IfcSite", "IfcBuilding", "IfcBuildingStorey")
    whole, tail = collections.Counter(), collections.Counter()
    for root in model.by_type("IfcRoot"):
        if root.is_a() in spatial:
            whole[(root.is_a(), root.GlobalId in stored)] += 1
            tail[(root.is_a(), bool(by_suffix.get(root.GlobalId[1:])))] += 1
    for cls in spatial:
        n = whole[(cls, True)] + whole[(cls, False)]
        print(f"{cls}: {n}, GlobalId a stored string {whole[(cls, True)]}, its last 21 characters {tail[(cls, True)]}")

    # The first character carries the top two bits, so a GUID encoder writes
    # 0 to 3 there.
    print("first character of every GlobalId, by class:")
    by_class = collections.defaultdict(collections.Counter)
    for root in model.by_type("IfcRoot"):
        by_class[root.is_a()][root.GlobalId[0]] += 1
    for cls, chars in sorted(by_class.items(), key=lambda kv: -sum(kv[1].values())):
        print(f"  {cls} {sum(chars.values())}: " + ", ".join(f"{c} {n}" for c, n in sorted(chars.items())))


if __name__ == "__main__":
    main()
