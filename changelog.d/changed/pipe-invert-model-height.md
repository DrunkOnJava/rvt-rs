- **A pipe's `InvertElevation` is the height above the model's zero, and a
  Revit 2024 vertical pipe has one (W7, RE-157).** Revit's export writes the
  lower end's height in the model's frame, not above the pipe's storey: the
  BIM4LCA office's 12 vertical pipes stand on a storey 55.5 m up and carry
  55.32 m. RE1's pipes are all on the storey at zero, which hid the
  difference. rvt-rs no longer takes the storey's elevation off, and a
  vertical pipe's invert, its lower end, no longer waits for an inner
  diameter that Revit 2024 pipes do not have read, so the office's 12 pipes
  as built and at building permit have Revit's value where they had none.
