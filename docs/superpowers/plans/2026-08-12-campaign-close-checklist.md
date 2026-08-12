# Editor Authoring Campaign — Close Checklist

This is a gate, not authorization. Do not merge, push rewritten history,
publish the wiki, deploy, or close issues merely because every box below is
green.

## Automated close

- [ ] Review `origin/main...HEAD` for specification coverage and code quality.
- [ ] Run formatting, linting, all 339 Cargo tests, all 289 gdUnit cases in all
  31 suites, editor probes, 19-class census, and ten-icon manifest.
- [ ] Run the full export/browser smoke pipeline with an explicitly
  non-deployable destination. Never invoke deployment tooling.
- [ ] Confirm the handoff and its temporary `AGENTS.md` section remain until
  integration.

## One consolidated human editor session

- [ ] Hover warning triangles, create and clear a bad placement, duplicate
  `WaveSpawn`, then clear its loser warning.
- [ ] Find the fan/radio/cat blueprints and all ten authoring icons.
- [ ] Create the seventh mutually touching object and observe the source's
  starvation warning.
- [ ] Drag, rotate, and save/reload chair, table, doorway, and room prefabs.
- [ ] Edit WaveRun endpoints and opening pairs; verify generated wall pieces and
  warnings update and clear.
- [ ] Place and rotate a nested WaveSpawn; confirm the player faces its global
  direction.
- [ ] Assign level 02 to `UnseeingGame.level_scene`, run main, then open level 02
  and use **Run Current Scene** (F6).

## Integration choice — user decides

- [ ] Choose exactly one: merge locally, push/open a PR, or retain the local
  branch. A merge choice authorizes only the merge.
- [ ] At merge, remove this campaign handoff and only the temporary in-flight
  section of `AGENTS.md`; retain the canonical policy.

## Separate post-merge authorizations

- [ ] Wiki publication: revive reverted wiki commit `9778a00`, add the
  Mechanics — Adding an Object page, and apply all four campaign debt sections.
- [ ] Deployment: only from a clean shared `main`, only after an explicit user
  instruction, using the repository's gated deployment workflow.
- [ ] Issue closure with evidence links: #16, #22, #30, #31, #32, #33, #34,
  #35, #36, scoped #38, #39, #41, #42, #44, #45.
