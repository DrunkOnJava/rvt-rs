- **A curved wall's chords stay within 0.0005 ft of its arc whatever its
  radius (#1947).** Each arc of a curved wall's outline was capped at 512
  chords, so a radius past about 100 ft over a half turn gave chords straying
  further inside the arc than the 0.0005 ft its doc and RE-75 promise
  (0.0047 ft at 1,000 ft). The cap is now 8,192 chords, which holds the
  tolerance up to a radius of about 27,000 ft. An arc that would need more is
  not drawn as an arc, and the wall keeps the body it would otherwise have.
  Found by the batch 9 export audit.
