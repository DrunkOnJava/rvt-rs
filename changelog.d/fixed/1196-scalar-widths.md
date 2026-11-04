- **`Formats/Latest`: a `char` field is 1 byte and a `short` field 2, not 2
  and 4 (#1196, RE-325).** `FieldType::decode` gave the descriptors with base
  byte 2 a size of 2 (a `u16`) and base byte 3 a size of 4 (a "deprecated
  i32 alias seen in 2016 to 2018 only"); the native graph decoder had read
  them as 1 and 2 bytes. The schema names the types (every `std::pair<
  char, K >` slot carries base 2 and every `std::pair< short, K >` slot base
  3, in all 13 family files, 2014 and 2016 to 2027), and the instances settle
  the width: the 58 Autodesk sample projects of Revit 2015 to 2027 hold
  402,652 native records with an instance of such a field, 402,293 of them
  decodable without the extensible-storage catalog (the other 359 are
  Revit 2024 records that need it), and all 402,293 are read to their last
  byte at 1 and 2 bytes (683,250 instances of 20 fields). `FieldType`'s
  widths read 65 of them, those whose only instances are empty byte buffers,
  and each field rewritten alone to another width leaves none of its
  non-empty records whole. The byte buffers are files that frame their own
  length: 13,407 PNGs end with their IEND chunk at exactly the declared
  count, and 8,157 RTF streams open with `{\rt`. Base 3 is not confined to
  2016 to 2018: the schema has 24 to 27 `short` fields and three `short[3]`
  arrays in every release 2014 to 2027. `walker::read_field_by_type` and
  `write_field_by_type` read and write a `0x02` primitive as a `u8` and a
  `0x03` primitive as an `i16`, and a `0x02` vector or container (a byte
  blob, which the walker could only skip the count of) as its whole bytes.
  Report: `reports/element-framing/RE-325-scalar-widths.md`.
