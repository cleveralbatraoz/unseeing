# AI-Agent Documentation Source of Truth and Wiki Mirror Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the repository's competing documentation surfaces with one
AI-oriented local authority and publish that authority to the GitHub Wiki with
a deterministic, provenance-checked, one-way mirror.

**Architecture:** Canonical Markdown under `docs/current/` is validated by a
standard-library Python contract layer shared with a pure Wiki renderer. A
POSIX shell publisher owns only the Git boundary and is exercised against local
bare repositories; GitHub Actions publishes only after both required main
checks succeed. All documentation tooling is checkout-only and passes through
an archive-aware CI adapter so it cannot become a game or deployment
dependency.

**Tech Stack:** Markdown, Python 3 standard library, POSIX shell, Git, GitHub
Actions, existing Rust/Godot/gdUnit/browser verification stack.

**Spec:**
`docs/superpowers/specs/2026-08-15-ai-documentation-source-of-truth-design.md`

**Post-integration plan:**
`docs/superpowers/plans/2026-08-15-ai-documentation-issue-migration.md`

## Global Constraints

- No task changes gameplay, physics, sound propagation, rendering, content, or
  a platform implementation. Documentation claims come from current code and
  executable evidence; code wins whenever prose disagrees.
- Preserve the outline-only perception law, the same-facing coplanar
  superface merge law in `rust/src/render/superface.rs`, separate seams for
  touching solids and semantic source roles, `MIN_SEP = 0.08`, the safe label
  band `[0.15, 0.96]`, and the sole standalone-radio `Role::Case = 0.05`
  exception. Documentation tests must not become gameplay mirror-constant
  assertions.
- Preserve the Godot-object/Rust-law split: designer-facing composition is
  Godot scenes and registered Rust tool nodes; pure behavior remains total,
  deterministic, dependency-injected Rust with no mutable global state;
  tracked GDScript remains tests and probes only.
- Preserve one Godot 4.7 project at `game/`, one Rust behavior source for
  native and wasm, and x86_64/arm64/wasm32 support. Introduce no new language,
  runtime, package, action, or long-lived credential.
- `AGENTS.md` owns project policy, `docs/current/` owns shipped mechanics and
  current engineering procedure, GitHub Issues own residual work, and frozen
  specs/plans own decisions and execution history. Current docs contain no
  roadmaps, task boxes, volatile test totals, or disguised backlog prose.
- The film-grain, vignette, and filled-void conflict with the outline-only
  policy is explicitly deferred. Do not change behavior, document a
  reconciliation, or create an issue for it; name the deferral in the final
  user report.
- Do not modify `tools/superpowers`, its gitlink, or any upstream submodule
  file. `docs/superpowers/README.md` and these plans belong to the parent
  repository.
- All code/configuration behavior slices follow strict red-green-refactor TDD.
  Every task receives a fresh requirements review and code-quality review.
  Mutation checks target realistic branches and side effects rather than
  duplicated gameplay constants.
- Every commit is small and green, authored as
  `Dmitrii Galchenko <dggrus@gmail.com>`, with an evocative narrative subject
  and explanatory body. Never add assistant attribution. Commit steps below
  deliberately do not prescribe literal commit messages.
- Work only in the existing isolated worktree. Never merge, push, publish,
  mutate issues, or deploy the game without the later user-selected
  finish-branch path. This plan creates publication machinery but performs no
  live Wiki write.
- The production Wiki remote is
  `https://github.com/cleveralbatraoz/unseeing.wiki.git`, its branch is
  `master`, and the only markerless head accepted for takeover is
  `3780b28869c0ab53d8375a3b4211e6e7f3c15de3`. A later reset exactly to that
  audited head is safely re-taken over; every other markerless head is refused.
- Documentation renderer/publisher code and its focused tests are
  developer-only, excluded from `git archive`, and never become a game, build,
  export, or deploy dependency. Canonical Markdown itself remains tracked.
- Every Python command uses `-B`; thin executable CLIs set
  `sys.dont_write_bytecode = True` before importing sibling modules. No
  `tools/__pycache__` or other generated Python artifact may appear.
- Issue references in the artifact registry are offline syntax contracts.
  CI never queries GitHub. Unknown future issue numbers are never guessed.
- Project-owned production Rust and shader comments may document a current
  local boundary but not an actionable next step, deferred remedy, or missing
  gate. Task 9 removes the three audited remedy passages, corrects two audited
  false shader-comment blocks and two stale settings-test/source comment
  blocks, and preserves executable semantics; review and tests must prove the
  code tokens and behavior did not change. The corrected wave facts are kinds
  0/1/2 returning zero after any counted source-side crossing, kind 3 returning
  `pow(HUM_THROUGH, float(blocked))`, and `wall_crossings_from` skipping the
  birth wall. The corrected settings fact is that the overlay borrows pause and
  mouse modes, ordinary close restores both exact prior values, and tree exit
  restores the prior pause only; it does not own an unconditional unpause.

## File and Interface Map

| Path | Responsibility |
| --- | --- |
| `tools/documentation_markdown.py` | Bounded Markdown scanner, heading anchors, and destination rewriting primitives. |
| `tools/documentation_contract.py` | Canonical layout, manifest, local link, owner/evidence, source-migration, inline-prose, tool inventory, router, and artifact-registry validation. |
| `tools/check-docs.py` | Thin CLI over `validate_repository(repo_root)`. |
| `tools/wiki_mirror.py` | Git-object reader, deterministic Wiki renderer, generated-state digest, and exact tree comparison. |
| `tools/render-wiki.py` | Thin agent-facing renderer/verification CLI consumed by tests and the publisher. |
| `tools/publish-wiki.sh` | Production-guarded and hermetic-test Git publisher; no Markdown policy. |
| `ci/run_documentation_self_test.sh` | Complete-checkout test runner and explicit deployment-archive skip boundary. |
| `test/ci_documentation_tooling_gate_test.sh` | Behavioral proof of that checkout/archive boundary. |
| `test/documentation_markdown_test.py` | Bounded Markdown parser/rewriter tests. |
| `test/documentation_contract_test.py` | Canonical-document, inventory, and registry contract tests. |
| `test/wiki_renderer_test.py` | Pure renderer, state, link, and complete-tree tests. |
| `test/wiki_publisher_test.sh` | Local-bare-repository publisher state-machine tests. |
| `test/wiki_workflow_test.py` | Source-level workflow permissions, dependency, event, and concurrency tests. |
| `docs/wiki-pages.tsv` | Ordered source-path/slug/title/section publication manifest. |
| `docs/current/**` | The only editable shipped mechanics and current engineering documentation. |
| `docs/superpowers/README.md` | Complete artifact registry; active rows become terminal in closeout. |

The Python modules expose these stable interfaces:

```text
# tools/documentation_markdown.py
MarkdownLink(kind: str, destination: str, start: int, end: int, line: int, column: int)
MarkdownDocument(links: Sequence[MarkdownLink], headings: Sequence[str], reference_ids: frozenset[str])
parse_markdown_format_1(text: str, source: str) -> MarkdownDocument
parse_markdown(text: str, source: str) -> MarkdownDocument
decode_destination_format_1(raw: str) -> str
decode_destination(raw: str) -> str
rewrite_destinations_format_1(text: str, replacements: Mapping[MarkdownLink, str]) -> str
rewrite_destinations(text: str, replacements: Mapping[MarkdownLink, str]) -> str
github_anchor_format_1(heading: str) -> str
github_anchor(heading: str) -> str

# tools/documentation_contract.py
ManifestEntry(source: PurePosixPath, slug: str, title: str, section: str)
Violation(path: str, line: int, message: str)
parse_manifest_format_1(text: str, source: str = "docs/wiki-pages.tsv") -> Sequence[ManifestEntry]
parse_manifest(text: str, source: str = "docs/wiki-pages.tsv") -> Sequence[ManifestEntry]
expected_parent_tools(repo_root: Path) -> Sequence[str]
validate_repository(repo_root: Path) -> Sequence[Violation]

# tools/wiki_mirror.py
MirrorState(format_version: int, source_sha: str, content_sha256: str)
render_wiki(repo_root: Path, source_sha: str, output_dir: Path, format_version: int | None = None) -> None
verify_generated_tree(tree: Path) -> MirrorState
verify_git_tree(wiki_repo: Path, commit: str) -> MirrorState
compare_git_tree(wiki_repo: Path, commit: str, generated: Path) -> None
```

`MarkdownLink.destination` is the exact raw substring selected by
`text[start:end]`, including Markdown backslash escapes. Resolution decodes
that raw value through one bounded internal escape function; rewriting and
stale-range checks always compare the raw substring first.

## Legacy Source Migration Ledger

The content tasks preserve these fact families before Task 9 reduces or
deletes their old surfaces. `MigrationLedgerTest` encodes every row below as
an independent source path, stable family ID, nonempty ordered destination-path
set, required
owner/evidence token set, and `migrate` or `remove` disposition. It never
collapses several semicolon-separated facts into one source-level assertion.
A destination without a repository prefix is relative to `docs/current/`;
retained inline-comment rows may additionally name their production source
path. A `migrate` row is green only when every destination exists and carries every
listed contract; a `remove` row is green only when its representative obsolete
bytes are absent from every current page. The exact row set is closed, so one
small fact (for example the crash beacon) cannot disappear while a broader
"infrastructure migrated" assertion remains green. Closely related discarded
presentation/tutorial witnesses may share one `remove` row because none is a
current contract; migrated facts never share a family row.

| Legacy surface | Stable family ID | Canonical destination | Required proof tokens or removal witness | Disposition |
| --- | --- | --- | --- | --- |
| `README.md` | `root-controls` | `mechanics/overview.md` | `ensure_actions`, `unhandled_input`, `tap_target` | migrate |
| `README.md` | `root-demo-query` | `mechanics/overview.md` | `fire_demo_tap`, `?demo` | migrate |
| `README.md` | `root-data-channels` | `mechanics/rendering.md` | `pack_data`, `CUSTOM0` | migrate |
| `README.md` | `root-superface-labels` | `mechanics/rendering.md` | `COPLANAR_EPS`, `PATCH_EPS`, `MIN_SEP` | migrate |
| `README.md` | `root-wall-reveal-law` | `mechanics/waves.md` | `wall_crossings_from`, `source_reveal_vis` | migrate |
| `README.md` | `root-echo-reflection-law` | `mechanics/waves.md` | `RAYS`, `emit_reflecting`, `EchoQueue`, `test_no_echoes_in_acoustic_shadow` | migrate |
| `README.md` | `root-source-role-law` | `mechanics/sound-sources.md` | `SourceRoleInput`, `touching_source_roles_join_the_same_separation_graph_as_world_faces` | migrate |
| `README.md` | `root-platform-matrix` | `engineering/build-test-deploy.md` | `game/export_presets.cfg`, `game/unseeing.gdextension`, `wasm32-unknown-emscripten` | migrate |
| `README.md` | `root-setup-and-run` | `engineering/setup.md` | `.godot-version`, `unseeing_engine_select`, `tools/run_game.sh` | migrate |
| `README.md` | `root-test-and-deploy` | `engineering/build-test-deploy.md` | `ci/pipeline.sh`, `deploy.sh` | migrate |
| `README.md` | `root-gdunit-vendoring` | `engineering/build-test-deploy.md` | `ci/gdunit4.lock`, `ci/vendor-gdunit4.sh`, updater disabled | migrate |
| `README.md` | `root-presentation` | none | screenshot/status/tutorial prose | remove |
| `game/README.md` | `game-composition` | `mechanics/overview.md` | `UnseeingGame`, `WaveLevel` | migrate |
| `game/README.md` | `game-render-pipeline` | `mechanics/rendering.md` | `data_pass.gdshader`, `hearing_post.gdshader` | migrate |
| `game/README.md` | `game-wall-authoring` | `mechanics/levels-and-objects.md` | `WaveWall`, `plan_wall_transform`, `authored_geometry_edit_is_live`, `get_configuration_warnings`, `sync_body_contract` | migrate |
| `game/README.md` | `game-spawn-authoring` | `mechanics/levels-and-objects.md` | `WaveSpawn`, `choose_spawn`, `warnings_from_level` | migrate |
| `game/README.md` | `game-run-coordinates` | `mechanics/levels-and-objects.md` | `run_segments`, `parent-local`, `absolute start`, `not an offset` | migrate |
| `game/README.md` | `game-run-pose` | `mechanics/levels-and-objects.md` | `absorb_run_pose`, `RunPose` | migrate |
| `game/README.md` | `game-run-lifecycle` | `mechanics/levels-and-objects.md` | `authored_geometry_edit_is_live`, `WaveRun::ready`, `runtime-inside-tree` | migrate |
| `game/README.md` | `game-fan-authoring` | `mechanics/sound-sources.md` | `SoundFan`, `volume`, `cadence`, `wave_speed`, `beam_cos`, `Spread::cone` | migrate |
| `game/README.md` | `game-radio-authoring` | `mechanics/sound-sources.md` | `SoundRadio`, `volume`, `cadence`, `wave_speed`, `Spread::Even` | migrate |
| `game/README.md` | `game-editor-warnings` | `engineering/editor-authoring.md` | `get_configuration_warnings`, `warnings_from_level` | migrate |
| `game/README.md` | `game-bootstrap-class-load` | `engineering/setup.md` | `unseeing_engine_select`, `MissingNode`, `bootstrap: OK` | migrate |
| `game/README.md` | `game-missing-node-restart` | `engineering/setup.md` | `MissingNode`, `quit and relaunch` | migrate |
| `game/README.md` | `game-runner-composition` | `engineering/editor-authoring.md` | `UnseeingGame`, `level_scene`, `main.tscn`, `F5`, `F6` | migrate |
| `game/README.md` | `game-paint-failure-taxonomy` | `mechanics/rendering.md` | `PaintPlanError`, `paint_labels`, `starved_classes`, `starved_entries` | migrate |
| `game/README.md` | `game-completed-status` | none | status table and audio/phantom TODO rows | remove |
| `infra/README.md` | `infra-hook-and-retry` | `engineering/build-test-deploy.md` | `infra/post-receive`, `production/main`, `deploy-retry/` | migrate |
| `infra/README.md` | `infra-archive-boundary` | `engineering/build-test-deploy.md` | `git archive`, `BUILD_SHA` | migrate |
| `infra/README.md` | `infra-core-stamp` | `engineering/build-test-deploy.md` | `core.commit`, `BUILD_SHA` | migrate |
| `infra/README.md` | `infra-compression` | `engineering/build-test-deploy.md` | `gzip_static`, `brotli_static` | migrate |
| `infra/README.md` | `infra-crash-beacon` | `engineering/build-test-deploy.md` | `/err?b=`, `unseeing-err.log` | migrate |
| `infra/README.md` | `infra-recovery-topology` | `engineering/build-test-deploy.md` | `infra/post-receive`, `/var/www/unseeing` | migrate |
| `infra/README.md` | `infra-host-tutorial` | none | mutable host paths, measured sizes/timings, copy-over-SSH steps | remove |
| `docs/opening-in-godot.md` | `opening-checkout` | `engineering/setup.md` | `git worktree list`, `--show-toplevel` | migrate |
| `docs/opening-in-godot.md` | `opening-engine-selection` | `engineering/setup.md` | `.godot-version`, `unseeing_engine_select` | migrate |
| `docs/opening-in-godot.md` | `opening-native-toolchain` | `engineering/setup.md` | `build-essential`, `Desktop development with C++`, `rustup` | migrate |
| `docs/opening-in-godot.md` | `opening-missing-node-restart` | `engineering/setup.md` | `MissingNode`, `quit every Godot process`, `bootstrap: OK` | migrate |
| `docs/opening-in-godot.md` | `opening-project` | `engineering/setup.md` | `game/project.godot`, `--editor` | migrate |
| `docs/opening-in-godot.md` | `opening-wall-editing` | `engineering/editor-authoring.md` | `WaveWall`, `plan_wall_transform`, `get_configuration_warnings` | migrate |
| `docs/opening-in-godot.md` | `opening-spawn-editing` | `engineering/editor-authoring.md` | `WaveSpawn`, `choose_spawn`, plain `Marker3D`, duplicate/missing warnings | migrate |
| `docs/opening-in-godot.md` | `opening-run-coordinates` | `engineering/editor-authoring.md` | `run_segments`, `parent-local`, `absolute start`, `not an offset` | migrate |
| `docs/opening-in-godot.md` | `opening-run-pose` | `engineering/editor-authoring.md` | `absorb_run_pose`, `RunPose` | migrate |
| `docs/opening-in-godot.md` | `opening-run-lifecycle` | `engineering/editor-authoring.md` | `authored_geometry_edit_is_live`, `WaveRun::ready`, `runtime-inside-tree` | migrate |
| `docs/opening-in-godot.md` | `opening-warning-loop` | `engineering/editor-authoring.md` | `get_configuration_warnings`, `update_configuration_warnings` | migrate |
| `docs/opening-in-godot.md` | `opening-cli-run-override` | `engineering/setup.md` | `tools/run_game.sh`, `game/override.cfg` | migrate |
| `docs/opening-in-godot.md` | `opening-f5-default` | `engineering/editor-authoring.md` | `F5`, `main.tscn`, `level_01.tscn` | migrate |
| `docs/opening-in-godot.md` | `opening-f6-runner` | `engineering/editor-authoring.md` | `F6`, `WaveLevel`, `UnseeingGame`, `level_scene` | migrate |
| `docs/opening-in-godot.md` | `opening-prefab-level` | `engineering/editor-authoring.md` | `PackedScene`, `owner`, typed nested children, recursive census, ownerless generated limbs | migrate |
| `docs/opening-in-godot.md` | `opening-mcp-install` | `engineering/debugging.md` | `tools/setup-mcp.sh`, `game/addons/godot_mcp` | migrate |
| `docs/opening-in-godot.md` | `opening-missing-node-diagnosis` | `engineering/debugging.md` | `MissingNode`, `.godot-version`, `bootstrap: OK` | migrate |
| `docs/opening-in-godot.md` | `opening-runner-injection-diagnosis` | `engineering/debugging.md` | `WaveLevel`, `UnseeingGame`, `level_scene` | migrate |
| `docs/opening-in-godot.md` | `opening-black-frame-diagnosis` | `engineering/debugging.md` | `black first frame`, `injection error`, `sound source` | migrate |
| `docs/opening-in-godot.md` | `opening-click-narration` | none | campaign branch/path, volatile counts, screenshot-like clicks | remove |
| `docs/agent-workflow.md` | `agent-install` | `engineering/agent-workflow.md` | `tools/setup-agents.sh`, `superpowers-dev` | migrate |
| `docs/agent-workflow.md` | `agent-diagnose` | `engineering/agent-workflow.md` | `ci/verify-superpowers.sh`, `diagnose` | migrate |
| `docs/agent-workflow.md` | `agent-upgrade` | `engineering/agent-workflow.md` | `tools/update-superpowers.sh`, `.gitmodules` | migrate |
| `docs/agent-workflow.md` | `agent-archive-boundary` | `engineering/tooling.md` | `tools/superpowers`, `export-ignore` | migrate |
| `docs/agent-workflow.md` | `agent-version-status` | none | duplicated version/status prose | remove |
| `docs/reports/2026-08-13-agent-portability-audit.md` | `audit-authority-pin` | `engineering/agent-workflow.md` | `AGENTS.md`, `CLAUDE.md`, `tools/superpowers` | migrate |
| `docs/reports/2026-08-13-agent-portability-audit.md` | `audit-worktree-metadata` | `engineering/agent-workflow.md` | `git worktree`, `ci/verify-superpowers.sh` | migrate |
| `docs/reports/2026-08-13-agent-portability-audit.md` | `audit-deployment-archive` | `engineering/build-test-deploy.md` | `test/deployment_archive_test.sh`, `export-ignore` | migrate |
| `docs/reports/2026-08-13-agent-portability-audit.md` | `audit-history-recommendations` | none | addressed narrative and speculative deployment recommendations | remove |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-install-ignore` | `engineering/debugging.md` | `tools/setup-mcp.sh`, `game/addons/godot_mcp` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-freeze-input-step-observe` | `engineering/debugging.md` | `freeze first`, `godot_input`, `godot_game_time`, `observer.snapshot`, `ProcessMode::ALWAYS`, `take_explanation` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-observer-results` | `engineering/debugging.md` | `WaveObserver`, `take_explanation`, `unavailable`, `unknown` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-pure-outward-winding` | `mechanics/rendering.md` | `FACE_CORNERS`, `wedge_triangles`, `column_triangles`, `torus_triangles` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-clockwise-submission` | `mechanics/rendering.md` | `labelled_box_arrays`, `triangle_arrays`, `godot_validate_meshes` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-direct-limb-winding` | `mechanics/rendering.md` | `LimbBuf`, `resize_triangle_surface`, `cull_back` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-evidence-limits` | `engineering/debugging.md` | `explain_ray`, `screenshot`, `mesh readback` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-headless-gdunit-fallback` | `engineering/debugging.md` | `gdUnit4`, `ci/pipeline.sh`, `same observables` | migrate |
| `docs/superpowers/mcp/godot-mcp-loop.md` | `mcp-campaign-debt` | none | closeout counts/instructions and missing-proof wishlist | remove |
| `docs/screenshot.png` | `legacy-screenshot` | none | exact unreferenced media path | remove |
| `game/shaders/data_core.gdshaderinc` | `shader-pulse-cost-fact` | local source comment plus `mechanics/waves.md` | `O(live pulses × walls)`, `WebGL2` | migrate |
| `game/shaders/data_core.gdshaderinc` | `shader-kind3-occlusion-fact` | local source comment plus `mechanics/waves.md` | `source_reveal_vis`, `HUM_THROUGH`, `wall_crossings_from` | migrate |
| `game/shaders/data_core.gdshaderinc` | `shader-remedy` | none | shadow-map next step and deferred profiling | remove |
| `rust/src/cat_brain.rs` | `cat-current-determinism` | local source comment plus `mechanics/levels-and-objects.md` | `deterministic per platform`, `PCG32` | migrate |
| `rust/src/cat_brain.rs` | `cat-remedy` | none | future cross-platform quantization proposal | remove |
| `rust/src/render/labels.rs` | `label-current-ownership` | local source comment plus `mechanics/rendering.md` | `MIN_SEP`, `hearing_post.gdshader` | migrate |
| `rust/src/render/labels.rs` | `label-remedy` | none | missing Rust/shader comparison gate | remove |

