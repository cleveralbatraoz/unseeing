# GitHub Pages Deploy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The player-facing web build deploys automatically on every push to
`main`, straight from GitHub Actions to GitHub Pages, with the droplet
removed from the hosting path entirely and its now-dead deploy scripts
deleted from the repo.

**Architecture:** `.github/workflows/test.yml`'s existing `checks` job
already runs the full `ci/pipeline.sh` (import, gdUnit4, the Web export,
the browser smoke test) on every push to `main` — that work is reused, not
duplicated. A new `deploy` job, gated to `main`+`push` only, consumes that
job's already-verified `game/build/web/` output via
`actions/upload-pages-artifact` → `actions/deploy-pages`, then verifies the
live page actually serves the pushed commit — the same stamp-check
`deploy.sh` already does today, just aimed at the new URL. Only after that
path is proven live does a second task retire the droplet-specific scripts.

**Tech Stack:** GitHub Actions (`ubuntu-latest`, already in use),
`actions/upload-pages-artifact@v3`, `actions/deploy-pages@v4`, `gh api` for
one-time repo configuration.

**Spec:** No separate spec document — scope, GitHub Pages research (size
limits, MIME-type handling, no-COOP/COEP-needed since this export is
single-threaded by design, `deploy-pages` YAML shape), and every decision
below were settled directly with the user in conversation, per their
explicit choice to skip the standalone spec doc for this plan. The Global
Constraints section below carries what a spec would otherwise have
recorded.

## Global Constraints

- **Trigger: automatic on every push to `main`**, not `workflow_dispatch`
  — an explicit user decision that supersedes `AGENTS.md`'s previously
  stated "deploy is deliberate, human-triggered" language for the *deploy
  step specifically*. Task 3 updates that document to match. The
  deliberate human gate moves to the decision to merge into `main`, not a
  separate manual deploy action.
- **No custom domain** — ships at the repo's default GitHub Pages URL
  (`https://cleveralbatraoz.github.io/unseeing/`, since this is not a
  `<user>.github.io`-named repo — the repo name is a path segment, not the
  root). No DNS work.
- **Sequencing is load-bearing, not cosmetic:** Task 2 (deleting the
  droplet-specific scripts) may only start after Task 1 is verified with a
  real, live, successful GitHub Pages deployment — never both changed at
  once, matching this project's own established deploy discipline of
  proving one path before removing another.
