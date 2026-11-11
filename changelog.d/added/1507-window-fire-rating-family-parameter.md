- **A window type's `FireRating` is read from the family parameter named
  `Fire Rating` where it holds none under the built-in -1001206 (#1507).**
  The R2025 models' window types hold their `0.5h` so, under two definitions
  of that name (RE-185 section 3.3, RE-207); the ids are found by the name,
  and the entries keep RE-354's attribution rule and its agreeing-copies
  guard. No value changes yet on the models measured: the definitions are
  not found there, and
  `probes/examples/probe_1507_window_fire_rating_definitions.rs` prints
  which step loses them.
