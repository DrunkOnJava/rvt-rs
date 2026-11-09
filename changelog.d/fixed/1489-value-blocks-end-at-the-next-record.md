- **A type's value block ends where the next native record starts, so the
  doors of `rac_basic`'s 2027 save have their Width and Height (#1489,
  RE-373).** Three family instances' own Width, Height, Rough Width and Rough
  Height are stored right after a door symbol's record, with no value block
  mark between. The length scans gave them to the symbol, which then held two
  of each and wrote neither, so two doors lost their `Qto_DoorBaseQuantities`
  Width and Height. Each block now also ends at the first native record after
  its mark, from the handle's record index. Since records do not nest, that
  drops only entries outside the owner's record. In an unfiltered Measure run
  (42558908204) 2 of 111 exports move. One is that save, now 418 compared
  values like its 2026 save. The other is the held-out AA house (Revit 26.4,
  IFC4X3, counts only), whose door Width, Height and Area each go from 11 to
  13 of 13 equal. No value goes from equal to different or missing anywhere,
  and export time is 0.98 to 1.01 times as before. The family type table's
  text form (`FireRating`, `Manufacturer`) keeps its own bounds.