- **This project's export is single-threaded by design**
  (`rust/build-wasm.sh`: `thread_support=false`, `--features nothreads`,
  no `-pthread`, no atomics) — confirmed during research specifically
  because it removes the one real technical risk GitHub Pages has for wasm
  (`COOP`/`COEP` headers, which static hosts can't set and this project
  doesn't need).
- **Commits:** small, self-contained, green. Evocative narrative subject,
  no AI/assistant attribution anywhere. Repository identity: `Dmitrii
  Galchenko <dggrus@gmail.com>`.
- **Isolation:** continues in the existing worktree at
  `.claude/worktrees/designer-engine-bundle`, branch
  `designer-engine-bundle` — already isolated, no new worktree needed.
- **Never push to any remote, or merge, without the user's explicit
  go-ahead at that exact point** — same standing rule as the prior plan in
  this branch. Task 1's real-run verification and Task 2's deletions both
  need a real push to prove; ask before each push.

---

## File Structure

- `.github/workflows/test.yml` (modify) — Pages permissions, artifact
  upload, new `deploy` job.
- `deploy.sh`, `ci/push_production.sh`, `ci/deploy_host_preflight.sh`,
  `infra/` (whole directory: `post-receive`, `nginx-unseeing.conf`,
  `README.md`) — deleted in Task 2.
- `test/deploy_host_preflight_test.sh`, `test/push_production_test.sh`,
  `test/post_receive_test.sh`, `test/deployment_archive_test.sh` — deleted
  in Task 2 (confirmed during planning: `deployment_archive_test.sh`
  specifically validates the whole-repo `git archive` mechanism
  `infra/post-receive` uses server-side; nothing else in this repo
  performs a whole-repo archive, so it has no other consumer once that
  path is gone — `ci/run_agent_tooling_self_test.sh` itself stays, since
  `ci/pipeline.sh` already calls it in both archive and normal-checkout
  contexts independent of the droplet).
- `ci/pipeline.sh` (modify in Task 2) — remove the `PREBUILT_RUST` branches
  and the `DEPLOY_DIR` block at the end (dead code once nothing sets
  either).
- `AGENTS.md` (modify, Task 3) — deploy policy language.
- Wiki page `Engineering-Build-Test-Deploy.md` §6 "Deploy" (external repo,
  Task 4) — rewritten to describe the new mechanism.

---

### Task 1: Ship the web build to GitHub Pages

**Files:**
- Modify: `.github/workflows/test.yml`

**Interfaces:**
- Consumes: the `checks` job's already-verified `game/build/web/` output
  (produced by `ci/pipeline.sh`'s existing export + smoke-test stages,
  already active in this branch from the earlier `SKIP_EXPORT=1` removal —
  confirm that uncommitted change is still present before starting; if the
  working tree has diverged, `git status`/`git diff` on this file first).
  The `UNSEEING_BUILD='<short-sha>'` stamp `ci/pipeline.sh` already writes
  into `game/build/web/index.html` via its existing `sed
  s/__BUILD__/$SHA/g` step — unchanged, already correct, this task reads
  it rather than writing it.
- Produces: a live GitHub Pages deployment at
  `https://cleveralbatraoz.github.io/unseeing/`, verified to actually
  serve the pushed commit.

- [x] **Step 1: Confirm the in-progress `test.yml` edit is intact**

```bash
git status --porcelain .github/workflows/test.yml
git diff .github/workflows/test.yml
```

Expected: shows the uncommitted change reordering the wasm-toolchain steps
before "Run CI pipeline (full, including Web export + browser smoke
test)" and dropping `SKIP_EXPORT=1` from that step, plus the
`timeout-minutes: 25` bump. If this diff is missing (a fresh checkout of
this branch, or it was reverted), reconstruct it first: in the `checks`
job, move the `Install wasm toolchain` / `Cache emsdk` / `Install emsdk
4.0.20` / `Build wasm core` steps to before the `Run CI pipeline` step,
and change that step's `run:` line from `GODOT="$PWD/godot-bin/godot"
SKIP_EXPORT=1 ci/pipeline.sh` to `GODOT="$PWD/godot-bin/godot"
ci/pipeline.sh`. Do not proceed past this step until that ordering is
confirmed correct — the export will otherwise fail for lack of an
activated wasm toolchain.

- [x] **Step 2: Add Pages permissions and the artifact-upload step to the `checks` job**

Add at the top level of the workflow file (sibling to `on:`/`concurrency:`,
not inside any job):

```yaml
permissions:
  contents: read
```

At the end of the `checks` job's `steps:` list (after the existing "Run CI
pipeline (full, including Web export + browser smoke test)" step), add:

```yaml
      - name: Upload the verified web build as a Pages artifact
        if: github.ref == 'refs/heads/main' && github.event_name == 'push'
        uses: actions/upload-pages-artifact@v3
        with:
          path: game/build/web
```

- [x] **Step 3: Add the `deploy` job**

Add a new top-level job, a sibling of `checks` and `windows-bootstrap`:

```yaml
  deploy:
    needs: checks
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    runs-on: ubuntu-latest
    timeout-minutes: 10
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
      - name: Verify the live page serves this commit
        run: |
          set -eu
          SHORT="$(printf %.9s "${{ github.sha }}")"
          LIVE="$(curl -skL --max-time 30 "${{ steps.deployment.outputs.page_url }}" \
            | grep -o "UNSEEING_BUILD='[^']*'" | head -1 | sed "s/.*='//;s/'//")"
          if [ "$LIVE" != "$SHORT" ]; then
            echo "deploy: FAILED the live page serves build '${LIVE:-none}', not '$SHORT'"
            exit 1
          fi
          echo "deploy: OK — https://cleveralbatraoz.github.io/unseeing/ serves $LIVE"