---

### Task 1: Keep Documentation Tooling Out of the Deployment Archive

**Files:**

- Create: `ci/run_documentation_self_test.sh`
- Create: `test/ci_documentation_tooling_gate_test.sh`
- Modify: `.gitattributes`
- Modify: `ci/pipeline.sh`
- Modify: `test/deployment_archive_test.sh`

**Interfaces:**

- Consumes: an optional repository-root argument; complete checkout or
  git-exported tree.
- Produces: exit `0` after running every present all-or-none documentation test
  group, exit `0` with an explicit `SKIP` when all checkout-only groups are
  absent, and exit `1` for a partial group.

- [ ] **Step 1: Write every failing archive and pipeline-boundary test**

Create a shell fixture that exercises five independent all-or-none groups:

```sh
markdown='tools/documentation_markdown.py test/documentation_markdown_test.py'
contract='tools/documentation_contract.py tools/check-docs.py test/documentation_contract_test.py'
renderer='tools/wiki_mirror.py tools/render-wiki.py test/wiki_renderer_test.py'
publisher='tools/publish-wiki.sh test/wiki_publisher_test.sh'
workflow='test/wiki_workflow_test.py'
```

The test must prove a complete fixture executes each group, an empty archive
prints `SKIP`, every missing member of a multi-file group is refused, and the
single-file workflow group executes rather than silently skipping when
present. It also requires `ci/run_documentation_self_test.sh` to exist and be
executable, and requires `ci/pipeline.sh` to contain each of the two exact
documentation-gate invocations once, adjacent to each other, before the first
Rust or Godot stage.

Before changing `.gitattributes`, extend `test/deployment_archive_test.sh` to
require effective `export-ignore` attributes for the exact six tool and five
focused-test paths in the groups above, omission of all eleven from the
candidate archive, retention of the runner and its gate test, and a successful
explicit `SKIP` when that retained runner executes inside the archive. Resolve
attributes from the staged candidate with `git check-attr --cached`; an
unstaged working-tree attribute must not make the test pass. Remove
`git archive --worktree-attributes`: archive exactly
`DEPLOY_ARCHIVE_TREEISH` so ambient working-tree attributes cannot alter a
staged candidate's result.

- [ ] **Step 2: Stage only the tests and observe all three red boundaries**

Stage only `test/ci_documentation_tooling_gate_test.sh` and the modified
`test/deployment_archive_test.sh`, write `CANDIDATE_TREE="$(git write-tree)"`,
then run:

```sh
sh test/ci_documentation_tooling_gate_test.sh
DEPLOY_ARCHIVE_TREEISH="$CANDIDATE_TREE" sh test/deployment_archive_test.sh
```

Expected: FAIL naming the absent runner, absent pipeline wiring, and missing
effective export-ignore contract. No production/configuration path has changed.

- [ ] **Step 3: Add the minimal archive-aware runner**

Implement `run_group LABEL FUNCTION_NAME -- REQUIRED_PATH_LIST` in POSIX shell.
Count present members without evaluating their contents; run the command only
when all members exist, print a group-specific `SKIP` when none exist, and fail
with the missing roster when the count is partial. Use these exact group
functions:

```sh
run_markdown_tests() { python3 -B test/documentation_markdown_test.py -v; }
run_contract_tests() {
  python3 -B test/documentation_contract_test.py -v
  if [ -f "$ROOT/docs/superpowers/README.md" ]; then
    python3 -B tools/check-docs.py --repo-root "$ROOT"
  fi
}
run_renderer_tests() { python3 -B test/wiki_renderer_test.py -v; }
run_publisher_tests() { sh test/wiki_publisher_test.sh; }
run_workflow_tests() { python3 -B test/wiki_workflow_test.py -v; }
```

Invoke each function from `ROOT` only when its exact group is complete. The
runner accepts a root argument so its own self-test never touches this worktree.
Rerun `sh test/ci_documentation_tooling_gate_test.sh`; the runner behavior is
now green and the named pipeline-wiring assertion remains red.

- [ ] **Step 4: Make the candidate archive contract green**

Add exact `.gitattributes` entries for the six `tools/` files and five focused
test files listed above. Keep `ci/run_documentation_self_test.sh` and
`test/ci_documentation_tooling_gate_test.sh` in the archive. Stage the runner
and `.gitattributes` with the two tests, rewrite `CANDIDATE_TREE`, and rerun
`test/deployment_archive_test.sh` against that tree. Expected: PASS; the test
change preceded the attribute change.

- [ ] **Step 5: Put the adapter behind its own self-test in the cheap CI stage**

Add these adjacent stages before Rust/Godot work:

```sh
echo "ci: documentation-tooling checkout/archive gate self-test"
"$DIR/test/ci_documentation_tooling_gate_test.sh" || exit 1
echo "ci: canonical-documentation and Wiki self-tests"
"$DIR/ci/run_documentation_self_test.sh" "$DIR" || exit 1
```

Rerun `sh test/ci_documentation_tooling_gate_test.sh`. Expected: PASS; deleting,
duplicating, separating, or moving either call below Rust/Godot work makes the
named wiring test fail.

- [ ] **Step 6: Stage the candidate tree and run the focused boundary tests**

Because `test/deployment_archive_test.sh` archives a Git tree rather than
unstaged working-tree bytes, stage exactly the five task paths before testing
and address that candidate tree explicitly:

Run:

```sh
sh test/ci_documentation_tooling_gate_test.sh
git add .gitattributes ci/pipeline.sh ci/run_documentation_self_test.sh \
  test/ci_documentation_tooling_gate_test.sh test/deployment_archive_test.sh
CANDIDATE_TREE="$(git write-tree)"
DEPLOY_ARCHIVE_TREEISH="$CANDIDATE_TREE" sh test/deployment_archive_test.sh
sh test/shell_syntax_test.sh
```

Expected: PASS; the current checkout reports the five not-yet-created groups
as explicit skips, and a git archive reports the same boundary without a
partial leak.

- [ ] **Step 7: Request review and commit the green boundary**

Review the staged diff for archive composition and POSIX portability. Stage
only the five task files and commit with the mandated identity, a narrative
subject, an explanatory body, and no attribution.

### Task 2: Parse Only the Markdown Grammar the Mirror Can Prove

**Files:**

- Create: `tools/documentation_markdown.py`
- Create: `test/documentation_markdown_test.py`

**Interfaces:**

- Consumes: decoded Python `str` Markdown text and a repository-relative source
  label.
- Produces: exact destination character ranges in that `str`, GitHub-compatible
  heading anchors,
  normalized reference IDs, or a source-located `MarkdownSyntaxError`.
  `start`/`end` are zero-based half-open Python-string indices; diagnostic
  `line`/`column` are one-based and count decoded Unicode characters. The
  initial frozen compatibility entry points are
  `parse_markdown_format_1`, `decode_destination_format_1`,
  `rewrite_destinations_format_1`, and `github_anchor_format_1`; the unsuffixed
  names are current-format aliases, not dependencies of an old renderer.

- [ ] **Step 1: Write failing table-driven scanner tests**

Cover inline links and images, reference definitions and uses, autolinks,
same-page fragments, angle-delimited destinations, balanced nested
parentheses, ordinary backslash escapes, duplicate heading anchors, fenced
code, indented code, and variable-length inline code spans. Include these
rejections with exact line/column assertions:

```python
BAD = {
    "raw HTML link": '<a href="inside.md">inside</a>',
    "multiline destination": "[inside](inside.md\n#part)",
    "unclosed escape": "[inside](inside.md\\)",
    "unbalanced destination": "[inside](inside(one.md)",
}
```

Prove link-shaped text inside every code form yields no destination and a
reference use without one unique definition is refused. Exercise
both `rewrite_destinations_format_1` and its initial current alias with a
negative range, reversed range, end beyond the
text, overlapping ranges, and a stale `MarkdownLink` whose selected substring
no longer equals its destination; every malformed public input must return a
defined error. For an escaped destination, assert raw `destination` is exactly
`text[start:end]` character-for-character and separately assert the bounded
decoder's resolved value. For a parenthesized inline destination and a
reference definition, require the range to exclude surrounding `()`; for an
angle-delimited destination and an autolink, require it to exclude surrounding
`<>`. A reference use owns no second rewrite span: its one unique definition
owns exactly one destination range. Assert `[use][id]` remains byte-exact while
the destination in `[id]: <a\(b\).md#part>` alone is selected and rewritten.
For every accepted/rejected fixture, require the initial unsuffixed current
alias and explicit format-1 entry to produce the same value or exception.

- [ ] **Step 2: Run the scanner tests and observe the import failure**

Run:

```sh
python3 -B test/documentation_markdown_test.py MarkdownScannerTest -v
```

Expected: FAIL because `tools/documentation_markdown.py` does not exist.

- [ ] **Step 3: Implement the total scanner state machine**

Use immutable dataclasses and an index-bounded loop. Track fence delimiter and
length, indentation, inline backtick length, bracket nesting, parenthesis
depth, and escape state explicitly. Never use a regex to guess balanced
destinations. After recognizing `http`, `https`, and `mailto` autolinks, reject
outside code every raw HTML start whose `<` is followed by an ASCII letter,
`/`, `!`, or `?`; match tag names case-insensitively. Include negative fixtures
for `<A HREF=...>`, `<IMG SRC=...>`, `<video src=...>`, and
`<source src=...>`, while the same bytes in fences, indented code, and inline
code remain untouched. Reject multiline destination constructs and return a
defined error for every malformed input rather than indexing past the text.
Put every recognition, escape-decoding, and acceptance branch in frozen
format-1 entries; make `parse_markdown` and `decode_destination` thin
current-format aliases. A future grammar/validation change adds suffixed
compatibility entries and repoints only the current aliases, never edits format
1 in place. `parse_markdown_format_1` calls
`github_anchor_format_1` directly and never reaches the unsuffixed
`github_anchor` alias.

- [ ] **Step 4: Implement stable heading and rewrite helpers**

For headings used as fragment targets, accept the deliberately bounded ASCII
subset `[A-Za-z0-9 _-]+`; reject a referenced heading outside that subset
rather than pretending to implement GitHub's complete Unicode slugger.
`github_anchor` lowercases, converts whitespace to hyphens, preserves existing
hyphens/underscores, and the document parser suffixes duplicate anchors `-1`,
`-2`, continuing with monotonically increasing integer suffixes.
Freeze that behavior as `github_anchor_format_1`; the unsuffixed helper is the
current-format alias.
`rewrite_destinations_format_1` verifies nonnegative, ordered, in-bounds,
non-overlapping ranges and requires each selected substring to equal that
link's recorded destination before applying replacements from the end of the
string toward the start. Add a case with non-ASCII text before the first
destination and prove the recorded character offsets select and replace only
that destination. Initially alias `rewrite_destinations` to the frozen helper;
future byte-affecting rewrite behavior gets a new suffixed helper and repoints
only the current alias.

- [ ] **Step 5: Run the scanner tests, group runner, and mutation fixtures**

Run the focused test and `sh ci/run_documentation_self_test.sh .`; the Markdown
group runs while the four later groups skip explicitly. Then use the test's
subject-path injection to load copies with
the code-span skip, balanced-parenthesis decrement, and raw-HTML rejection
branches each removed. Each mutated subject must make its named test fail.

- [ ] **Step 6: Request review and commit the green Markdown boundary**

Review totality, malformed-input coverage, and the absence of third-party
imports. Stage the module and test together and commit under the global commit
rules.

### Task 3: Validate the Canonical Repository Contract Offline

**Files:**

- Create: `tools/documentation_contract.py`
- Create: `tools/check-docs.py`
- Create: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: a repository working tree and index; no network, Wiki, or GitHub
  API.
- Produces: a sorted tuple of `Violation` values and CLI exit `0` only when the
  tuple is empty. Manifest format 1 is frozen as
  `parse_manifest_format_1`; `parse_manifest` is the current-format alias used
  by the live canonical-doc validator.

- [ ] **Step 1: Write a failing synthetic valid-repository test**

The fixture must contain the exact future authority layout, short README
routers, a byte-exact three-line `CLAUDE.md`, a TSV manifest, a tooling table,
an artifact table, one resolvable owner/evidence row, and a real Git index.
Assert `validate_repository(fixture) == ()` and exact equality between
`parse_manifest` and `parse_manifest_format_1` on valid and malformed rows.

- [ ] **Step 2: Add one named failure test per contract family**

Delete or mutate one fixture fact at a time and assert a source-located error:

```python
CASES = (
    "missing current page",
    "duplicate or unindexed manifest source",
    "ASCII-case-folded slug collision",
    "reserved output slug",
    "Markdown-active or padded manifest title/section",
    "broken local path or heading fragment",
    "missing owner path or declared symbol",
    "unknown evidence kind or missing evidence path",
    "README procedure duplication",
    "task box or backlog marker in current docs",
    "gap/flaw/bug or deliberately unfinished prose",
    "actionable inline production-source documentation",
    "volatile test total",
    "mutable main/master repository link",
    "checkout-specific home path",
    "missing or duplicate parent-tool row",
    "invalid parent-tool execution kind",
    "submodule-internal tool row",
    "missing or duplicate artifact row",
    "invalid outcome or residual-issue cell",
)
```

