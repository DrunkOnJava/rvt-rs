# Container wire format across kinds (L5B-09.3)

Date: 2026-04-21
Corpus: 11 family RFAs (2016-2026) + 2 project RVTs (2023, 2024)
Probes: `examples/probe_container_kinds.rs` + `examples/probe_container_bodies.rs`

## Summary

`FieldType::Container { kind, cpp_signature, body }` currently over-reads
the `body` field for scalar-base Container kinds (0x01 / 0x02 / 0x04 /
0x05 / 0x07 / 0x0d). The real wire format is a **4-byte header only**
— no payload follows. The existing decoder sees ~28 bytes of the
*next* field's header accidentally bundled as body, because
`FieldType::decode` is called on a 32-byte window and greedily takes
`bytes[4..]` as `body_after_header`.

## Per-kind wire format (verified)

| kind | sub   | header        | body after header | semantic |
|------|-------|---------------|-------------------|----------|
| 0x01 | 0x0050| `01 50 00 00` | empty             | bool container |
| 0x02 | 0x0050| `02 50 00 00` | empty             | u16 container |
| 0x04 | 0x0050| `04 50 00 00` | empty             | legacy-u32 container |
| 0x05 | 0x0050| `05 50 00 00` | empty             | u32 container |
| 0x07 | 0x0050| `07 50 00 00` | empty             | f64 container |
| 0x0d | 0x0050| `0d 50 00 00` | empty             | point/transform container |
| 0x0e | 0x0050| `0e 50 00 00` | optional ASCII C++ signature + padding | reference container |

The 0x0e path is the only one that carries variable payload. All
other kinds end at byte offset 4 relative to the type-encoding
block.

## Evidence

Hex dumps of `Container.body` for each non-0x0e kind, harvested from
the first occurrence in schema-parse order:

```
kind 0x01 RebarSystemLayer.m_includedUncutBars (body=28 B)
  00 00 00 00 0e 00 00 00 "m_marginBottom" 07 00 00 00 0b 00
                          ^ next field: type 0x07 (f64), ...

kind 0x02 ControlledDocAccess.m_VSTAMacroProjZip (body=28 B)
  0e 00 00 00 "m_ElemProjDate" 04 00 00 00 12 00 00 00 "m_..."
              ^ next field name, 14 chars long

kind 0x04 RbsSystemNavigatorColumn.m_rgColWidths (body=28 B)
  11 00 00 00 "m_rgZoneColWidths" 04 50 00 00 0f 00 00
              ^ next field name, 17 chars

kind 0x05 AnalyticalOctreeCells.m_cells (body=28 B)
  0a 00 00 00 "m_oOutline" 0e 01 00 00 00 00 00 00 00 00
              ^ next field: 0x0e Pointer kind=1

kind 0x07 RbsOffsetSetWrapper.m_setOffset (body=28 B)
  11 00 00 00 "m_dSelectedOffset" 07 00 00 00 00 00 00
              ^ next field: f64 primitive

kind 0x0d Dimension.m_refPnts (body=28 B)
  01 00 00 00 20 07 10 00 00 03 00 00 00 11 00 00 00 "m_oldRefSeg..."
  ^ u32 header   ^ next field: 0x07 Vector              ^ next name
```

Every body shows the pattern:
```
<u32: next field's name_len> <ASCII: next field's name> <next field's 4-byte type-encoding header>
```

This is exactly what `parse_schema` reads *after* calling
`FieldType::decode` — proving the 28-byte body is noise, not payload.

## Decode fix (L5B-09.4)

`FieldType::decode`'s scalar-container arm (sub == 0x0050, kind is
scalar-base) should emit:

```rust
FieldType::Container {
    kind,
    cpp_signature: None,
    body: Vec::new(),
}
```

The 0x0e path (already handled by `extract_container`) stays
unchanged.

## Encode round-trip (L5B-09.5)

`FieldType::encode` currently writes `[kind, 0x50, 0x00, 0x00] ++ body`.
After the decode fix, body is empty for non-0x0e kinds, so the
encoded output is just the 4-byte header — byte-identical to the
wire format. Round-trip property `FieldType::decode(&x.encode()) == x`
holds trivially for all scalar-base Container variants after L5B-09.4.

## Frequency (from probe_container_kinds output)

Total Container occurrences across 13 corpus files:

| kind | count |
|------|-------|
| 0x01 | 3     |
| 0x02 | 26    |
| 0x04 | 132   |
| 0x05 | 13    |
| 0x07 | 51    |
| 0x0d | 61    |
| 0x0e | 2308  |

Aggregate non-0x0e impact: **286 fields** across the corpus carry
corrupted body bytes today. Fixing the decode drops schema memory
by ~8 KB per file and unblocks byte-identical round-trip on every
class that uses scalar-base containers.
