- **An instance whose data copies disagree takes its current copy's
  transform (RE-216, #227).** A family instance can have its element data
  in more than one partition. Where the copies' transforms differed, the
  instance had no transform at all, so its turned box, facing, opening and
  ports fell back to what needs none. It now takes the copy in the partition
  its current record is in, as the native reader picks records (RE-145). On
  Core Interior 8 stacked doors had two copies an inch apart across their
  host wall; the current one places Revit's opening exactly, and all 132
  doors' openings are now within 0.001 ft of Revit's, where 8 were within
  0.26 ft. On the BIM4LCA office at building permit 10 more door types take
  Revit's GlobalId, since their flip comes from the transform. No other
  measured model's export changes.
