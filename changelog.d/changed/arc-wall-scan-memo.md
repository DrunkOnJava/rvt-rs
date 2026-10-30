- **A Revit 2023 file's partition ArcWalls are scanned once per export, not up
  to four times.** The production walk, the partition recovery and the IFC
  export each ran the same scan over every `Partitions` stream. The file now
  keeps the scan for its release and limits. The exports are unchanged.
