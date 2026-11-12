- **A crafted file can no longer panic a reader through an overflow in
  arithmetic on a value it stores (#1831).** An element's creation episode at
  `u32::MAX`, or two wall shell-layer counts whose sum passes the address
  limit, panicked the 32-bit and wasm32 builds the viewer runs. Extreme stair
  riser indices, an increment's episode at `i64::MAX`, an electrical circuit
  list's entry count and a very large `TransmissionData` stream overflowed
  where overflow checks are on. Each sum is now checked, and a value that
  overflows is refused. No real file of the measured corpus changes.
