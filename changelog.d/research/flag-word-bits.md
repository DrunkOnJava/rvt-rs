- **Three bits of the element record's flag word (RE-261, #223).** Checked
  against Revit's exports, bit 12 of `+0x46` is exactly a room Revit exports
  as a space (302 of 302 rooms on Core Interior, RE1 and the R2025 models)
  and a wall with an end joined (8 of 8 on RE1, the only export that writes
  wall joins), and bit 7 is exactly a type record. These confirm three rows
  of puzzbobb's bit table; the rest need member values or one-change saves.
  Nothing reads the word yet.
