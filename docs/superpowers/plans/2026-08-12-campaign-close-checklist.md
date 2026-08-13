# Editor-Authoring Campaign — Close Checklist

**Current record:** 2026-08-13, after the rebase onto `origin/main` at
`dfbb69a`. This checklist is a gate, never authorization. Green boxes do not
authorize integration, a push, wiki publication, deployment, or issue closure.

The settled source census is 406 Cargo tests and 326 gdUnit cases in 31 suites,
with 19 registered classes and ten icons. The final full runner must still
print the exact 326/326 and 31/31 zero-error terminal summaries.

## Automated close

- [ ] Confirm the merge base is `dfbb69a`, inspect the shared worktree status,
  and account for every change without overwriting concurrent/user work.
- [ ] Review `origin/main...HEAD` for campaign-spec compliance, architecture,
  truthful prose, and code quality. Fix findings and review the resulting diff
  again.
- [ ] Run Rust formatting, Clippy with warnings denied, the final source-censused
  Cargo total, the `editor-docs` build, and the release build.
- [ ] Import with Godot 4.7, then run the final source-censused gdUnit cases and
  suites through `ci/run_gdunit.sh`. Require the exact overall,
  executed-suite, and executed-case records with zero errors, failures, skips,
  or orphans.
- [ ] Run repository hygiene, vendored-addon verification, GDScript
  formatting/lint, boot-error gate, determinism/restore probes, all editor
  probes, both 19-class rosters, and the ten-icon manifest.
- [ ] Run `SKIP_EXPORT=1 ci/pipeline.sh` after all focused fixes are green.
- [ ] Run the full export/browser-smoke pipeline with an explicit
  non-deployable destination and verify it copied/deployed nothing. Never call
  `deploy.sh` during campaign verification.
- [x] With the final release library imported, Godot MCP 4.1.0/addon 4.1.0
  confirmed Godot 4.7.1-stable, project `Unseeing`, the exact campaign path,
  and no editor errors after restart.
- [x] `godot_validate_meshes` on raw level 02 checked 14 meshes/surfaces with
  zero findings.
- [x] `godot_validate_meshes` on raw level 01 checked 127 meshes/surfaces with
  zero findings, covering wedges, columns, both source blueprints, and the cat
  blueprint.
- [x] `godot_validate_meshes` on running main checked 144 mesh instances / 142
  surfaces with zero findings, covering the runtime hero body/cane and rebuilt
  geometry.
- [ ] Confirm this handoff and only the temporary `AGENTS.md` in-flight section
  remain ready for removal at an explicitly authorized merge.

## One consolidated human editor session

Ask for this only after the automated and MCP passes have exhausted what can be
proved without the user.

- [ ] Hover warning triangles on intentionally invalid authored nodes, repair
  their transforms/knobs, and confirm the warnings clear without reopening the
  scene.
- [ ] Find all ten class icons and inspect the fan, radio, and cat blueprint
  geometry in the Create Node/editor workflow.
- [ ] Drag, rotate, duplicate, save, reload, and independently edit chair,
  table, doorway, and room prefabs; confirm no derived limbs become authored
  Scene-dock children.
- [ ] Edit WaveRun endpoints and opening pairs. Confirm Inspector `Vector2.y`
  controls the parent's local Z coordinate, generated segments rebuild, and
  diagonal/invalid warnings appear and clear.
- [ ] Place and rotate a WaveSpawn, including one nested under a rotated plain
  `Node3D`; confirm the player wakes at it and faces the composed global
  direction. Duplicate it once and confirm loser warnings name the duplicate,
  then remove it and confirm the warnings clear.
- [ ] Assign level 02 to **Level Scene** on a code-free `UnseeingGame` runner,
  make that runner tab active, and choose **Run Current Scene** (F6). Confirm
  the player, hearing pass, level geometry, source, and demo crossing work.
- [ ] Confirm a raw `WaveLevel` tab is treated as level content, not presented
  as a complete standalone F6 game.

## Authorization 1 — integration choice

- [ ] The user chooses exactly one: merge locally, push/open a PR, or keep the
  local branch. No choice may be inferred from completing this checklist.
- [ ] A local merge is performed only in the clean shared checkout on the
  expected `main` branch.
- [ ] At merge, remove the active campaign handoff and only its temporary
  `AGENTS.md` in-flight section. Retain the canonical project policy.
- [ ] A merge choice authorizes only that merge. Pushing a branch/PR requires
  the push/PR choice explicitly.

## Authorization 2 — wiki publication

- [ ] Obtain an explicit wiki-publication instruction after integration.
- [ ] Re-read the current code and live wiki, then apply the current debt in
  `2026-08-11-editor-authoring-wiki-debt.md`, including the new
  **Mechanics — Adding an Object** page.
- [ ] Treat reverted commit `9778a00` only as historical research. Do not
  cherry-pick, revive, or publish it verbatim: its six-slot source-colouring
  model predates the superface rebase. Rewrite applicable prose around
  per-face labels, merged superfaces, fixed source/creature roles, and current
  file owners.
- [ ] Review and push the wiki as its own authorized action.

## Authorization 3 — deployment

- [ ] Obtain a separate explicit deployment instruction after an approved
  merge.
- [ ] Deploy only from a clean shared `main` whose native/wasm cores were built
  from that exact tree, using the repository's gated deployment workflow.
- [ ] Do not infer deployment authority from merge, PR, or wiki authority.

## Authorization 4 — issue closure

- [ ] Obtain a separate explicit issue-closure instruction.
- [ ] Attach current evidence to each applicable issue before closing: #16,
  #22, #30, #31, #32, #33, #34, #35, #36, scoped #38, #39, #41, #42,
  #44, and #45.
- [ ] Do not infer issue-closure authority from merge, wiki, or deployment
  authority.
