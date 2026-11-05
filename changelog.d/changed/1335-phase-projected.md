- **The phase read no longer builds the values of the room fields it ignores
  (#1335).** The read of the project's phases, and of the phase each room
  names, decoded every `RoomElem` graph in full to read three fields of it. It
  reads and checks the same bytes, with the same refusals, and builds only
  those fields. No export changes. The figures are in the pull request.
