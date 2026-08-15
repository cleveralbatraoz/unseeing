# AI-Agent Documentation Source of Truth and Wiki Mirror — Design

**Date:** 2026-08-15
**Status:** approved in conversation; this document freezes the approved design
before implementation planning.
**Repository baseline:** `bd86e81d5047a48982a1a49d3af5f02c750df77d`
**Audited Wiki baseline:** `3780b28869c0ab53d8375a3b4211e6e7f3c15de3`
on `master`

## Problem

Project knowledge is split across root and scoped READMEs, `AGENTS.md`,
`CLAUDE.md`, loose files under `docs/`, frozen Superpowers specs and plans,
inline code documentation, and a separately versioned GitHub Wiki. Those
surfaces do not have explicit authority boundaries. The Wiki has already
accepted descriptions from an unmerged development branch, while several
local documents retain completed work, old architecture, volatile test counts,
or proposed work as if it were current behavior.

The primary documentation consumer is an AI coding agent. The documentation
therefore needs deterministic routing, precise code ownership, machine-checkable
links, and a sharp distinction between what ships and what remains to do. It
does not need duplicated narrative for different audiences.

## Goals

1. Give every kind of durable project knowledge exactly one authoritative
   home.
2. Reconstruct shipped mechanics from current code and tests, with special
   care around rendering, superface merging, labels, waves, and sources.
3. Make repository-local Markdown the only editable behavior and engineering
   documentation.
4. Publish the GitHub Wiki automatically as a deterministic, one-way mirror of
   that Markdown after a green `main` build.
5. Move every actionable defect, missing proof, and future change to GitHub
   Issues, except the user's explicitly deferred mood-policy conflict; remove
   obsolete or speculative notes that the project does not need.
6. Keep frozen decisions and execution records discoverable without allowing
   old plan checkboxes to masquerade as current work.
7. Prove the migration with documentation, renderer, publisher, game, native,
   wasm, and browser tests.

## Non-goals

- This work changes no gameplay, physics, sound propagation, rendering,
  content, or platform implementation.
- It does not modify the `tools/superpowers` submodule, its gitlink, or any file
  inside it. `docs/superpowers/README.md` is parent-repository documentation
  about this project's artifacts, not a Superpowers plugin change.
- It does not deploy the game. Wiki publication is the only automatic external
  publication in scope, and it occurs only after the branch is integrated by
  the user's chosen finish-branch path.
- It does not reconcile the current film-grain, vignette, or filled-void mood
  layer with the outline-only/no-visual-noise policy. That conflict is
  deliberately deferred without a documentation note or new issue in this
  change and must be called out in the final report.
- It does not import behavior from the separate `pr47-gaps` worktree or any
  other unmerged branch.

## Authority model

The authority order is explicit rather than inferred from file age or detail.

| Knowledge | Authority | Contract |
| --- | --- | --- |
| Runtime behavior and constants | Code and executable tests | Code wins whenever prose disagrees. |
| Project policy and agent routing | `AGENTS.md` | Short, normative, and free of duplicated mechanics tutorials. |
| Shipped mechanics and current engineering procedures | `docs/current/` | Describes only the current repository and names the owning file and symbol for quoted constants or laws. |
| Defects, missing evidence, and future work | GitHub Issues | The exclusive live backlog except for the user's one explicitly deferred mood-policy decision; canonical docs contain no task lists or disguised backlog prose. |
| Approved decisions and their rationale | `docs/superpowers/specs/` | Frozen historical artifacts, not current-behavior documentation. |
| Approved execution sequences and evidence | `docs/superpowers/plans/` | Frozen historical artifacts; unchecked boxes have no live status. |
| Public Wiki | Generated from repository docs | Read-only mirror; direct edits are invalid. |
| Local API details | Rust/Godot inline documentation | Lives beside the API and cannot redefine cross-project policy or behavior. |

`CLAUDE.md` remains a thin compatibility adapter that points agents to
`AGENTS.md` and the pinned repository workflow. READMEs remain only where a
directory needs an entry point. They state scope and route to the canonical
page instead of repeating commands, constants, architecture, or status.

