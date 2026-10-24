- **Changelog entries are fragments under `changelog.d/` (#938).** Each
  pull request writes its entry as `changelog.d/<section>/<name>.md`, and a
  release step (`tools/release/changelog.py assemble`) moves them into this
  file. Every pull request used to add its entry at the top of the same
  section here, so each one that landed left the others in conflict with
  `main`. CI checks that every fragment is in a section and starts an item.