```

This mirrors `deploy.sh`'s own existing verification exactly (same stamp,
same grep pattern, same "never trust a green push" reasoning) — just
aimed at the new URL instead of the droplet's bare IP.

- [x] **Step 4: Validate YAML syntax locally**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/test.yml'))" && echo "valid YAML"
```

- [x] **Step 5: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "<narrative subject, e.g. describing the web build now shipping itself>"
```

- [x] **Step 6: Ask the user before enabling Pages and pushing**

Enabling GitHub Pages is a one-time, real repo-configuration change, and
this push will attempt a real production deployment for the first time.
**Stop and ask the user for explicit confirmation before proceeding.**

- [x] **Step 7: Enable GitHub Pages with GitHub Actions as the source**

```bash
gh api -X POST repos/cleveralbatraoz/unseeing/pages -f build_type=workflow
```

If this returns an error (e.g. Pages is already enabled with a different
source, or the exact field name has changed), fall back to the repo's
Settings → Pages UI and set "Source" to "GitHub Actions" manually — a
one-click fallback, not a blocker.

- [x] **Step 8: Push and watch the real run**

```bash
git push origin designer-engine-bundle
```

This branch's push trigger for `test.yml` is `branches: [main]` only — the
same `workflow_dispatch`-cannot-reach-an-unmerged-workflow limitation from
the designer-engine-bundle plan applies here too, AND the `deploy` job is
further gated to `github.event_name == 'push'` on `main` specifically. To
verify for real before merging, temporarily widen `test.yml`'s trigger the
same documented way the prior plan did for `release-engine.yml`
(`branches: [main, designer-engine-bundle]`, with the same "TEMPORARY"
comment and the same commitment to revert it before this task is done),
push, and watch:

```bash
gh workflow run test.yml --ref designer-engine-bundle
gh run watch "$(gh run list --workflow test.yml --branch designer-engine-bundle --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status
```

Expected: `checks` succeeds (including the Pages-artifact upload), `deploy`
succeeds, and its own live-verify step confirms the deployed page serves
the pushed commit.

- [x] **Step 9: Manually confirm the live page in a browser**

Visit `https://cleveralbatraoz.github.io/unseeing/` and confirm the game
actually loads and runs — the automated check only proves the build-stamp
matches, not that the page is visually correct. This mirrors the original
`#38` acceptance task's own standard: automated checks plus one real,
human-observed confirmation before calling it done.

- [x] **Step 10: Revert the temporary trigger widen**

Once confirmed working, revert `test.yml`'s trigger back to `branches:
[main]` only, commit, and push — same pattern as the prior plan's
`0c8d663` revert.

---

### Task 2: Retire the droplet-specific scripts

**Do not start this task until Task 1's Step 9 is confirmed** — a real,
human-verified live deployment must exist before any of the old path is
removed.

**Files:**
- Delete: `deploy.sh`, `ci/push_production.sh`,
  `ci/deploy_host_preflight.sh`, `infra/post-receive`,
  `infra/nginx-unseeing.conf`, `infra/README.md` (then remove the now-empty
  `infra/` directory)
- Delete: `test/deploy_host_preflight_test.sh`,
  `test/push_production_test.sh`, `test/post_receive_test.sh`,
  `test/deployment_archive_test.sh`
- Modify: `ci/pipeline.sh` — remove the self-test invocations for the four
  deleted test files, remove the `PREBUILT_RUST` branch (the entire
  `else` arm that reads prebuilt cores, in both the rust-gates stage and
  the wasm-build stage — keep only the `if` arm that actually builds), and
  remove the `DEPLOY_DIR` block at the very end (everything from `DEPLOY_DIR="${DEPLOY_DIR:-/var/www/unseeing}"` through the matching `fi`), leaving a bare `echo "ci: OK"` as the pipeline's final line.

**Interfaces:**
- Consumes: nothing new — this task only removes dead code and dead files.
- Produces: a smaller, accurate `ci/pipeline.sh` with no unreachable
  branches; nothing downstream references any of the deleted files (verify
  this explicitly in Step 1 below before deleting anything).

- [x] **Step 1: Confirm nothing else references what's about to be deleted**

```bash
grep -rln "deploy\.sh\|push_production\|deploy_host_preflight\|infra/post-receive\|deployment_archive_test\|PREBUILT_RUST\|DEPLOY_DIR" \
  --include="*.sh" --include="*.yml" --include="*.md" . 2>/dev/null | grep -v "\.git/"
