- **A body whose shared symbol's graphics are read but not decoded says why
  (#2085).** Its saved-mesh refusal named such a symbol `missing current
  symbol graphics`, as if the file did not hold it; it now names it `not
  decoded` and gives the reader's own reason, such as the graph value budget,
  so a symbol the file lacks and one it holds but rvt-rs refused read apart.
