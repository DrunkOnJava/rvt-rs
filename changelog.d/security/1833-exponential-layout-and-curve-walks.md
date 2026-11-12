- **A crafted file can no longer make a native read or the saved-graphics
  pass take exponential time (#1833).** The layout of an element index row
  was bounded only by depth, so a schema whose classes each name the next
  twice took 2^127 steps. It is now held to 10,000 class visits and fields,
  where a real row has at most 40 leaves. The check that a graphics branch
  holds only curves walked a child shared at every level once per path. It is
  now held to 100,000 calls, as the graphics walk itself is, and a branch over
  that is walked, or refused, as one that holds more than curves. No real file
  of the measured corpus changes.
