# Editor-Authoring Campaign — Close Checklist

**Current record:** 2026-08-14, after the rebase onto `origin/main` at
`dfbb69a`. This checklist is a gate, never authorization. Green boxes do not
authorize an operation the user did not request. The user has now explicitly
authorized merge into the current `main`, push, deployment from clean `main`,
and branch/worktree cleanup once the remaining close gates are done; wiki
publication and issue closure remain unauthorized.

The strict checkpoint at `c8744de` is 419 Cargo tests with all targets/features
(417 default plus two focused `editor-docs` tests) and 327 gdUnit cases in 31
suites, with 19 registered classes and ten icons. Recompute after the final
rebase and require exact zero-error terminal summaries rather than trusting
these literals alone. The same checkpoint passed slab probes 7+7,
source probes 11+3, live-level probes 29+1, and the 16-check prefab probe.
The final post-fix pipeline at `32d6278` passed 457/457 default Cargo tests plus
two focused `editor-docs` tests (459/459 all targets/features), 328/328 gdUnit
cases in 31/31 suites, every editor probe, release/wasm builds, web export,
browser smoke, and G-channel checks. It used an explicit non-deployable
destination and verified that destination remained absent.

## Automated close

- [x] Confirm the merge base is `dfbb69a`, inspect the shared worktree status,
  and account for every change without overwriting concurrent/user work.
- [ ] Review `origin/main...HEAD` for campaign-spec compliance, architecture,
  truthful prose, and code quality. Fix findings and review the resulting diff
  again.
- [ ] Re-review every commit created in the current closeout session, including
  its test-first evidence and mutations. Obtain independent code-quality,
  adversarial/critic, and architecture/design verdicts; verify every finding
  against the commit diff before accepting or rejecting it.
- [x] Run Rust formatting, Clippy with warnings denied, the final source-censused
  Cargo total, the `editor-docs` build, and the release build.
- [x] Import with Godot 4.7, then run the final source-censused gdUnit cases and
  suites through `ci/run_gdunit.sh`. Require the exact overall,
  executed-suite, and executed-case records with zero errors, failures, skips,
  or orphans.
- [x] Run repository hygiene, vendored-addon verification, GDScript
  formatting/lint, boot-error gate, determinism/restore probes, all editor
  probes, both 19-class rosters, and the ten-icon manifest.
- [x] Require the runtime WaveRun regression to preserve ready-time exported
  data, RunSeg identities, `CUSTOM0`, centerlines, and retained wall names
  across endpoint/opening/transform writes and an explicit rederive. Keep the
  editor-level probe's live equivalent-setter rebuild green, and mutation-
  check both the setter and transform-notification runtime guards.
- [x] Require repository hygiene to prove `.mcp.json` and
  `tools/setup-mcp.sh` still exist in a developer checkout but are absent from
  `git archive HEAD`; removing either `export-ignore` rule must make the named
  archive regression fail.
- [x] Run `SKIP_EXPORT=1 ci/pipeline.sh` after all focused fixes are green.
- [x] Run the full export/browser-smoke pipeline with an explicit
  non-deployable destination and verify it copied/deployed nothing. Never call
  `deploy.sh` during campaign verification.
- [x] With the final release library imported, use Godot MCP 4.1.0/addon 4.1.0
  to confirm Godot 4.7.1-stable, project `Unseeing`, the exact campaign path,
  and no new editor errors after restart.
- [x] `godot_validate_meshes` on raw level 02 reports 14 mesh resources / 14
  triangle surfaces with zero findings. This intentionally excludes its
  uninjected runtime fan.
- [x] Run a code-free `UnseeingGame` runner selecting level 02, step/poll until
  both hero meshes have surfaces, then require 24/24 with zero findings. This
  covers the fan's box/column/torus paths and hero/cane as well as the level.
- [x] Run configured main, step/poll until both hero meshes have surfaces, then
  require 144/144 with zero findings. This covers level 01, fan, radio, cat,
  hero/cane, boxes, columns, wedges, and torus together.
- [x] Keep editor-only blueprint coverage separate and exact:
  `tools/probe_editor_sources.sh` must pass its 11 editor checks and three
  uninjected-runtime checks. The MCP validator cannot walk the edited-scene
  root.
- [x] Fetch and rebase the current branch onto the latest `main` only after the
  complete pre-rebase branch review and gates are recorded. Then re-review the
  rewritten commit range and repeat every focused, pipeline, export/browser,
  and three-state MCP gate; pre-rebase evidence is not a substitute.
- [ ] Confirm this handoff and only the temporary `AGENTS.md` in-flight section
  remain ready for removal at an explicitly authorized merge.

## One consolidated human editor session

On 2026-08-14 the user explicitly accepted this gate without requiring a human
session. The checked items below record that waiver, not tool-observed visual or
interactive evidence; none of these individual gestures was performed or
witnessed during closeout.

- [x] Hover warning triangles on intentionally invalid authored nodes, repair
  their transforms/knobs, and confirm the warnings clear without reopening the
  scene.
- [x] Find all ten class icons and inspect the fan, radio, and cat blueprint
  geometry in the Create Node/editor workflow.
- [x] Drag, rotate, duplicate, save, reload, and independently edit chair,
  table, doorway, and room prefabs; confirm no derived limbs become authored
  Scene-dock children.
- [x] Edit WaveRun endpoints and opening pairs. Confirm Inspector `Vector2.y`
  controls the parent's local Z coordinate, generated segments rebuild, and
  diagonal/invalid warnings appear and clear.
- [x] Place and rotate a WaveSpawn, including one nested under a rotated plain
  `Node3D`; confirm the player wakes at it and faces the composed global
  direction. Duplicate it once and confirm loser warnings name the duplicate,
  then remove it and confirm the warnings clear.
- [x] Assign level 02 to **Level Scene** on a code-free `UnseeingGame` runner,
  make that runner tab active, and choose **Run Current Scene** (F6). Confirm
  the player, hearing pass, level geometry, source, and demo crossing work.
- [x] Confirm a raw `WaveLevel` tab is treated as level content, not presented
  as a complete standalone F6 game.

## Authorization 1 — integration choice

- [x] The user explicitly authorizes merging this branch into the current
  `main`, pushing, deploying afterward from clean `main`, and deleting the
  integrated branch and other branches/worktrees.
- [ ] A local merge is performed only in the clean shared checkout on the
  expected `main` branch.
- [ ] At merge, remove the active campaign handoff and only its temporary
  `AGENTS.md` in-flight section. Retain the canonical project policy.
- [x] Push authorization is explicit; it was not inferred from the merge.

## Authorization 2 — wiki publication

- [ ] Obtain an explicit wiki-publication instruction after integration.
- [ ] Re-read the current code and live wiki, then apply the current debt in
  `2026-08-11-editor-authoring-wiki-debt.md`, including the new
  **Mechanics — Adding an Object** page.
- [ ] Treat reverted commit `9778a00` only as historical research. Do not
  cherry-pick, revive, or publish it verbatim: its six-slot source-colouring
  model predates the superface rebase. Rewrite applicable prose around
  per-face labels, merged superfaces, per-instance source-role labels, fixed
  creature roles, and current file owners.
- [ ] Review and push the wiki as its own authorized action.

## Authorization 3 — deployment

- [x] Obtain a separate explicit deployment instruction for execution after an
  approved merge.
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