The documentation section of `AGENTS.md` changes with this migration. It must
route implementation and research through `docs/README.md` and the relevant
`docs/current/` page, retain the rule that code wins, require canonical local
docs to change with shipped behavior, and forbid direct Wiki edits. Integrating
canonical documentation into `main` authorizes only its deterministic
post-green Wiki publication; it does not authorize merge, game deployment,
issue mutation, or another external action. References to the removed MCP page
are replaced by the canonical debugging/agent-workflow location. The existing
spec/plan distinction remains normative.

## Canonical repository layout

The live documentation tree becomes:

```text
docs/
  README.md
  current/
    mechanics/
      overview.md
      rendering.md
      waves.md
      sound-sources.md
      levels-and-objects.md
    engineering/
      setup.md
      editor-authoring.md
      build-test-deploy.md
      debugging.md
      agent-workflow.md
      tooling.md
  superpowers/
    README.md
    specs/
    plans/
```

`docs/README.md` is the sole local documentation index. Its first decision is
where an agent should go for mechanics, engineering procedure, project policy,
current work, or historical rationale.

`docs/current/engineering/tooling.md` is the concise capability map for an
agent deciding which repository tool to invoke. It registers every tracked
parent-repository regular file or gitlink immediately under `tools/` and every
tracked regular support file recursively under `tools/lib/` exactly once, with
its path, purpose, execution context, and the situation in which an agent
should use it. Directories themselves receive no row; a parent-owned tool in a
different nested directory is rejected until this authority rule deliberately
admits that directory. Platform-specific siblings remain separate rows because
they admit different hosts. The
`tools/superpowers` gitlink is one opaque, developer-only submodule entry that
routes to the pinned workflow; its upstream files are neither copied into the
page nor treated as parent-repository tools. Tooling introduced by this design
is included in the same inventory before the live-tree gate becomes green.

The root, `game/`, and `infra/` READMEs become scoped routers. Durable content
from `docs/opening-in-godot.md`, `docs/agent-workflow.md`,
`docs/superpowers/mcp/godot-mcp-loop.md`, and the portability report is moved
to the appropriate `docs/current/` page, then the old files are removed.
Unreferenced documentation-only media and obsolete research/campaign prose are
removed after their durable facts have either been incorporated or rejected.
Git history retains the old material.

### Historical artifact registry

`docs/superpowers/README.md` explains the spec/plan distinction and registers
every artifact under `specs/` and `plans/` exactly once. Each row contains:

- artifact path and kind;
- decision or campaign it records;
- outcome: transient `active`, or terminal `shipped`, `superseded`, or
  `closed without implementation`;
- the canonical current-document page that describes the resulting behavior;
- one or more residual GitHub issue numbers, or `none`.

`shipped` means that the artifact's resulting behavior or procedure is present
in the current repository tree; it does not claim that the current branch has
already been integrated. A residual-issue cell is exactly `none` or a
comma-separated, non-empty list of unique positive issue references in
`#<number>` form.

The registry, not an old checkbox, is the status surface. `active` exists only
to make the required plan-first workflow truthful while an approved artifact
is being executed; the execution or rollout closeout must replace it with a
terminal outcome. The checker accepts `active` as a valid transient state; the
execution and finish workflows enforce its eventual transition. A residual
action without an issue is invalid. Artifact bodies remain frozen unless a
factual provenance correction is required; current behavior is never repaired
by rewriting history.

## Current-document content contract

Every current page answers an agent's implementation questions directly:

1. what the current behavior or procedure is;
2. which file and symbol own each quoted law, constant, boundary, or command;
3. which executable evidence proves it;
4. which parts are source-text contracts and which are observed through Rust,
   Godot, mesh readback, wasm, or rendered pixels;
5. what inputs, failure states, and platform boundaries the component admits.

Pages do not contain roadmaps, historical campaigns, unchecked boxes,
"deliberately unfinished" sections, stale numeric test totals, or links to a
mutable branch view. A shipped constraint may be stated neutrally when an
agent must know it to operate the project, but its proposed remedy belongs only
in an issue.

Mechanics claims are reconstructed from `main`, not copied from the Wiki. In
particular, the migrated wave documentation must keep three distinct GPU
effects separate:

- surface reveal from a player tap, echo, or footstep becomes zero after a
  source-side wall crossing; the corresponding in-flight shell stops at the
  front scene surface;
- kind-3 source surface reveal is attenuated by
  `HUM_THROUGH ^ source-side-crossings`, where `HUM_THROUGH = 0.55` is owned by
  `rust/src/level_plan.rs` and mirrored in
  `game/shaders/pulse_pool.gdshaderinc`; the visible in-flight source shell in
  `game/shaders/hearing_post.gdshader` instead applies one `HUM_THROUGH` factor
  when its ray intersection lies at or behind the front scene surface—it does
  not count wall crossings;
- a source's standing silhouette is attenuated by
  `SOURCE_THROUGH ^ camera-side-crossings`, where `SOURCE_THROUGH = 0.30` is
  owned by `rust/src/level_plan.rs` and consumed through the source
  data/hearing path.

The live Wiki currently contains an unmerged claim that all traveling waves
stop at walls. That text is evidence of drift, not an input to the new page.
The source and observer contracts in `rust/src/observe/`, the shader includes,
mesh/data-path tests, and rendered gates must be considered together.

The rendering page must likewise distinguish:

- the R/G/B data-pass channel contract;
- same-facing coplanar superface merging in
  `rust/src/render/superface.rs`, including the separate roles of
  `COPLANAR_EPS` and `PATCH_EPS`;
- graph-coloured face and semantic source-role labels in
  `rust/src/render/labels.rs`, including `MIN_SEP = 0.08` and the safe label
  band, plus the sole grandfathered standalone-radio preview exception
  `Role::Case = 0.05`;
- fixed creature role labels versus per-placed-source numeric labels;
- facts proven from shader source from facts proven by G-channel readback or a
  rendered pixel oracle.

It must not revive the older per-object-ID or production-GDScript architecture.
The current composition root is `rust/src/nodes/game.rs::UnseeingGame` with
`game/scenes/main.tscn`; shipped gameplay logic remains Rust, and tracked
GDScript remains tests and probes only.

## Issue migration

Issue state changes happen only after the documentation implementation is
green and present on `main`, so issue claims always point to an integrated
tree. Before each mutation, the implementation must reread the issue and
reverify its resolution against current code and tests.

### Close as already resolved

The following open issues describe behavior that current `main` already
replaced or proofs that current tests now carry:

- #7;
- #12, #13;
- #16;
- #22;
- #30 through #36;
- #39;
- #41, #42;
- #44, #45.

Closure comments link the owning integrated commit, the canonical current page,
and the strongest relevant executable evidence. A documentation rewrite alone
is never presented as the fix.

### Keep, rewrite, or narrow

- #14 remains open but is rewritten. Its retired per-object-ID mechanism,
  deleted owners, old checkout warnings, and abandoned fix menu are replaced by
  the current superface/paint owners and evidence. Its sole residual acceptance
  criterion is a deterministic rendered regression oracle for the original
  jagged wall-junction artifact; structural mesh/label proof alone does not
  close it.
