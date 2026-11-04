- **The viewer's browser tests are part of `CI passed`, so a pull request
  cannot merge before they have passed on its head (#1394, #1374).** They ran
  in Deploy viewer's pull-request run, which `CI passed` did not wait for:
  #1370 merged before that run had finished and Deploy viewer went red on
  main. They are now `viewer-test.yml`, called by `ci.yml` as its `viewer`
  job in the full set (a ready pull request, or a push to main), in three
  parallel legs: the suite in two shards, and the saved-mesh measurement. A
  change that touches only documentation skips them, and a skipped job
  passes. Deploy viewer builds and publishes the site without its
  `pull_request` trigger and without running the tests itself; its deploy
  job still waits for `CI passed`. To check a draft's viewer, dispatch
  `viewer-test.yml` on its branch.
