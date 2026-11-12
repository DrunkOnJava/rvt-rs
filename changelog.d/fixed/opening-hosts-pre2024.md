- **Revit 2015 to 2023 doors and windows are set in the wall their own
  record names (RE-448).** On those releases a door or window was set in
  the nearest wall its record's reference list named, which is not always
  its host; the correction from the element's own record (RE-245) ran on
  2024 to 2027 only and read the host only in 2024's 8-byte form. It now
  runs on 2015 to 2023 too, in either form. Autodesk's `rac_advanced` has
  28 doors and windows and `rac_basic` one moved into the wall the same
  project's 2024 save sets them in, their openings in that wall where the
  2024 save's are.