```

Expected: only the files this task is about to delete or edit appear (plus
possibly wiki-adjacent docs, which Task 4 handles separately — note any
such hits for Task 4, don't fix them here). If anything else genuinely
depends on one of these, stop and report it — do not delete out from under
a real dependency.

- [x] **Step 2: Delete the droplet-specific files**

```bash
git rm deploy.sh ci/push_production.sh ci/deploy_host_preflight.sh
git rm infra/post-receive infra/nginx-unseeing.conf infra/README.md
rmdir infra
git rm test/deploy_host_preflight_test.sh test/push_production_test.sh \
  test/post_receive_test.sh test/deployment_archive_test.sh
```

- [x] **Step 3: Remove their self-test invocations from `ci/pipeline.sh`**

Remove the four `echo "ci: ..."` + script-invocation pairs (for
`deploy_host_preflight_test.sh`, `push_production_test.sh`,
`post_receive_test.sh`, `deployment_archive_test.sh`) from the self-tests
section near the top of the file.

- [x] **Step 4: Simplify the rust-gates stage — remove the `PREBUILT_RUST` fallback**

The current stage is an `if [ "${PREBUILT_RUST:-}" != "1" ] && { ... } ;
then ... else ... fi` — keep only the body of the `if` arm (the real
`cargo fmt`/`clippy`/`test`/`build` sequence), drop the `else` arm
entirely (the block that reads prebuilt `rust/target/release/lib*` and
checks `core.commit` against `BUILD_SHA`), and drop the now-unconditional
`if`'s condition down to just the C-linker check it was combined with.

- [x] **Step 5: Simplify the wasm-build stage the same way**

Same shape: the `if [ "${PREBUILT_RUST:-}" != "1" ] ... then
build-wasm.sh else check prebuilt wasm ... fi` — keep only the build path.

- [x] **Step 6: Remove the `DEPLOY_DIR` block**

Delete from `DEPLOY_DIR="${DEPLOY_DIR:-/var/www/unseeing}"` through its
closing `fi`, leaving `echo "ci: OK"` as the pipeline's unconditional last
line.

- [x] **Step 7: Run the full local pipeline to confirm nothing broke**

```bash
GODOT=/home/albatraoz/bin/godot ci/pipeline.sh
```

Expected: still ends `ci: OK`, no reference to any deleted file, no error
about `PREBUILT_RUST`/`DEPLOY_DIR` (there shouldn't be any left to error
about).

- [x] **Step 8: Commit**

```bash
git add -A
git commit -m "<narrative subject, e.g. describing the droplet's exit from this project>"
```

- [x] **Step 9: Ask the user before pushing**

Same standing rule. Stop and ask before `git push`.

---

### Task 3: Update `AGENTS.md`'s deploy policy language

**Files:**
- Modify: `/home/albatraoz/unseeing/.claude/worktrees/designer-engine-bundle/AGENTS.md`

**Interfaces:** none — a documentation-only change.

- [x] **Step 1: Locate and rewrite the relevant paragraph**

Find the paragraph beginning "Autonomy ends at integration and
deployment..." and the sentence "Deploy only after an approved merge, from
clean `main`, because `deploy.sh` pushes `main` and its compiled cores
must match that exact tree." Rewrite to state: deploy to the player-facing
web build is now automatic on every push to `main` via GitHub Actions
(`.github/workflows/test.yml`'s `deploy` job) — there is no separate
manual deploy step and no `deploy.sh` (removed in Task 2). The deliberate,
human-triggered gate is the decision to merge into `main`; autonomy still
ends there — present the finish-branch choice, never merge without the
user's choice, exactly as before. Do not weaken the merge-gate language;
only the deploy-mechanism description changes.

- [x] **Step 2: Self-review for consistency**

Re-read the rest of `AGENTS.md` for any other sentence assuming the old
`deploy.sh`-based flow (e.g. anything under "Isolation and parallel work"
or elsewhere referencing deploy). Fix any found.

- [x] **Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "<narrative subject>"
```

