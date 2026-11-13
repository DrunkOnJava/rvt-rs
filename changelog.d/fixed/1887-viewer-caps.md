- **The viewer bounds what a large or crafted file can make the page hold
  (#1887).** A second reader of `viewer/src/main.ts` found no injection and no
  network path for file data; what it found was growth a file controls, and
  each part is now bounded. The scene tree builds each group's rows (a
  storey's categories, a category's elements, an element's parts) 500 at a
  time: a model of up to 150,000 rows, which holds the largest Measure has
  (the BIM4LCA office, 91,723 entities), is built whole when it opens as it was
  before, and past that a group builds its first block when it is opened and a
  "Show more" row at its end builds the next. The filter looks at every element
  and lists the first 2,000 matches, and says when there are more; a click on
  an element in the 3-D view builds the row it needs. The phase field lists at
  most 200 phases (the first 199 and the last, the default) and says how many
  the file names; the page keeps at most 2,000 of the decoder's progress
  reports and of what the loading card said; and the saved parameters it
  remembers are bounded to 8 Mi characters of JSON as well as 64 elements, the
  answer just read always staying. The address (`?parametersRemembered=`) could
  set the cache's size without a bound; it can now only lower it. No export
  moves.
