- **Two bits of the element record's flag word (RE-261, #223).** Checked
  against Revit's exports, bit 12 of `+0x46` is exactly a room Revit exports
  as a space (302 of 302 rooms on Core Interior, RE1 and the R2025 models)
  and a wall with an end joined (8 of 8 on RE1, the only export that writes
  wall joins). That confirms two rows of puzzbobb's bit table. Bit 7 reads as
  a type record, as RE-128 found; their `WallSweep` row for it is not
  testable on these models, and the rest of the table needs member values or
  one-change saves. Nothing reads the word yet.