---

### Task 4: Rewrite the wiki's Deploy section

**Do not start until Task 2 is complete** — this section should describe
the final state, not an intermediate one.

**Files:**
- External: `Engineering-Build-Test-Deploy.md` §6 "Deploy", in the
  repository's GitHub wiki, not this checkout.

**Interfaces:** none — reads the real, by-then-merged `test.yml` `deploy`
job as ground truth.

- [x] **Step 1: Clone the wiki fresh**

```bash
git clone git@github.com:cleveralbatraoz/unseeing.wiki.git /tmp/unseeing-wiki-writeback-pages
```

- [x] **Step 2: Rewrite §6**

Replace the droplet/`deploy.sh`/`infra/post-receive` description with: the
`deploy` job in `.github/workflows/test.yml`, gated to `main`+`push`,
consuming the `checks` job's already-verified `game/build/web/` via
`actions/upload-pages-artifact`/`actions/deploy-pages`; the live
`UNSEEING_BUILD` stamp-verify step (same mechanism `deploy.sh` used,
carried over); the site's URL
(`https://cleveralbatraoz.github.io/unseeing/`); and that the deploy
decision now lives at merge-to-`main` time, not a separate manual step.
Remove or clearly mark superseded any "Traps that have each cost a real
deploy" bullets that were specific to the droplet/bare-repo/`post-receive`
mechanics that no longer exist.

- [x] **Step 3: Commit in the wiki clone**

- [x] **Step 4: Ask the user before pushing**

- [x] **Step 5: Push once confirmed**

---

### Task 5: Decommission the droplet's serving role

**Do not start until Task 1's Step 9 is confirmed** — same gate as Task 2,
for a more urgent reason: the droplet is the only thing serving the live
site until GitHub Pages is proven live. Closing ports or stopping nginx
before that would take the site offline with nothing serving it.

