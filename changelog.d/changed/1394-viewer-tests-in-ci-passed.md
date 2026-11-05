- **The viewer's browser tests are part of `CI passed`, so a pull request
  cannot merge before they have passed on its head (#1394, #1374).** They ran
  in Deploy viewer's pull-request run, which `CI passed` did not wait for:
  #1370 merged before that run had finished and Deploy viewer went red on
  main. They are now `viewer-test.yml`, called by `ci.yml` as its `viewer`
  job in the full set (a ready pull request, or a push to main), in three
  parallel groups of spec files, on a WASM built without LTO (55 s warm
  against 150 to 225 s). A change that touches only documentation skips
  them, and a skipped job passes. The saved-mesh measurement, which took 12
  minutes of every pull request's run, moves to Deploy viewer (pushes to
  main and dispatch); its one assertion that gates a merge, the toggle on
  Core Interior, stays in the suite. Deploy viewer has no `pull_request`
  trigger; on a push to main it runs the suite and the measurement again on
  the release WASM it publishes, and its deploy job waits for those and for
  `CI passed`. To check a draft's viewer, dispatch `viewer-test.yml` on its
  branch.
