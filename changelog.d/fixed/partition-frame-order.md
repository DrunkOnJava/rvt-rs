- **Frames in more than one partition are ordered by partition number, not
  by stream name, on Revit 2024 and later.** Three places picked an
  element's copy by comparing its stream's name as text:
  - the newest frame of an instance whose copies tie on vertical extent;
  - the first copy of a sketch line;
  - the first copy of a roof's sketch line.
  As text, `Partitions/9` sorts after `Partitions/46`, so a file whose
  copies sit in both one- and two-digit partitions kept the older copy
  as the newest. Revit 2023 and earlier already compared the numbers.
  No Measure model has such copies: all 111 exports are unchanged.