- #15 is rewritten from its historical observability campaign inventory to a
  bounded GPU-evidence issue: branch-sensitive Rust/GLSL wall-crossing parity
  beyond the existing single-source probe; rendered hearing-post composition
  of R reveal, G crease, and B silhouette; visible shell-raytrace behavior; and
  structured framebuffer facts sufficient to diagnose a failed oracle. It
  removes the repaired gdUnit gate, retired GDScript/seed claims, MCP install,
  generic trace-capture wishlist, and observer-shipping discussion. It excludes
  acoustic-image depth (#4), multi-source identity (#1), the wall-junction
  oracle (#14), cross-target determinism (#5), the new `MIN_SEP`/shader-knee
  issue, and the explicitly deferred mood layer.
- #38 is narrowed to the remaining fresh-host Godot acquisition boundary;
  current bootstrap now builds and validates the Rust engine but does not make
  every supported host obtain a pinned Godot editor automatically.

All other existing issues retain their state unless a fresh verification during
implementation proves that this approved disposition has become stale.

### Create from verified residual work

Only three new issues are justified by the audit:

1. replace `SMOKE_WAIT` in `test/web_probe.py` with an observable first-paint
   readiness condition;
2. mechanically hold Rust's `MIN_SEP` and the hearing shader's upper crease
   knee in agreement without introducing a gameplay mirror-constant test;
3. exercise clean-host native GDExtension loading on every supported desktop
   architecture, separating real native-load evidence from cross-compilation.

Each issue names the current owner, missing externally visible proof, and
acceptance evidence. Vague audio ideas, phantom work, and speculative
deployment hardening are deleted from documentation rather than promoted to
issues.

## Wiki mirror

### Manifest and renderer

`docs/wiki-pages.tsv` is the ordered publication manifest. Each non-comment
row identifies one canonical source path, one unique Wiki slug, its navigation
title, and its section. Every `docs/current/**/*.md` page appears exactly once.
The manifest may also expose the documentation index, but never a README
router, historical spec, plan, report, or issue ledger as a behavior page.
`Home`, `_Sidebar`, `Mirror-Metadata`, and `.unseeing-wiki-mirror` are reserved
renderer outputs. Manifest slugs omit `.md` and are restricted to ASCII
`[A-Za-z0-9][A-Za-z0-9_-]*`; validation rejects ASCII-case-folded collisions
with another slug or reserved output.

`tools/render-wiki.py` is a pure, deterministic Python standard-library
renderer. It receives the repository root, full source commit, and output
directory explicitly. It reads only tracked inputs, writes only the requested
output tree, and refuses malformed rows, duplicate sources or slugs, missing
files, path traversal, symlinks outside the repository, invalid UTF-8, and a
source commit that does not name the input tree.

For each manifest page the renderer:

- preserves the canonical body rather than maintaining a second template;
- adds a generated/read-only notice with the full source SHA and canonical
  repository path;
- rewrites repository-relative file, heading, and asset links to
  commit-pinned `blob/<full-sha>/...` or `tree/<full-sha>/...` GitHub links;
- leaves external links and issue links unchanged;
- rejects links that cannot be resolved deterministically.

Canonical Markdown uses a deliberately bounded link grammar: inline links and
images, reference definitions and uses, autolinks, and same-page fragments.
Destinations may be angle-delimited or use balanced parentheses and ordinary
Markdown escapes. The scanner skips fenced code, indented code, and inline code
exactly; it never rewrites link-shaped text inside them. Raw-HTML links,
multiline destinations, malformed escapes, and any construct the scanner cannot
classify are rejected with a source location instead of guessed at. Fixtures
cover inline and reference links, images, fragments, nested parentheses,
escapes, and code exclusions.

It also generates `Home.md`, `_Sidebar.md`, `Mirror-Metadata.md`, and the
non-page `.unseeing-wiki-mirror` state file. Home and sidebar follow manifest
order. Metadata names the exact repository commit and the no-direct-edit
contract. The state file records the format version, source SHA, and a digest
over the sorted, length-delimited `(relative path, bytes)` mapping of every
other generated file. Including paths makes a rename observable. The digest is
a fast corruption check, not proof of provenance: provenance is established by
checking out and independently rendering the recorded source commit. Generated
content contains no wall-clock timestamp, hostname, workspace path, or branch
name. Rendering the same tracked tree twice is byte-identical.

The generated tree is complete: publishing removes old Wiki working-tree pages
that are absent from the manifest. Their history remains in the Wiki Git
repository, but stale research and superseded mechanics no longer appear as
current documentation.

### Publisher safety

`tools/publish-wiki.sh` owns the Git boundary and is tested against local bare
repositories. It never edits canonical docs. It clones/fetches the Wiki's
`master`, renders into a fresh directory, compares content, and exits without a
commit when the mirror is already current.

Production mode refuses unless the Actions repository, push event, `main` ref,
checked-out `HEAD`, and `GITHUB_SHA` all agree with the canonical repository.
The hermetic test mode injects explicit local source and Wiki repositories and
has no production credential path. Wiki commits use the mandated repository
identity and contain no tooling or assistant attribution.

The first takeover is allowed only when the remote head is exactly the audited
`3780b28869c0ab53d8375a3b4211e6e7f3c15de3`. This supersedes the earlier
`46485be548ad8956ffb82ca5602a2c20de6940fa` audit because the intervening merge
contains only the deployment recovery page published with current `main`. If
the Wiki advances again before takeover, publication stops for a new audit; it
never silently overwrites the new head.

After a generated mirror exists, every run must:

1. require the audited legacy head to remain in Wiki `master` ancestry;
2. parse the recorded source SHA and fast-check the existing content digest;
3. check out that source commit from full canonical repository history,
   independently render it, and compare the complete Wiki tree byte-for-byte;
4. reject a direct edit, forged digest, stray file, or malformed marker;
5. prove the previous source SHA is an ancestor of the requested source SHA;
6. create one ordinary descendant commit only when generated content differs;
7. perform a credentialed `git push --dry-run` before the real push;
8. push without force or ref deletion;
9. fetch the remote again and verify its head, complete tree, and source marker.

Here, complete-tree equality means the exact sorted relative paths, Git entry
types and modes, and regular-file bytes. Generated and published trees contain
regular files only; symlinks, submodules, devices, and every other non-regular
entry are refused.

The ancestry check refuses source rollback or divergence. It is not an external
monotonic anchor for Wiki history: if someone force-resets Wiki `master` to an
older, otherwise valid generated descendant of the audited legacy head, a later
publisher may safely regenerate the newest source on top of it. A reset exactly
to the audited legacy head is observationally identical to the first takeover;
because that exact commit and tree were audited, the publisher safely performs
the locked takeover again and does not claim to detect that reset. Every other
markerless head, a reset that loses the audited legacy head from ancestry, or a
tree not independently reproduced from its marker is refused.

A remote advance after dry-run makes the ordinary push fail atomically. The
publisher does not force or merge and does not hide the race with an unbounded
retry; the failed Actions job is rerun from a fresh clone. A direct edit is
recovered by an audited ordinary revert that restores the exact independently
rendered tree, followed by a fresh publisher run.

### GitHub Actions

The existing tests workflow gains a final `publish-wiki` job with
`needs: [checks, windows-bootstrap]`. It runs only for a successful push to
`main`; pull requests render and test the mirror but cannot publish. Workflow
permissions explicitly set `contents: read` at top level, with
`contents: write` granted only to the publication job. The publisher fetches
full source history (`fetch-depth: 0`) so ancestry and historical re-rendering
work when the previous source is many commits behind. Checkout persistence is
disabled, no third-party action runs in the write-capable job, and
`GITHUB_TOKEN` is exposed only to the final dry-run/real-push step through a
non-logging credential helper; it never enters a remote URL, Git config, page,
commit, or captured output.

The workflow keeps feature/PR cancellation but disables cancellation for
`main`, so a newer push cannot kill a publisher after its push and before
readback. Main runs serialize through a dedicated Wiki concurrency group with
`cancel-in-progress: false`. The source-ancestry guard remains authoritative if
GitHub drops a pending run or schedules an older rerun after a newer source
while a managed source marker exists. Exact-legacy-head retakeover has no
previous marker to compare and therefore accepts only the requested,
production-guarded canonical `main` event.

GitHub documents a Wiki as its own `<repository>.wiki.git` repository but does
not explicitly guarantee Wiki pushes from the built-in token. The first main
run therefore proves the exact push with `--dry-run`; if permission is denied,
the job fails with the Wiki unchanged. There is no personal-access-token
fallback and no long-lived secret introduced by this design.

A small workflow with explicit read-only permissions uses the `gollum` event to
detect human or external-token Wiki edits. It does not trust the event's
`GITHUB_SHA`, which names the source repository's default-branch head rather
than a Wiki commit. It explicitly fetches Wiki `master`, reads its recorded
source SHA, checks out that commit from the source repository, independently
re-renders it, and compares the complete tree for diagnosis. It then fails the
job unconditionally: even an external commit whose bytes exactly reproduce an
authorized tree is still an unauthorized Wiki write that tree equality cannot
authenticate. A direct web or Git edit therefore fails visibly and is never
adopted as a new source of truth. A mirror push made with `GITHUB_TOKEN`
normally does not recursively trigger this workflow; the publisher's own
remote readback is the verification path for that push. If GitHub unexpectedly
does emit a recursive `gollum` event, that failure is intentionally visible.

## Executable documentation contract

Every renderer, publisher, guard, and hygiene behavior slice begins with its
named failing test and proceeds red-green-refactor. The minimum gates are:

- every authority surface exists in its allowed location;
- every canonical page is indexed and appears exactly once in
  `docs/wiki-pages.tsv`;
- every local Markdown link, heading fragment, code-owner path, and declared
  owner symbol resolves;
- README routers do not duplicate canonical mechanics or procedures;
- current docs contain no task boxes, backlog markers, volatile test totals,
  mutable-branch links, or host-specific paths;
- every tracked parent-owned `tools/` entry and `tools/lib/` support file is
  registered exactly once in the tooling capability map, while the
  `tools/superpowers` gitlink is represented once and its contents are not
  inventoried as parent-owned tools;
- every spec and plan is registered exactly once with a valid outcome and
  either `none` or a comma-separated, duplicate-free non-empty list of
  `#<positive-number>` residual issues; an `active` row is accepted only during
  execution and must become terminal in the closeout;
- two renders of one commit are byte-identical and use the full commit SHA;
- link rewriting, navigation generation, reserved-name rejection, stale-page
  removal, idempotence, takeover locking, digest validation, independent
  historical re-rendering, source-rollback refusal, non-force push, and remote
  readback are exercised hermetically;
- a reset exactly to the audited legacy Wiki head is safely re-taken over,
  while every other markerless head is refused;
- a page edit accompanied by a forged matching digest is still rejected;
- an old source several commits behind proves full-history ancestry, and a
  remote advance between dry-run and push fails without altering that remote;
- workflow tests prove pull requests cannot publish and a main publication
  depends on both Linux/wasm and Windows jobs;
- guard-workflow tests prove `gollum` runs read-only, fetches Wiki `master`
  explicitly, never treats the event's source-repository `GITHUB_SHA` as a Wiki
  revision, and fails every event after read-only diagnostic verification.

Mutation checks must kill realistic changes to manifest uniqueness and reserved
names, provenance stamping, link resolution and code-span exclusion, content
digests, independent historical re-rendering, source ancestry, takeover
ancestry, the audited takeover head, race handling, publish conditions, job
dependencies, guard Wiki-ref selection, and permission scope. They must not add
a test that merely repeats a gameplay constant in a second language. Mechanics
continue to be proved at the owning pure Rust component and at the relevant
Godot/GPU boundary.

## Verification and rollout

Before editing, the complete pipeline was run from the isolated
`docs/ai-documentation-mirror` worktree at the repository baseline. Rust
formatting, Clippy, all Cargo tests, the release build, GDScript format/lint,
headless boot, all gdUnit suites, determinism and restore probes, editor slab,
source, level, and prefab probes, class census, wasm build, clean web export,
browser first-paint smoke, and browser G-channel smoke all passed.

Implementation verification repeats the full pipeline rather than relying on
documentation-only tests. Mechanics pages receive an independent read-only
review against current code, tests, shader text, and the strongest available
runtime trace. Review must call out where a source-text assertion is the best
existing evidence rather than implying that pixels were observed.
`tools/probe_visibility.sh` is run explicitly as the on-demand native rendered
visibility probe; it is supporting evidence, not falsely described as a gate
inside `ci/pipeline.sh`.

Immediately before final review, the branch compares `origin/main`, local
`main`, and its merge base to the recorded baseline. If `main` has advanced,
the branch is updated, every touched mechanics claim is re-audited, the live
Wiki head is rechecked, issue dispositions are reread, and the complete
verification suite is rerun.

Integration remains a user choice under the finish-branch workflow. Once an
approved integration reaches green `main`, Wiki publication is automatic and
the verified issue mutations may be applied. No game deployment follows from
this documentation change.

## Preserved project invariants

The outline-only perception law, same-facing coplanar superface merge law,
separation of touching solids and semantic source roles, label-safe band,
`MIN_SEP`, Godot-object/Rust-law split, explicit dependency injection, total
pure domain functions, absence of mutable global state, one Godot project, one
native/web Rust behavioral source, and x86_64/arm64/wasm32 support remain
unchanged. Documentation tooling is developer-only, excluded from deployment
archives, and cannot become a game, build, export, or deploy dependency.
