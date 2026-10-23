# RE-208: Revit 2015's element GlobalIds are not in the file

**Date:** 2026-10-23
**Issues:** #878 (Revit's GlobalIds on 2015); follows RE-48 (episode GUID XOR ElementId), RE-108 (the episode number in 28-byte ElemTable records) and RE-198 (the BIMcollab 2015 pair)
**Artefacts:** `02_BIMcollab_Example_STR.rvt` (Revit 2015, build 20141119_0715(x64), 13,979,648 bytes, sha256 `8cd1f122e751c33ea69e6f16c23ed4cd572ded3d0d04dfab50aaf0973e38e193`) and Revit 2015's export of it, `02_BIMcollab_Example_STR_optimized.ifc` (455,050 bytes, sha256 `1d0d047a5519a2ea143b9d07625dcc94bd3a1c3b01cb86607b16d850f0a5081d`), both as RE-198 describes them: no licence stated, local only, fetched by Measure with `-f bimcollab=true`. Controls: `2024_Core_Interior.rvt` and the RE1 models (MIT) with Revit's exports of them.
**Run:** Measure [39998800211](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39998800211), probe `examples/probe_re208_episodes_2015.rs` and scorer `tools/re/episodes_vs_ifc.py`, both at `re/878-2015-episodes` 66ac3475. `tools/re/episodes_first_char.py` reads that run's probe output and the same export.
**Result:** negative. Revit 2015 did not make this export's element GlobalIds from the episode history, so RE-48's rule cannot rebuild them. A parameter stored in the file holds 126 of each GlobalId's 128 bits, for 229 of the 241 elements. The other 2 bits are in no field tested here. The project, site, building and five storeys are the exception: their GlobalIds are stored in the file exactly.

## 1. The history parses, and no element GlobalId comes from it

On the 2015 file, `Global/History` parses with RE-48's layout: 291 episodes in one run. `Global/ElemTable` has 21,814 records of 28 bytes. Revit's export has 241 elements with a Tag (openings left out: they carry their host's Tag). XOR the Tag out of each GlobalId's last 32 bits, and RE-48 says what is left is the GUID of the episode that made the element.

| | BIMcollab 2015 | Core Interior 2024 | RE1 Architecture 2025 |
|---|---:|---:|---:|
| element GlobalIds | 241 | 854 | 73 |
| distinct episode prefixes (first 12 bytes, Tag XORed out) | **241** | 5 | 4 |
| largest group | **1** | 460 | 39 |
| prefixes that are a history episode | **0** | all | all |

Elements made in one episode share their first 12 bytes, so on both controls they fall into a handful of groups. On the 2015 export, every element has a prefix of its own and none is a history episode. That holds whatever the episode number's offset in the record, so it is not a framing problem: these GlobalIds were not made from episodes.

## 2. Types and some openings do follow an episode

Twenty-one GlobalIds share bytes 1 to 11 with a history episode, all with episode 26:

| | count | first byte against the episode's | low 32 bits |
|---|---:|---|---|
| `IfcWallType` | 6 | equal | XOR the type's ElementId |
| `IfcColumnType` | 5 | equal | XOR an id next to the type's |
| `IfcOpeningElement` | 10 | XOR `0xc0`: the top two bits changed from 01 to 10 | XOR the opening's own ElementId (407624, 407626, …) |

The types follow RE-48, which matches RE-198's 113 of 129 type GlobalIds. The 10 openings are cut by slabs 407614, 407672 and 407718. The other 41 openings share bytes with no episode, and none with its host's GlobalId (the host and opening values XOR to no fixed pattern).

## 3. A stored IfcGUID holds all but the first character

The file holds 320 strings of 22 characters in IFC's GlobalId alphabet, with a first character from 0 to 3. They are UTF-16, in `Partitions/25`. Each one that matches is preceded by the same 8 bytes, `8873f0ff 16000000`: the parameter id -1019000 and the string's length, 22. That is where Revit keeps an element's IfcGUID parameter.

- **229 of the 241 elements:** a stored string has the last 21 characters of the element's GlobalId. Its first character is always different.
- **Project, site, building and the five storeys:** all 8 GlobalIds are stored strings, character for character. rvt-rs writes 0 of the 5 storeys' GlobalIds today (`global_ids_vs_ifc`, RE-198).

The first character of a GlobalId carries the top two bits of its 128. The table below compares the stored first character with Revit's, for the 229 elements:

| stored → Revit's | 1 | 2 | 3 |
|---|---:|---:|---:|
| 0 | 24 | 27 | 8 |
| 1 | | 35 | 23 |
| 2 | 33 | | 26 |
| 3 | 16 | 37 | |

Revit's first character is never 0, and never the stored one. The stored first characters are spread over 0 to 3, like the relationship entities' (`IfcRelDefinesByProperties`: 130, 108, 123 and 125). So once the stored character is known, Revit's still has two or three possible values: 1.075 bits of uncertainty. `tools/re/episodes_first_char.py` measures how much of that each candidate removes:

| candidate | values | bits left |
|---|---:|---:|
| the stored first character alone | 4 | 1.075 |
| plus the best one of the 126 shared bits (bit 16) | 8 | 1.025 |
| plus the Tag mod 4 | 16 | 1.032 |
| plus the entity class | 20 | 0.977 |
| plus the 24 bytes before the stored string | 24 | 0.949 |

None of them determines it. The 24 bytes before the string group the elements by category layout. Within the largest group, 91 beams, Revit's first character is 1 for 36, 2 for 33 and 3 for 22.

Three openings start with 4, 5 and 6. An encoding of a 128-bit GUID writes 0 to 3 there, and revit-ifc's `GUIDUtil.ConvertToIFCGuid` takes that character from two bits only. So those three values come from something that changed the top character after encoding.

## 4. What this measures, and what it does not

This is one export, written on 2014-10-08 by Revit Architecture 2015. Solibri's IFC Optimizer later rewrote it (its `FILE_NAME`). The file was saved later, by the November 2014 build. RE-198 says the optimizer kept the GlobalIds, but no copy from before the optimizer exists to check that. So this report cannot say whether Revit's exporter or the optimizer changed the top two bits. Either way, they cannot be read from the `.rvt`.

For the 1.0 criteria (W5, identity), this file stays a failure that no reader change can fix. The 2 bits are not in the file, so even the stored parameter's 126 give no exact GlobalId. A second Revit 2015 project, with an unprocessed Revit 2015 export, is needed to tell the exporter's rule from the optimizer's.

What the file does give:

- **The storeys' GlobalIds:** the stored IfcGUID of each Level is its storey's GlobalId, 5 of 5. Reading it needs the parameter reader (#887) to reach 2015 records. The next step is measured there.
- **The types:** the types keep RE-48's rule on 2015.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref re/878-2015-episodes -f ref=re/878-2015-episodes -f base=none \
  -f bimcollab=true -f probe=probe_re208_episodes_2015 -f probe_scorer=episodes_vs_ifc
gh run download <run id>   # measure-head/<model>/probe_score.txt
python3 tools/re/episodes_first_char.py measure-head/bimcollab-str-2015/probe.txt 02_BIMcollab_Example_STR_optimized.ifc
```