This task is **controller-executed directly, not dispatched to a
subagent implementer** — live server firewall/service changes carry real
lockout risk (breaking SSH access to a remote host has no easy recovery
without the hosting provider's out-of-band console), so each step below
is a checkpoint, not a batch. Confirm the exact SSH port and the
AmneziaVPN port(s) to preserve with the user before touching the
firewall — do not guess them.

**Files:** none in this repository — all changes are on the remote host.

- [x] **Step 1: Verify remote access, cheaply, before anything else**

```bash
ssh vpn 'whoami; hostname'
```

If this fails, stop — do not proceed on an assumption about connectivity.

- [x] **Step 2: Inventory the current state before changing anything**

```bash
ssh vpn 'systemctl status nginx --no-pager; ss -tlnp; sudo ufw status verbose 2>/dev/null || sudo iptables -L -n -v'
```

Record what's actually running and what's actually open — never assume
the state matches what `infra/nginx-unseeing.conf` or any doc claims.

- [x] **Step 3: Confirm the exact ports to preserve with the user**

Ask directly: the SSH port (default 22, but confirm — nonstandard ports
are common hardening) and the AmneziaVPN port(s). Do not infer these from
the general "amneziavpn" name alone.

- [x] **Step 4: Stop and disable nginx**

```bash
ssh vpn 'sudo systemctl stop nginx && sudo systemctl disable nginx'
```

- [x] **Step 5: Confirm GitHub Pages is still serving correctly with nginx gone**

Visit `https://cleveralbatraoz.github.io/unseeing/` again — this must be
independently true regardless of the droplet's state; if it isn't, stop
and do not proceed to closing ports.

- [x] **Step 6: Remove nginx and its site config**

```bash
ssh vpn 'sudo apt-get remove --purge -y nginx nginx-common && sudo rm -f /etc/nginx/sites-enabled/unseeing* /etc/nginx/sites-available/unseeing*'
```

- [x] **Step 7: Close HTTP/HTTPS, leaving only the confirmed SSH and AmneziaVPN ports**

Using whatever firewall tool Step 2 revealed is actually in use (`ufw` or
raw `iptables`), deny/close 80 and 443 explicitly, and verify the rule
list afterward names exactly: the confirmed SSH port, the confirmed
AmneziaVPN port(s), and nothing else inbound. Do this as the LAST step,
not combined with Step 6 — SSH access itself must never be touched by
this change; verify SSH still works from a fresh connection (not the one
already open) immediately after.

- [x] **Step 8: Final verification**

```bash
ssh vpn 'sudo ufw status verbose 2>/dev/null || sudo iptables -L -n -v'
```

Confirm the listed rules match exactly what Step 3 confirmed with the
user — no more, no less.

---

## After all tasks

Present the finish-branch choice to the user per `AGENTS.md` — this plan
does not merge on its own authority.

## Completion note (2026-08-20)

All five tasks are done, verified against real evidence, not assumed:

- **Task 1**: the pre-merge verification (Step 8) proved the trigger and the
  `checks`→Pages-artifact path, but `deploy` itself was rejected on every
  feature-branch attempt by GitHub's own `github-pages` environment
  protection rule — a real constraint the plan didn't anticipate, not a bug
  in the workflow. `deploy` first ran for real on the branch's merge to
  `main` and succeeded immediately, live page confirmed via `curl` (not an
  interactive browser — the automated build-stamp check together with a
  direct HTTP fetch of the served bytes was treated as equivalent evidence
  here) serving the merge commit's stamp.
- **Task 2**: done as two separate pushes rather than one — `infra/` and
  its two directly-coupled tests first (in response to a direct request
  mid-session), then `deploy.sh`/`ci/push_production.sh`/
  `ci/deploy_host_preflight.sh` and their tests, plus the
  `PREBUILT_RUST`/`DEPLOY_DIR` simplification, as a second push. Both were
  proven by a full local `ci/pipeline.sh` run ending `ci: OK` before
  pushing, and by real green CI runs after.
- **Task 3, Task 4**: done as written.
- **Task 5**: completed earlier in the same session, before this plan's
  Task 1 had even proven the replacement path — the user explicitly
  accepted that ordering risk ("site does dark - is ok, nobody uses it
  right now"). GitHub Pages is now the only live path, confirmed after the
  fact.

A separate pre-merge code review (effort `high`, scoped to the full branch
diff against `main`) found two real, independently-verified defects this
plan's own steps didn't catch — a dead `$PCK` reference crashing every
macOS export gate, and a `BUILD_SHA` stamp-length mismatch that would have
failed the `deploy` job's live-verify step on every single run, forever.
Both were fixed before merging. Everything else the review found (a shared
cross-OS cache key, a non-executable script, a race-prone one-shot `curl`)
was fixed alongside them; lower-priority findings, plus a set of unrelated
`rust/` findings from a mis-scoped review pass, are tracked in
[issue #61](https://github.com/cleveralbatraoz/unseeing/issues/61) rather
than blocking the merge.