The current-doc hygiene scan deliberately excludes frozen spec/plan bodies.
The manifest case includes `**Title**`, `<Section>`, a backslash, doubled
spaces, and leading/trailing padding, and requires each to fail before renderer
templates can consume it. Add a positive scope fixture whose frozen artifact
body contains both a deliberately nonexistent historical local link and
backlog-shaped prose: the repository remains valid because that whole body is
unscanned, while copying either byte sequence into a current authority surface
fails. Its artifact path and registry current-result link still resolve.

- [ ] **Step 3: Run the contract tests and observe the missing-module failure**

Run:

```sh
python3 -B test/documentation_contract_test.py RepositoryContractTest -v
```

Expected: FAIL because `tools/documentation_contract.py` and its CLI are
absent.

- [ ] **Step 4: Implement manifest and authority validation**

Parse comment/blank TSV lines and exactly four tab-separated fields. Restrict
slugs to ASCII `[A-Za-z0-9][A-Za-z0-9_-]*`, reject `.md`, path traversal,
duplicates, and ASCII-case-folded collisions with `Home`, `_Sidebar`,
`Mirror-Metadata`, or `.unseeing-wiki-mirror`. Require title and section to
match the complete ASCII safe-label grammar
`[A-Za-z0-9][A-Za-z0-9 ,&+()/'\.:-]*`, with no leading/trailing whitespace or
two adjacent spaces; this deliberately excludes every Markdown control
character, backslash, raw-HTML delimiter, tab, and newline before the values
are inserted verbatim into generated headings/link labels. Reject all other
padding/control characters and require each section's rows to be contiguous. Require every
`docs/current/**/*.md` exactly once and allow `docs/README.md` exactly once.
Put all manifest acceptance in `parse_manifest_format_1` and initially alias
`parse_manifest` to it. A future acceptance change adds a new suffixed parser
and repoints only the current alias; historical renderers call their exact
suffixed parser.

- [ ] **Step 5: Implement link, owner, router, and hygiene validation**

