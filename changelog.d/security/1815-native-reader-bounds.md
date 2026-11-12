- **A crafted file can no longer crash the browser viewer through a native
  record's length, or make a curved face's trim take unbounded work (#1815).**
  A record group whose length field put its end within 4 bytes of the address
  limit panicked the 32-bit and wasm32 builds the viewer runs. The group is now
  refused, and the rest of the file reads on. A cylinder face's trim ring could
  take 100,000 points from every edge it shares with a plane. It is now held to
  20,000 in all, and an element with a face over that is drawn without its
  saved mesh. The cuts of a trimmed parametric face, which could ask about
  4e10 comparisons of a hostile trim, are held to 250,000,000. No real file of
  the measured corpus reaches either bound. The walker's two-column container
  read is checked as its trial walk already was, though no file reaches it
  through the public reader.