Use `parse_markdown`; on current authority surfaces (`AGENTS.md`, scoped
README routers, `docs/README.md`, every manifest source, and the artifact
registry's current-result cells), resolve relative paths from the source file,
require heading fragments to exist, and reject workspace paths matching
`/home/`, `/Users/`, or drive-letter user profiles. Exclude each frozen
spec/plan body in its entirety from link and hygiene scanning: the validator
does not try to infer whether a historical local path was once valid or later
retired. This whole-body exemption does not include the artifact registry row
that names the artifact, its current-result cell, or any current page. Parse
the uniform owner/evidence table:

```markdown
| Contract | Owner | Symbol | Evidence kind | Evidence |
| --- | --- | --- | --- | --- |
```

Require `Owner` to be a Markdown link whose label is the backticked
repository-relative path, `Symbol` to be one backticked literal searched in
that owner blob, and `Evidence` to contain one or more repository-relative
Markdown links.

Allow evidence kinds `pure Rust`, `Godot behavior`, `mesh readback`,
`shader source`, `source contract`, `shell behavior`, `workflow contract`,
`archive behavior`, `native pixels`, and `web pixels`. The engineering kinds
are not aliases for mechanics evidence: a workflow-source assertion cannot be
reported as runtime behavior, and an archive extraction test cannot be
reported as a source-only contract. Resolve owner/evidence paths and require
the declared symbol token in the owner file without copying its numeric value
into the test. Require each router to link to the canonical index, contain at
most twenty-four nonblank lines, and contain no fenced code, owner/evidence
table, shell prompt, numeric mechanics constant, or duplicated command
tutorial. Allow only a title, scope paragraph, canonical route, and
root-license pointer.

Reject case-insensitive backlog language through one finite tested table. For
each non-code logical line, casefold and trim indentation, then run one loop
bounded by the original line length: strip exactly one leading Markdown
blockquote marker or exactly one unordered/one-to-nine-digit ordered-list
marker, trim again, and repeat until neither form matches. This normalizes
nested mixtures such as `> -` and `- >` rather than assuming blockquotes
precede one list layer. Around a candidate prefix token, strip at most one
balanced matching one-to-three-character `*` or `_` emphasis wrapper; parse
ATX heading text separately and repeatedly strip balanced outer
one-to-three-character `*`/`_` wrappers, with every loop bounded by the line
length.
Reject headings containing whole words `gaps?`, `flaws?`, or `bugs?`, plus the
exact headings `issue`, `issues`, `future work`, `remaining work`, `todo`, and
`fixme`. Reject phrases
`(?:known|remaining|unresolved) (?:gaps?|flaws?|bugs?|issues?)`,
`deliberately (?:unfinished|undone)`, `not yet implemented`, and sentence form
`(?:this|the|a|these|those) (?:gaps?|flaws?|bugs?)`. Apply line prefixes
`TODO`, `FIXME`, `Future work:`, `We should `, `We need to `, and
`Still to do:` to the normalized logical line, so `- TODO:` and
`> Future work:` cannot evade the gate.

The words `boundary` and `limitation` are not exemptions from those rules. A
logical line containing either word and a canonical issue link is accepted as
a neutral current constraint only when it contains none of the whole-word
remedy vocabulary `should`, `need`, `fix`, `implement`, `add`, `remove`,
`replace`, `investigate`, `future`, `todo`, or `remaining`; otherwise it is
disguised backlog. Add negative fixtures for `## Known Gaps`, `## Bugs`,
`## **Known Gaps**`, `- TODO:`, `**TODO:**`, `__FIXME:__`,
`> Future work:`, `> - **TODO:**`, `- > __FIXME:__`,
`> - > **TODO:**`, `1) __FIXME:__`, and
`Boundary: we should automate this [#38](https://github.com/cleveralbatraoz/unseeing/issues/38)`.
Keep positive fixtures for “wall openings are actual gaps”,
“snapshot/explain limitations,” and `debugging`, so mechanics nouns, evidence
scope, and substrings are not false positives.

Apply a second bounded prose surface to tracked mode-`100644` production files
under `rust/src/**/*.rs` and `game/shaders/**/*.gdshader*`. Extract only
complete line/doc-comment text beginning with Rust/GLSL `//`, `//!`, or `///`;
do not parse code strings or block comments. Exclude vendored code, tests,
fixtures, generated files, and frozen artifacts by construction. Reuse the
logical-line backlog rules and additionally reject whole phrases matching a
`documented next step`, profiling/work/gate/proof described as `deferred`, a
`future cross-platform build would` remedy, `still not single-sourced`, or
`no gate compares|checks|proves`. Positive fixtures retain current complexity,
evidence, per-platform behavior, local extensibility rationale, and the Godot
API word `call_deferred` without a proposed action.

`InlineDocumentationTest` also carries two exact, path-scoped migration
witnesses for the currently false `data_core.gdshaderinc` explanations: the
block beginning `EVERY wave obeys this` and the sentence ending `never through
a wall`. These are not generalized banned phrases; the test requires their
absence and the replacement comment's explicit kind-0/1/2 zero versus kind-3
`HUM_THROUGH`/`wall_crossings_from` semantics. This turns correction of known
mechanics misinformation into a one-time audited migration without rejecting
unrelated accurate prose.

- [ ] **Step 6: Implement the exact parent-tool inventory**

Derive the expected set and entry modes from
`git ls-files --stage -z -- tools`: direct tracked regular files/gitlinks under
`tools/`, plus recursively tracked regular files under `tools/lib/`.
`tools/lib` itself gets no row. Collapse and require the mode-`160000`
`tools/superpowers` gitlink once, reject all paths beneath it, and reject a
nested parent-owned tool outside `tools/lib`. Accept only stage-zero
`100644`/`100755` regular entries and the one `160000` gitlink; a conflicted or
unexpected index mode is a violation. Parse exact backticked path cells beneath
`## Tool registry` and require nonempty `Kind`, `Purpose`, and `Use when`
cells. Accept only `POSIX-host shell command`, `Command Prompt command`,
`PowerShell command`, `POSIX-host shell library`, `Python command`, `Python
library`, and `developer gitlink` as `Kind`, and apply this total classifier:

- exact `tools/superpowers` mode `160000` is `developer gitlink`;
- a direct `*.cmd` or `*.ps1` at mode `100644` or `100755` is respectively a
  `Command Prompt command` or `PowerShell command`; host context, not the Unix
  executable bit, owns that classification;
- a direct `*.sh` must be mode `100755` and is `POSIX-host shell command`, while
  `tools/lib/**/*.sh` must be mode `100644` and is `POSIX-host shell library`;
- a direct `*.py` at mode `100755` with a Python shebang is `Python command`;
  a direct `*.py` at mode `100644` without a shebang or CLI main block is
  `Python library`; and `tools/lib/**/*.py` must be mode `100644` and is
  `Python library`;
- every other path/extension/mode/shebang combination is a violation.

`POSIX-host` names the execution context, not a portability claim about shell
syntax. Each shell row's purpose/use text names its actual shebang interpreter
(`Bash` where required, otherwise POSIX `sh`), and exact current-roster tests
reject a Bash script described as generic POSIX `sh`.

Pin the new roster exactly: `documentation_markdown.py`,
`documentation_contract.py`, and `wiki_mirror.py` are mode-`100644` Python
libraries; `check-docs.py` and `render-wiki.py` are mode-`100755` shebang
Python commands; `publish-wiki.sh` is a mode-`100755` POSIX-host shell command.
Fixture
mutations that add a tool must add it to the fixture's Git index; an untracked
operator scratch file is not a repository capability.

- [ ] **Step 7: Implement artifact-registry validation**

Discover every parent-repository Markdown file under
`docs/superpowers/specs/` and `docs/superpowers/plans/`. Require set equality
with registry paths and no duplicate row. Accept `active`, `shipped`,
`superseded`, and `closed without implementation`. Require a current-result
link, plus either `none` or a comma-separated nonempty list of unique positive
`#N` references. Uniqueness is within one row; shared residual work may appear
on each artifact it genuinely affects.

- [ ] **Step 8: Add the thin CLI and make every diagnostic deterministic**

`tools/check-docs.py --repo-root PATH` prints sorted
`path:line: message` diagnostics to stderr and returns `1` for violations,
`2` for invalid invocation/read failure, and `0` only for a clean contract.
It contains no policy beyond argument parsing and rendering violations. Give
it a Python shebang and mode `100755`; keep the imported contract module at
mode `100644` without a CLI main block.

- [ ] **Step 9: Run the focused tests and CLI fixture tests**

Run:

```sh
python3 -B test/documentation_contract_test.py -v
sh ci/run_documentation_self_test.sh .
```

Expected: unit tests PASS; the live CLI remains explicitly inactive because
`docs/superpowers/README.md` has not been created.

- [ ] **Step 10: Request review and commit the green contract layer**

Review offline behavior, Git index handling, deterministic diagnostics, and
the historical-artifact exclusion. Stage only this task's files and commit
under the global rules.

### Task 4: Reconstruct the Rendering Contract from Owners and Evidence

**Files:**

- Create: `docs/current/mechanics/rendering.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: current render owners and the strongest evidence already in the
  repository.
- Produces: an AI-facing rendering page that distinguishes laws, adapters, and
  evidence strength without inventing work or copying the drifted Wiki.

- [ ] **Step 1: Write the failing rendering-page contract test**

Require the page and its owner/evidence rows for:

```python
RENDERING_OWNERS = (
    ("data-pass R/G/B", "game/shaders/data_core.gdshaderinc", "pack_data"),
    ("same-facing coplanar merge", "rust/src/render/superface.rs", "COPLANAR_EPS"),
    ("patch overlap", "rust/src/render/superface.rs", "PATCH_EPS"),
    ("separation graph", "rust/src/render/labels.rs", "MIN_SEP"),
    ("fixed semantic role table", "rust/src/render/labels.rs", "role_label"),
    ("radio preview label exception", "rust/src/render/labels.rs", "Role::Case"),
    ("label safe-band lower bound", "rust/src/render/paint_plan.rs", "LABEL_MIN"),
    ("label safe-band upper bound", "rust/src/render/paint_plan.rs", "LABEL_MAX"),
    ("safe-band submission validation", "rust/src/render/paint_plan.rs", "valid_label"),
    ("atomic paint plan", "rust/src/render/paint_plan.rs", "PaintPlan"),
    ("recoverable local label starvation", "rust/src/render/labels.rs", "starved_classes"),
    ("fatal paint-request validation", "rust/src/render/paint_plan.rs", "PaintPlanError"),
    ("plan-before-application refusal guard", "rust/src/nodes/level.rs", "paint_labels"),
    ("mesh label submission", "rust/src/render/paint.rs", "relabel"),
    ("pure box outward winding", "rust/src/render/paint.rs", "FACE_CORNERS"),
    ("pure prop outward winding", "rust/src/prop_shape.rs", "wedge_triangles"),
    ("pure column outward winding", "rust/src/prop_shape.rs", "column_triangles"),
    ("pure source outward winding", "rust/src/source_shape.rs", "torus_triangles"),
    ("Godot-clockwise conversion", "rust/src/render/paint.rs", "labelled_box_arrays"),
    ("Godot-clockwise triangle conversion", "rust/src/render/paint.rs", "triangle_arrays"),
    ("already-clockwise limb buffers", "rust/src/nodes/limbs.rs", "LimbBuf"),
    ("already-clockwise direct submission", "rust/src/render/paint.rs", "resize_triangle_surface"),
    ("two-sided world data skin", "game/shaders/data_pass.gdshader", "cull_disabled"),
    ("back-face-culled source skin", "game/shaders/data_xray.gdshader", "cull_back"),
    ("post-pass crease and silhouette", "game/shaders/hearing_post.gdshader", "fragment"),
)
```

The test verifies owner/symbol/evidence structure and forbids retired
per-object-ID and production-GDScript architecture language. It does not copy
numeric gameplay values into Python.

- [ ] **Step 2: Run the focused test and observe the missing-page failure**

Run:

```sh
python3 -B test/documentation_contract_test.py RenderingDocumentationTest -v
```

Expected: FAIL naming `docs/current/mechanics/rendering.md`.

- [ ] **Step 3: Write the data-pass and superface sections**

Explain R reveal, G per-vertex label, and B camera distance; name the data
skins and post-pass consumers. Describe same-facing coplanar merge, the
different responsibilities of `COPLANAR_EPS` and `PATCH_EPS`, why bends/steps
remain, and why separate touching solids remain separated. State the current
constant values only beside their owners, not in the test.

- [ ] **Step 4: Write the label allocation and semantic-role sections**

Explain graph colouring, the safe band, the sole radio-preview exception,
fixed creature role labels, and per-placed-source graph-coloured semantic
roles. Name `paint_plan::LABEL_MIN`/`LABEL_MAX` plus `valid_label` as the
enforced submission-band owner; name `labels::MIN_SEP` and its role table as
the separation/colour owners. Name `paint_plan` as the pure atomic planner and
`paint` as the mesh submission boundary. Do not describe label cycling as an
available strategy. State neutrally that Rust `MIN_SEP` owns allocation while
the hearing shader's upper `smoothstep` knee independently owns rendered crease
strength; do not keep a proposed gate or future remedy in current docs.

Preserve the two different failure contracts currently carried by
`game/README.md`. A valid plan whose local separation graph exhausts the
palette remains total: it returns fallback labels plus exact starved
class/entry/source ownership, the level and affected authoring nodes warn, and
the game continues with those named seams at risk. A globally invalid request
(invalid/conflicting/empty palette, invalid anchor, overflow, or bounded-size
failure) returns `PaintPlanError` and exposes no `PaintPlan` or command set to
the caller, although the pure function may have allocated internal candidate
vectors before a later error; `WaveLevel::paint_labels` records the refusal and
returns without applying any mesh command, so every existing label remains
unchanged. This all-or-nothing statement is limited to planning failure:
successful application is a per-command loop, and a malformed mesh may make
`render::paint::relabel` no-op that surface while later commands continue.
Never collapse recoverable local starvation into fatal validation or imply
that a command from a rejected plan can be applied.

Add a separate winding/submission section. Mathematical box, wedge, column,
and source-torus generators use conventional counter-clockwise/outward
triangles. The `render::paint` ArrayMesh edge reverses complete triples into
Godot's clockwise front-face convention. Animated creature/viewmodel limb
buffers are the explicit other input contract: they are born
Godot-clockwise and use the direct submission door without a second reversal.
The world data skin is two-sided (`cull_disabled`), which can mask wrong world
winding; acoustic-image/source limbs use `data_xray` with `cull_back`, so their
submitted winding is load-bearing. Do not turn separate mesh-winding and
shader-source assertions into a claim of rendered culling proof.

- [ ] **Step 5: Write an evidence-strength matrix**

Separate shader-source assertions from Cargo law tests, real Godot mesh
`CUSTOM0` readback, browser G-channel readback, and rendered-pixel evidence.
Name `game/tests/map_test.gd`, `game/tests/source_test.gd`,
`game/tests/mesh_label_test.gd`, `game/tests/shader_contract_test.gd`, and the
browser raw-pass gate in `test/web_probe.py`. Describe
`tools/probe_visibility.sh` as on-demand supporting native-pixel evidence, not
a `ci/pipeline.sh` stage. For winding, name the pure generator tests plus
Godot-clockwise submitted-mesh cases in `mesh_label_test.gd`, `props_test.gd`,
`source_test.gd`, `viewmodel_test.gd`, and `cat_test.gd`; pair the source skin
claim with `data_skins_test.gd`. State explicitly that no current rendered
test isolates culling and no pre-existing engine/Godot source test pins the
world skin's `cull_disabled` token; the new documentation owner resolver proves
only that the declared token/path exists, not rendered culling behavior.
For paint failure, cite the pure impossible-clique/starvation-owner tests and
fatal `PaintPlanError` boundary tests, plus the source-role warning Godot test.
Classify the existing-label adapter return as inspected source-contract
evidence: no current engine test injects a globally invalid paint request and
then reads the pre-existing mesh back, so do not upgrade it to mesh-readback
proof.

- [ ] **Step 6: Recheck the page against current owners**

Run:

```sh
(cd rust && cargo test render::)
python3 -B test/documentation_contract_test.py RenderingDocumentationTest -v
```

Then inspect each quoted value directly in its named owner and confirm every
evidence row says whether it observes source, a pure result, mesh data, or
pixels.

- [ ] **Step 7: Request independent mechanics review and commit the page**

Give a read-only reviewer the page, design spec, owners, and named evidence.
Resolve factual findings against code/tests, rerun the focused checks, stage
the page and test, and commit under the global rules.

### Task 5: Keep Traveling Waves, Source Reveal, and Silhouettes Distinct

**Files:**

- Create: `docs/current/mechanics/waves.md`
- Create: `docs/current/mechanics/sound-sources.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: pure wave/source laws, shader consumers, Godot boundary tests, and
  the existing single-source rendered probe.
- Produces: two pages that cannot collapse the three different wall effects
  into the Wiki's false “all traveling waves stop” claim.

- [ ] **Step 1: Write failing owner/evidence tests for both pages**

Require distinct contract rows for player/echo/footstep surface reveal,
player shell front-surface stop, kind-3 source-side crossing reveal, kind-3
visible-shell one-factor composition, and camera-side standing silhouette.
Require `rust/src/pulse_pool.rs::MAXP` plus its shader scan-bound consumer and
`rust/src/ray_fan.rs::RAYS` as owners of the pool/fan bounds. Require separate
rows for `ray_fan::fan_directions` and `ffi::cast_reflection_fan` so the prose
distinguishes 26 candidates from the hemisphere-culled actual casts. Require separate
reflection rows for `rust/src/ffi.rs::emit_reflecting`,
`rust/src/echo_queue.rs::EchoQueue`, and the acoustic-shadow Godot evidence
`game/tests/echo_reflection_test.gd::test_no_echoes_in_acoustic_shadow`.
The documentation test verifies those owner/evidence rows and symbols; it does
not mirror the numeric `64` or `26` literals in Python. The writer/reviewer
reads the prose values from their owners.
Require source cadence, fan, radio, recursive discovery, and role-label rows.
For authoring, require distinct `SoundFan` owner rows for `volume`, `cadence`,
`wave_speed`, and `beam_cos` plus the directed `Spread::cone` law, and distinct
`SoundRadio` rows for its shared three knobs plus even `Spread::Even` law.
Reject the drifted `wave_transmission`, deleted `oids()`, and “another room is
silent” statements.

- [ ] **Step 2: Run the focused tests and observe both missing pages**

Run:

```sh
python3 -B test/documentation_contract_test.py WaveDocumentationTest -v
python3 -B test/documentation_contract_test.py SoundSourceDocumentationTest -v
```

Expected: each fails only for its absent page.

- [ ] **Step 3: Document the pulse pool and reflection path**

In `waves.md`, describe the 64-slot pool, scan bound, expiration/eviction
order, CPU birth/packed-lane ownership, shader radius/fade/reveal consumers,
26 candidate golden-angle reflection directions, hemisphere culling in
`fan_directions` (so actual casts need not equal `RAYS`), clustering,
travel-time echo scheduling,
secondary-emitter firing when the wavefront arrives, acoustic-shadow silence,
and qualitative `O(live scan × walls)` cost. Name the separate ray-fan,
`emit_reflecting`, appointment-book, and real-physics shadow owners/evidence so
the reflection mechanism cannot disappear behind the wall-reveal row. Do not
publish measured-looking live-count estimates without a benchmark.

- [ ] **Step 4: Document the three wall-dependent effects verbatim from the spec**

Keep these in separate subsections and owner rows:

1. player tap/echo/footstep surface reveal becomes zero after a source-side
   crossing, and its visible shell stops at the front scene surface;
2. kind-3 surface reveal uses `HUM_THROUGH` raised to source-side crossings,
   while its visible shell applies one `HUM_THROUGH` at/behind the front
   surface and does not count crossings;
3. the standing source silhouette uses `SOURCE_THROUGH` raised to camera-side
   crossings.

Quote values only with `rust/src/level_plan.rs` ownership and the actual shader
consumer paths. Mark the visible-shell rule as shader-source evidence rather
than a numeric rendered proof.

- [ ] **Step 5: Document current sound-source behavior**

In `sound-sources.md`, cover kind 3, total volume/reach calculation, fan/radio
voice contracts, cadence booking/retuning, recursive capability discovery,
dependency injection before tree entry, camera-to-hub standing images, and
per-instance graph-coloured semantic-role labels. Spell out the shared
volume/cadence/wave-speed Inspector knobs, the fan-only beam-cosine/directed
cone, and the radio's even radiation; do not let naming both node classes stand
in for these independently losable contracts. Separate preview fallback labels
from authored-level output.

- [ ] **Step 6: Recheck pure and Godot boundary evidence**

Run:

```sh
(cd rust && cargo test pulse_pool)
(cd rust && cargo test sight)
(cd rust && cargo test sound_source)
(cd rust && cargo test fan_wave)
(cd rust && cargo test radio_wave)
python3 -B test/documentation_contract_test.py WaveDocumentationTest SoundSourceDocumentationTest -v
```

Read the exact shader branches and the named `observer_test.gd`,
`source_test.gd`, `map_test.gd`, `wave_core_parity_test.gd`, and
`data_skins_test.gd` evidence before accepting prose.

- [ ] **Step 7: Request independent wave/source review and commit**

Require the reviewer to identify any source-text claim presented as a pixel
observation. Correct only findings proven against owners/evidence, rerun the
focused tests, stage the two pages and test, and commit under the global rules.

### Task 6: Record the Rust Composition Root and Designer-Owned World

**Files:**

- Create: `docs/current/mechanics/overview.md`
- Create: `docs/current/mechanics/levels-and-objects.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: `UnseeingGame`, `UnseeingPlayer`, the settings overlay,
  `WaveLevel`, pure level plans, authored scenes, editor probes, and
  engine/Godot tests.
- Produces: current high-level mechanics and level/object contracts without
  exact scene census or retired script ownership.

- [ ] **Step 1: Write failing overview and level-page tests**

Require owner rows for `UnseeingGame`, `game/scenes/main.tscn`, frame order,
bounded simulation time, capture/restore, `WaveLevel` census/injection,
`WaveSpawn`, demo targeting, `WaveWall`, `WaveRun`, slabs, placement, wall
packing, and prefab recursion. Also require rows for
`rust/src/nodes/player.rs::ensure_actions`, its physical-position WASD map,
captured-mouse look, physics-queued click/cane modes,
`rust/src/nodes/settings.rs::unhandled_input`, and
`rust/src/nodes/settings.rs::watch_capture`,
`rust/src/settings_menu.rs::capture_loss_opens`, and
`rust/src/nodes/game.rs::fire_demo_tap`. Require WaveRun rows for
`rust/src/level_plan.rs::run_segments`, `absorb_run_pose`, and
`authored_geometry_edit_is_live`, plus the
`rust/src/nodes/run.rs::WaveRun` adapter. Require separate WaveWall rows for
`rust/src/level_plan.rs::plan_wall_transform`,
`rust/src/nodes/wall.rs::get_configuration_warnings`, and
`sync_body_contract`, plus WaveSpawn rows for
`rust/src/level_plan.rs::choose_spawn` and the level warning owner. Name
movement, cane, settings, pure
web-capture, seed/demo, WaveRun pure/Godot/editor, and web-smoke evidence with
its exact strength. Reject `UnseeingMain`, `game/scripts/main.gd`, a plain
named spawn marker, scene-order demo targeting, the obsolete claim that Escape
only releases the mouse, the false binary claim that web replaces direct
Escape with capture loss, and exact current object counts.

- [ ] **Step 2: Run the focused tests and observe the missing-page failures**

Run:

```sh
python3 -B test/documentation_contract_test.py OverviewDocumentationTest LevelDocumentationTest -v
```

- [ ] **Step 3: Write the composition and frame contract**

Describe `rust/src/nodes/game.rs::UnseeingGame` plus
`game/scenes/main.tscn`, selected-level fallback, five shared materials and
`WaveCore`, and the actual frame sequence from clock through demo behavior.
Document the bounded renderer-visible time law, seed/demo distinction, and
transactional capture/restore with the precise strength of existing evidence.
Add a concise player-input section: WASD means physical key positions on every
keyboard layout; captured mouse motion owns look; left click queues a tap that
the physics tick resolves as an aimed strike, supported cane-rest tap, or
silent unsupported-air swish; and Escape opens/closes the settings overlay.
Opening stores the existing pause and mouse modes, then pauses the world and
frees the mouse; closing restores those exact pre-open modes, so only the
ordinary running/captured case thaws and recaptures. Delivered `ui_cancel` has
that behavior on every platform. On web only, after a click has
obtained pointer lock, a captured-to-uncaptured transition additionally stands
in for an Escape swallowed by the browser; ordinary desktop capture loss does
not open the menu. Attribute this fallback to the pure
`capture_loss_opens` law and the `watch_capture` adapter, and describe it as
pure-law plus inspected-adapter evidence—not browser-proven behavior, because
the current web smoke sends no Escape/pointer-lock transition. Preserve the
agent-relevant web `?demo` entry contract: it arms the ordinary
composition-root demo-tap schedule for an input-less watch run and does not
replace the normal player controls. Name `movement_test.gd`, `cane_test.gd`,
`settings_test.gd`, `game_root_test.gd`, the pure `settings_menu` tests, native
display-probe Escape checks, and the real web smoke path as evidence of their
actual strength. Classify exact prior mouse-mode restoration as an inspected
source contract: the headless settings tests prove prior-pause restoration but
do not obtain or assert mouse capture, and the current native display probe
exercises Escape without asserting the restored mouse-mode value. Do not turn
either into a stronger mouse-restoration oracle.

- [ ] **Step 4: Write the level census and injection contract**

Describe recursive designer-object census, dependency injection before tree
entry, pure derived plans, typed spawn depth-first selection/faults, and
nearest-source demo targeting with deterministic tie break. State that a plain
named `Marker3D` is not a spawn.

- [ ] **Step 5: Write walls, runs, slabs, placement, and prefab contracts**

Cover snapped wall geometry, wall runs/openings as actual gaps, derived floor
and runtime/editor ceiling behavior, conservative placement faults, declared
wall/packing ceilings, and reusable scenes composed only from typed nodes.
For WaveWall, keep quarter-turn/world-space normalization, inherited-scale
discard, editor-live versus runtime-frozen length/geometry, ownerless generated
body/skin/collider, relayed collision settings/signals, and repairable
transform/length/collision warnings distinct. For WaveSpawn, keep first typed
depth-first selection, fallback, duplicate/missing faults, and the fact that a
plain named marker is not a spawn.
For WaveRun, state that `From`/`To` are parent-local `(X,Z)`—the displayed
`Vector2.y` is horizontal Z—and each opening is an absolute start on the
selected axis plus a width, not an offset from `From`; negative widths become
magnitudes. Describe `absorb_run_pose` mapping both interval endpoints into
parent-local data. Pre-tree setters accept and store authoring values without
building children; `WaveRun::ready` absorbs its planar pose and performs the
initial ownerless `RunSeg` build. Editor-after-ready setters and transform
notifications rebuild, while runtime-inside-tree endpoint/opening writes are
rejected and own-transform notifications reset the transform so retained wall
handles, paint bytes, and the occlusion snapshot stay frozen; ancestor prefab
transforms remain ordinary composition. Back those claims with the pure run-segment/pose/
lifecycle tests, `props_test.gd` runtime/doorway cases, and editor probe Phase
4. Avoid mutable scene counts and retired editor-debt prose.

- [ ] **Step 6: Run the complete checks-only game pipeline**

Run:

```sh
. tools/lib/engine.sh
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
GODOT="$GODOT_BIN" SKIP_EXPORT=1 ci/pipeline.sh
```

Expected: all repository, Rust, Godot, gdUnit, determinism, restore, editor,
and census checks pass.

When a real display is available, also run the native settings/input probe:

```sh
GODOT="$GODOT_BIN" tools/probe_display.sh
```

Require its Escape/settings cases to pass and confirm that its ignored
`game/override.cfg` is removed. If the execution host genuinely has no usable
display, record that environmental inability and retain the page's honest
pure-law/Godot-test evidence classification; never relabel an unrun probe as
observed evidence.

- [ ] **Step 7: Request independent mechanics review and commit**

Review the two pages against `movement_test.gd`, `cane_test.gd`,
`settings_test.gd`, `game_root_test.gd`, `level_test.gd`, `props_test.gd`,
`map_test.gd`, the native display and editor probes, web smoke, and pure Rust
movement/cane/settings/seed/level-plan tests. Resolve proven findings, preserve
each evidence-strength caveat, rerun focused tests and the affected evidence,
then commit the green slice.

### Task 7: Give Agents a Setup, Authoring, and Tool Capability Map

**Files:**

- Create: `docs/current/engineering/setup.md`
- Create: `docs/current/engineering/editor-authoring.md`
- Create: `docs/current/engineering/tooling.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: current bootstrap/run/probe scripts, pins, platform declarations,
  editor laws, and tool comments/tests.
- Produces: task-oriented setup/authoring guidance and an exact, dynamically
  checked parent-tool inventory.

- [ ] **Step 1: Write failing engineering-page and inventory tests**

Require all three pages, supported architecture/pin boundaries, editor
composition laws, and a `## Tool registry` table. Require owner/evidence rows
for setup pins, platform/bootstrap boundaries, editor composition, node
registration/census/warnings, `WaveSpawn`/`choose_spawn`, recursive typed
prefab discovery/ownerless generated limbs, and the tool-inventory derivation itself; use the
engineering evidence kinds defined in Task 3 rather than mislabelling source,
shell, or archive checks as mechanics observations. Compute expected tool
paths with `expected_parent_tools`; assert exact set equality, no duplicates,
no `tools/superpowers/` child, and nonempty `Kind`, `Purpose`, and `Use when`
cells. Assert every `Kind` is the exact Task 3 execution-context enum and
matches the file's host/language/library/gitlink role.

- [ ] **Step 2: Run the focused tests and observe the missing pages**

Run:

```sh
python3 -B test/documentation_contract_test.py SetupDocumentationTest EditorDocumentationTest ToolRegistryTest -v
```

- [ ] **Step 3: Migrate fresh-host setup facts**

Write platform prerequisites, pinned Godot/Rust selection, POSIX and Windows
bootstrap entry points, import/census success criteria, architecture boundaries,
and total failure behavior. Preserve the exact class-load recovery boundary:
`MissingNode` means bootstrap/load failed, a successful `bootstrap: OK` is
required, and the agent must quit every Godot process before reopening because
the failed extension is not retried in-process. Cover `tools/run_game.sh` and
its always-removed `game/override.cfg` windowed-run boundary. State the remaining pinned-Godot acquisition
boundary neutrally with existing issue #38; do not retain a prose task list.

- [ ] **Step 4: Migrate editor-authoring facts**

Explain opening `game/`, the separate F5 `main.tscn`/level-01 fallback and F6
raw-`WaveLevel` versus `UnseeingGame.level_scene` runner semantics, typed Rust tool nodes, plain `.tscn`
prefabs, live derivation, warning channels, ownership/ownerless blueprint
children, editor-only ceiling behavior, the `editor-docs` build/registration
boundary that makes Rust comments visible in the Inspector, the complete
new-object checklist, and the ignored temporary `game/override.cfg`. Keep
shipped GDScript forbidden. Preserve the concrete `WaveSpawn` authoring
boundary: add the typed class rather than a named `Marker3D`, first typed
depth-first selection wins, and missing/duplicate spawns surface warnings.
Preserve reusable `PackedScene` roots with recursively censused typed nested
children, while Rust-generated ownerless limbs are rebuilt rather than saved.

- [ ] **Step 5: Inventory every currently present parent tool**

Create one concise row for each dynamically discovered path. List each OS
wrapper separately, distinguish executable entry points from sourced support
libraries, and describe `tools/superpowers` once as the sole opaque,
developer-only gitlink. Include `tools/documentation_markdown.py`,
`tools/documentation_contract.py`, and `tools/check-docs.py`; later tasks add
the mirror/publisher rows in the same commit that creates those paths.

Use this exact machine-readable header:

```markdown
| Path | Kind | Purpose | Use when |
| --- | --- | --- | --- |
```

- [ ] **Step 6: Kill realistic inventory mutations**

Using fixture copies, remove one row, duplicate one path, blank one purpose,
add and Git-index `tools/unregistered-probe.sh`, change the Superpowers index
mode, give a POSIX script the generic kind `tool`, swap each Python
command/library kind, swap each Python command/library mode/shebang contract,
and add a row beneath the gitlink. Each mutation must fail `ToolRegistryTest`
for the intended reason.

- [ ] **Step 7: Request review and commit the three pages**

Verify every purpose against the tool's source header and behavioral test,
rerun the focused tests, stage the pages/test, and commit under the global
rules.

### Task 8: Migrate Current Build, Debugging, and Agent Procedures

**Files:**

- Create: `docs/current/engineering/build-test-deploy.md`
- Create: `docs/current/engineering/debugging.md`
- Create: `docs/current/engineering/agent-workflow.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: durable facts from the five removed surfaces and current scripts,
  workflow, observer, and deployment owners.
- Produces: three current procedure pages ready for the atomic authority
  cutover in Task 9; legacy sources remain until that cutover is green.

- [ ] **Step 1: Write the failing engineering-procedure tests**

Require the new pages, their owner/evidence rows, and no current-doc
task/backlog/count, mutable-branch, or checkout-home patterns. The test must
prove every durable fact selected from the old procedure/report/MCP documents
has a destination before Task 9 removes those sources.

- [ ] **Step 2: Run the focused tests and observe the missing-page failures**

Run:

```sh
python3 -B test/documentation_contract_test.py EngineeringProcedureTest -v
```

- [ ] **Step 3: Migrate build, test, and deployment procedure**

Document cheap gates, checks-only/full pipeline, Rust/Godot/gdUnit/editor/wasm
and browser stages, platform exports, deterministic probes, production archive
boundary, and the user-controlled game deployment boundary. Preserve durable
gdUnit vendoring law: `ci/gdunit4.lock` owns the pin,
`ci/vendor-gdunit4.sh update <tag>` is the sole update path, verification must
pass, and the addon updater remains disabled so changes are reviewed bytes.
Preserve durable
infra topology and recovery facts without copying a host checkout path. State
that integrating documentation never authorizes game deployment. At this task
boundary, make no current-behavior claim about automatic Wiki publication;
Task 15 adds that claim atomically with its workflow owner and evidence.

- [ ] **Step 4: Migrate debugging and observability procedure**

Document structured observers, snapshot/explain limitations, mesh readback,
the optional ignored MCP addon installation/loop, dump-scene fallback,
on-demand native rendered probes, and screenshot-last policy. Preserve separate
diagnoses for MissingNode/version/bootstrap failure, raw-level versus
`UnseeingGame.level_scene` injection failure, the legitimate black first frame
versus an injection error, and missing ignored `game/addons/godot_mcp` after
`tools/setup-mcp.sh`. Preserve the deterministic MCP loop as the exact closed
five-step sequence: freeze first; inject the tap/walk with `godot_input` while
frozen; advance only by an explicit `godot_game_time` step; read
`observer.snapshot`; then request explain/`WaveObserver::take_explanation` only
when the snapshot is insufficient. The input must occur between freeze and
step so it creates the deterministic state being observed. The observer remains
callable while paused because its adapter uses `ProcessMode::ALWAYS`. State that unavailable and
unknown structured results are meaningful bounded outcomes, not permission to
guess from a screenshot. Preserve the non-MCP fallback exactly: the same
observables and invariants remain exercised headlessly through the vendored
gdUnit4 suites invoked by `ci/pipeline.sh`; do not substitute an ad-hoc dump or
generic `--headless` invocation for that evidence. Keep the old MCP
page in place until Task 9 switches its caller and removes it atomically.

- [ ] **Step 5: Migrate the agent workflow**

Document the authority order, pinned Superpowers workflow, isolated worktree,
brainstorm/spec/plan gates, strict TDD, per-task reviews, verification, and the
finish-branch choice. Keep host-specific memory and external mutation out of
the procedure.

- [ ] **Step 6: Run focused procedure and owner/link tests**

Run:

```sh
python3 -B test/documentation_contract_test.py EngineeringProcedureTest -v
```

- [ ] **Step 7: Request review and commit the three procedure pages**

Review migrated facts against their former source and current code/tool owner,
rerun the focused checks, and commit the green migration. No authority path is
deleted or redirected in this task.

### Task 9: Cut Every Authority Router Over to One Ordered Manifest

**Files:**

- Create: `docs/README.md`
- Create: `docs/wiki-pages.tsv`
- Modify: `AGENTS.md`
- Modify: `README.md`
- Modify: `game/README.md`
- Modify: `infra/README.md`
- Modify: `game/shaders/data_core.gdshaderinc`
- Modify: `rust/src/nodes/settings.rs`
- Modify: `rust/src/cat_brain.rs`
- Modify: `rust/src/render/labels.rs`
- Modify: `game/tests/settings_test.gd`
- Modify: `tools/setup-mcp.sh`
- Modify: `test/documentation_contract_test.py`
- Delete: `docs/opening-in-godot.md`
- Delete: `docs/agent-workflow.md`
- Delete: `docs/reports/2026-08-13-agent-portability-audit.md`
- Delete: `docs/superpowers/mcp/godot-mcp-loop.md`
- Delete: `docs/screenshot.png`

**Interfaces:**

- Consumes: all eleven current pages, the three reduced READMEs, five
  superseded paths, three inline remedy passages, two stale shader-comment
  claims, two stale settings-comment claims, project policy, and scoped
  entry points.
- Produces: one AI routing index, twelve unique ordered Wiki publication
  entries, an explicit source-migration proof, and an atomic
  policy/README/MCP/inline-prose cutover with no broken intermediate links.

- [ ] **Step 1: Write failing index, manifest, and authority-cutover tests**

Require the first routing decision to distinguish policy, mechanics,
engineering procedure, live work, and historical rationale. Require every
current page exactly once in the index and manifest, `docs/README.md` exactly
once in the manifest, no README router/spec/plan/report, and stable manifest
order. Also require exact `CLAUDE.md` bytes; local-doc/code-wins/update/direct-
Wiki-edit policy; short scoped routers; no removed-path references on current
authority surfaces; and no orphaned documentation media. At this task boundary
the test must reject a claim that automatic Wiki publication already ships;
Task 15 changes that assertion atomically with the workflow. Frozen spec/plan
bodies remain provenance text and are exempt from removed historical-path link
resolution; their registry/current-result links remain validated.

Add `MigrationLedgerTest` with the exact global ledger rows. For every
`migrate` family, require its destination page plus the stable owner/evidence
token named by the ledger; for every `remove` family, require representative
obsolete text/media absent from every current page. Assert set equality with
the three reduced READMEs, four removed text paths, and removed screenshot so
a source cannot silently fall out of the audit. Independently run the bounded
production-line-comment scan and require it to fail initially on exactly the
three audited remedy passages plus the two path-scoped stale shader claims
containing `EVERY wave obeys this` and `never through a wall`, and the two
path-scoped stale settings claims `PAUSE IS OWNED HERE, AND RELEASED ON THE WAY
OUT` and `the pause it owns`, while retaining their surrounding current facts.
The settings-test witness is a migration-specific comment check; it does not
expand the ordinary production-comment scan to test fixtures. Line numbers
may appear only in diagnostics; the immutable witnesses are path plus exact
phrase, not mutable offsets.

- [ ] **Step 2: Run the focused test and observe missing index/manifest failures**

Run:

```sh
python3 -B test/documentation_contract_test.py DocumentationIndexTest ManifestTest AuthorityCutoverTest MigrationLedgerTest InlineDocumentationTest -v
```

Expected: FAIL on the pre-cutover authorities, the ledger's not-yet-applied
remove dispositions, three inline remedy passages, and two stale shader claim
blocks plus two stale settings comment blocks. Its migrated-fact rows
must already pass against Tasks 4–8; no tracked source has yet been removed or
rewritten.

- [ ] **Step 3: Write the sole local documentation index**

Make `docs/README.md` route directly to five mechanics pages, six engineering
pages, `AGENTS.md`, GitHub Issues, and the existing `docs/superpowers/`
artifact directory. Task 16 replaces that directory link with the concrete
registry file as soon as it exists. Explain in one sentence that code wins,
repository docs are canonical, and Wiki content is non-authoritative; do not
claim automatic generation yet or duplicate page content.

- [ ] **Step 4: Write the ordered TSV manifest**

Use literal tab separators and this exact ordered four-field manifest:

```text
# source	slug	title	section
docs/README.md	Documentation	Documentation	Start
docs/current/mechanics/overview.md	Mechanics-Overview	Mechanics Overview	Mechanics
docs/current/mechanics/rendering.md	Mechanics-Rendering	Rendering	Mechanics
docs/current/mechanics/waves.md	Mechanics-Waves	Waves	Mechanics
docs/current/mechanics/sound-sources.md	Mechanics-Sound-Sources	Sound Sources	Mechanics
docs/current/mechanics/levels-and-objects.md	Mechanics-Levels-and-Objects	Levels and Objects	Mechanics
docs/current/engineering/setup.md	Engineering-Setup	Setup	Engineering
docs/current/engineering/editor-authoring.md	Engineering-Editor-Authoring	Editor Authoring	Engineering
docs/current/engineering/build-test-deploy.md	Engineering-Build-Test-Deploy	Build, Test, and Deploy	Engineering
docs/current/engineering/debugging.md	Engineering-Debugging	Debugging	Engineering
docs/current/engineering/agent-workflow.md	Engineering-Agent-Workflow	Agent Workflow	Engineering
docs/current/engineering/tooling.md	Engineering-Tooling	Tooling	Engineering
```

- [ ] **Step 5: Rewrite policy, scoped routers, MCP route, and inline prose**

Update only the `AGENTS.md` documentation routing and obsolete MCP reference
while preserving every non-documentation law and its 24 KiB bound. Keep
`CLAUDE.md` byte-exact. Reduce root, `game/`, and `infra/` READMEs to scope,
the canonical index route, and essential entry-point context. Point
`tools/setup-mcp.sh` at `docs/current/engineering/debugging.md`.

Make comment-only edits in five files. In
`data_core.gdshaderinc`, retain the actual `O(live pulses × walls)`
bound/radius gate and current WebGL2 smoke evidence, but remove the shadow-map
“next step” and deferred profiling remedy. In the same file, replace the two
stale broad claims that every wave stops at walls and that a source reveals
only through doorways with the implemented split: kinds 0/1/2 return zero
after any counted source-side wall crossing; kind 3 returns
`pow(HUM_THROUGH, float(blocked))`; `wall_crossings_from` omits the birth
wall; therefore a world source can reveal a surface through a wall, dimmed
rather than black.
In `rust/src/nodes/settings.rs`, replace the stale module comment that says the
overlay owns/releases pause and unconditionally unpauses on exit with the
implemented contract: the `ProcessMode::ALWAYS` adapter borrows and records the
pre-open pause and mouse modes; ordinary close restores both, while tree exit
restores only the prior pause so teardown cannot strand the tree frozen. In
`game/tests/settings_test.gd`, replace the stale
header claim that the overlay owns the pause with the exact test boundary: the
suite proves open/close behavior and restoration of an already-paused tree,
while headless execution neither obtains nor asserts mouse capture.
In `cat_brain.rs`, retain that physics-float branches are deterministic per
platform and continuous outputs stay smooth, but remove the future
cross-platform quantization proposal. In `render/labels.rs`, retain the current
independent ownership boundary—Rust `MIN_SEP` governs allocation and the
hearing shader's upper knee governs rendered response—but remove the campaign
history and missing-gate work statement. Compare the pre/post shader after
removing line comments and whitespace and require every remaining token to be
identical; likewise require the Rust syntax outside the edited doc-comment
ranges byte-exact. Require both settings files to remain byte-exact outside the
edited comment ranges and rerun their Rust and gdUnit evidence. Do not change a
code token, numeric literal, shader expression, public API contract, or
behavior; the later issue rollout is the only owner of residual work.

- [ ] **Step 6: Remove superseded sources and media**

After making every ledger row green against Tasks 4–8, delete the five listed
paths. Leave frozen spec/plan provenance text untouched. A reviewer must see
the complete ledger result, not a generic assertion that the old documents
were read.

- [ ] **Step 7: Run link, heading, index, policy, and archive checks**

Run:

```sh
python3 -B test/documentation_contract_test.py -v
sh test/repo_hygiene.sh
sh test/deployment_archive_test.sh
sh test/shell_syntax_test.sh
ci/verify-superpowers.sh metadata
. tools/lib/engine.sh
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
GODOT="$GODOT_BIN" SKIP_EXPORT=1 ci/pipeline.sh
```

Expected: PASS. The live repository CLI remains gated until the artifact
registry appears in Task 16; `CLAUDE.md` is unchanged, `AGENTS.md` is within its
size gate, the comment-only Rust/shader edits compile and pass the complete
checks-only game pipeline, the settings source/test comment edits pass their
Rust and gdUnit evidence, and the Superpowers gitlink is exact. Inspect all five
comment-only diffs independently and prove every non-comment token stream is
byte-identical to its parent before accepting the green result.

- [ ] **Step 8: Request review and commit the atomic authority cutover**

Review route completeness, TSV bytes, every ledger disposition and migrated
fact, inline-comment semantics, policy preservation, and link resolution.
Stage only the listed cutover paths and commit under the global rules.

### Task 10: Render Commit-Pinned Pages Without a Second Template

**Files:**

- Create: `tools/wiki_mirror.py`
- Create: `tools/render-wiki.py`
- Create: `test/wiki_renderer_test.py`
- Modify: `docs/current/engineering/tooling.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: a repository path, an exact full commit SHA, and an explicit output
  directory.
- Produces: one stamped regular `.md` file per manifest entry, using only Git
  objects from the requested commit; no working-tree content or network. This
  is an explicitly provisional page-rendering component: it has no mirror
  marker, complete-tree verifier, supported-format dispatch, or publishable
  compatibility promise until Task 11 assembles and freezes the whole format.

- [ ] **Step 1: Write a failing committed-tree renderer test**

Create a temporary Git repository with a valid manifest/page, commit it, then
dirty the working page. Call:

```sh
python3 -B tools/render-wiki.py render-pages-provisional \
  --repo-root "$SOURCE" --source-sha "$SHA" --output-dir "$OUT"
```

Assert the rendered page contains committed bytes, not dirty bytes, begins
with a generated/read-only notice naming the full SHA and source path, and is
mode `0644`. Require the provisional operation to emit only the manifest page
set and to expose neither `CURRENT_FORMAT`, `FORMAT_RENDERERS`, a `--format`
option, nor any state/verification operation. A test that expects a complete
publishable tree must fail at this task boundary.

- [ ] **Step 2: Write failing renderer-boundary tests**

Cover malformed/duplicate manifest rows, reserved/invalid slugs, missing
source, traversal, a manifest-source symlink, both inside- and outside-pointing
linked-target symlinks, invalid UTF-8, abbreviated/non-commit SHA, output equal
to the repository root, and a supplied repository path that is relative
(`.`), has an absolute `..` spelling or trailing-separator alias, traverses a
symlink component, is a subdirectory or
superdirectory, or is another textual spelling of the root instead of the
single canonical absolute output of
`git rev-parse --path-format=absolute --show-toplevel`. Trailing-separator
rejection is a raw-string CLI invariant because a library `Path` does not retain
that spelling; the library enforces every remaining canonical-root rule. Cover a local
`refs/replace/*` that replaces the requested commit/tree, a present
`$GIT_COMMON_DIR/info/grafts`, `objects/info/alternates`,
`objects/info/http-alternates`, each partial-clone or
promisor config spelling, and a promisor fixture whose requested tree has a
missing blob; all refuse, and the promisor fixture's upload-pack/network
sentinel remains untouched. Also inject every scrubbed inherited `GIT_*`
override and prove it cannot redirect discovery or object reads. Cover an output parent
symlink resolving into either the actual Git directory or common Git
directory, an output-root symlink passed to verification, an existing
non-directory output parent, and a dirty working manifest. Include a real
linked-worktree fixture whose `.git` is a pointer file and a symlinked parent
into its common Git directory. Assert every
pre-existing target remains byte-exact after refusal. Every Git mode `120000`
source or target in the superproject commit is refused rather than resolved;
the scan never descends the mode-`160000` `tools/superpowers` gitlink or treats
initialized submodule contents as superproject entries. The latter
dirty-manifest case must still render committed bytes.

Before implementation, add a failing end-to-end `LinkRewriteTest` covering an
inline link, image, angle destination, balanced and escaped parentheses, one
unique reference definition plus its unchanged uses, same-page fragment,
local regular file, directory, asset, external `http`/`https`/`mailto`,
canonical issue URL, unresolved target, unresolved fragment, and all code
exclusions. Assert exact full-SHA blob/tree URLs, URL quoting, unchanged
same-page/external/issue/code bytes, and exactly one replacement at the
reference definition. Give committed source/target paths separate fixtures
containing spaces, `%`, `#`, and non-ASCII UTF-8, and assert exact Git-object
lookup plus the correctly quoted commit-pinned URL for each.

- [ ] **Step 3: Run the renderer tests and observe the missing CLI failure**

Run: `python3 -B test/wiki_renderer_test.py WikiPageRenderTest -v`

Expected: FAIL because `tools/render-wiki.py` and `tools/wiki_mirror.py` are
absent.

- [ ] **Step 4: Implement the exact Git-object reader**

Build one closed Git-subprocess environment before repository discovery. Start
from an allowlist of necessary non-Git process keys and omit every inherited
key whose name begins `GIT_`, plus `SSH_ASKPASS` and curl trace/credential
keys; do not try to enumerate today's override names. Add back only
`GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`,
`GIT_TERMINAL_PROMPT=0`, `GIT_NO_REPLACE_OBJECTS=1`, and
`GIT_NO_LAZY_FETCH=1`; and invoke every Git command as the resolved absolute
Git executable with `--no-replace-objects -C "$repo_root"`. Reject any
`refs/replace/*`, any `lstat` entry at either resolved Git/common-Git
`info/grafts`, any `objects/info/alternates` or `objects/info/http-alternates`
resolved through `git --git-path`, and effective
`extensions.partialClone`, `remote.*.promisor=true`, or
`remote.*.partialCloneFilter` configuration before reading authority, even
though replacement lookup and lazy fetching are disabled. Require
`git rev-list --objects --missing=print "$source_sha^{commit}"` to report no
missing reachable object; a promisor/partial clone must fail locally rather
than lazily fetch. Tests inject every scrubbed override plus replacement,
graft, and promisor objects and prove neither selected bytes nor network access
changes.

Resolve the requested SHA with `git rev-parse --verify "$source_sha^{commit}"`, require
the returned forty lowercase hexadecimal characters to equal the argument,
and read manifest/pages/linked targets with `git cat-file` or `git show` at
that commit. Inspect `git ls-tree` modes and reject every symlink source or
link target, even when its text would resolve inside the repository, so a Git
symlink is never mistaken for file content. Operate on the superproject tree
only and reject gitlinks as link targets without traversing their worktrees.
Validate the raw CLI spelling before constructing/normalizing a `Path`: it
must be absolute and byte-equal to `os.path.normpath(raw)`, so `.`, `..`, a
trailing separator, and equivalent spellings cannot be erased before the
check. Walk its existing components with `lstat` and reject any symlink. Run
the hardened
`git -C "$raw" rev-parse --path-format=absolute --show-toplevel`, require its
single normalized output string to equal `raw`, then strict-resolve both and
require equality. The library applies the same canonical-root invariant to
its `Path` argument. Reject a subdirectory,
superdirectory, symlink alias, or second path spelling rather than widening the
read boundary.
Decode Markdown strictly as UTF-8.

- [ ] **Step 5: Implement commit-pinned link resolution**

Use only the already frozen prospective-format-1 primitives, beginning with
`parse_markdown_format_1`, and resolve each
repository-relative destination against its source page in the commit tree.
Decode the link's raw Markdown escape sequences exactly once through
`decode_destination_format_1` before classifying or resolving it; preserve the
raw range and apply replacements only through
`rewrite_destinations_format_1`. Preserve same-page
fragments. Rewrite regular files/assets to:

```text
https://github.com/cleveralbatraoz/unseeing/blob/{source_sha}/{path}#{fragment}
```

Rewrite directories with `tree` instead of `blob`. Leave `https`, `http`,
`mailto`, and GitHub issue links unchanged. Validate the target and heading
fragment before rewriting; reject every unclassified destination with source
location. Never touch code spans/fences/indented code.

Prepend each page with exactly one blockquote sentence whose stable fields are
the words `Generated mirror - do not edit`, the full source SHA, the canonical
repository blob URL, and the canonical source path. Separate the notice and
unchanged canonical body with one blank line. Encode URL path segments with
`urllib.parse.quote(..., safe="/")`.

Keep every page-byte or page-acceptance rule in one pure provisional function
that calls only `parse_manifest_format_1`,
`parse_markdown_format_1`, `decode_destination_format_1`,
`rewrite_destinations_format_1`, and `github_anchor_format_1`, never an
unsuffixed current-format alias. It never calls the current repository-wide
`validate_repository` or another unsuffixed validator; every acceptance rule
for its manifest/pages/links is owned by this path. These names reserve the
components that Task 11 will assemble into `render_format_1`; they do not yet
declare a complete supported mirror format. The shared Git-object/path readers
may be reused only where they cannot change page bytes or whether a page input
is accepted. No historical renderer code is ever executed.

- [ ] **Step 6: Make output creation bounded and transactional**

Require the explicit output directory not to exist. Strictly resolve its
existing parent and source root before leaf creation. Resolve Git metadata with
`git rev-parse --path-format=absolute --git-dir` and
`git rev-parse --path-format=absolute --git-common-dir`, then strictly resolve
both returned paths; never assume `<source>/.git` is a directory. Reject when
the resolved candidate equals or is below the resolved source root, Git
directory, or common Git directory, including through a symlinked parent.
Create only that absent leaf at mode `0700`, immediately `lstat` it as a real
directory, and write only top-level regular files at mode `0644`.
Verification/comparison also rejects a symlink output root. On error, cleanup
may remove only the exact directory inode created by this invocation and must
leave every pre-existing target byte-exact. The publisher always gives a fresh temporary directory;
stale Wiki removal belongs to exact Git-tree replacement, not recursive
deletion of an arbitrary renderer argument.

- [ ] **Step 7: Add the thin provisional page-render CLI**

Use `argparse` with required subcommand and arguments. Convert domain errors to
one-line stderr diagnostics and exit `1`; reserve exit `2` for invocation
errors. Expose only `render-pages-provisional` with no format argument or
verification operation. It is a task-local integration seam, not a supported
Wiki/publisher format; Task 11 replaces it atomically with the final complete
`render` and verification interface. Keep all rendering policy in
`wiki_mirror.py`. Give the CLI a Python shebang and mode `100755`; keep the
imported library at mode `100644` without a CLI main block.

- [ ] **Step 8: Register the two new tool paths in `tooling.md`**

Describe `tools/wiki_mirror.py` as a sourced/imported deterministic provisional
page-rendering library and `tools/render-wiki.py` as its explicitly
unpublishable provisional CLI. State that neither owns a supported mirror
format, state file, verification path, or workflow caller yet. Rerun
`ToolRegistryTest`; neither file may enter the tree undocumented or claim the
Task 11 boundary early.

- [ ] **Step 9: Stage the inventory candidate and run focused tests**

Stage the five task files before the live inventory check so
`expected_parent_tools` observes the two new tracked tool paths from the Git
index:

Run:

```sh
git add tools/wiki_mirror.py tools/render-wiki.py \
  test/wiki_renderer_test.py docs/current/engineering/tooling.md \
  test/documentation_contract_test.py
python3 -B test/wiki_renderer_test.py WikiPageRenderTest -v
python3 -B test/documentation_contract_test.py -v
sh ci/run_documentation_self_test.sh .
```

- [ ] **Step 10: Request review and commit the renderer slice**

Review Git-object isolation, path containment, UTF-8 errors, output-directory
safety, and Markdown exclusions. Stage the five task files and commit under
the global rules.

### Task 11: Generate Navigation, State, and an Exact Complete Tree

**Files:**

- Modify: `tools/wiki_mirror.py`
- Modify: `tools/render-wiki.py`
- Modify: `test/wiki_renderer_test.py`
- Modify: `docs/current/engineering/tooling.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: the provisional page component, an exact source commit, or a Wiki
  Git commit.
- Produces: deterministic `Home.md`, `_Sidebar.md`, `Mirror-Metadata.md`,
  `.unseeing-wiki-mirror`, verified `MirrorState`, and exact path/mode/byte
  comparison. This is the first complete supported mirror format and the point
  where format integer `1` becomes immutable.

- [ ] **Step 1: Write failing navigation and determinism tests**

Render the same commit into differently named parent directories. Assert exact
byte equality, manifest-order Home/sidebar navigation, full-SHA metadata, no
clock/hostname/workspace/branch text, and that a later fresh render omits a page
removed from the manifest. Task 12 proves replacement of the Wiki Git tree
removes the corresponding stale page.

Require `CURRENT_FORMAT == 1`, a closed
`FORMAT_RENDERERS == {1: render_format_1}` dispatch, identical omitted-format
and explicit-format-1 complete output, and defined refusal of zero, negative,
unknown, or removed formats. Require the old provisional CLI operation to be
absent: only a complete tree may be called `render` or selected by a format.

- [ ] **Step 2: Write failing state/digest tests**

Require this state grammar:

```text
format=1
source=[0-9a-f]{40}
sha256=[0-9a-f]{64}
```

The digest excludes the state file and hashes each sorted mapping entry as
eight-byte big-endian path length, path bytes, eight-byte big-endian content
length, then content bytes. Prove content edit, rename, deletion, extra file,
mode change, symlink, duplicate/malformed field, and forged old digest fail.
Require the parsed format to be an exact positive integer present in the
closed trusted compatibility-renderer dispatch. An unknown/removed format is
a defined refusal before any historical render.

- [ ] **Step 3: Write failing Git-complete-tree comparison tests**

Commit a valid generated tree, then independently vary a path, regular-file
byte, Git mode, symlink, gitlink, extra file, nonempty subtree, and empty
subtree. `compare_git_tree` must accept only the exact sorted top-level
regular-`100644` blob mapping; an empty directory object cannot disappear from
the verifier's view. Add top-level Git pathnames containing invalid UTF-8 bytes
or ASCII control characters and require a bounded one-line refusal before any
filesystem write or implicit subprocess decoding; valid non-ASCII UTF-8 remains
covered by the renderer link fixtures. Repeat with a replacement ref, graft file, and missing
promisor blob; every Git-state operation refuses without a network attempt.

- [ ] **Step 4: Run the focused tests and observe missing generated outputs**

Run:

```sh
python3 -B test/wiki_renderer_test.py NavigationStateTest CompleteTreeTest -v
```

- [ ] **Step 5: Implement navigation and metadata generation**

Build Home and sidebar exclusively from ordered `ManifestEntry` values. Group
the sidebar by first-seen section without sorting away manifest intent. Metadata
states the full source SHA, canonical source URL, and no-direct-edit contract.
Treat all four generated names as reserved independently of manifest case.

Use stable templates: `Home.md` has `# Unseeing Documentation`, the same
generated/no-edit notice, then one `## {section}` and one `- [{title}]({slug})`
line per entry; `_Sidebar.md` uses one bold section label and one
`- [{title}]({slug})` line per entry; `Mirror-Metadata.md` has
`# Mirror Metadata`, the no-direct-edit rule, full source SHA, and commit-pinned
source index link. End every generated text file with one LF and emit no trailing
spaces.

Assemble the page output and these generated files in one pure
`render_format_1` compatibility entry and install the closed integer dispatch
only now. The entry owns every output- or acceptance-affecting manifest,
Markdown, page/link, navigation, metadata, state, reserved-name, mode, and
complete-tree semantic. It calls only `parse_manifest_format_1`,
`parse_markdown_format_1`, `decode_destination_format_1`,
`rewrite_destinations_format_1`, and `github_anchor_format_1`, never an
unsuffixed current-format alias or `validate_repository`. The provisional
Task 10 function becomes an internal component of this entry and is not itself
a supported format.

- [ ] **Step 6: Implement length-delimited digest and state verification**

Use `hashlib.sha256`; gather only mode-`0644` regular files beneath the
generated root, reject nested directories/symlinks/nonregular or differently
moded entries, and compare with `hmac.compare_digest`.
`verify_generated_tree` parses exactly three unique lines and returns
`MirrorState` only after digest, supported-format dispatch, and complete-file
checks. The state records the validated integer format as well as the source
and digest; it never silently substitutes the current format.

Freeze format 1's complete output and accepted-input domain at this step. A
future change that changes any byte or tightens/widens acceptance adds a higher
format while retaining this complete compatibility entry; validation-only
tightening may not brick an earlier valid format-1 source. Historical source
renderer code is never executed.

- [ ] **Step 7: Implement exact Git-tree comparison**

Parse the NUL-delimited records from non-recursive
`git ls-tree -z "$commit^{tree}"` as raw bytes, not locale-decoded subprocess
text. Strictly decode each direct filename as UTF-8 and reject ASCII controls
before constructing a path or diagnostic. Require every top-level entry to have mode
`100644`, type `blob`, and a direct filename with no slash; reject every tree,
including an empty one, before comparing the exact path set and bytes obtained
from each blob object. Compare them to `lstat`-verified regular generated
files; do not trust a working checkout's file types or executable-bit
normalization.

Implement `verify_git_tree` over the same parsed Git-object mapping. It reads
and validates `.unseeing-wiki-mirror` from its blob, computes the digest from
the remaining blob bytes, and returns `MirrorState` without checkout, archive
extraction, filter application, or filesystem traversal. A malformed mode,
type, nested path, duplicate path, missing marker, or digest mismatch is a
defined refusal.

- [ ] **Step 8: Add verification CLI subcommands**

Replace the provisional CLI with a complete `render` operation that defaults
to `CURRENT_FORMAT` and accepts an explicit exact positive `--format` only for
compatibility rendering; unknown formats refuse. Also provide these exact
operations for the publisher:

```text
render-wiki.py verify-state --tree PATH
render-wiki.py state-field --tree PATH --field format|source
render-wiki.py verify-git-state --wiki-repo PATH --wiki-commit SHA
render-wiki.py git-state-field --wiki-repo PATH --wiki-commit SHA --field format|source
render-wiki.py compare-git-tree --wiki-repo PATH --wiki-commit SHA --generated-dir PATH
```

`verify-state` and `verify-git-state` print no content on success;
`state-field` and `git-state-field` print only the requested validated value;
comparison returns nonzero with a one-line reason. Every failure prints one
diagnostic line to stderr and no partial field value to stdout. The only field
choices are exactly `format` and `source`.

- [ ] **Step 9: Run determinism and realistic mutation checks**

Run the whole renderer test, the exact tooling contract, and the complete
checkout-only documentation self-test. Then load subject copies with path length omitted,
state included in its own digest, reserved-name case folding removed, mode
comparison removed, tree-entry rejection removed, and extra-file rejection
bypassed. Change every unsuffixed parser/decoder/rewriter/anchor alias and the
current repository validator/acceptance path, and prove explicit format-1
output and acceptance remain byte-exact. Mutate one format-1 navigation,
template, state, or acceptance rule and require the compatibility fixture to
fail. Each named mutation must fail its focused test.

Update `tooling.md` and its exact contract test in the same slice: both paths
now own a complete deterministic format-1 renderer/verifier usable by the
future publisher, but no publisher or workflow caller exists yet. The row may
not retain Task 10's provisional claim or claim automatic publication early.

```sh
python3 -B test/wiki_renderer_test.py -v
python3 -B test/documentation_contract_test.py ToolRegistryTest -v
sh ci/run_documentation_self_test.sh .
```

- [ ] **Step 10: Request review and commit the complete renderer**

Review byte ordering, tree types/modes, deterministic output, complete
format-compatibility closure, tooling truth, and CLI stdout contracts. Stage
only the five task files and commit under the global rules.

### Task 12: Lock Initial Takeover and Idempotent Local Publication

**Files:**

- Create: `tools/publish-wiki.sh`
- Create: `test/wiki_publisher_test.sh`
- Modify: `docs/current/engineering/tooling.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: a full local source repository, exact source SHA, local bare Wiki
  remote, and injected audited head in hermetic test mode; the read-only
  `audit-production-contract` operation consumes no repository or credential.
- Produces: one ordinary Wiki `master` descendant commit or an idempotent no-op;
  production credentials are unreachable in test mode. The audit operation
  prints the public production remote, branch, and audited head from the same
  internal values production publication consumes.

- [ ] **Step 1: Write failing local-bare takeover tests**

Build source and Wiki fixtures with local Git only. Test this interface:

```sh
tools/publish-wiki.sh test \
  --source-repo "$SOURCE" --source-sha "$SOURCE_SHA" \
  --wiki-remote "$WIKI_BARE" --audited-head "$LEGACY_SHA"
```

Require exact legacy-head takeover, refusal of another markerless head,
ordinary parentage, generated complete tree, mandated Git identity, and no
assistant/tool attribution in commit metadata or pages.

Before implementation, put a logging `git` wrapper first on `PATH` and require
the successful fixture trace to contain exactly one
`push --dry-run "$remote" "$candidate_sha:refs/heads/master"` immediately
before exactly one ordinary real push of the same refspec. Reject any
force/delete/leading-plus/wildcard ref, a real push without the preceding
dry-run, an extra push, or any push on an idempotent second run. This is the
initial externally visible push contract that Task 14 later extends with race
hooks and remote readback.

Define the test driver's selector contract now: one recognized selector runs
that suite, an unknown selector exits `2`, and no arguments run every suite
implemented at the current task boundary. At Task 12 that no-argument set is
exactly `takeover`; Tasks 13 and 14 extend the same aggregator rather than
creating tests CI cannot reach.

Also invoke `tools/publish-wiki.sh audit-production-contract` and require the
exact three-line public contract approved in the design:

```text
wiki_remote=https://github.com/cleveralbatraoz/unseeing.wiki.git
wiki_branch=master
audited_head=3780b28869c0ab53d8375a3b4211e6e7f3c15de3
```

This is an externally visible, hand-derived infrastructure contract, not a
source-text grep or duplicated gameplay constant.
Set hostile environment variables named like the remote, branch, and audited
head before invoking the audit operation and require the same exact output;
production pins are constants, never configuration seams.

- [ ] **Step 2: Add failing idempotence and exact-reset tests**

Run twice at one source SHA and assert the second run adds no commit. Force the
fixture remote back to the exact audited legacy head and assert a deterministic
retakeover succeeds. Force it to any other markerless commit and assert refusal
with the remote unchanged.

- [ ] **Step 3: Run the publisher test and observe the missing-subject failure**

Run: `sh test/wiki_publisher_test.sh takeover`

Expected: FAIL naming `tools/publish-wiki.sh`.

- [ ] **Step 4: Implement strict mode/argument parsing and isolation**

Use POSIX shell `set -eu`, a `mktemp -d` workspace, and a cleanup trap.
`test` mode requires absolute local source/remote paths and injected audited
head; it rejects production remote URLs and never reads a token or credential
helper. `production` and `verify-production` accept no test injections.
For every source/Wiki object or ancestry command, inherit Task 10's closed Git
environment, `--no-replace-objects`, replacement-ref/grafts refusal, and
replacement/alternate/partial-clone refusal plus `GIT_NO_LAZY_FETCH=1`.
Explicit clone/fetch/push operations are the only
network-capable Git calls; they never use a filter, and after each full fetch
the required source/Wiki reachability is proved with `rev-list
--objects --missing=print` before any authority decision. A missing promisor
object refuses rather than fetching implicitly.
`audit-production-contract` performs no Git, filesystem mutation, environment
guard, or credential access. Define the three production values once so audit,
production publication, and production verification cannot drift onto
different pins. In this task's commit, `production` and `verify-production`
fail immediately with an explicit “not enabled until guarded” diagnostic;
Task 14 removes that fail-closed stub only after its production and read-only
guard tests are red.

- [ ] **Step 5: Implement clone, marker classification, and takeover**

Initialize/fetch Wiki `master` with full history. Production and verification
obtain the remote, branch, and audited head from the single production
contract; test mode obtains its local remote and audited head from explicit
fixture arguments and exercises the same `master` state machine. If no marker
exists, require exact head equality with the mode's audited head and render the
requested source into a fresh directory.
Hash every verified mode-`0644` regular file into the temporary Wiki object
store, construct one flat sorted tree with `git mktree -z`, and create one
ordinary descendant with `git commit-tree -p`. Any other markerless head exits
before changing the remote.

- [ ] **Step 6: Implement generated-tree idempotence**

When the current Wiki commit already carries a state marker, defer full
provenance validation to Task 13, render requested source, build its Git tree,
and avoid a commit when the tree object equals `HEAD^{tree}`. Supply mandated
author/committer identity to `commit-tree`; never create a checkout, apply
filters, or write a credential helper/token into Git config. This tree-object
boundary makes stale-page deletion exact by construction.

- [ ] **Step 7: Push the local fixture with ordinary ref syntax**

For this first slice, use the same dry-run then real ordinary push path that
Task 14 hardens. Push the candidate commit object to `refs/heads/master` without
`--force`, `--delete`, a leading `+`, or a wildcard refspec.

- [ ] **Step 8: Register the publisher in `tooling.md`**

Describe it as the verified one-way Git boundary exercised by hermetic local
tests. State the current truth: its production entry point is fail-closed at
this task boundary, direct operator publication is prohibited, and its usable
mode is the hermetic local-bare-remote test boundary. Do not describe a future
workflow or future work in the current page.

- [ ] **Step 9: Stage the inventory candidate and run takeover tests**

Stage the four task paths first so the index-derived tool inventory sees the
publisher in the same candidate tree as its registry row:

Run:

```sh
git add tools/publish-wiki.sh test/wiki_publisher_test.sh \
  docs/current/engineering/tooling.md test/documentation_contract_test.py
tools/publish-wiki.sh audit-production-contract
sh test/wiki_publisher_test.sh takeover
sh test/wiki_publisher_test.sh
sh test/shell_syntax_test.sh
python3 -B test/documentation_contract_test.py ToolRegistryTest -v
sh ci/run_documentation_self_test.sh .
```

Load a subject copy with one nibble of the production audited head changed and
require the public-contract test to fail while hermetic injected-head scenarios
remain independent.

- [ ] **Step 10: Request review and commit the takeover slice**

Review temp cleanup, exact-head classification, test/production separation,
ordinary Git history, and identity. Commit the four task files under the global
rules.

### Task 13: Prove Historical Provenance and Source Ancestry

**Files:**

- Modify: `tools/publish-wiki.sh`
- Modify: `test/wiki_publisher_test.sh`

**Interfaces:**

- Consumes: a managed Wiki marker and full source history.
- Produces: publication only when the current Wiki tree independently
  reproduces from its recorded source and that source is an ancestor of the
  requested source; untrusted Wiki content is read only as typed Git objects,
  never checked out or extracted.

- [ ] **Step 1: Write failing managed-state corruption tests**

After a valid takeover, mutate a page without changing state, forge a matching
fast digest after the page edit, add a stray file, change a Git mode, commit a
symlink/gitlink, and malform the marker. Every case must fail before a new
commit or push.

- [ ] **Step 2: Write failing historical re-render tests**

Create several source commits after the recorded one. Require the publisher to
read the recorded SHA, verify its fast digest, independently render that old
source, and compare its exact tree to Wiki `HEAD`. Delete the historical source
object, use a shallow clone, install a source/Wiki replacement ref or graft, or
use a partial/promisor source with a missing object and assert a loud refusal
rather than takeover or lazy fetch.

Add an A-to-B format-upgrade fixture: Wiki `HEAD` is a valid format-1 render of
source A; the trusted current renderer now selects format 2 for source B while
retaining the frozen `render_format_1` compatibility entry. Require the
publisher to validate A with the current trusted format-1 function, render B
with current format 2, and create one ordinary format-2 descendant. Unknown
format, removed compatibility entry, using current format 2 to validate A, or
executing renderer code read from source A must all refuse or fail a named
mutation test. Make format 2 also tighten one manifest/Markdown acceptance
rule that A would violate; A must still reproduce through format 1. A mutant
that applies the new validation rule to the historical format must fail.

- [ ] **Step 3: Write failing source/legacy ancestry tests**

Cover forward source history, source rollback, source divergence, an older
valid generated Wiki descendant, loss of the audited legacy head from Wiki
ancestry, and exact-legacy-head retakeover. Forward/healable cases pass;
rollback/divergence/lost-legacy cases leave the remote unchanged.

- [ ] **Step 4: Run the provenance tests and observe false acceptances**

Run:

```sh
sh test/wiki_publisher_test.sh provenance
```

Expected: at least the forged-digest and source-rollback cases fail their test
until the independent checks exist.

- [ ] **Step 5: Validate the current managed tree independently**

For a managed head: require the audited legacy commit as an ancestor; call
`verify-git-state`/`git-state-field` to validate and read both the format and
source directly from mode-`100644` Git blobs; render the recorded source from
the full source repository into a separate fresh directory with that exact
format selected through the current trusted compatibility dispatch; call
`compare-git-tree` against Wiki `HEAD`; and only then consider the Wiki
trusted. Before the historical render or either Wiki/source ancestry query,
repeat the replacement-ref, graft, alternate, partial/promisor, missing-object,
and closed-environment checks from Tasks 10/12 and require both repositories
non-shallow. All proof commands use `--no-replace-objects` and no-lazy-fetch.
Render the requested new source with the current highest format. Do
not use `checkout`, `git archive`, `tar`, or filesystem materialization for
the untrusted Wiki commit, and never import or execute renderer code from a
historical source tree—especially inside the credential-bearing process.

- [ ] **Step 6: Enforce source ancestry before rendering the new source**

Use `git merge-base --is-ancestor "$OLD_SOURCE" "$NEW_SOURCE"`. Treat nonzero
as refusal, never as permission to merge, reset, or force. If Wiki was reset to
an older independently valid managed descendant, use that recorded source as
the ancestry base and safely regenerate forward.

- [ ] **Step 7: Preserve the exact-legacy exception narrowly**

When and only when current Wiki head exactly equals the injected/production
audited legacy head and has no marker, skip prior-source ancestry because no
source marker exists. The requested source still must be the explicit test
source or the production-guarded canonical-main event.

- [ ] **Step 8: Run provenance tests and kill validation mutations**

Run the full publisher suite. Then test subject copies with independent
historical rendering bypassed, source ancestry inverted, audited-head ancestry
removed, state digest trusted as provenance, and complete-tree comparison
reduced to file bytes. Also mutate the recorded-format dispatch to the current
format, apply format-2 validation to format 1, remove format-1 compatibility,
and permit an unknown format; each mutant must fail the A-to-B scenario. Require the
no-argument driver to run both `takeover` and `provenance` at this boundary, and
run `sh ci/run_documentation_self_test.sh .` before review.

- [ ] **Step 9: Request security/state-machine review and commit**

Review every accepting transition and early return against the design table.
Confirm no path converts invalid remote state into generated history. Commit
the two-file green slice under the global rules.

### Task 14: Make Push Races, Credentials, and Readback Fail Closed

**Files:**

- Modify: `tools/publish-wiki.sh`
- Modify: `test/wiki_publisher_test.sh`
- Modify: `docs/current/engineering/tooling.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: a fully validated local Wiki candidate, ordinary remote, and
  test-only hook seams around the real push.
- Produces: credentialed dry-run then one non-force real push, followed by
  remote ref/tree/marker readback; races fail without retry or overwrite.

- [ ] **Step 1: Write a failing end-to-end race/readback trace test**

Extend Task 12's already-green exact dry-run/real-push trace rather than
reasserting it. Put a logging `git` wrapper and logging before/after hook
fixtures around one successful publication. Require this complete order:
validated remote fetch, one dry-run, before-real-push hook, one ordinary real
push, after-real-push hook, a new remote-ref fetch, then exact ref/tree/marker
readback. Require no second real push, retry, merge, force, or post-readback
mutation. This test is necessarily red at the Task 13 boundary because neither
hook interface nor post-push fetched readback exists yet.

- [ ] **Step 2: Write failing before-push race test**

Add a test-only `--before-real-push-hook PATH`. The hook advances bare Wiki
`master` after dry-run. Require the publisher's ordinary push to fail, retain
the competing remote commit exactly, avoid merge/force/retry, and report that a
fresh run is required.

- [ ] **Step 3: Write failing after-push/readback tests**

Add a test-only `--after-real-push-hook PATH` that advances the remote between
the successful push and readback. Require a failing exit. Also mutate fetched
tree bytes/state and assert exact ref, complete tree, and source marker are all
checked.

- [ ] **Step 4: Write failing production guard tests**

Production must refuse unless all are exact:

```text
GITHUB_ACTIONS=true
GITHUB_REPOSITORY=cleveralbatraoz/unseeing
GITHUB_EVENT_NAME=push
GITHUB_REF=refs/heads/main
GITHUB_SHA=$checked_out_full_head
GITHUB_WORKSPACE=$source_repository_root
```

Test every missing/mismatched value independently and require refusal before
clone or credential use. Require a non-shallow, non-partial, clean source
checkout with no replacement refs, grafts, alternates, or missing reachable
objects and whose
`origin` is the canonical public repository. Give every mismatch a distinct
guard diagnostic and assert it, and require one exact valid fixture to proceed
past guarding to a controlled fake-Git boundary. Task 12's generic “not
enabled” stub therefore makes these tests red instead of satisfying them by
refusing everything; remove the stub only in the implementation step. Set
hostile remote/branch/audited-head environment variables in the valid fixture
and require Git traces still use the single audited production contract. Every
guard/discovery command—including HEAD, status, origin, shallow/partial, and
object checks—runs under the closed Git environment; an inherited
`GIT_INDEX_FILE` or any other `GIT_*` key cannot fake eligibility.
Before implementation, add a failing `ToolRegistryTest` assertion for the
intermediate current truth: production and read-only modes accept only their
exact Actions guards, no workflow caller is wired yet, direct operator
production use remains prohibited, and hermetic local test mode remains
usable. Task 12's fail-closed/hermetic-only row makes this assertion red.

- [ ] **Step 5: Implement one-shot push and race behavior**

Run dry-run, invoke the optional test hook, run one real ordinary push, and
propagate failure. Do not loop. The only recovery message directs a fresh run
from a fresh clone.

- [ ] **Step 6: Implement fetched remote readback**

Fetch `refs/heads/master` after push into a new local ref. Require it equals the
candidate commit; independently verify its generated state, exact complete
tree, and requested source marker. Invoke the optional post-push test hook only
in hermetic mode.

- [ ] **Step 7: Implement production and read-only guard modes**

Use Task 12's single audited production contract for the canonical HTTPS
remote, `master` branch, and legacy head. Production reads no token itself: Git
receives credentials only through the Action step's ephemeral `GIT_ASKPASS`.
`verify-production` requires the canonical repository and `gollum` event,
explicitly fetches Wiki `master`, validates its marker through the Git-object
interface, selects the marker's supported format through the current trusted
compatibility dispatch, renders that recorded source commit from full source
history without executing historical code, and ignores event `GITHUB_SHA` as
a Wiki revision. It never commits or pushes.
Atomically update the tooling row to the guarded-but-not-yet-wired truth tested
in Step 4; do not claim automatic publication until Task 15 creates the caller.

- [ ] **Step 8: Run push, race, guard, and secret-hygiene tests**

Run:

```sh
sh test/wiki_publisher_test.sh push
sh test/wiki_publisher_test.sh guards
sh test/wiki_publisher_test.sh
python3 -B test/documentation_contract_test.py ToolRegistryTest -v
```

Capture stdout/stderr/Git config/commit/tree in a fixture with a sentinel token
and assert the sentinel appears nowhere.

- [ ] **Step 9: Request security review and commit**

Review argument quoting, credential lifetime, refspecs, race timing, readback,
read-only mode, and intermediate tooling truth. Commit the four-file green
publisher hardening under the global rules.

### Task 15: Publish Only After Both Main Gates and Guard Direct Wiki Edits

**Files:**

- Modify: `.github/workflows/test.yml`
- Create: `.github/workflows/wiki-guard.yml`
- Create: `test/wiki_workflow_test.py`
- Modify: `AGENTS.md`
- Modify: `docs/README.md`
- Modify: `docs/current/engineering/build-test-deploy.md`
- Modify: `docs/current/engineering/tooling.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: GitHub `push`, `pull_request`, and `gollum` events.
- Produces: serialized publication only for successful main pushes, with up to
  GitHub's documented 100 pending Wiki jobs retained and overflow visibly
  cancelled; PR render/tests without publication; read-only direct-edit
  detection.

- [ ] **Step 1: Write failing workflow permission/dependency tests**

The bounded source validator must prove top-level `contents: read`, publication
job `contents: write`, exact `needs: [checks, windows-bootstrap]`, main-push
condition, an unauthenticated full-history fetch of canonical `main` through a
literal `origin` whose fetch/push URL is the canonical public repository, and
no `uses:` entry at all inside the write-capable job. Add a failing
documentation assertion that current build/tooling pages name the installed
workflow owner, both prerequisite jobs, automatic-only use, and the fact that
Wiki publication does not authorize game deployment. Add failing index/policy
assertions that `docs/README.md` and `AGENTS.md` now name the repository-to-Wiki
automatic mirror while retaining code/repository authority and the direct-edit
prohibition. Require the existing read-only `checks` job, for both pull requests
and pushes, to render the real checked-out full `GITHUB_SHA` into a fresh absent
temporary child and run `verify-state` on that tree before cleanup. The source
test rejects a working-tree/default-branch SHA, reused output directory,
publisher invocation, network Git command, token environment, or missing
verification/cleanup in this candidate-render step.

- [ ] **Step 2: Write failing queue, cancellation, and credential-scope tests**

Require pull-request cancellation but a unique `github.run_id` top-level group
for every main run, so main never shares the default single-pending queue.
Require a dedicated Wiki job group with `queue: max` and no in-progress
cancellation; reject the old shared-main group, default/single queue, missing
queue, and any `cancel-in-progress: true` at that job. Model three waiting main
publication jobs in the source test and require none of the first two is
replaced; separately encode the documented 100-pending bound and require
overflow to be described as a visible cancellation, not retried manually.
Expose `GITHUB_TOKEN` only on the final publisher step. Reject token
interpolation in a URL, Git config, earlier step, or workflow-level environment.
Require the final step to reset inherited credential helpers before forcing
`GIT_ASKPASS`; reject PAT/fallback secret names. Inject
`GIT_CONFIG_PARAMETERS` in the hostile environment fixture and require the
final step to unset it before any credential-bearing Git command. Inject
`GIT_DIR`, `GIT_OBJECT_DIRECTORY`, `GIT_REPLACE_REF_BASE`, replace refs, a
grafts file, and a promisor/lazy-fetch sentinel in source-level and hermetic
fixtures; require the first three are scrubbed and the latter three refuse
before publication.

- [ ] **Step 3: Write failing gollum-guard tests**

Require only `gollum`, explicit top/job `contents: read`, unauthenticated
full-history manual source fetch, no `uses:` action, no write permission/token,
and a call to `publish-wiki.sh verify-production`.
Treat the workflow and its invoked `publish-wiki.sh verify-production`
implementation as one transitive boundary: the workflow owns the explicit
source fetch, while the publisher owns the explicit Wiki `master` fetch and
must never substitute event `GITHUB_SHA` for Wiki state. Load and mutate both
subjects. Mutations replacing Wiki `master` with event `GITHUB_SHA`, deleting
either fetch, adding a push, widening permission, or allowing the job to
succeed after diagnosis must fail. The test proves handling of a delivered
page-change event; it must not assert that GitHub emits `gollum` for a
metadata-only commit or non-page ref write.

- [ ] **Step 4: Run the workflow test and observe missing-job/workflow failures**

Run: `python3 -B test/wiki_workflow_test.py -v`

- [ ] **Step 5: Add the live candidate gate and safe main publication scheduling**

In the existing `checks` job, after checkout and before the game stages, create
a mode-private temporary parent, name an absent child beneath it, render the
exact full `GITHUB_SHA` from the real checkout, run `verify-state`, and remove
only that temporary parent in a trap. Give this step no token, publisher call,
or network command. Thus a pull request and every main candidate exercise real
manifest paths, Git modes, links, fragments, gitlinks, and symlink refusal
before any write-capable job can become eligible.

Change top-level cancellation to a pull-request-only expression and make the
top-level group unique by `github.run_id` for main while preserving a shared
PR group. Add `publish-wiki` with exact needs, event/ref guard, manual Git
initialization, a literal `origin` whose fetch and push URLs are both the public
canonical repository, full-history fetch from that origin, detached checkout
at `GITHUB_SHA`, job-only write permission, a dedicated `queue: max` Wiki
concurrency group without cancellation, and one final publisher step. A queue
overflow fails visibly; do not add a manual publication fallback. Do not use
`actions/checkout` in the write-capable job: it would receive the implicit
token even with credential persistence disabled.

- [ ] **Step 6: Add the non-logging ephemeral credential helper**

In the final step, create a mode-`0700` temporary `GIT_ASKPASS` script under a
mode-`0700` temp directory, return `x-access-token` for username and the step's
`GITHUB_TOKEN` environment for password, set `GIT_TERMINAL_PROMPT=0` and
`GIT_ASKPASS_REQUIRE=force`, and use Git's environment config
count/key/value mechanism to reset `credential.helper` without writing Git
config or repurposing `HOME`. Set `GIT_CONFIG_NOSYSTEM=1` and
`GIT_CONFIG_GLOBAL=/dev/null` so inherited URL rewrites, helpers, aliases, or
trace settings cannot affect the credential-bearing operation; explicitly
unset `GIT_CONFIG_PARAMETERS`, repository/object-store overrides,
`GIT_REPLACE_REF_BASE`, and Git/curl trace variables; set
`GIT_NO_REPLACE_OBJECTS=1` and `GIT_NO_LAZY_FETCH=1`. Invoke
`tools/publish-wiki.sh production`, and remove the directory in a trap. Never
echo shell tracing or token-bearing values. The workflow test injects hostile
system/global URL rewrites, `GIT_CONFIG_PARAMETERS`, and trace configuration
and proves the final step neutralizes them. This credential-bearing network
child uses a second allowlisted environment: it starts from the same no-`GIT_*`
base and adds the exact askpass and count/key/value entries described here plus
`GITHUB_TOKEN` only for the publisher's credential-bearing dry-run and sole
real-push Git children. No earlier verifier/fetch/render process receives that
key, and the step removes it with the temporary helper afterward. Behavior
tests give the token a sentinel value, prove the askpass child can answer the
password prompt, and require the sentinel absent from argv, Git config, trace,
stdout/stderr, and every non-push child; the environment never inherits
arbitrary indexed Git configuration.

- [ ] **Step 7: Add the read-only gollum workflow**

Manually initialize and fetch full public canonical-main history without a
token. Invoke `verify-production`; it reads the Wiki marker and addresses that
recorded source commit from the fetched history. Then unconditionally exit
nonzero for every delivered human/external-token Wiki page-change event, even
when its bytes reproduce a generated tree. The publisher fetches Wiki
`master`; the workflow never passes event `GITHUB_SHA` as Wiki state and has no
mutation step. State the coverage honestly: `gollum` delivery is a page-event
guard, not authentication of metadata-only Git commits or arbitrary ref
writes; publisher readback and the next complete-tree/provenance check remain
the backstop for undelivered tree drift. A normal publisher push uses
`GITHUB_TOKEN` and therefore does not recursively trigger this workflow; an
unexpected recursive event is intentionally visible.

- [ ] **Step 8: Document the now-shipped automatic boundary**

Update `build-test-deploy.md` with the main-only post-green publication path,
its two job dependencies, failure/readback behavior, and the separation from
game deployment. Update the publisher's tooling row from its
guarded-but-not-yet-wired Task 14 state to its current automatic role plus
hermetic-test role. Add
owner/evidence rows pointing to the workflow and its source/publisher tests;
do not document a manual production invocation. In the same commit, update
`docs/README.md` and only the documentation-routing paragraph of `AGENTS.md`
from non-authoritative/direct-edit policy to the now-current automatic mirror
contract. The workflow, tests, policy, index, and engineering prose change
atomically; no earlier commit claims the automation ships. Document that the
guard fails delivered page-change events, while publication readback and later
complete-tree/provenance verification cover tree drift; make no claim that
`gollum` observes metadata-only commits or arbitrary ref writes.
Name the `queue: max`/100-pending bound and state that a cancelled overflow run
is simply ineligible for issue migration and is never repaired by manual Wiki
publication.

- [ ] **Step 9: Run workflow and full checkout-only documentation tests**

Stage exactly the eight Task 15 paths first. To exercise those staged bytes
rather than the preceding `HEAD`, clone the current repository without local
hard links into a private temporary directory, apply the exact cached binary
diff there, require its index tree OID equals this worktree's staged tree OID,
and create a temporary commit only in that clone. Render and verify that
temporary commit; never write a candidate ref or dangling commit into the
real repository. Then run:

```sh
git add .github/workflows/test.yml .github/workflows/wiki-guard.yml \
  test/wiki_workflow_test.py AGENTS.md docs/README.md \
  docs/current/engineering/build-test-deploy.md \
  docs/current/engineering/tooling.md test/documentation_contract_test.py
python3 -B test/wiki_workflow_test.py -v
sh test/wiki_publisher_test.sh guards
sh ci/run_documentation_self_test.sh .
CANDIDATE_PARENT="$(mktemp -d)"
CANDIDATE_REPO="$CANDIDATE_PARENT/source"
CANDIDATE_PATCH="$CANDIDATE_PARENT/candidate.patch"
trap 'rm -rf "$CANDIDATE_PARENT"' EXIT INT TERM HUP
CANDIDATE_TREE="$(git write-tree)"
git diff --cached --binary --full-index HEAD -- \
  .github/workflows/test.yml .github/workflows/wiki-guard.yml \
  test/wiki_workflow_test.py AGENTS.md docs/README.md \
  docs/current/engineering/build-test-deploy.md \
  docs/current/engineering/tooling.md test/documentation_contract_test.py \
  >"$CANDIDATE_PATCH"
git clone --no-local --no-checkout . "$CANDIDATE_REPO"
git -C "$CANDIDATE_REPO" checkout --detach HEAD
git -C "$CANDIDATE_REPO" apply --index "$CANDIDATE_PATCH"
test "$(git -C "$CANDIDATE_REPO" write-tree)" = "$CANDIDATE_TREE"
git -C "$CANDIDATE_REPO" -c user.name='Dmitrii Galchenko' \
  -c user.email=dggrus@gmail.com commit --no-gpg-sign --no-verify \
  -m 'Ephemeral candidate for mirror verification'
CANDIDATE_SHA="$(git -C "$CANDIDATE_REPO" rev-parse HEAD)"
MIRROR_VERIFY_OUT="$CANDIDATE_PARENT/wiki"
python3 -B "$CANDIDATE_REPO/tools/render-wiki.py" render \
  --repo-root "$CANDIDATE_REPO" --source-sha "$CANDIDATE_SHA" \
  --output-dir "$MIRROR_VERIFY_OUT"
python3 -B "$CANDIDATE_REPO/tools/render-wiki.py" verify-state \
  --tree "$MIRROR_VERIFY_OUT"
rm -rf "$CANDIDATE_PARENT"
trap - EXIT INT TERM HUP
```

- [ ] **Step 10: Request workflow-security review and commit**

Review event expressions, permissions, dependencies, cancellation, action
provenance, token scope, gollum semantics, and truthfulness of the newly current
engineering prose. Commit the green workflows, docs, and tests under the global
rules. Immediately render and `verify-state` the resulting actual full `HEAD`
from this canonical worktree into a fresh absent temporary child, then rerun
the workflow and complete checkout-only documentation tests. A post-commit
failure is not waived or amended away: add a failing regression, fix it in a
separately reviewed green follow-up commit, and repeat the actual-`HEAD` gate.

### Task 16: Register Every Historical Artifact and Activate the Live Gate

**Files:**

- Create: `docs/superpowers/README.md`
- Modify: `docs/README.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: the discovered parent-repository spec/plan set, canonical result
  pages, existing issue numbers, and the transient state of these two active
  plans.
- Produces: one complete artifact registry and the marker that activates
  `tools/check-docs.py` on the live repository in every complete checkout.

- [ ] **Step 1: Write failing artifact-registry tests**

Require exact discovered set equality, one row per artifact, valid
kind/campaign/outcome/current-result/residual cells, unique residual issue
references within each row, and prose explaining that historical checkboxes
carry no live status. The same issue may appear on multiple artifacts when it
is genuinely shared. Require `active` for this design, this implementation
plan, and the post-integration issue plan until closeout.

- [ ] **Step 2: Run the focused test and observe the missing registry failure**

Run:

```sh
python3 -B test/documentation_contract_test.py ArtifactRegistryTest -v
```

- [ ] **Step 3: Create the registry with this exact disposition map**

Use this exact header; every artifact/result path is a repository-relative
Markdown link and every artifact appears in its own row:

```markdown
| Artifact | Kind | Campaign | Outcome | Current result | Residual issues |
| --- | --- | --- | --- | --- | --- |
```

Use one row per concrete path; expand grouped campaigns below rather than
using wildcard/group rows:

| Artifact or campaign | Outcome | Current result | Residual |
| --- | --- | --- | --- |
| debug-observability design | `superseded` | engineering/debugging | `#15` |
| debug-observability state-layer plan | `shipped` | engineering/debugging | `#15` |
| reproduction-loop design | `superseded` | engineering/debugging | `none` |
| reproduction-substrate plan | `shipped` | engineering/debugging | `none` |
| capture/restore plan | `shipped` | engineering/debugging | `none` |
| pixel-oracle design | `closed without implementation` | engineering/debugging | `#15` |
| wall-junction design and plan | `superseded` | mechanics/rendering | `#14` |
| superface design and plan | `shipped` | mechanics/rendering | `#14` |
| editor-authoring campaign design | `shipped` | mechanics/levels-and-objects | `#38` |
| editor-authoring SP1 plan | `shipped` | mechanics/levels-and-objects | `#38` |
| editor-authoring Wiki-debt plan | `superseded` | `docs/README.md` | `none` |
| editor-authoring SP2 plan | `shipped` | mechanics/rendering | `none` |
| editor-authoring SP3 plan | `shipped` | mechanics/levels-and-objects | `none` |
| editor-authoring SP4 plan | `shipped` | mechanics/overview | `none` |
| editor campaign close checklist | `superseded` | engineering/agent-workflow | `#38` |
| cross-platform bootstrap design and plan | `shipped` | engineering/setup | `#38` |
| pure-paint design | `shipped` | mechanics/rendering | `none` |
| engine-selection design | `shipped` | engineering/setup | `none` |
| tooling-portability plan/closeout | `shipped` | engineering/tooling | `#38` |
| deployment-stability design and plan | `shipped` | engineering/build-test-deploy | `none` |
| AI-documentation design | `active` | `docs/README.md` | `#14, #15, #38` |
| this implementation plan | `active` | `docs/README.md` | `#14, #15, #38` |
| post-integration issue plan | `active` | `docs/README.md` | `#14, #15, #38` |

Reverify each classification against artifact scope and current tree before
writing it. The grouped map is guidance for repeated individual rows, not a
license to omit a file. The three not-yet-created issue operations are active
plan scope, not guessed residual references; the post-rollout closeout adds
their actual GitHub-assigned numbers alongside #14, #15, and #38.

- [ ] **Step 4: Finish the index route to the concrete registry**

Change the historical-rationale link in `docs/README.md` from the directory
route used during intermediate commits to `docs/superpowers/README.md`.

- [ ] **Step 5: Run the now-live complete repository contract**

Run:

```sh
python3 -B tools/check-docs.py --repo-root .
sh ci/run_documentation_self_test.sh .
```

Expected: all contract, renderer, publisher, workflow, and live repository
checks PASS. No network is used.

- [ ] **Step 6: Kill registry and live-gate mutations**

Using fixture copies, remove an artifact, duplicate one, misspell an outcome,
repeat a residual issue, insert a guessed issue reference, break a
current-result link, and delete the historical-checkbox explanation. Each
named mutation must fail.

- [ ] **Step 7: Request registry/contract review and commit**

Review set equality, every disposition, active lifecycle, issue syntax, and
live gate activation. Stage the registry/index/test and commit under the global
rules.

### Task 17: Re-Audit Moving State and Prove the Whole Game Still Ships

**Files:**

- Modify only when a factual review finding requires a correction in files
  already owned by Tasks 1–16.

**Interfaces:**

- Consumes: branch tip, current `origin/main`, live Wiki `master`, current issue
  bodies/states, all local tests, game/runtime probes, and independent review.
- Produces: final green branch evidence and a finish-branch choice; no
  integration, Wiki write, issue mutation, or game deployment.

- [ ] **Step 1: Compare the branch with the recorded source baseline**

Run every command below through the same resolved absolute Git executable and
closed no-`GIT_*`, no-replacement, no-lazy-fetch environment constructed in
Task 10; the illustrative unsets are a visible minimum, not permission to
inherit an unlisted Git variable. Before trusting a commit/tree/ancestry
result, refuse replacement refs, grafts, both alternate files,
partial/promisor configuration, shallow history, and any missing reachable
object exactly as production does. Then run:

```sh
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TERMINAL_PROMPT=0
unset GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT GIT_ASKPASS SSH_ASKPASS
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
unset GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_CEILING_DIRECTORIES
unset GIT_TRACE GIT_TRACE2 GIT_TRACE_PACKET GIT_TRACE_PERFORMANCE
unset GIT_TRACE_SETUP GIT_CURL_VERBOSE
test "$(git remote get-url origin)" = "https://github.com/cleveralbatraoz/unseeing"
test "$(git remote get-url --push origin)" = "https://github.com/cleveralbatraoz/unseeing"
git -c credential.helper= -c core.askPass= fetch --no-tags origin \
  refs/heads/main:refs/remotes/origin/main
git rev-parse HEAD origin/main main
git merge-base HEAD origin/main
git status --short
git submodule status tools/superpowers
```

If `origin/main` advanced beyond the design baseline, update this isolated
branch using the repository's non-destructive branch workflow, then repeat all
touched mechanics audits and tests. Never move HEAD in the primary checkout
during review.

- [ ] **Step 2: Recheck the live Wiki and issue assumptions read-only**

In a fresh temporary Git repository, install the literal canonical Wiki remote,
assert both effective URLs before network access, and fetch only
`refs/heads/master:refs/remotes/origin/master` under Step 1's isolated,
credential-free Git environment. Do not check out Wiki content. Require the
head still equals the audited legacy SHA. Resolve/hash one absolute GitHub CLI,
use a fresh noninteractive environment plus literal
`github.com/cleveralbatraoz/unseeing`, and reread every issue named in the
design and post-integration plan through explicitly hosted/repository-qualified
read-only calls. If Wiki head or a disposition changed, stop the affected
rollout assumption, audit the new state, update design/plan only with user
approval where scope changes, and rerun relevant tests. Do not mutate either
service.

- [ ] **Step 3: Run every documentation/mirror gate directly**

Run:

```sh
python3 -B test/documentation_markdown_test.py -v
python3 -B test/documentation_contract_test.py -v
python3 -B test/wiki_renderer_test.py -v
sh test/wiki_publisher_test.sh
python3 -B test/wiki_workflow_test.py -v
sh test/ci_documentation_tooling_gate_test.sh
sh test/deployment_archive_test.sh
sh test/repo_hygiene.sh
sh test/shell_syntax_test.sh
ci/verify-superpowers.sh metadata
python3 -B tools/check-docs.py --repo-root .
MIRROR_VERIFY_PARENT="$(mktemp -d)"
MIRROR_VERIFY_OUT="$MIRROR_VERIFY_PARENT/wiki"
trap 'rm -rf "$MIRROR_VERIFY_PARENT"' EXIT INT TERM HUP
SOURCE_SHA="$(git rev-parse HEAD)"
python3 -B tools/render-wiki.py render --repo-root "$PWD" \
  --source-sha "$SOURCE_SHA" --output-dir "$MIRROR_VERIFY_OUT"
python3 -B tools/render-wiki.py verify-state --tree "$MIRROR_VERIFY_OUT"
rm -rf "$MIRROR_VERIFY_PARENT"
trap - EXIT INT TERM HUP
```

- [ ] **Step 4: Run the complete existing game pipeline**

From this worktree, with the exact pinned Godot binary already used for the
baseline, run:

```sh
. tools/lib/engine.sh
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
VERIFY_TMP="$(mktemp -d)"
trap 'rm -rf "$VERIFY_TMP"' EXIT INT TERM HUP
GODOT="$GODOT_BIN" DEPLOY_DIR="$VERIFY_TMP/no-game-deploy" ci/pipeline.sh
rm -rf "$VERIFY_TMP"
trap - EXIT INT TERM HUP
```

Expected: repository/tooling/archive/deployment self-tests, Rust fmt/Clippy/
tests/release, GDScript format/lint, boot, all gdUnit suites, determinism,
restore, editor probes, engine census, wasm build, clean Web export, browser
first-paint, and browser G-channel smoke all PASS. The runtime hashes may be
recorded as evidence but never copied into durable docs as fixed expected
values. The deliberately nonexistent deploy target must produce the pipeline's
build-only message; any deployment message is a failure.

- [ ] **Step 5: Run the on-demand native rendered visibility and input probes**

Run:

```sh
. tools/lib/engine.sh
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
GODOT="$GODOT_BIN" tools/probe_visibility.sh
GODOT="$GODOT_BIN" tools/probe_display.sh
```

Expected: both warm/cold rendered passes agree and pass; the temporary ignored
`game/override.cfg` is removed afterward. The real-window display probe's
Escape/settings, fullscreen, input, and native-load cases also pass. These are
supporting native evidence, not pipeline stages. If the final execution host
cannot provide a real display, report that exact environmental limitation and
do not claim the display-probe cases were observed; the implementer must still
run them on a display-capable host before using them as closeout evidence.

- [ ] **Step 6: Obtain independent final mechanics review**

Give a read-only reviewer the five mechanics pages, current owners, named
evidence, branch diff, and explicit requirement to distinguish source-text,
pure Rust, Godot behavior, mesh readback, native pixels, and web pixels. Verify
every finding directly before changing prose.

- [ ] **Step 7: Obtain final plan/spec and security review**

Review spec coverage, incomplete-marker absence, task boundaries, totality, Git tree
state machine, Actions permissions/token scope, gollum failure semantics,
archive exclusion, and absence of any `tools/superpowers` change. Resolve only
evidence-backed findings and rerun every affected test plus Steps 3–5 when
their evidence domain changes.

- [ ] **Step 8: Inspect final provenance and cleanliness**

Run:

```sh
git diff --check origin/main...HEAD
git status --short --branch
git submodule status tools/superpowers
git log --format='%H %an <%ae>%n%s%n%b' origin/main..HEAD
```

Require a clean worktree, unchanged Superpowers gitlink, mandated identity,
small green commits, no attribution, no build/export/report artifacts, and no
live external mutation.

- [ ] **Step 9: Invoke the finish-branch workflow and stop for user choice**

Use `superpowers:finishing-a-development-branch`. Present its integration
options with the exact verification evidence. Do not merge, push, publish,
deploy, or start the issue plan until the user selects a path and all
post-integration preconditions in that plan are true. If the selected path
eventually reaches remote `main`, record the resulting full main commit as
`INTEGRATED_MAIN_SHA` in the handoff—never infer it from the feature tip,
especially after squash/rebase or asynchronous PR integration.

## Execution Handoff

Execute Tasks 1–17 with fresh task implementers and two-stage review. After a
user-approved integration reaches remote green `main`, automatic Wiki
publication is observed—not manually repeated—and the separate
`2026-08-15-ai-documentation-issue-migration.md` plan becomes eligible. If the
built-in token cannot push the Wiki, publication fails unchanged and there is
no PAT fallback.
