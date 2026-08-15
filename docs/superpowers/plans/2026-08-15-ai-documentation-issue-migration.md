# AI-Agent Documentation Issue Migration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to execute this external-state rollout only after
> its integration preconditions are true. Steps use checkbox (`- [ ]`) syntax
> for tracking.

**Goal:** Reconcile the live GitHub Issues backlog with the integrated
canonical documentation and verified shipped behavior, without letting a docs
rewrite masquerade as an implementation fix.

**Architecture:** This plan is a post-integration runbook, separate from the
repository implementation plan because every mutation targets live external
state. It first anchors one exact green remote-main SHA and its automatically
published Wiki mirror, creates the three verified residual issues, narrows the
three issues that remain open, closes only reverified resolved issues, then
reads everything back. A final isolated repository closeout replaces transient
artifact-registry outcomes with terminal truth and records only actual issue
numbers.

**Tech Stack:** Git, GitHub CLI, GitHub Actions readback, the integrated
standard-library documentation/Wiki tools, and the existing complete game
verification stack.

**Spec:**
`docs/superpowers/specs/2026-08-15-ai-documentation-source-of-truth-design.md`

**Repository implementation plan:**
`docs/superpowers/plans/2026-08-15-ai-documentation-source-of-truth.md`

## Global Constraints

- Every implementer inherits the project laws even though this rollout changes
  no mechanics: preserve outline-only perception, the same-facing coplanar
  superface merge, separate touching/source-role seams, `MIN_SEP = 0.08`, the
  `[0.15, 0.96]` safe band and sole radio-preview `Role::Case = 0.05`
  exception; preserve Godot designer objects plus pure total
  dependency-injected Rust laws, one `game/` project, and
  x86_64/arm64/wasm32 support. Closeout commits use
  `Dmitrii Galchenko <dggrus@gmail.com>`, carry no assistant attribution, and
  leave the pinned `tools/superpowers` gitlink/content untouched.
- Every task carries the repository-pinned workflow skills explicitly:
  `test-driven-development` for helper or repository behavior,
  `systematic-debugging` for an unexpected result,
  `verification-before-completion`, `requesting-code-review` after the task and
  before continuing, and `receiving-code-review` for every finding. Debugging a
  live-service failure is read-only and may never replay a mutation, weaken the
  fixed operation order, or turn ambiguous state into success. Each task's
  final review covers requirements, quality, receipt/recovery truth, and the
  complete diff or authoritative readback appropriate to that task.
- Do not begin this plan merely because the feature branch is green. A
  user-selected finish-branch path must have put the implementation on remote
  `main`; the exact `checks`, `windows-bootstrap`, and `publish-wiki` jobs for
  that SHA must succeed; Wiki readback must independently reproduce it.
- A kept branch, unmerged PR, local-only merge, failed/cancelled/skipped job,
  Wiki permission failure, or Wiki tree mismatch makes this plan ineligible.
  Automatic publication is sufficient: never manually publish, directly edit,
  force, or adopt the Wiki.
- Before every issue edit/close, reread its current title, body, labels, state,
  and update time. Reverify its disposition against the exact integrated tree.
  If one issue became stale, stop the entire ordered rollout before that
  mutation; never skip it or continue a blind batch. With no prior mutation,
  abandon the receipt and seek approval for a revised plan/new receipt. With
  any prior mutation, preserve the receipt, perform readback only, and seek an
  explicit recovery/revision plan; the fixed 40-operation prefix is never
  edited in place.
- Before every live create, edit, comment, close, reopen, restore, or comment
  deletion, refetch `origin/main` and Wiki `master` and require both still equal
  the frozen rollout anchor. An advance invalidates eligibility before the next
  mutation. Because the post-approval helper installs a decision intent before
  this fetch, an observed advance always closes that intent with an immutable
  `anchor-advance` block. If every operation is still unstarted, explicit user
  direction may then take the exact zero-mutation abandonment path and restart
  Task 1 under renewed approval. If any mutation may have occurred, retain the
  blocked receipt, perform only the operation readback permitted by its state,
  and stop for a new explicit user-approved recovery plan; never ordinary-
  resume, silently re-anchor, or rewrite immutable audit anchors in created
  issues.
- Do not create a tracked issue ledger, reusable project dependency, body
  snapshot, or mutation script. A mode-`0700`, untracked per-rollout receipt
  under the repository's common Git directory is required for crash recovery.
  It contains the full before-state, exact anchor, rendered request bodies,
  returned IDs/responses, immutable per-operation records, and one exact
  temporary standard-library guard/executor helper. That reviewed helper is the
  sole exception to the mutation-script ban: after approval it may execute only
  the 40 contract-derived GitHub operations while holding the receipt lock.
  These are operational state, never committed. A possibly mutated receipt is
  logically retired as an inactive immutable audit record only after the
  closeout branch is integrated and independently read back; the sole earlier
  retirement case is a reviewed, explicit abandonment whose closed schema
  proves no child could have spawned and no external mutation was possible.
  Receipt retirement never physically deletes its evidence.
- All rollout helpers sharing this repository also hold one fixed BSD flock at
  `<common Git directory>/unseeing-issue-rollout.lock` across every remote
  eligibility decision or issue mutation. This cross-receipt lock is permanent
  local operational state, not part of a receipt and never removed by receipt
  retirement. Approval is globally exclusive until the approved receipt is
  abandoned without any possible mutation or is terminally proved and retired:
  no second receipt may seal approval while such an active sibling exists, and
  every remote helper command rechecks that its receipt remains the sole active
  one. This prevents an API-consistency gap after a locally completed request
  from admitting a second rollout; it does not lock out a third-party GitHub
  operator, so the quiet-window and remote-TOCTOU rules still apply.
- Use only existing `enhancement` labels for the three new issues. Do not add
  labels, milestones, projects, assignees, PRs, releases, or comments unrelated
  to the approved disposition. Freeze the exact existing `enhancement` label in
  the anchor and revalidate it before every create.
- Create no vague audio, phantom-sound, speculative deployment-hardening, or
  mood-layer issue. The film-grain/vignette/filled-void policy conflict remains
  deliberately skipped and must be named in the final report.
- Issue and PR numbers share one sequence. Never predict or reserve the three
  new numbers; capture actual returned URLs, verify numeric IDs, then use those
  values in #15/#38 and registry closeout.
- No game code, behavior, deployment, Wiki fallback credential, or
  `tools/superpowers` change is authorized here. Repository changes happen only
  in the final dedicated closeout worktree and still require a second
  finish-branch choice.
- The protocol proves the complete normalized issue-and-comment surface defined
  below. Approved create/edit/comment/close requests necessarily create target
  timeline activity; provider timeline/events, notifications, subscriptions,
  reactions, and project membership are explicitly outside the guarantee. To
  avoid deliberate cross-target timeline events, no rendered issue body or
  closure comment may contain an autolinking issue reference; the final report
  states both boundaries.
- Every closure comment links an owning implementation commit, the canonical
  current page at the integrated full SHA, strongest executable evidence at
  that SHA, and the exact successful Actions run attempt. It explicitly states
  that the issue was reverified and was not closed because prose changed.
- Integration itself does not grant issue-mutation authority. Task 1 must prove
  the authenticated viewer has `WRITE`, `MAINTAIN`, or `ADMIN`
  permission and present the frozen anchor to the user. Tasks 2–4 start only
  after the user explicitly approves this live issue rollout; record that
  approval in the temporary receipt. `TRIAGE` is insufficient for this closed
  protocol because the approved create argv applies `enhancement` during issue
  creation and GitHub may otherwise create the issue while silently omitting
  the label.
- Pin `GH_TARGET=github.com/cleveralbatraoz/unseeing`,
  `GH_API_HOST=github.com`, and
  `CANONICAL_ORIGIN=https://github.com/cleveralbatraoz/unseeing` as literal
  read-only shell values, and set `GH_PROMPT_DISABLED=1`. Every `gh issue`/`gh run` call uses
  `--repo "$GH_TARGET"`; every `gh api` call uses
  `--hostname "$GH_API_HOST"`; authentication uses the same hostname. Never
  infer a host/repository from the current directory or inherited `GH_REPO`/
  `GH_HOST`. Validate every returned issue/run/comment URL against the exact
  `https://github.com/cleveralbatraoz/unseeing/` prefix and its expected
  positive ID before recording it.
- Before the first `gh` call, resolve one absolute no-follow regular GitHub CLI
  executable and hash its bytes, and strictly resolve one private,
  non-group/world-writable `GH_CONFIG_DIR` with no symlink component. Every
  read and mutation invokes only that absolute executable with explicit
  host/repository and a freshly constructed allowlisted environment; never
  use a shell alias, later `PATH` lookup, inherited `GH_REPO`/`GH_HOST`/token,
  or ambient current-directory discovery. The config directory supplies the
  existing authenticated session but is never copied, hashed, logged, or
  sealed because it contains a credential; viewer identity/permission are
  re-read and sealed separately.
- Before any Git authority, object, ancestry, worktree, status, or network read,
  construct a closed Git environment: start from an allowlist of required
  non-Git process keys, omit every inherited key beginning `GIT_` plus
  `SSH_ASKPASS` and curl credential/trace keys, then add only
  `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`,
  `GIT_TERMINAL_PROMPT=0`, `GIT_NO_REPLACE_OBJECTS=1`, and
  `GIT_NO_LAZY_FETCH=1`. Invoke proof commands with
  `git --no-replace-objects`. In every primary, rollout, closeout, temporary
  Wiki, and initialized Superpowers repository, reject `refs/replace/*`, any
  `info/grafts`, `objects/info/alternates`, or
  `objects/info/http-alternates` path, effective `extensions.partialClone`,
  `remote.*.promisor`, or `remote.*.partialCloneFilter`. Require non-shallow
  full history for the primary, rollout, closeout, and temporary Wiki
  repositories. The sole exception is the developer-only pinned
  `tools/superpowers` repository initialized by `tools/setup-agents.sh`, whose
  intended `git submodule update --init --depth 1` result may be shallow; still
  require its exact superproject gitlink/tag/content plus no replacement,
  graft, alternate, promisor, partial-clone, or lazy-fetch state. Never widen
  this exception to the parent or Wiki history.
  Require the needed anchor closure to produce no `?OID` from
  `rev-list --objects --missing=print` with lazy fetch disabled. Validate the effective source fetch
  and push URLs before fetching, use explicit refspecs for branch-oriented
  source/Wiki reads, and invoke public fetches with
  `-c credential.helper= -c core.askPass=`. This rollout never needs a Git
  credential for its public source/Wiki reads, and inherited URL
  rewrites/helpers may not redirect an authority. Scope these overrides to the
  read-only rollout/closeout fetch shells; if the user later selects a
  finish-branch option that writes Git state, invoke that repository workflow
  from a fresh shell under its own explicit authentication rules.
  Every bare `git` spelling in the illustrative blocks below denotes the
  resolved executable under this closed environment with
  `--no-replace-objects`; the snippets never weaken this global command
  contract.

### Workflow bootstrap before execution

Before loading or invoking this plan's required Superpowers execution/worktree
skill, use the clean durable primary checkout and a credential-free explicit
main fetch to resolve `PREBOOT_MAIN_SHA`, then prove the repository-pinned
installation. Require `PREBOOT_MAIN_SHA:tools/superpowers` is one
mode-`160000` entry; parse `pin=` from
`PREBOOT_MAIN_SHA:ci/superpowers.lock` without executing repository
code and require the exact OID matches. Require the durable primary's gitlink
equals that OID, run `ci/verify-superpowers.sh full`, then run
`tools/setup-agents.sh <active-host>` so the active host's installed cache is
derived from the same pin. If setup changes an installation, stop and begin a
new agent session before executing this plan. Apply the closed no-replacement,
no-graft, no-alternate, no-partial/lazy-fetch Git proof to these reads. Never
initialize the detached rollout worktree's submodule. Task 1 must require its
later `MAIN_SHA` equals `PREBOOT_MAIN_SHA` or repeat this entire bootstrap in a
new session. Seal the verified OID in
meta and repeat this proof against `CLOSEOUT_BASE_SHA` before Task 6 invokes
worktree or finish-branch skills; any pin/blob change requires renewed review.

## External Mutation Protocol

### Immutable receipt and replay guard

Do not use one mutable journal. Before the directory exists, Task 1 generates
one canonical lowercase RFC 4122 UUID with the standard library and creates the
exact basename `unseeing-issue-rollout-<UUID>`. The UUID component encoded by
that basename is the durable bootstrap `rollout_id`: the guard derives and
validates it before `meta.json` exists, and every later `rollout_id`, including
`meta.json`, must equal that UUID.
Task 1 creates this exact untracked layout.
Angle-bracket names are validated patterns, not optional places to invent new
files. Every directory is mode `0700`, every file—including both Python
sources—is mode `0600`, and no path is a symlink. The guard is always invoked
as data with the frozen absolute interpreter and exact flags `-I -B`; it is
never executable.

```text
RECEIPT_DIR/
  rollout_guard.py
  rollout_guard_test.py
  receipt.lock
  meta.json
  approval.json
  anchor/
    worktree-isolation.intent.json
    actions-runs.json
    actions-jobs.json
    helper-review.json
    worktree-isolation.json
    request-contract.json
    request-contract-review.json
    disposition-review.json
    auth-status.stdout
    auth-status.stderr
    auth-status-result.json
    wiki-verification.json
    viewer-permission.json
    enhancement-label.json
    issues-pages.json
    issue-<positive-issue-number>.json
    comments-<positive-issue-number>-pages.json
    pipeline.stdout
    pipeline.stderr
    pipeline-result.json
    visibility.stdout
    visibility.stderr
    visibility-result.json
    index.json
  before/
    issues.json
  body-<operation-id>.md
  preflights/
    <anchor-probe-or-operation-id>-<four-digit-sequence>.json
  decisions/
    <slot>-<four-digit-sequence>.intent.json
  operations/
    <operation-id>.intent.json
    <operation-id>.stdout
    <operation-id>.stderr
    <operation-id>.observation-<four-digit-sequence>.json
    <operation-id>.readback-<four-digit-sequence>.json
    <operation-id>.verified.json
  after/issues.json
  closeout/
    observations/
      <four-digit-sequence>.intent.json
      <four-digit-sequence>.verified.json
    isolation.intent.json
    isolation.json
    commit.json
    proof.json
    withdrawal.json
  recovery/<four-digit-sequence>.json
  blocked.json
  abandonment.json
  retirement.json
```

Directory creation installs only the directories and empty `receipt.lock`;
Task 1 then adds and reviews the two Python source files, seals the helper
review, uses that exact helper to install the worktree-isolation intent before
invoking the facility, and immediately seals the matching success. Every
other listed file is absent until its named
transition atomically seals it; `blocked.json`, `abandonment.json`,
`retirement.json`, `after/issues.json`, closeout records, recovery records, and operation
outputs remain absent unless that state occurs. No unlisted persistent path is
allowed.

The fixed mode-`0600` `unseeing-issue-rollout.lock` is a direct child of the
strictly resolved common Git directory, outside every receipt. It is the one
explicit persistent-path exception to the receipt roster, and receipt
retirement never deletes or renames it.

The reviewed helper owns the worktree intent/success transition before any
remote observation. A host-native facility is eligible only when its handle
can be queried by the planned path after caller death; otherwise this rollout
uses a Git worktree. On resume, intent plus exact facility enumeration proves
one of absent, correctly created, or malformed without relying on shell memory.
Both records are included in the anchor index.

`rollout_guard.py` is a temporary, standard-library-only external guard,
contract executor, and local receipt writer. It is passed as data to the
frozen interpreter with `-I -B`; it is never executable. Before approval it is
read-only.
After approval, only its `execute-operation` transition may spawn one of the
four exact contract-derived `gh issue create`/`edit`/`comment`/`close` argv
forms; it has no reopen/delete/push/general-command path. Before receipt
creation, Task 1 strictly resolves and explicitly reviews one absolute regular
host Python interpreter outside the repository, common Git directory, and
receipt; it freezes that path and byte SHA-256 in the helper review and meta.
Task 3 embeds that pair as literal `EXPECTED_PYTHON_EXECUTABLE` and
`EXPECTED_PYTHON_SHA256` constants in both reviewed helper/test sources before
their first invocation; those constants are the closed pre-review authority,
and the later helper review/meta must equal them byte-for-byte. Every guard and
guard-test invocation uses exactly that interpreter with `-I -B`; bare
`python3`, a PATH lookup after freezing, and non-isolated guard startup are
forbidden. The guard refuses unless `sys.flags.isolated` and
`sys.flags.dont_write_bytecode` are true, `sys.executable` strict-resolves and
hashes to the embedded interpreter, and `sys.path` contains neither the receipt,
repository/cwd, user site, nor an injected path. Isolated mode is still paired
with a constructed environment that removes every `PYTHON*` key. System Python
itself and its system site are an explicitly reviewed host dependency; receipt-
local modules and user/environment customization are not. The tracked renderer
is a separate boundary: it uses the same frozen interpreter with `-B`, its
verified absolute CLI and three sibling libraries, explicit rollout-worktree
cwd, the existing sanitized/no-shadow environment, and the main plan's normal
`tools/` sibling-import contract. It is never invoked with `-I`.
`rollout_guard.py`'s SHA-256 and the adjacent test's SHA-256 are frozen in
`meta.json`, and every later invocation
verifies both files before doing anything. The fixture test is run before
approval and after every new shell/compaction.
Every invocation validates `receipt.lock` without following links and proves
its inode/mode before and after open; POSIX record locks/`lockf` are forbidden.
Local-only `self-test`, `status`, and bootstrap writers acquire
`fcntl.flock(receipt_lock_fd, LOCK_EX | LOCK_NB)` for the entire command. The
closed command set `preflight`, `execute-operation`, pending-operation
`reconcile`, `seal-after`, `seal-eligibility-withdrawal`,
`begin-closeout-isolation`, `preflight-closeout-integration`,
`seal-closeout-proof`, `retire-receipt`, `seal-closeout-withdrawal`, and
`seal-approval` instead first no-follow opens the fixed common-Git lock. Each
proves it is a direct mode-`0600`, link-count-1 regular file owned by the
current uid with the same inode before/open/after, acquires its nonblocking BSD
flock, and only then opens/acquires the receipt flock. Lock order is always
global then receipt and release order is the reverse; a busy lock refuses
without waiting. After acquiring both locks, `seal-approval` and every remote
command no-follow enumerate only direct
`unseeing-issue-rollout-<UUID>` sibling receipts under that common directory.
An active receipt is a well-formed sibling containing `approval.json` without a
valid zero-mutation `abandonment.json` or valid `retirement.json`, regardless
of pending, verified, blocked, ambiguous, not-applied, or
terminal-but-unretired state. At most one
active receipt may exist: approval refuses another active sibling and every
remote command requires the current receipt be the sole active one. A valid
abandoned or retired receipt does not exclude a new approval; its complete
sealed shape is still validated, while a malformed or unreadable candidate
always refuses. Thus a second rollout cannot exploit API consistency
lag even after the first operation has a terminal local record. `execute-operation` retains both locks continuously across its decision
fence, preflight, body rendering, operation-intent installation, child
execution, stream capture, authoritative readback, observation, and
verification. It starts the `gh` child with
`subprocess.Popen(..., shell=False, close_fds=True,
pass_fds=(global_lock_fd, receipt_lock_fd), start_new_session=True)` and tests
that the child keeps both BSD-style flock-bearing descriptors open, so a guard
crash cannot admit reconciliation or another receipt while the issued request
is still running. The POSIX host running this shell plan must provide
standard-library `fcntl`; there is no unchecked fallback.

The executor also uses `start_new_session=True` and a reviewed finite
`CHILD_TIMEOUT_SECONDS=120`. `communicate(timeout=...)` reaps an ordinary
exit; on timeout it sends `SIGTERM` to the child process group, allows at most
five seconds for a reaped exit, then sends `SIGKILL` and waits for the leader.
Once spawn succeeded, timeout/termination is still `ambiguous` unless the
authoritative consistency poll proves the exact applied delta; it is never
replayed or called not-applied. A fake child that hangs, ignores TERM, forks,
or exits while holding/writing streams fixture-tests bounded termination,
reaping, lock exclusion, and absence of a second command. A surviving
descriptor-holding descendant keeps reconciliation safely locked out and is
reported for operator recovery rather than bypassed.

The fixed lock serializes every compliant rollout receipt in this common Git
directory, while the receipt lock serializes one receipt. GitHub Issues
offers no compare-and-swap precondition to the four approved `gh issue`
commands, so it cannot make the remote read/mutate pair transactional. The
locked immediate preflight makes the window as small as this interface allows,
and readback catches divergence that remains observable, but an authorized
third party could edit an issue title/body in that interval and have the three
planned `edit-issue` operations overwrite those same fields. Before approval,
the user must explicitly confirm a quiet rollout window for issue operators;
that coordination remains in force through closeout proof and receipt
retirement, whose fresh issue projections fail closed on any drift.
Any known or observed concurrent writer—including activity that later restores
identical normalized bytes—must atomically seal an
`eligibility-withdrawn/quiet-window-violation` block before terminal readback,
or `closeout/withdrawal.json` after terminal readback, and stop the rollout. The receipt and
final report must state this irreducible remote TOCTOU limitation and must not
claim proof that no transient overwritten edit occurred.

All receipt writes use one implementation. Validate a receipt-relative final
path against the roster above with no empty, absolute, dot, dot-dot, or
symlink component. Serialize or read the complete candidate bytes once before
installation and compute their decimal byte size and SHA-256. Create only a
same-directory
`.<final-basename>.tmp-<decimal-size>-<64-lowercase-hex-sha256>-<32-lowercase-hex>`
file with `O_CREAT | O_EXCL` at mode `0600`; write canonical JSON
(`sort_keys=True`, compact separators, one terminal LF) or the already
rendered bytes; flush and `fsync`; install with a same-filesystem hard link so
an existing destination fails; `fsync` the directory; unlink the temporary
name; then `fsync` the directory again. The authenticated size/hash fields are
part of the only accepted temporary grammar, so a crash-shortened otherwise
valid body or opaque stream cannot be mistaken for the complete candidate.
Operation state is flat, so installing one intent file is the atomic transition
from `UNSTARTED`; there is no separately created operation directory.

Every helper command first refuses unresolved temporary files. `reconcile` is
the only command allowed to repair them. For an exact temporary pattern whose
mapped final is absent, it recomputes the size/hash, requires both to equal the
temporary name, validates the complete bytes/schema, re-`fsync`s the file, and
finishes the link/fsync/unlink/fsync sequence. If the final is the same inode,
it removes only the redundant temporary link and fsyncs. A different final
inode is a hard refusal. A size/hash-mismatched partial temporary with no final
can be removed only after `reconcile` proves from immutable state that no later
operation file could depend on it and seals an exact recovery record first;
it is never finalized. A pending live operation is recovered from authoritative
API readback, never from partial stdout. Body/opaque writer leftovers use this
same authenticated filename mapping and rule, and their original closed
external candidate remains until installation is verified. Malformed JSON,
an unexpected key/path, duplicate operation ID, path escape, symlink, unknown
temporary, or existing different destination is a hard refusal rather than an
invitation to hand-edit the receipt; the one size/hash-mismatched
known-pattern partial is removable only through the recovery transition above.

`meta.json` contains exactly:

```json
{
  "schema_version": 1,
  "rollout_id": "<UUID>",
  "host": "github.com",
  "repository": "cleveralbatraoz/unseeing",
  "source_remote": "https://github.com/cleveralbatraoz/unseeing",
  "wiki_remote": "https://github.com/cleveralbatraoz/unseeing.wiki.git",
  "primary_root": "<strictly resolved durable-primary absolute path>",
  "common_git_dir": "<strictly resolved absolute common Git directory>",
  "rollout_worktree": "<strictly resolved absolute detached-worktree path>",
  "worktree_facility": "<git-worktree|host-native>",
  "worktree_cleanup_handle": "<null or nonempty opaque host-native handle>",
  "worktree_isolation_sha256": "<anchor/worktree-isolation.json SHA-256>",
  "integrated_main_sha": "<40 lowercase hex>",
  "main_sha": "<40 lowercase hex>",
  "run_id": "<positive decimal string>",
  "run_attempt": <positive decimal integer>,
  "run_url": "<canonical Actions URL>",
  "run_attempt_url": "<run_url>/attempts/<run_attempt>",
  "wiki_head": "<40 lowercase hex>",
  "wiki_source_sha": "<same as main_sha>",
  "viewer_login": "<authenticated login>",
  "viewer_permission": "<WRITE|MAINTAIN|ADMIN>",
  "superpowers_gitlink_oid": "<40 lowercase tools/superpowers gitlink OID>",
  "python_executable": "<absolute strictly resolved reviewed regular interpreter path>",
  "python_sha256": "<that interpreter's SHA-256>",
  "gh_executable": "<absolute resolved regular executable path>",
  "gh_sha256": "<that executable's SHA-256>",
  "gh_config_dir": "<strictly resolved absolute private GitHub CLI config directory>",
  "helper_sha256": "<rollout_guard.py SHA-256>",
  "helper_test_sha256": "<rollout_guard_test.py SHA-256>",
  "verification_tool_sha256": {
    "tools/documentation_markdown.py": "<MAIN_SHA blob SHA-256>",
    "tools/documentation_contract.py": "<MAIN_SHA blob SHA-256>",
    "tools/wiki_mirror.py": "<MAIN_SHA blob SHA-256>",
    "tools/render-wiki.py": "<MAIN_SHA blob SHA-256>"
  },
  "request_contract_sha256": "<anchor/request-contract.json SHA-256>",
  "disposition_review_sha256": "<anchor/disposition-review.json SHA-256>",
  "evidence_index_sha256": "<anchor/index.json SHA-256>",
  "created_at": "<UTC API-compatible timestamp>",
  "operation_order": ["<all 40 stable operation IDs in planned order>"]
}
```

`run_url` must be exactly
`https://github.com/cleveralbatraoz/unseeing/actions/runs/<run_id>`, and
`run_attempt_url` is derived—not accepted from provider or shell—as that exact
URL plus `/attempts/<run_attempt>`. Both URLs, the positive integer attempt,
and the decimal run ID must agree before meta can seal.

The exact order is:

```text
create-first-paint
create-crease-knee
create-native-load
edit-14
edit-15
edit-38
comment-7
close-7
comment-12
close-12
comment-13
close-13
comment-16
close-16
comment-22
close-22
comment-30
close-30
comment-31
close-31
comment-32
close-32
comment-33
close-33
comment-34
close-34
comment-35
close-35
comment-36
close-36
comment-39
close-39
comment-41
close-41
comment-42
close-42
comment-44
close-44
comment-45
close-45
```

Each raw GitHub response in `anchor/` is a closed envelope containing exactly
`schema_version`, `captured_at`, complete literal `command_argv`,
`command_exit_status`, and `payload`; only `payload` may contain arbitrary
provider-owned JSON keys. `helper-review.json` contains exactly
`schema_version`, `python_executable`, `python_sha256`, `helper_sha256`,
`helper_test_sha256`, `requirements_status`, `requirements_reviewed_at`,
`security_status`, `security_reviewed_at`, and `blocker_count`, which must be
zero. The interpreter pair equals meta and the strict no-follow byte review;
neither record admits a later PATH lookup.

`worktree-isolation.intent.json` contains exactly `schema_version`,
`rollout_id`, `main_sha`, strictly resolved absolute `primary_root`,
`common_git_dir`, `planned_worktree`, `requested_facility` (`git-worktree` or
`host-native`), and `requested_at`. It is installed before the facility may
create anything. `worktree-isolation.json` contains exactly `schema_version`,
`rollout_id`, `main_sha`, `intent_path`, `intent_sha256`, the same strictly
resolved `primary_root`/`common_git_dir`, `rollout_worktree`, `facility`,
nullable `cleanup_handle`, and `created_at`. Its intent path is exactly
`anchor/worktree-isolation.intent.json`, its hash matches, and every repeated
value agrees. `cleanup_handle` is null for `git-worktree` and a nonempty opaque
string for `host-native`. The success record is sealed immediately after
creation, before any remote observation or repository edit. Together the
two records are the durable recovery/cleanup authority: the helper never
executes an opaque host-native handle, and the same isolation facility may
interpret it only after independently revalidating paths, common directory,
HEAD, cleanliness, and its queryable mapping.

`request-contract.json` contains exactly `schema_version`, `rollout_id`,
`main_sha`, positive integer `run_attempt`, `run_url`, `run_attempt_url`, and an
operation-order-preserving `operations` list.
Each operation object contains exactly `sequence`, `operation_id`, `kind`,
nullable `issue_number`, nullable `title`, sorted `label_names`, nullable exact
LF-only `body_template`, and a token-sorted `substitutions` list. Each
substitution contains exactly `token`, `source_kind` (`meta`,
`verified_issue_number`, or `literal`), `source_name`, and nullable
`literal_value`. `meta` admits only `main_sha`, `run_attempt`, `run_url`, and
`run_attempt_url` with null literal; integer `run_attempt` renders as its
canonical unsigned decimal text, while the other meta sources are strings.
`verified_issue_number` admits only a preceding create operation ID with null
literal; `literal` requires a nonempty literal value and a stable descriptive
source name. Token set equality with the template is mandatory: no undeclared,
unused, duplicate, or unresolved `@@[A-Z0-9_]+@@` token is accepted.
The contract's four top-level meta values equal meta exactly, and its attempt
URL must equal its run URL plus `/attempts/` and the canonical decimal attempt.

The 40 rows are exact copies of this plan's approved operation kinds, fixed
existing issue numbers, six create/edit titles, label sets, and fenced body or
closure-comment templates. The only dynamic body sources are
`meta.main_sha`, `meta.run_attempt`, `meta.run_url`,
`meta.run_attempt_url`, the verified issue numbers from
`create-crease-knee`/`create-native-load`, and preapproved literal URL lists.
Before approval, every literal implementation URL resolves to its verified
full ancestor commit, while every current-page and evidence blob URL uses
exactly `meta.main_sha`; every named path is checked in the SHA it cites and
stored in this contract. No environment value or shell-local URL enters
rendering. Every nonnull body template contains both `@@RUN_ATTEMPT@@` and
`@@RUN_ATTEMPT_URL@@`, contains no `@@RUN_URL@@`, and renders the exact
attempt-qualified evidence line shown below. Close rows have null title/body
and no substitutions.

Create rows have exactly `label_names=["enhancement"]`. Every edit, comment,
and close row copies the exact sorted label names from that subject's complete
normalized object in sealed `before/issues.json`; both operations in one
comment/close pair carry the same frozen set. Contract review proves these
values against before-state and each preflight requires current labels equal
both the row and the expected replayed state. Existing labels are immutable
anchor inputs, never body substitutions. After all substitutions, every
body/comment is rejected if it contains an autolinking cross-target form:
`#<positive-number>`, `GH-<positive-number>`,
`owner/repository#<positive-number>`, or a canonical `/issues/<positive-number>`
URL. Prose names issue numbers without those link-generating forms.

`request-contract-review.json` contains exactly `schema_version`,
`request_contract_sha256`, `plan_path`, `plan_sha256`, `reviewed_at`,
`requirements_status`, `security_status`, and zero `blocker_count`. The plan
path is this integrated artifact and its SHA-256 is computed from `MAIN_SHA`'s
Git blob, not the rollout worktree. Review proves every row/body/title/URL
against these fenced approvals and the exact owner/evidence paths before the
user sees the anchor.

`disposition-review.json` contains exactly `schema_version`, `rollout_id`,
`main_sha`, `request_contract_sha256`, `requirements_status`,
`requirements_reviewed_at`, `evidence_status`, `evidence_reviewed_at`, zero
`blocker_count`, and an operation-order-preserving `rows` list. It has exactly
23 rows and covers every one of the 40 operation IDs exactly once: three
single-operation creates, three single-operation rewrites, and 17 closure rows
that each own `[comment-N, close-N]`. Each row contains exactly `sequence`,
`disposition_id`, `operation_ids`, `decision` (`create`, `rewrite`, or
`close`), nullable positive `issue_number`, `residual_present`,
`shipped_resolution_present`, and path-sorted `owner_evidence`. Each evidence
item contains exactly repository-relative `path`, Git `mode`, `blob_oid`, byte
`sha256`, and sorted nonempty `tokens`. The eligibility matrix is exact:
`create` requires residual present and shipped resolution absent; `rewrite`
requires both residual and shipped resolution present; `close` requires
residual absent and shipped resolution present. Both independent reviewers
must report pass. Every path/mode/blob/hash/token is measured at exact
`MAIN_SHA`; this artifact, not a shell opinion, proves each proposed issue is
still necessary and each closure is still justified.

`wiki-verification.json` contains exactly
`schema_version`, `source_remote`, `source_branch`, `source_sha`,
`wiki_remote`, `wiki_branch`, `wiki_head`, `wiki_source_sha`,
`renderer_format`, `compared_tree_oid`, `command_argv`, `observed_at`, and
`success`. `pipeline-result.json`, `visibility-result.json`, and
`auth-status-result.json` contain exactly `schema_version`, `command_argv`,
`command_exit_status`, `stdout_path`, `stdout_sha256`, `stderr_path`,
`stderr_sha256`, `started_at`, `finished_at`, and `success`.
`anchor/index.json` contains exactly `schema_version` and a path-sorted
`entries` list of exact `path`, byte `size`, and `sha256` for every other
sealed `anchor/` file plus `before/issues.json`; its own hash is frozen in
`meta.json`.

`approval.json` contains exactly `schema_version`, `rollout_id`,
`meta_sha256`, `approved_scope`, `approved_at`, and
`conversation_reference`; `approved_scope` is the exact ordered 40-item
`meta.operation_order` list, not free-form prose or a broader authority grant.
`before/issues.json` and `after/issues.json` each
contain exactly `schema_version`, `captured_at`, and a number-sorted `issues`
list. The normalized protocol issue object uses snake_case only: `number`,
`url`, `title`, `body`, `state`, `state_reason`, `locked`, nullable
`active_lock_reason`, nullable `issue_type` object containing exactly positive
`id` and nonempty `name`, nullable
`pinned_comment_id`, sorted `labels`, sorted `assignees`, `milestone`
number/title or null, `author`, `created_at`, `updated_at`, nullable `closed_at`,
nullable `closed_by`, and a comment list sorted by numeric ID. Every comment
contains exactly `id`, `url`, `author`, `body`, `created_at`, and `updated_at`.
`state`/`state_reason` values are normalized to lowercase. REST
`state_reason`, provider `type` to `issue_type`, provider `pinned_comment.id`
to `pinned_comment_id`, and camel-case CLI fields are converted at the
boundary; comparisons never mix spellings. The protocol deliberately scopes
repository project membership, reactions, timeline/events, notifications, and
subscriptions out. Approved operations do create target timeline activity, so
the protocol does not claim equality or absence there and does not call this
REST normalization every possible GitHub issue surface. Task 1
captures the full paginated comments for every issue, so whole-backlog equality
never silently ignores a comment field.

Every preflight/readback snapshot contains exactly `schema_version`,
`rollout_id`, `slot`, one-based `snapshot_sequence`, `phase` (`preflight` or
`readback`), `observed_at`, `main_sha`, `wiki_head`, `wiki_source_sha`,
`viewer_login`, `viewer_permission`, nullable `enhancement_label_sha256`,
`subject_kind` (`existing` or
`creation`), nullable `subject_issue_number`/`subject_title`, and a normalized
`issues` list. An existing subject has exactly one complete issue; a creation
subject has the complete paginated issue set. A creation preflight has a
nonnull label hash equal to the normalized label frozen in the expected anchor;
every non-creation snapshot has null. A preflight filename's slot is
one of the 40 operation IDs or the two fixed probes `anchor-existing-14` and
`anchor-create-first-paint`; sequences are append-only.

Every decision-intent file contains exactly `schema_version`, `rollout_id`,
`slot`, one-based `decision_sequence`, `decision_kind` (`preflight`,
`after-backlog`, or `eligibility-withdrawal`), nullable `operation_id`,
`meta_sha256`, `expected_anchor_sha256`, `subject_kind` (`existing`,
`creation`, `backlog`, or null), nullable `subject_issue_number`, nullable
`subject_title`, nullable `expected_state_sha256`, nullable `disposition_id`,
nullable `disposition_row_sha256`, and `requested_at`. The slot is one of the
40 operation IDs, the two fixed anchor probes, `after-backlog`, or
`eligibility-withdrawal`; its filename sequence equals the record. The expected
anchor hash is over canonical JSON plus LF containing exactly frozen main SHA,
Wiki head/source SHA, viewer login/permission, and a nullable normalized
`enhancement`-label hash (nonnull only for a creation-capable slot).

A preflight intent has the exact existing/creation subject and expected-state
hash plus a disposition ID/row hash; each operation maps to the sole reviewed
row that contains it. `anchor-create-first-paint` maps to
`create-first-paint`, and `anchor-existing-14` maps to `edit-14`. An
after-backlog intent has backlog subject and the complete expected final-set
hash but null operation/disposition fields. An eligibility-withdrawal intent
has null subject/state/operation/disposition fields. Under both locks, the
guard installs this intent after approval and before the first source/Wiki
fetch, GitHub auth, label, issue, comment, or other remote re-observation used
to decide whether the slot can proceed. The read-only bootstrap observations
that construct the immutable anchor precede approval and are sealed by their
bootstrap snapshot transitions instead. A successful preflight is closed only by the same
slot/sequence preflight snapshot; a successful after-backlog decision is
closed only by `after/issues.json`; a failed decision is closed only by
`blocked.json` referencing this intent. At most one decision intent may lack a
closing artifact.

Every `<operation-id>.intent.json` contains exactly `schema_version`,
`rollout_id`, stable `operation_id`, one-based operation `sequence`, `kind`,
nullable `issue_number` and `comment_id`, `meta_sha256`,
`expected_before_path`/`expected_before_sha256`, nullable
`body_path`/`body_sha256`/`title`, sorted `label_names`, the complete literal
`command_argv`, and `requested_at`. Every observation contains exactly
`schema_version`, `rollout_id`, `operation_id`, `sequence`,
`observation_sequence`, `observed_at`, nullable `command_exit_status`,
nullable paired `stdout_path`/`stdout_sha256`, nullable paired
`stderr_path`/`stderr_sha256`, nullable
`returned_issue_number`/`returned_issue_url`, nullable
`returned_comment_id`/`returned_comment_url`, and
`readback_path`/`readback_sha256`. A verified record contains exactly
`schema_version`, `rollout_id`, `operation_id`, `outcome` (`applied`,
`not-applied`, or `ambiguous`), `observed_at`, nullable
`issue_number`/`issue_url`, nullable `comment_id`/`comment_url`, nullable
`actor`, `readback_path`/`readback_sha256`, nullable final `updated_at`, and a
nonempty `reason`. Each optional stdout/stderr path/hash pair is either both
strings or both null, and each nullable ID/URL pair is either both nonnull or
both null; every observation/verification has a nonnull authoritative
readback path/hash.

`kind` is a closed enum and is derived from, not chosen independently of, the
operation ID: the three `create-*` IDs are `create-issue`; `edit-14`,
`edit-15`, and `edit-38` are `edit-issue`; every `comment-<N>` is
`create-comment`; every `close-<N>` is `close-issue`. Apply this exact matrix:

- a `create-issue` intent has null issue/comment IDs, nonnull title/body
  path/body hash, and exactly `label_names=["enhancement"]`;
- an `edit-issue` intent has the positive issue suffix, null comment ID,
  nonnull title/body path/body hash, and the exact sorted preflight labels that
  the operation preserves;
- a `create-comment` intent has the positive issue suffix, null comment ID,
  null title, nonnull body path/body hash, and the exact preserved preflight
  labels;
- a `close-issue` intent has the positive issue suffix, null comment ID/title/
  body path/body hash, and the exact preserved preflight labels.

The executor derives the complete child argv in this exact order, substituting
only the sealed contract/receipt values (the first element is the `gh`
executable resolved before approval):

```text
create-issue:   ABSOLUTE_GH issue create --repo github.com/cleveralbatraoz/unseeing --title TITLE --label enhancement --body-file ABSOLUTE_SEALED_BODY
edit-issue:     ABSOLUTE_GH issue edit ISSUE --repo github.com/cleveralbatraoz/unseeing --title TITLE --body-file ABSOLUTE_SEALED_BODY
create-comment: ABSOLUTE_GH issue comment ISSUE --repo github.com/cleveralbatraoz/unseeing --body-file ABSOLUTE_SEALED_BODY
close-issue:    ABSOLUTE_GH issue close ISSUE --repo github.com/cleveralbatraoz/unseeing --reason completed
```

The same immutable tuple is recorded in intent and given to `Popen`; no
operator-supplied option, environment repository, body path, or alternate host
is accepted.

In an observation, issue response ID/URL may be nonnull only for
`create-issue`, comment response ID/URL may be nonnull only for
`create-comment`, and both response pairs are null for edit/close. The allowed
pair may still be null when command output was lost; authoritative readback is
never null. An `applied` verified record requires: for create, a nonnull issue
pair, null comment pair, author actor, and issue `updated_at`; for edit, the
canonical issue pair whose number equals `intent.issue_number`, null comment
pair/actor, and issue `updated_at`; for comment, that same canonical issue pair,
the new comment pair, comment-author actor, and final issue `updated_at`; and
for close, that same canonical issue pair, null comment pair/actor, and issue
`updated_at`. Every identity URL is canonical and matches its positive ID. A
`not-applied` or `ambiguous` verified record has all
issue/comment identity, actor, and `updated_at` fields null regardless of kind,
plus authoritative readback and a nonempty reason; the known existing issue
remains frozen in its intent. These matrices are fixture-tested for every
nullable/nonnullable swap and prefix/kind mismatch.

`blocked.json` contains exactly `schema_version`, `rollout_id`, `block_kind`
(`anchor-advance`, `issue-divergence`, `backlog-divergence`,
`eligibility-withdrawn`, or `interrupted-decision`), `detected_at`, nonnull
`decision_intent_path`/`decision_intent_sha256`, nullable
`old_main_sha`/`new_main_sha`, nullable `old_wiki_head`/`new_wiki_head`, nullable
`subject_kind` (`existing` or `creation`), nullable positive
`subject_issue_number`, nullable `subject_title`, nullable
`expected_state_sha256`/`observed_state_sha256`, nullable
`observed_issue_set`, nullable
`withdrawal_kind` (`quiet-window-violation`,
`stale-disposition-evidence`, `authority-withdrawn`, or
`local-integrity-failure`), nullable
`withdrawal_reference`, operation-order-preserving
`complete_operation_ids`, nullable `pending_operation_id`, and nonempty
`reason`. Each recovery record contains
exactly `schema_version`, `rollout_id`, one-based `recovery_sequence`,
`recovered_at`, `temporary_path`, `final_path`, nonnull `temporary_sha256`,
`action` (`finalized`, `unlinked-redundant`, or
`removed-partial-uninstalled`), and `reason`.

Each blocked old/new pair is either wholly null or two full SHAs; each nonnull
old value equals `meta`'s frozen anchor and its new value differs. For
`anchor-advance`, at least one head pair is nonnull and every subject/state/
observed-issue-set field is null. For `issue-divergence`, both head
pairs are null; subject kind, the two
distinct canonical state hashes, and `observed_issue_set` are nonnull. An
existing subject embeds exactly one complete normalized issue; a creation
subject embeds the complete paginated issue set; canonical serialization of
that embedded value must equal `observed_state_sha256`. An existing subject has its
positive number and null title; a creation subject has null number and the
exact frozen title. For `backlog-divergence`, both head pairs and every subject
field are null; the two distinct complete-set
hashes and a nonnull complete normalized `observed_issue_set` are present, its
canonical hash equals `observed_state_sha256`, all 40 completed IDs are
present, and pending is null. Completed IDs are otherwise exactly a prefix of the operation order,
and the nullable pending ID is either their immediate successor or null. Every
non-withdrawal block has null withdrawal fields. For
`eligibility-withdrawn`, every head/subject/state/observed-set field
is null, `withdrawal_kind` and a nonempty immutable
`withdrawal_reference` are present, completed IDs are the exact verified prefix,
and pending is null. Every
recovery transition concerns a no-follow-opened regular temporary and therefore
requires a nonnull SHA-256 of its actual bytes; the schema retains no null
exception for `temporary_sha256`.

Every block closes exactly the referenced decision intent; the path is the
canonical receipt-relative decision filename and its hash matches the
no-follow-opened bytes. No success counterpart for that intent may exist. For
`interrupted-decision`, every head, subject, state, observed-set, and withdrawal
field is null, completed IDs equal the verified operation prefix, pending is
null, and the reason identifies conservative recovery from an unclosed
decision. It never repeats the remote observation and cannot be cleared by a
later matching remote state.

`abandonment.json` contains exactly `schema_version`, `rollout_id`,
`abandoned_at`, `conversation_reference`, `phase_at_abandonment` (`bootstrap`,
`anchored`, `approved`, or `blocked`), nullable `blocked_sha256`, nullable
`safe_body_path`/`safe_body_sha256`, and a nonempty `reason`. It can be sealed
only after explicit user direction when no operation intent, observation,
verified record, or other evidence of a possible external mutation exists and
every operation is still `UNSTARTED`. Every decision intent must already have
its valid success or block counterpart; an unresolved decision permanently
forbids abandonment. Completed read-only anchor probes are allowed. A
`bootstrap` abandonment requires the exact
reviewed helper/test bytes and `helper-review.json`, valid sealed worktree
isolation, no meta/approval/block/body/operation/after/closeout artifact, no unresolved
temporary, and validation of every installed bootstrap artifact. An
`anchored` abandonment requires valid meta, no approval/block/body/operation
or closeout artifact, and an empty prefix. An `approved` or zero-prefix `blocked` receipt may contain exactly the correct
first-operation body and no other body: the guard independently re-renders it
from the contract and requires exact path, bytes, hash, mode, and inode, then
records that paired safe-body path/hash. If no body exists both fields are
null. If a block exists, its hash must match and its complete/pending lists
must both be empty; otherwise `blocked_sha256` is null. Any installed operation
intent or nonempty verified prefix permanently forbids abandonment.

`retirement.json` is an immutable logical tombstone; receipt retirement never
deletes or renames the receipt or any child. It contains exactly
`schema_version`, `rollout_id`, `underlying_phase` (`TERMINAL` or `ABANDONED`),
nullable `meta_sha256`, `terminal_record_path`/`terminal_record_sha256`,
nullable `retirement_observation_intent_path`/
`retirement_observation_intent_sha256`, nullable
`retirement_observation_verified_path`/
`retirement_observation_verified_sha256`, `repository_root`, `common_git_dir`,
`receipt_root_device`, `receipt_root_inode`, `receipt_root_mode`, path-sorted
`receipt_manifest`, `receipt_manifest_sha256`, and `retired_at`. Each manifest
item contains exactly receipt-relative `path`, `kind` (`file` or `directory`),
four-character string `mode`, nullable decimal `size`, nullable `sha256`, decimal `device`,
decimal `inode`, and decimal `nlink`. It covers every pre-retirement receipt
child and directory exactly once, excluding only the not-yet-installed
`retirement.json`; file size/hash are nonnull and directory size/hash are null.
File mode is exactly `0600`, directory and receipt-root mode exactly `0700`;
Canonical JSON-plus-LF hashing of the complete list equals the manifest hash.
Future validation requires the same no-follow root/path identities, modes,
link counts, file sizes/hashes, complete path set plus exactly the tombstone,
the tombstone as a link-count-1 mode-`0600` regular file, and no authenticated
temporary or unlisted child.

A terminal tombstone has nonnull meta, names/hashes `closeout/proof.json`, and
names/hashes the fresh same-sequence retirement observation intent and verified
record. An abandoned tombstone names/hashes `abandonment.json`, uses the meta
hash exactly when that receipt has meta, and has all four observation-reference
fields null. The repository/common-Git roots equal the already sealed
authority. Installation is the sole retirement transition: once present, the
receipt is inactive but remains a complete read-only audit record. A malformed
or mutated tombstone/manifest still makes global sibling validation refuse;
there is no cleanup command, recursive deletion, or shell-memory recovery path.

Closeout state is immutable and separately crash-resumable. Its observation
sequences are one-based, gap-free, and shared by all four remote closeout
stages. At most one intent may be unclosed, and it is the highest sequence.
Each `closeout/observations/<sequence>.intent.json` contains exactly
`schema_version`, `rollout_id`, `meta_sha256`, `observation_sequence`, `stage`
(`begin-closeout-isolation`, `preflight-closeout-integration`,
`seal-closeout-proof`, or `retire-receipt`), `prior_substate`,
`authority_record_path`/`authority_record_sha256`, nullable
`expected_main_sha`, nonnull `expected_projection_sha256`, and `requested_at`.
The exact matrix is:

| Stage | Prior substate | Authority record | Expected main |
|---|---|---|---|
| `begin-closeout-isolation` | `NO_CLOSEOUT` | `meta.json` | `meta.main_sha` |
| `preflight-closeout-integration` | `COMMITTED` | `closeout/commit.json` | `commit.base_sha` |
| `seal-closeout-proof` | `COMMITTED` | `closeout/commit.json` | null; accepted integration ancestry is derived from that commit record |
| `retire-receipt` | `PROVED` | `closeout/proof.json` | `proof.integrated_main_sha` |

The authority path/hash matches the no-follow-opened immutable record, and the
projection hash is always the canonical exact 23-subject projection derived
from `after/issues.json`. A
`closeout/observations/<sequence>.verified.json` is allowed only for
`preflight-closeout-integration` or `retire-receipt` and contains exactly
`schema_version`, `rollout_id`, `meta_sha256`, `observation_sequence`, `stage`,
`observation_intent_path`/`observation_intent_sha256`, `observed_main_sha`,
`observed_projection_sha256`, number-sorted `disposition_issues`, and
`verified_at`. Its main and projection equal the intent expectations, and the
helper derives every field.

The five exact closeout state records are:

- `closeout/isolation.intent.json`: `schema_version`, `rollout_id`,
  `meta_sha256`, `observation_intent_path`, `observation_intent_sha256`,
  `base_sha`, `branch`, `planned_worktree`,
  `requested_facility`, `primary_root`, `common_git_dir`, path-sorted
  `artifact_blob_oids`, `base_run_id`, positive `base_run_attempt`,
  `base_run_url`, `base_run_attempt_url`, `base_wiki_head`,
  `base_wiki_source_sha`, number-sorted `disposition_issues`, and `requested_at`;
- `closeout/isolation.json`: `schema_version`, `rollout_id`, `meta_sha256`,
  `isolation_intent_path`, `isolation_intent_sha256`, `base_sha`, `branch`,
  `worktree`, `facility`, nullable `cleanup_handle`, `primary_root`,
  `common_git_dir`, and `created_at`;
- `closeout/commit.json`: `schema_version`, `rollout_id`, `meta_sha256`,
  `isolation_sha256`, `base_sha`, `closeout_commit_sha`, `parent_sha`,
  `tree_oid`, `registry_blob_oid`, `registry_sha256`,
  `contract_test_blob_oid`, `contract_test_sha256`, the same
  `artifact_blob_oids`, and `committed_at`;
- `closeout/proof.json`: `schema_version`, `rollout_id`, `meta_sha256`,
  `observation_intent_path`, `observation_intent_sha256`, `isolation_sha256`,
  `commit_sha256`, `integration_mode` (`fast-forward`,
  `merge`, or `squash-rebase`), `integrated_main_sha`, `source_remote_head`,
  `actions_run_id`, positive `actions_run_attempt`, `actions_run_url`,
  `actions_run_attempt_url`, sorted `successful_jobs`, `wiki_head`,
  `wiki_source_sha`, `wiki_tree_oid`, `registry_blob_oid`, `registry_sha256`,
  `contract_test_blob_oid`, `contract_test_sha256`, the same
  `artifact_blob_oids`, number-sorted `disposition_issues`, `primary_root`, `primary_head`,
  `rollout_worktree_absent`, `closeout_worktree_absent`, and `observed_at`.
- `closeout/withdrawal.json`: `schema_version`, `rollout_id`, `meta_sha256`,
  `substate_before_withdrawal` (`NO_CLOSEOUT`, `ISOLATION_PENDING`, `ISOLATED`,
  `COMMITTED`, or `PROVED`), `stage` (`begin-closeout-isolation`,
  `preflight-closeout-integration`, `seal-closeout-proof`, `retire-receipt`,
  or `explicit-operator-report`), `withdrawal_kind`
  (`quiet-window-violation`, `main-advance`,
  `integration-shape-divergence`, `issue-projection-divergence`, `authority-withdrawn`, or
  `local-integrity-failure`, or `interrupted-closeout-observation`), nullable
  `observation_intent_path`/`observation_intent_sha256`, nullable `expected_main_sha`/
  `observed_main_sha`, nullable `expected_projection_sha256`/
  `observed_projection_sha256`, nullable `observed_disposition_issues`,
  nonempty `withdrawal_reference`, nonempty `reason`, and `detected_at`.

Every observation intent has exactly one counterpart: begin-isolation closes
with `closeout/isolation.intent.json`; pre-integration closes with its
same-sequence verified record; proof closes with `closeout/proof.json`;
retirement closes with its same-sequence verified record; and any classified
failure closes with `closeout/withdrawal.json`. Each existing-state success
record carries the exact intent path/hash named above. No intent may have two
counterparts, no verified record may be orphaned, and completed pairs do not
change the core closeout substate. A successful retirement installs its
verified counterpart, revalidates the complete receipt, and then seals its
tombstone without deleting anything. If it crashes after verification but
before retirement, the completed pair
remains under core substate `PROVED`; a later retirement attempt creates a
fresh sequence and repeats every remote read rather than reusing that result.

The artifact map contains exactly the design, repository-plan, and this-plan
paths/OIDs as a path-sorted list whose items contain exactly repository-relative
`path` and 40-hex `blob_oid`; every repeated map is canonical-byte identical.
`base_sha` must equal `meta.main_sha`; this closed protocol does not perform a
second semantic disposition review on a later main descendant. If remote main
has advanced, retain the terminal receipt and obtain a separate reviewed
closeout/recovery plan rather than inventing an in-place successor intent.
Every Actions check in meta, isolation, proof, and retirement is keyed by the
immutable tuple `(run_id, positive run_attempt, run_url, run_attempt_url)`; no command may omit
`--attempt` when reading that run's jobs, and a later attempt under the same
run ID is distinct authority rather than an update to a sealed observation.
`successful_jobs` is exactly `checks`, `publish-wiki`, and
`windows-bootstrap` in sorted order. The closeout commit has sole parent
`base_sha` and changes only `docs/superpowers/README.md` and
`test/documentation_contract_test.py`. Each isolation intent is installed
before its facility runs; an interrupted transition is resolved only by exact
facility enumeration. `seal-closeout-commit` derives Git identities and the
two-path diff from objects rather than trusting shell SHA variables.
`seal-closeout-proof` performs the canonical remote-main, Actions, Wiki,
registry/test, frozen-artifact, issue-disposition, primary-checkout, and both-worktree-absence
checks itself under global then receipt lock. A Boolean absence field must be
true and is accepted only when the recorded facility and Git worktree list both
prove absence. `disposition_issues` contains exactly 23 complete normalized
issue/comment objects: the three verified creates, issues 14/15/38, and the 17
verified closures. Its canonical bytes must equal that exact projection from
`after/issues.json` in both the isolation intent and the later proof; an issue
missing, reopened/closed, edited, relabelled,
locked, typed, pinned, or comment-changed since terminal readback refuses proof
rather than making a stale registry claim.

A `main-advance` withdrawal has two distinct full main SHAs and null projection
fields. An `integration-shape-divergence` withdrawal is valid only for
`seal-closeout-proof`: it has a nonnull observation-intent reference, null
expected main, one full observed main SHA, null projection fields, and a
withdrawal reference that seals the rejected ancestry/tree evidence. An
`issue-projection-divergence` withdrawal has null main fields, two
distinct projection hashes, and the complete number-sorted normalized
23-subject projection whose canonical hash equals
`observed_projection_sha256`. An `interrupted-closeout-observation` withdrawal
has null comparison fields and observed projection and references the unclosed
intent; the helper performs no new remote read. Every other non-divergence
withdrawal has null comparison fields and a null observed projection; its
immutable conversation, diagnostic, or evidence-hash reference explains the
known fact. An automatically observed or interrupted withdrawal has nonnull
intent path/hash and exactly the intent's stage/prior substate. An explicit
operator report has null intent fields and cannot be used while an observation
intent is pending. The helper derives the
meta hash, prior substate, stage, comparison values, and timestamp; a caller
cannot submit observed remote values. A withdrawal may coexist only with the
records valid for its recorded prior substate, all earlier completed
observation pairs, and—when nonnull—its one referenced intent without another
counterpart; no record from a later substate may exist. Receipt-level
`blocked.json` remains absent.

Within receipt-level `TERMINAL`, `status` derives the closeout substate solely
from these files with exact precedence `WITHDRAWN > OBSERVATION_PENDING >
PROVED > COMMITTED > ISOLATED > ISOLATION_PENDING > NO_CLOSEOUT`. An unclosed
highest observation intent yields `OBSERVATION_PENDING`, and its recorded prior
substate must equal the independently derived core substate. It rejects an
out-of-order/mismatched record; no mutable phase field or shell-only resume
value exists. `TERMINAL/OBSERVATION_PENDING` permits only `self-test`, `status`,
and `reconcile`; reconciliation finalizes an authenticated success/withdrawal
temporary or otherwise seals `interrupted-closeout-observation` without a
remote read. `TERMINAL/WITHDRAWN` permits only `self-test`, `status`,
authenticated-temporary reconciliation, and read-only validation for a
separately approved recovery plan. It permanently forbids ordinary closeout
creation, pre-integration, proof, and receipt retirement even if remote bytes
later return to their expected values.

Operation state is derived only from immutable flat-file presence: an absent
operation-intent file is `UNSTARTED`; valid operation intent without verified
is `PENDING/INDETERMINATE` and may never be replayed; verified `applied` is
`COMPLETE`; `not-applied` or
`ambiguous` stops for user direction. Only one operation may be nonterminal,
and the next planned operation cannot start until every predecessor is
verified `applied`. This receipt accepts only the 40 frozen operation IDs. Any
compensation requires a separately approved recovery plan and separate
receipt/operation namespace; no current receipt record is edited or extended.

For a body-bearing next operation, the one allowed pre-operation-intent crash
shape is its already installed exact `body-<operation-id>.md` with no operation
intent or later artifact. Because no child can spawn before operation-intent installation, this remains
`UNSTARTED`: `execute-operation` re-renders from the immutable contract,
requires byte/hash/mode/inode equality, and reuses that body before installing
the sole operation intent. A wrong body, a body for any non-next operation, or
any post-operation-intent artifact without that intent is a hard refusal. Tests
interrupt after body installation and separately after operation-intent
installation; only the former
is safely resumable, while the latter is reconciled as potentially issued and
never replayed.

The helper accepts only these complete command forms; `GUARD` is the absolute
`rollout_guard.py` path and every input path is an already closed mode-`0600`
regular file outside the receipt unless the command names a receipt path:

```text
ABSOLUTE_PYTHON -I -B GUARD self-test --receipt RECEIPT_DIR
ABSOLUTE_PYTHON -I -B GUARD begin-worktree-isolation --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-worktree-isolation --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-meta --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-approval --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-abandonment --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-eligibility-withdrawal --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-snapshot --receipt RECEIPT_DIR --destination RELATIVE_PATH --schema api-envelope|helper-review|request-contract|request-contract-review|disposition-review|wiki-verification|command-result|issue-set|anchor-index|opaque-bytes --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-after --receipt RECEIPT_DIR
ABSOLUTE_PYTHON -I -B GUARD execute-operation --receipt RECEIPT_DIR --operation-id OPERATION_ID
ABSOLUTE_PYTHON -I -B GUARD status --receipt RECEIPT_DIR
ABSOLUTE_PYTHON -I -B GUARD reconcile --receipt RECEIPT_DIR [--operation-id OPERATION_ID]
ABSOLUTE_PYTHON -I -B GUARD preflight --receipt RECEIPT_DIR --slot SLOT
ABSOLUTE_PYTHON -I -B GUARD begin-closeout-isolation --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-closeout-isolation --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD seal-closeout-commit --receipt RECEIPT_DIR
ABSOLUTE_PYTHON -I -B GUARD seal-closeout-withdrawal --receipt RECEIPT_DIR --input FILE
ABSOLUTE_PYTHON -I -B GUARD preflight-closeout-integration --receipt RECEIPT_DIR
ABSOLUTE_PYTHON -I -B GUARD seal-closeout-proof --receipt RECEIPT_DIR --repository-root PATH
ABSOLUTE_PYTHON -I -B GUARD retire-receipt --receipt RECEIPT_DIR --repository-root PATH
```

Create candidate inputs and command captures only under a fresh mode-`0700`
scratch directory with `umask 077`, exclusive mode-`0600` files, flush, and
`fsync`. The helper uses no-follow open plus pre/post `fstat` identity/size
checks, reads each input once, and copies the validated bytes through its own
atomic installer. Remove the exact scratch directory only after the sealed
receipt hashes match; a crash with only an intent remains recoverable from
the decision/operation rules below, never shell memory.

`seal-snapshot` permits only the exact roster/schema combinations above.
`begin-worktree-isolation` is available only after the zero-blocker helper
review and atomically installs the closed intent before returning permission to
invoke the named facility. `seal-worktree-isolation` accepts only the exact
success derived from that intent and a fresh bounded facility enumeration; it
refuses a second/mismatched mapping. If the caller dies between them,
`reconcile` never guesses: it validates one matching mapping and seals success,
proves absence before permitting the same requested facility to retry, or
refuses malformed/multiple state. Neither command has a network path.
`seal-eligibility-withdrawal` accepts a closed input containing only
`rollout_id`, `withdrawal_kind`, `withdrawal_reference`, and `reason`; under
both locks it first installs the sole `eligibility-withdrawal` decision intent,
then derives the timestamp and exact completed prefix and atomically installs
the only block. If interrupted before the block is final, only `reconcile` may
close it conservatively as `interrupted-decision`. It is permitted from
`ANCHORED` or `APPROVED` only when there is no pending operation intent. Known issue-operator activity in the approved
quiet window—including activity that restores byte-identical final state—and
Task 4's stale owner/evidence failure must use this transition before stopping.
After a decision intent exists, disposition-row drift uses
`stale-disposition-evidence`; viewer/permission/label loss uses
`authority-withdrawn`; and any worktree/path/mode/hash/blob/shadow/import/Git
replacement/graft/alternate/promisor or closed-environment mismatch uses
`local-integrity-failure`, with an immutable diagnostic/hash reference. Each
known failure atomically closes the decision with its withdrawal block; an
interrupted block installation is reconciled only as `interrupted-decision`.
The same local failure discovered before a decision exists is a plain refusal
with no invented remote observation.
`seal-closeout-withdrawal` accepts a closed input containing exactly
`rollout_id`, `withdrawal_kind`, `withdrawal_reference`, and `reason`. It is
available from every non-withdrawn, non-pending receipt-level `TERMINAL`
closeout core substate
for known operator activity or independently known authority/local-integrity
failure. Under global then receipt lock it validates the existing closeout
record prefix, derives every other withdrawal field, and atomically installs
the sole `closeout/withdrawal.json`; it refuses while any closeout observation
intent is pending because reconciliation owns that stop. Its explicit input kind is limited to
`quiet-window-violation`, `authority-withdrawn`, or
`local-integrity-failure`, and the derived stage is
`explicit-operator-report`; remote main/projection kinds can originate only
inside the command that made that authenticated observation. Before
`after/issues.json` is sealed,
known operator activity uses `seal-eligibility-withdrawal`; afterward it uses
this terminal transition. Neither record can be cleared by later restoration
of identical normalized bytes.
`seal-after` accepts no candidate input and is available only after all 40
operations are verified applied. Under both locks it installs the sole
`after-backlog` decision intent before any GitHub read, fetches the complete
source/Wiki/viewer anchor exactly as `preflight` does, then fetches the complete
paginated issue set plus complete paginated comments for every issue itself,
normalizes it, and derives the exact expected set. An anchor or authority
mismatch closes the decision with its corresponding permanent block. Equality installs
`after/issues.json`; inequality atomically installs the sole
`backlog-divergence` block with that normalized candidate embedded and exits
nonzero. An interrupted remote read becomes `interrupted-decision`; it is never
retried under a new decision. The caller cannot supply, discard, or promote a
candidate or choose the outcome.
The closeout commands are available only in receipt-level `TERMINAL` and
enforce the exact substate chain. Each of `begin-closeout-isolation`,
`preflight-closeout-integration`, `seal-closeout-proof`, and terminal-shape
`retire-receipt` validates only local prerequisites, acquires global then
receipt lock, and installs its next observation intent immediately before its
first remote read. After installation that invocation must atomically install
the one success counterpart, atomically install a derived withdrawal, or leave
an unclosed intent through process death; it has no plain retry path.

`begin-closeout-isolation` then validates the rollout worktree is absent through
its recorded facility, rechecks the frozen artifact map and eligible closeout
base/run/Wiki result, fetches the full 23-issue/comment projection and requires
equality with terminal after-state, then installs the closeout isolation intent
as the observation's success counterpart before returning permission to create. Its matching
seal command enumerates and records exactly one mapping. `seal-closeout-commit`
accepts no input and derives the sole-parent/two-path commit proof from the
recorded checkout. `preflight-closeout-integration` is an externally read-only
receipt writer available only in `COMMITTED`: it requires remote main still
equals the base and the fresh 23-issue/comment projection equals the isolation
intent, then installs the same-sequence verified counterpart. It is rerun after
any pause before integration and never grants broader authority.
`seal-closeout-proof` accepts only the clean durable primary
root and itself performs every external/local readback named by the proof
schema, including a fresh full-comment read of all 23 disposition subjects; it
records `closeout/proof.json` as the observation's success counterpart only
when that projection still equals terminal after-state and both worktrees are
absent.
It owns the first post-integration Actions/Wiki observation: after installing
the proof observation intent, it uses an injectable monotonic clock and the
reviewed `CLOSEOUT_READY_TIMEOUT_SECONDS=86400` deadline to poll the exact
integrated SHA's one selected Actions run attempt and Wiki marker/tree. The
first matching Actions response pins one positive `(run_id, run_attempt,
run_url, run_attempt_url)` tuple for this observation; the attempt URL is
derived by the same exact rule as meta, all job polls use that ID plus explicit
`--attempt`, and proof records the tuple. Queued/running or not-yet-published
authenticated state is retried only inside that same locked invocation,
honoring a valid server retry interval clamped to 1–60 seconds and otherwise
polling after five seconds. A transport failure after the intent is
installed immediately seals `interrupted-closeout-observation`; it is never a
polling state and the helper performs no second remote read. Exact readiness
success is followed by a fresh canonical-main fetch under the same intent and
both locks: remote main must still equal the exact accepted integrated result
and integration shape before the helper reads the disposition projection or
installs proof. A main advance during the poll seals
`integration-shape-divergence` with the newly rejected main/shape evidence. A
terminal job failure/cancellation, wrong published state, authority loss, or
deadline exhaustion seals `authority-withdrawn` with the run/response evidence.
No caller performs a readiness query first, and a dead helper leaves the intent
pending rather than authorizing a fresh poll.

On an authenticated main, issue-projection, authority, or local-integrity
mismatch, `begin-closeout-isolation`, `preflight-closeout-integration`,
`seal-closeout-proof`, and `retire-receipt` atomically seal the derived
closeout withdrawal referencing that observation intent before returning
nonzero. A transport failure after intent installation also seals
`interrupted-closeout-observation`; process death leaves
`OBSERVATION_PENDING`. Reconcile performs no remote read: it finalizes only an
authenticated success-counterpart or withdrawal temporary, or otherwise seals
the interrupted-observation withdrawal. A subsequently restored remote state
is never reread by that sequence. Once the final withdrawal exists, no closeout
command can resume. A local refusal before observation-intent installation
writes no invented remote fact and remains retryable.
Within one call the helper checks local integrity, remote authority, remote
main (or the proof-stage integration shape), proof readiness when applicable,
the fresh post-readiness proof-stage integration shape, then the disposition
projection in that fixed order and stops at the first classified mismatch, so
the one withdrawal kind and field matrix are deterministic.
On a passing call, `preflight` chooses the next four-digit sequence, seals its snapshot, and
prints exactly one canonical JSON line containing only its receipt-relative
`path` and `sha256`; the internal begin transition consumes those exact values
in the intent. The slot alone selects its frozen request-contract subject:
callers cannot supply an issue number or creation title. The two non-operation
probes map exactly: `anchor-existing-14` reads the `edit-14` subject and
`anchor-create-first-paint` reads the `create-first-paint` subject; every other
probe alias is refused and fixture-tested. `execute-operation`
derives the title, labels, issue, body path/hash, and literal `gh` argv from the
contract, installs that exact intent, and exposes no caller-supplied mutation
argument. `status` and `reconcile` print one canonical derived-state JSON line.
`self-test` prints only `OK`; the other successful writers print nothing.
Every failure exits nonzero, prints one diagnostic line to stderr, and emits no
partial stdout. Unknown commands/options, extra positional arguments, invalid
schema/path combinations, and non-regular inputs exit `2` before a receipt
write or network read.

`retire-receipt` is the sole logical retirement boundary and is fixture-tested
before approval. It never deletes a receipt path and is never an external
mutation command. It strictly resolves the
supplied repository root and requires it equal that worktree's
`git rev-parse --path-format=absolute --show-toplevel`; resolves the absolute
common Git directory through Git; and requires the receipt to be one direct,
non-symlink directory child with the exact rollout basename. Under the same
nonblocking BSD flock, it revalidates helper/test hashes, the entire closed
roster/schema/modes/inodes and requires one of two terminal shapes: `TERMINAL`
with all 40 `applied` records, sealed after-state, no block, no
`closeout/withdrawal.json`, and exact
`closeout/proof.json`; or
`ABANDONED` with a valid abandonment record and no possible-mutation artifact.
For bootstrap abandonment without `meta.json`, it takes the helper hashes from
`helper-review.json` and validates the exact bootstrap roster and sealed
worktree isolation. For anchored/approved/blocked abandonment it validates meta
normally; an admitted safe first body remains in the manifest only after
independently re-rendering and matching it. Every abandonment shape—not only bootstrap—must
prove the recorded rollout worktree absent through both its recorded facility
and exact `git worktree list --porcelain` before the helper may seal retirement.
For the terminal shape, immediately before retirement the helper repeats under
the same locks every canonical remote-main/Actions/Wiki result, terminal
registry/test blob, frozen artifact OID, complete 23-issue/comment projection,
clean primary, and rollout/closeout
worktree-absence check sealed in the proof. It first installs the retirement
observation intent, performs those reads, installs the same-sequence verified
counterpart, revalidates the complete receipt including that pair, builds the
exact pre-retirement manifest, and atomically seals `retirement.json`. A crash
after verified installation but before the tombstone is final leaves `PROVED`;
reconcile may finalize an authenticated tombstone temporary, otherwise a later
attempt must create a fresh sequence and repeat every read. A changed result preserves the
receipt only after atomically sealing the derived closeout withdrawal. Both
shapes require no unresolved temporary, pending, ambiguous, or unlisted
path and no hard links outside the receipt. The abandoned shape builds the same
manifest and seals its tombstone without a remote observation pair. After the
atomic install, the helper reopens and validates the tombstone and manifest,
then releases both locks with every receipt inode untouched. Any mismatch
leaves the receipt unretired and intact.

Receipt phases are derived, never stored mutably:

When more than one terminal marker is structurally present, validation applies
this exact precedence and rejects any disallowed combination:
`RETIRED > ABANDONED > TERMINAL > BLOCKED > DECISION_PENDING > APPROVED > ANCHORED >
BOOTSTRAP`. An operation's `PENDING/INDETERMINATE` state is a substate beneath
`APPROVED`; it does not outrank a receipt-level block.

- `BOOTSTRAP` has no `meta.json`. It permits `self-test`, `status`,
  temporary reconciliation plus exact pending-worktree-isolation-intent
  reconciliation, and `seal-snapshot` for the exact anchor/before
  roster, plus `seal-abandonment` only in the reviewed zero-possible-mutation
  shape. Before `helper-review.json` exists, that review record is the helper's
  only permitted snapshot destination. Immediately after the two reviews, seal
  it. The next transition installs `worktree-isolation.intent.json`; only then
  may its recorded facility run, and the matching success record must be sealed
  before any remote observation. An interrupted intent is recovered only from
  exact facility enumeration. Every subsequent bootstrap writer requires both
  reviewed source hashes and both isolation records to match.
  `request-contract.json` may be sealed only after all facts
  it cites exist; `request-contract-review.json` must immediately follow it and
  match its hash plus the integrated plan bytes. `disposition-review.json`
  must then cover all 40 operations at exact `MAIN_SHA` with two passing
  reviews. All three are mandatory before
  `anchor/index.json`, which is the last permitted snapshot. After the index
  exists, exactly one valid `seal-meta` is the only forward transition.
- `ANCHORED` begins when `seal-meta` validates the closed evidence index,
  both reviewed source hashes, the worktree-isolation hash and every repeated
  path/facility/handle value, request-contract/review and disposition-review
  hashes, and exact meta schema. It permanently forbids
  `seal-snapshot`/`seal-meta`; only `self-test`,
  `status`, `reconcile`, one `seal-approval`,
  `seal-eligibility-withdrawal`, or explicit zero-mutation
  `seal-abandonment` remain.
- `APPROVED` begins when approval is sealed. It permanently forbids every
  bootstrap writer and permits only `self-test`, `status`, `reconcile`,
  read-only `preflight`, exactly one serialized `execute-operation` for the
  next ID, `seal-eligibility-withdrawal` when there is no pending operation intent,
  and—after all 40 operations are verified applied—one `seal-after`.
  The executor reads no body/title/label/issue/argv substitution input, resolves
  each declared source from immutable meta/contract/preceding verified records,
  validates UTF-8/LF and token set equality, installs the exact body and intent,
  then retains the lock through exact command/readback/verification. Close
  operations render no body. Before any operation intent exists, explicit user direction
  may instead seal zero-mutation abandonment if every decision intent is
  terminally paired; the only body admitted is the
  independently re-rendered exact first-operation body described above.
  An operation intent, unresolved decision, or completed prefix permanently
  forbids abandonment.
- `DECISION_PENDING` begins when exactly one valid decision intent has neither
  its matching preflight/after success artifact nor a referencing block. It
  permits only local `self-test`/`status` and `reconcile`. Reconcile first
  finalizes an authenticated matching success/block temporary if present;
  otherwise it performs no new remote read and atomically installs an
  `interrupted-decision` block. A later matching remote state cannot restore
  ordinary execution.
- `BLOCKED` begins when `blocked.json` exists and permits only read-only
  `self-test`/`status`/`reconcile`, plus `seal-abandonment` when no operation
  could have started. No block permits `seal-after`; even an advance after all
  40 operations requires a separately approved recovery/audit plan.
- `RETIRED` begins when `retirement.json` and its complete bound manifest are
  valid over an otherwise exact `ABANDONED` or `TERMINAL/PROVED` receipt. It
  permits only locked `self-test`/`status` and immutable audit reads; every
  writer, reconciliation transition, remote observation, and physical cleanup
  command is forbidden.
- `ABANDONED` begins when `abandonment.json` is sealed from reviewed
  `BOOTSTRAP`, `ANCHORED`, or the zero-mutation `APPROVED`/`BLOCKED` shape and permits only validation,
  reconciliation, and exact logical receipt retirement.
- `TERMINAL` begins when `after/issues.json` is sealed and
  permits read-only validation/reconciliation, the ordered closeout-intent,
  isolation, commit, and proof writers, and the exact `retire-receipt`
  transition only after `PROVED` and its additional preconditions hold. It
  permits no further Issue mutation. `TERMINAL/OBSERVATION_PENDING` admits only
  the conservative local reconciliation defined above.
  `TERMINAL/WITHDRAWN` is the immutable
  closeout-only stop defined above: it permits none of those forward writers
  or retirement and leaves receipt-level phase precedence unchanged.

The fixture tests kill wrong host/repository/origin, changed source or Wiki
head, wrong marker/tree, corrupt JSON/hash, path escape/symlink/body overwrite,
wrong/early body operation ID, altered contract title/template/token/URL,
unapproved but format-valid SHA/run/attempt/attempt-URL/issue/URL substitutions,
default-latest-attempt substitution, unresolved body
tokens/CR, wrong labels/argv/body hash, skipped/duplicate operation IDs,
mismatched issue
fields/comments, a replayed indeterminate operation, an empty-intent crash
window, the safe exact-body-before-operation-intent restart, every atomic-install
interruption, and same/different-inode temporary
reconciliation. They also kill early/nonterminal receipt retirement, a busy
lock, outside-common-Git or symlinked receipt roots, unexpected paths/modes/
hard links, issue/backlog/partial-anchor/eligibility-withdrawal blocks,
pending/ambiguous/temp state,
and any attempted mutation/deletion of a decoy or receipt inode. A positive terminal-retirement fixture admits
only the exact all-40 after-state with no block plus a fully revalidated
closeout proof; it kills missing/reordered closeout records, wrong integration
shape (including a third merge parent, reversed parents, or an unrelated tree
path), changed registry/test/artifact blob, failed/missing job, wrong Wiki
source/tree, any field/comment drift in the 23-issue proof projection,
the same drift at pre-integration, a closeout base that is merely a descendant
instead of exact `meta.main_sha`, dirty/wrong primary, either remaining worktree, and a remote main
advance after proof.
Retirement fixtures kill every manifest path/kind/mode/size/hash/device/inode/
link-count change, missing/extra child, wrong root, wrong underlying phase,
wrong terminal or observation reference, and a tombstone that includes itself.
They interrupt before/during tombstone installation and admit only
authenticated temporary finalization or a fresh validation; every successful
case proves all pre-retirement inodes remain present and byte-exact, the receipt
is inactive, and a second approval is admitted without deleting the audit
record.
Closeout-withdrawal fixtures drift and then restore remote main at each
applicable command; mutate and restore every normalized field/comment in the
23-subject projection; and report known byte-identical-restored operator
activity through `seal-closeout-withdrawal`. They cover all five prior
substates, require `TERMINAL/WITHDRAWN`, and reject every malformed field
matrix, wrong prior substate, later closeout record, duplicate withdrawal,
receipt-level block coexistence, proof after withdrawal, and retirement after
withdrawal. Observation-fence fixtures kill the helper immediately after
intent installation, after each remote response, and before/during every
success or withdrawal counterpart installation; pending reconciliation performs
zero remote rereads and admits only authenticated temporary finalization or an
`interrupted-closeout-observation` withdrawal. They bind begin/proof to their
exact intent, admit multiple gap-free successful pre-integration pairs, and
force a fresh retirement sequence after a crash following retirement
verification. They kill sequence gaps/duplicates, wrong stage/prior substate/
authority hash, double counterparts, orphan verified records, and explicit
withdrawals with nonnull intent fields. Proof fixtures reject every invalid
parent/tree integration shape, seal its exact ancestry evidence as
`integration-shape-divergence`, restore remote state, and still refuse to
resume. They advance canonical main while readiness polling is in progress and
require the final fresh main/shape read to seal the same permanent withdrawal;
restoring the earlier main cannot resume that receipt. They also inject a
transport failure after proof intent installation and require immediate
`interrupted-closeout-observation` with zero subsequent remote reads. Actions
fixtures reuse one run ID for a successful attempt followed by failed and
successful reruns: proof and retirement query the sealed positive attempt
explicitly, never accept default latest-attempt jobs, and fail mutations that
omit or change `run_attempt`. Restored
bytes never escape pending
reconciliation or withdrawal; the withdrawn terminal receipt remains the sole
active sibling and continues excluding another approval.
Abandonment tests admit reviewed partial-bootstrap cleanup, terminally paired
read-only probe decisions, and an exact first-operation body with no operation
intent, but kill an unreviewed helper, missing or
invalid worktree-isolation record, wrong/non-first/additional body, body plus
operation intent/temp, an unresolved decision, and any nonempty completed
prefix. Decision fixtures kill the helper before its first remote read, after
the read, and before/during the success or block install; after the decision
intent exists, reconciliation can only finalize the authenticated counterpart
or seal `interrupted-decision`. Restoring remote bytes never re-enables the
receipt.
Eligibility fixtures require quiet-window and stale-evidence withdrawal to seal
their exact immutable block before stop, and mutate every post-decision local
integrity check to require its exact `local-integrity-failure` block without a
network-sentinel touch. Two-receipt fixtures require B's approval and every
remote command to refuse while A remains active even after A has an applied,
ambiguous, blocked, or terminal local outcome; only A's valid abandonment or
terminal retirement releases global approval eligibility. Whole-backlog fixtures mutate every normalized top-level
field, comment field, issue set membership, and timestamp; each seals one
backlog-divergence block with the exact candidate embedded, permanently refuses `seal-after`, and
retains every receipt inode. `status` reconstructs state from the files; `reconcile`
performs authoritative readback for the sole pending operation but never
repeats its command.

The executor constructs one immutable argv tuple, writes that same tuple into
intent, and passes it directly to `subprocess.Popen(..., shell=False)`; there is
no separately typed shell command. It supplies a freshly built GitHub CLI
environment containing only the frozen private `GH_CONFIG_DIR`, literal
`GH_HOST=github.com`, `GH_PROMPT_DISABLED=1`, and noninteractive pager/color
settings; it never inherits `GH_REPO`, alternate-host/token variables, Git
configuration, trace variables, or repository discovery state. It captures
stdout/stderr without logging and passes only the already locked global and
receipt file descriptors to the child. Once a child is successfully spawned, a lost response,
nonzero exit, mismatched delta, or post-crash absence is `ambiguous` unless
authoritative polling proves the exact applied delta. `not-applied` is legal
only when child creation itself failed synchronously before any process existed.
After a crashed executor, `reconcile` first acquires the same lock (therefore
the child has exited), performs the bounded consistency poll and readback, and
seals `applied` or `ambiguous`; it never infers `not-applied` from an empty
readback and never replays. A later-visible ambiguous create/mutation remains a
hard stop for an explicit recovery plan.

`preflight` is the durable replacement for a shell-local
`revalidate_anchor`. On every call it first validates local receipt/approval
state, derives the exact expected anchor/subject and reviewed disposition row,
and constructs the closed Git/Python/GitHub environments without performing a
remote read. It acquires the global then receipt locks and atomically installs
the next decision intent. Only then may it observe any authority.

Before the first remote observation, it reopens every path in that intent's
sealed disposition row without following links and requires mode, Git blob,
byte hash, and every token to equal both `disposition-review.json` and exact
`MAIN_SHA`; the row's canonical hash, operation membership, and eligibility
matrix must equal the decision intent. Any mismatch closes that decision with
`eligibility-withdrawn/stale-disposition-evidence`; it never substitutes new
evidence or rewrites an issue request. It then proves both source `origin` URLs
canonical; strictly resolves `meta.rollout_worktree`; requires its Git
toplevel/common-dir/HEAD/detached/non-shallow/clean state still equals the
sealed values; and no-follow opens/hashes the four renderer/contract/Markdown
files against meta and their `MAIN_SHA` blobs. It enforces mode `100644` for
the three libraries and `100755` for `tools/render-wiki.py`, and refuses a
symlink or any ignored/untracked Python source, `__pycache__`, or `.pyc` below
`tools/` that could shadow an import. Renderer subprocesses use the guard's
exact frozen `sys.executable` with `-B`, sealed absolute CLI, explicit rollout
worktree cwd, and an environment without `PYTHONPATH`, `PYTHONHOME`, user-site
injection, Git/GitHub secret, or repository override. System Python and Git
are explicit trusted host dependencies.

Before a Git authority/object proof it rejects replacement refs, grafts, both
alternate files, partial/promisor configuration, shallow state, or
`rev-list --missing=print` output in the primary, rollout, and temporary Wiki
repositories; every read disables replacement and lazy fetch. It rechecks the
authenticated viewer through the literal host/repository and requires the
same login plus `WRITE|MAINTAIN|ADMIN`; a creation slot also reads the explicit
`enhancement` label endpoint and requires the frozen name, positive ID, and API
URL. Changed authority closes the already durable decision with
`eligibility-withdrawn/authority-withdrawn`. It fetches canonical main and Wiki
master, requires the frozen heads/source marker, independently renders the
source, and compares the complete Wiki tree. An advance closes the same
decision with `anchor-advance`. For an existing-issue subject it fetches the
issue and every comment through explicitly hosted APIs and compares the exact
state derived by replaying verified operations. For a creation subject it
fetches the complete paginated issue set and every issue's complete paginated
comments, then checks the frozen title. A mismatch closes the decision with
`issue-divergence`, embedding the complete normalized observation and exact
expected/observed hashes. A match atomically seals the same-slot/sequence
preflight snapshot; only that paired snapshot authorizes the operation-intent
and child spawn.

A remote/read failure or crash after decision-intent installation but before a
paired preflight/block is never an ordinary failed preflight. If an
authenticated matching success/block temporary exists, `reconcile` may finish
only that install. Otherwise it performs no new remote read and installs
`interrupted-decision`. The receipt therefore remains permanently blocked even
if remote bytes later match again. With no possible operation mutation,
explicit user direction may abandon the terminally blocked receipt; with any
body/operation intent or completed prefix, retain it for a separate approved
recovery/revision plan.

The same fence applies after all operations. `seal-after` installs its decision
before internally fetching the final complete issue/comment set. A mismatch
closes it as `backlog-divergence` with the full observation embedded; a read
failure/interruption becomes `interrupted-decision`; a match alone installs
`after/issues.json`. A later matching read cannot clear either block. With any
pending or completed mutation, no block allows further issue mutation,
`seal-after`, closeout, or an in-place anchor change.

### Contract-bound body rendering and execution

All Markdown sent to GitHub is rendered only by the reviewed guard from the
sealed request contract. The shell supplies no body bytes, title, URL, SHA,
run, issue-number reference, label, issue, or command value. For the next operation,
invoke the only mutation boundary:

```sh
"$PYTHON_BIN" -I -B "$RECEIPT_DIR/rollout_guard.py" execute-operation \
  --receipt "$RECEIPT_DIR" --operation-id "$OPERATION_ID"
```

The guard requires the requested ID to be the next contract row;
loads `MAIN_SHA`, `RUN_ATTEMPT`, `RUN_URL`, and derived `RUN_ATTEMPT_URL` only
from `meta`; loads future issue numbers only from the named preceding
`verified.applied` creation records; loads every URL list only from the sealed
literal substitution; validates every SHA, positive ID/attempt, canonical
repository/Actions URL, declared token, LF, and UTF-8 invariant; then
atomically installs the mode-`0600` body when the row requires one. It derives
and executes the repository-qualified `gh` argv under the same lock; close has
no body. A repeated/non-next invocation is a hard refusal. Never run a live
`gh issue` mutation outside this helper, or use a here-document, environment
substitution, `eval`, command substitution containing body text, `--body`, or
another body file.

The generic closure template frozen once per comment row is exactly:

```markdown
Reverified against integrated main `@@MAIN_SHA@@`.

- Owning implementation: @@IMPLEMENTATION_URLS@@
- Current contract: @@CURRENT_PAGE_URLS@@
- Strongest executable evidence: @@EVIDENCE_URLS@@
- Successful integration run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@

The current code and executable evidence implement this issue's requested
outcome. This closure is based on that implementation, not on the documentation
rewrite.
```

The URL-list rules require every item to be a full-SHA canonical repository
`commit`/`blob` URL at the verified owner or evidence path. Inside the one
locked transition, the helper performs `preflight`, seals its normalized
snapshot, renders the body, installs intent, executes/captures the exact argv,
fetches authoritative readback, and applies one pure expected-after comparator;
normal execution and `reconcile` call the same function over complete normalized
objects:

- create requires the complete preflight issue set plus exactly one new issue
  with the frozen title/body, canonical dynamic number/URL, viewer as author,
  `state=open`, null reason/milestone, sole `enhancement` label, empty
  assignees/comments, unlocked/null lock reason, null issue type/pinned comment/
  closed metadata, and valid receipt-recorded creation/update timestamps;
- edit permits only title, body, and its validated `updated_at` to differ from
  the complete before object; every label, assignee, milestone, identity,
  state, lock/type/pin/closure field, author, creation time, and complete
  pre-existing comment is equal;
- comment permits only the issue `updated_at` plus exactly one canonical new
  comment with the sealed body, viewer actor, returned ID/URL, and validated
  creation/update timestamps; every prior comment and other issue field is
  equal;
- close has no body and permits only `state=closed`,
  `state_reason=completed`, its validated `updated_at`/`closed_at`, and
  viewer-matching `closed_by` to differ; the complete body, labels, assignees,
  milestone, identities, lock/type/pin fields, author, creation time, and
  comments remain equal.

Every omitted top-level/comment field is therefore compared canonically. After
child spawn, a subset match, extra delta, or otherwise non-exact readback is
`ambiguous`, never `applied` or `not-applied`. Fixtures mutate every normalized
field and comment field for all four kinds. The helper seals observation and
verified only after the exact comparator result. A lost response leaves durable
intent and is reconciled read-only, never blindly replayed. No shell-local
function or memory is authoritative.

Recovery is deliberately narrow and never implicit. After new explicit user
approval, a separate recovery plan and receipt may restore an edited existing
issue's previous title/body/labels only when current normalized state still
equals this rollout's verified result; reopen an erroneous closure under the
same proof; or edit/delete only the exact recorded comment ID after proving
actor/body. Never use `--edit-last` or `--delete-last`. GitHub issues cannot be
deleted, so any erroneous or ambiguous new issue stops for user direction
rather than being auto-closed or repurposed. Unrelated concurrent backlog
changes are reported as external divergence and never restored by this
rollout.

Any separately approved exact-comment compensation must independently repeat
the executable/config/viewer/permission pin and addresses only the original
receipt-recorded ID (`RECOVERY_GH_BIN` is that newly sealed absolute path):

```sh
"$RECOVERY_GH_BIN" api --hostname "$GH_API_HOST" --method PATCH \
  "repos/cleveralbatraoz/unseeing/issues/comments/$COMMENT_ID" \
  --input "$COMMENT_PATCH_FILE"
"$RECOVERY_GH_BIN" api --hostname "$GH_API_HOST" --method DELETE \
  "repos/cleveralbatraoz/unseeing/issues/comments/$COMMENT_ID"
"$RECOVERY_GH_BIN" issue reopen "$ISSUE" --repo "$GH_TARGET"
```

---

### Task 1: Establish One Immutable Rollout Anchor

**Files:** No tracked files. This task is read-only across Git, Actions, Wiki,
and Issues; its local mode-`0700` receipt is operational recovery state.

**Interfaces:**

- Consumes: remote `main`, the resulting main commit named by the selected
  finish-branch handoff, one exact Actions run attempt, Wiki `master`,
  authenticated viewer permission, and the pre-mutation issue state map.
- Produces: a clean detached rollout worktree; frozen `PYTHON_BIN`; shell values
  `MAIN_SHA`, `RUN_ID`, positive `RUN_ATTEMPT`, `RUN_URL`, derived
  `RUN_ATTEMPT_URL`, `WIKI_HEAD`, and `RECEIPT_DIR`; a complete recoverable
  before-state; and explicit user approval for Tasks 2–4. This task performs no
  external mutation.

- [ ] **Step 1: Validate the origin, fetch, and freeze exact integrated main**

Use `superpowers:using-git-worktrees` from the clean durable primary checkout.
Fetch `origin/main` before creating anything. Consume
`INTEGRATED_MAIN_SHA` from the finish-branch handoff: it is the resulting commit
on `main`, not the feature-branch tip. Require it to be a full SHA and an
ancestor of the freshly resolved `MAIN_SHA`; this permits an explicitly chosen
squash or rebase integration while still proving the handoff reached main.

```sh
: "${INTEGRATED_MAIN_SHA:?set the resulting full main SHA from the finish-branch handoff}"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TERMINAL_PROMPT=0
unset GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT GIT_ASKPASS SSH_ASKPASS
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
unset GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_CEILING_DIRECTORIES
unset GIT_TRACE GIT_TRACE2
unset GIT_TRACE_PACKET GIT_TRACE_PERFORMANCE GIT_TRACE_SETUP GIT_CURL_VERBOSE
test "$(git remote get-url origin)" = "https://github.com/cleveralbatraoz/unseeing"
test "$(git remote get-url --push origin)" = "https://github.com/cleveralbatraoz/unseeing"
git -c credential.helper= -c core.askPass= fetch --no-tags origin \
  refs/heads/main:refs/remotes/origin/main
MAIN_SHA="$(git rev-parse refs/remotes/origin/main)"
test "$MAIN_SHA" = "$PREBOOT_MAIN_SHA"
git rev-parse --verify "$INTEGRATED_MAIN_SHA^{commit}"
git merge-base --is-ancestor "$INTEGRATED_MAIN_SHA" "$MAIN_SHA"
for ARTIFACT in \
  docs/superpowers/specs/2026-08-15-ai-documentation-source-of-truth-design.md \
  docs/superpowers/plans/2026-08-15-ai-documentation-source-of-truth.md \
  docs/superpowers/plans/2026-08-15-ai-documentation-issue-migration.md
do
  test "$(git rev-parse "$INTEGRATED_MAIN_SHA:$ARTIFACT")" = \
    "$(git rev-parse "$MAIN_SHA:$ARTIFACT")"
done
```

Under the same hardened object environment, parse the exact
`MAIN_SHA:ci/superpowers.lock` `pin=`, require `git ls-tree "$MAIN_SHA"
tools/superpowers` is exactly one mode-`160000` entry at that OID, and require
the clean durable primary index/gitlink plus `ci/verify-superpowers.sh full`
still agree with the pre-execution bootstrap. Record the OID for meta. Never
initialize the rollout worktree copy.

Do not substitute the pre-integration feature tip for
`INTEGRATED_MAIN_SHA`. The three artifact blob identities must remain exact
between the integration handoff and later main; an intervening edit to a
frozen design/plan stops for a newly reviewed plan rather than silently
changing the request contract. Unrelated descendant commits remain eligible.

- [ ] **Step 2: Create the receipt before any rollout worktree**

Strictly resolve the clean durable primary root and common Git directory. Under
an explicitly reviewed host-tool boundary, resolve `command -v python3` once
to canonical absolute `PYTHON_BIN`; require its strict target is a no-follow-
opened regular executable outside the repository, common Git directory, and
future receipt, record its byte SHA-256, and never look it up through PATH
again. Every receipt-helper/test invocation below is exactly
`"$PYTHON_BIN" -I -B`. Before meta, the reviewed shell and the helper's startup
check reopen/hash the path against the identical literal constants embedded in
both helper/test sources; after meta, the guard additionally requires those
constants and live bytes equal both helper review and meta.

Under
that absolute common Git directory, create or validate the fixed
`unseeing-issue-rollout.lock` before the receipt:
use no-follow `O_CREAT|O_EXCL` with mode `0600` under `umask 077` when absent,
or no-follow open when present; require it is the common directory's direct
child, owned by the current uid, regular, mode `0600`, link count one, and the
same inode before/open/after. Acquire and release its nonblocking BSD flock as
a positive control. Never truncate it, put it inside the receipt, or schedule
it for cleanup.

Under the same absolute common Git directory, generate `ROLLOUT_ID` with
`uuid.uuid4()` in canonical lowercase form and create the exact new mode-`0700`
`unseeing-issue-rollout-$ROLLOUT_ID` directory with `umask 077`; refuse to
overwrite an existing path. Create only the exact empty directory skeleton and
empty mode-`0600` `receipt.lock` from the receipt contract. Every helper
invocation derives `ROLLOUT_ID` from this basename;
`meta.json` is sealed after all eligibility facts exist, never partially filled
or rewritten. Select but do not yet create one canonical absolute planned
`ROLLOUT_WORKTREE` path whose existing parent strict-resolves, whose leaf is
absent, and which contains no alias/dot/dot-dot/symlink component, plus the exact facility (`git-worktree`, or a host-native
facility whose mapping/cleanup handle is durably queryable by path). Print and
retain the receipt path; later shells derive every rollout path and facility
from sealed records rather than these shell values.

- [ ] **Step 3: Build and self-test the temporary replay guard**

Use `apply_patch` at the explicit receipt paths to create
`rollout_guard.py` and `rollout_guard_test.py` from the complete receipt,
normalization, state-transition, CLI, and `preflight` contract above. Embed the
exact canonical `PYTHON_BIN` and interpreter SHA-256 as the two named literal
constants in both files before their first execution; neither CLI accepts a
pin argument or environment override. Keep both files mode `0600`. Start with
fixture tests for every named refusal, observe
the missing behavior fail, implement the minimal standard-library helper, and
run:

```sh
"$PYTHON_BIN" -I -B "$RECEIPT_DIR/rollout_guard_test.py" -v
"$PYTHON_BIN" -I -B "$RECEIPT_DIR/rollout_guard.py" self-test --receipt "$RECEIPT_DIR"
```

The isolated test process never adds the receipt/cwd to `sys.path` or imports
`rollout_guard` by name. CLI cases spawn the no-follow-verified absolute helper
with the same frozen interpreter/flags; any in-process unit access must use an
explicit file-location loader after hashing that exact helper path, without
adding its parent to module search.

The tests use local Git/bare-Wiki and saved GitHub JSON fixtures only; they
perform no live mutation or network write. Inspect the helper to require only
the four exact post-approval mutation argv constructors and to forbid every
general command, reopen/delete/push, alternate host/repository, or preapproval
spawn path. Startup fixtures place sentinel-writing `json.py` and
`sitecustomize.py` files in the receipt/cwd, inject separate `PYTHONPATH`,
`PYTHONHOME`, and user-site `PYTHONUSERBASE` paths, and set every inherited
`PYTHON*` variable supported by the host. The exact `-I -B` invocation must
execute no sentinel or injected customization: an unexpected receipt child is
then rejected safely by roster validation, while outside injected paths never
enter `sys.path`. Omitting or reordering `-I`, changing the frozen interpreter
path/hash or either embedded constant, making the helper/test constants differ,
adding the receipt or cwd to `sys.path`, or clearing either required `sys.flags`
value must fail the benign startup fixture before a network read or child
spawn. In a real linked-worktree fixture, mutate each verification-tool
byte and mode, move HEAD, attach a branch, dirty a tracked path, add an
untracked or ignored shadowing Python file/`__pycache__`/`.pyc`, substitute a
symlink, and pass a different rollout-worktree path; each case must refuse
before a network read or child spawn. Also inject a replacement commit/tree,
grafts, alternates and HTTP alternates, partial/promisor config, a missing
promisor blob with network sentinel, shallow history, and every inherited
`GIT_*` override; each must refuse without touching the sentinel. A clean detached checkout with the exact
four `MAIN_SHA` blobs and modes is the positive control. Compute both helper
source SHA-256 values and keep both files stable for the remainder of the
rollout. On every resumed shell, verify
both hashes recorded in `meta.json`, then rerun both commands before readback
or mutation.

Create two distinct receipt fixtures beneath one common Git directory. While
receipt A or its fake child holds the fixed global flock, require receipt B to
refuse before a remote read or spawn. After A's verified create releases both
locks, require B still cannot seal approval or perform a remote read while A's
approved receipt remains active—even if A is applied, ambiguous, blocked, or
terminal. Only a valid zero-mutation abandonment or terminal retirement of A
admits B. Also crash
A's guard after spawn, let its child exit, and require B's sibling-receipt
census to refuse until A is reconciled and ultimately retired; API consistency
lag cannot admit B. Kill each
decision path before the first remote read, after readback, and before/during
success/block installation; only a matching authenticated temporary may be
finalized, while an outcome-less intent becomes `interrupted-decision` without
a second remote read.

- [ ] **Step 4: Independently review the exact helper bytes before approval**

Give the complete helper/test bytes, closed schemas, CLI contract, Git/GitHub
fixtures, and SHA-256 to one requirements reviewer and one security/recovery
reviewer. Require them to inspect every accepting state transition, network
argv, contract-bound executor/body renderer, inherited-lock child lifecycle,
atomic-install crash window, temporary reconciliation path, and absence of any
mutation path outside the four exact approved forms. Verify every finding
against the bytes, apply any fix through another red-green fixture, rerun the
whole helper suite, and repeat both reviews until blocker-free. Freeze the
final reviewed interpreter path/hash and helper/test SHA-256 values; only those
exact values may enter `meta.json` or be used to request approval. Immediately
seal `anchor/helper-review.json` with the interpreter pair, both helper hashes,
and both zero-blocker verdicts.
Then call `begin-worktree-isolation` to seal the exact planned path, facility,
primary/common paths, `MAIN_SHA`, rollout ID, and request time before invoking
the facility. Create the clean detached worktree at exact `MAIN_SHA`, or resume
an interrupted intent by exact facility enumeration; immediately call
`seal-worktree-isolation` with the observed mapping and nullable cleanup handle.
Require the primary and rollout trees clean, the rollout HEAD detached and
exactly `MAIN_SHA`, the same common Git directory, and
`rev-parse --is-shallow-repository=false`. Do not initialize or modify the
rollout worktree's `tools/superpowers` gitlink. Every later bootstrap write
must verify helper review plus both isolation records before proceeding. Kill
the process before facility invocation, after creation, and before success
installation; each fixture must deterministically resume or refuse without an
unrecorded worktree.

- [ ] **Step 5: Require the exact remote-main workflow to be fully green**

Before this first GitHub CLI call, resolve `command -v gh` to `GH_BIN`, require
an absolute no-follow-opened regular executable, hash its bytes, and resolve
the private config directory contract above to `GH_CONFIG_DIR`. Build one
fresh allowlisted environment constructor and invoke every command in this and
later steps as the exact `GH_BIN`; each sealed response envelope records that
absolute argv. A changed binary/config path or inherited host/repository/token
variable is refusal, not a new lookup. Use:

```sh
"$GH_BIN" run list --repo "$GH_TARGET" --workflow tests --commit "$MAIN_SHA" --event push \
  --json attempt,databaseId,headSha,status,conclusion,url,createdAt
```

Select the unique completed run for `MAIN_SHA`; set `RUN_ID`, positive integer
`RUN_ATTEMPT`, and `RUN_URL` from that same returned object. Require overall
`success`, validate the exact canonical run URL shape, and derive
`RUN_ATTEMPT_URL="$RUN_URL/attempts/$RUN_ATTEMPT"` without accepting a caller or
provider override. Then inspect exactly
`"$GH_BIN" run view --repo "$GH_TARGET" "$RUN_ID" --attempt "$RUN_ATTEMPT" --json jobs`
and require successful `checks`, `windows-bootstrap`, and `publish-wiki` jobs.
A missing/skipped job is not success. Seal the complete response envelopes as
`anchor/actions-runs.json` and `anchor/actions-jobs.json`; the latter's argv and
payload must bind that exact attempt, and neither shell values nor a default
latest-attempt view are authority. Fixtures give one run ID a successful
attempt followed by failed and successful reruns; only the explicitly selected
attempt's three job results may satisfy the anchor, and omitting/changing
`--attempt` fails.

- [ ] **Step 6: Independently verify automatic Wiki publication**

Clone/fetch `https://github.com/cleveralbatraoz/unseeing.wiki.git` branch
`master` without credentials. Read its validated marker, require source SHA
equals `MAIN_SHA`, independently render `MAIN_SHA` from this full source
checkout, and use the integrated complete-tree verifier against Wiki `HEAD`.
Before executing the verifier, require `ROLLOUT_WORKTREE` is still the exact
clean detached, non-shallow `MAIN_SHA` worktree and no-follow open the exact
three mode-`100644` libraries plus mode-`100755` `tools/render-wiki.py`.
Hash each file's bytes, compare them with the corresponding `MAIN_SHA` Git
blob bytes, reject every untracked/ignored Python shadow under `tools/`, and
retain the exact four-path SHA-256 map for `meta.json`. Invoke the
CLI only by its absolute path with the trusted host Python, `-B`, an explicit
worktree current directory, and the sanitized Python/Git environment defined
by the protocol; never execute an unverified historical renderer or a module
found through ambient import state.
Set `WIKI_HEAD` to the verified full head and save it plus the verifier output
as the closed `anchor/wiki-verification.json`. Use the globally isolated Git
configuration and initialize a new repository with a literal canonical Wiki
`origin`; assert both effective fetch and push URLs remain exactly
`https://github.com/cleveralbatraoz/unseeing.wiki.git` before the first network
command. Fetch with `-c credential.helper= -c core.askPass=` and an explicit
`refs/heads/master:refs/remotes/origin/master` refspec; do not check out
untrusted Wiki content, call the publisher's production mode, or push anything.

- [ ] **Step 7: Prove issue-mutation authority and capture before-state**

Reuse and re-open/no-follow/fstat/hash the exact `GH_BIN` and private
`GH_CONFIG_DIR` frozen before Step 5; do not perform another lookup. Require
every later helper invocation to validate them against meta before network
access. Run `"$GH_BIN" auth status --hostname "$GH_API_HOST"` through the same
fresh allowlisted environment, then query the authenticated
viewer and repository
`viewerPermission` through GraphQL. Require `WRITE`, `MAINTAIN`, or `ADMIN`;
authentication and `TRIAGE` are insufficient. Record the viewer login,
permission, GraphQL response, and closed auth-status streams/result at their
exact `anchor/` paths.

```sh
"$GH_BIN" api --hostname "$GH_API_HOST" graphql -f query='query {
  viewer { login }
  repository(owner: "cleveralbatraoz", name: "unseeing") {
    viewerPermission
  }
}'
"$GH_BIN" api --hostname "$GH_API_HOST" \
  'repos/cleveralbatraoz/unseeing/labels/enhancement'
```

Require the label response's exact name is `enhancement`, positive numeric ID,
and API URL is exactly
`https://api.github.com/repos/cleveralbatraoz/unseeing/labels/enhancement`, then seal the full response envelope
as `anchor/enhancement-label.json`. Before each creation preflight, fetch that
same explicit endpoint again and require exact name/existence; a missing,
renamed, or inaccessible label blocks before child spawn. This permission and
label check closes the create-with-label contract rather than accepting a
Triage-only create whose label could be silently dropped.

Obtain the full paginated issue collection, not the CLI default page:

```sh
"$GH_BIN" api --hostname "$GH_API_HOST" --paginate --slurp \
  'repos/cleveralbatraoz/unseeing/issues?state=all&per_page=100'
```

Seal the raw pages as `anchor/issues-pages.json`. Normalize them with
standard-library Python, filtering out entries with `pull_request`, and
preserve every issue's complete normalized object. Reread every issue
individually through `"$GH_BIN" api --hostname "$GH_API_HOST"`; fetch and seal its
full paginated comments, including zero-comment results, at the exact numbered
anchor paths. Feed successful command output through the matching closed
`seal-snapshot` envelope; never redirect a possibly partial response onto its
final name. Seal the complete normalized set as `before/issues.json`. The
normalized receipt, not shell variables or a CLI search index, is Task 5's
before-state. Seal the GraphQL response as
`anchor/viewer-permission.json`, each issue response as
`anchor/issue-<number>.json`, and each complete comment-page response as
`anchor/comments-<number>-pages.json`.

- [ ] **Step 8: Reverify the tree and freeze every approved request**

Run the complete `ci/pipeline.sh` and `tools/probe_visibility.sh` on
`MAIN_SHA`, using `tools/lib/engine.sh::unseeing_engine_select` to select the
pinned engine. Set `DEPLOY_DIR` to a child of a fresh temporary directory that
does not exist, require the pipeline's build-only message, then remove the
temporary directory. One full successful run supplies shared evidence; do not
rerun the full pipeline separately for every issue. Capture stdout/stderr in
fresh files outside the receipt, then use `seal-snapshot` to install the two
streams and closed result for each command at the exact `anchor/` paths.

Before meta or user approval, resolve every abbreviated/current-owner commit in
Task 4's evidence map to a full SHA reachable from `MAIN_SHA`; implementation
URLs may cite that verified ancestor, but every current-page/evidence blob URL
must cite exactly `MAIN_SHA`. Validate each named path in the exact SHA its URL
cites and construct its canonical URL. From the integrated bytes of this plan, construct
the exact 40-row `request-contract.json`: fixed kinds/issues/titles/labels,
verbatim fenced body/comment templates, meta/preceding-create source
descriptors, and the issue-specific literal URL lists. Run the guard fixtures
with a changed title, body byte, label, issue, argv, dynamic source, positive
but different run attempt, mismatched derived attempt URL, and valid but
unapproved URL/SHA; each must fail before child spawn. Render all seven
distinct templates (all eight fenced occurrences) and all 23 nonnull contract
rows for the sealed attempt, then byte-compare the exact decimal attempt plus attempt-qualified URL;
rerender after a same-ID attempt change and require every body byte/hash
comparison to fail. Add rendered-body
fixtures for `#14`, `GH-14`, `cleveralbatraoz/unseeing#14`, and the canonical
issue URL; each deliberate cross-target autolink is rejected after
substitution, while the approved code-form number wording passes.

Give the candidate contract, integrated plan blob/hash, resolved evidence map,
helper, and tests to requirements and security reviewers. Require zero blockers
and exact row/template/URL agreement, then seal `request-contract.json` and
`request-contract-review.json` in that order. Their hashes are anchor facts;
neither Task 4 nor a shell may resolve or substitute a new value later.

Against exact `MAIN_SHA`, independently audit whether every disposition is
still needed and truthful. Re-prove the three missing residuals that authorize
creation: no observable first-paint readiness proof, no end-to-end
`MIN_SEP`-to-rendered-shader-knee agreement gate, and no six-case desktop
OS/architecture native-load proof. Re-prove that #14's native-pixel oracle,
#15's bounded GPU evidence, and #38's pinned-Godot acquisition boundary remain
open residuals even though each issue also has shipped scope. Re-prove every
one of the 17 closure outcomes through Task 4's current owner/evidence map. A
later main descendant is eligible only if this audit still passes; if it
implemented a proposed residual, satisfied a rewrite completely, or regressed a
closure, stop with zero mutations for a revised plan rather than create stale
work or close on stale evidence.

Construct the exact 23-row `disposition-review.json` from those results, bind
every row to path/mode/blob/hash/tokens at `MAIN_SHA`, and have independent
requirements and evidence reviewers reach zero blockers. Fixture-test three
later descendant trees that respectively implement each proposed create
residual, one that fully satisfies each rewrite residual, and one that regresses
a closure; every case must refuse before meta, approval, or issue mutation.
Seal `disposition-review.json` only after the request contract/review and before
the anchor index. Its hash is immutable eligibility evidence and is rechecked
by every operation preflight.

- [ ] **Step 9: Seal, present, approve, and preflight the immutable anchor**

Require both request files, the disposition review, and their zero-blocker/hash agreement, then seal
`anchor/index.json` over every closed anchor/before artifact and put that exact
index hash plus `request_contract_sha256` and
`disposition_review_sha256` in `meta.json`. Immediately before
constructing meta, repeat Step 6's worktree, mode, no-shadow, Git-blob, and
byte-hash checks and require the resulting exact-key
`verification_tool_sha256` map is unchanged from the successful Wiki
verification. Record that map, the strictly resolved rollout-worktree path,
the isolation record's primary/common paths, facility/handle/hash, the already
pinned interpreter path/hash and GitHub CLI/config values, the Task-1-proved
Superpowers gitlink OID, and every other exact meta field;
no shell-local tool path or hash is admitted later.
Seal the exact `meta.json` through the helper. Validate the 40-operation order
and require `status` reports every operation `UNSTARTED`. Report `MAIN_SHA`,
the exact `RUN_ID`/`RUN_ATTEMPT`/`RUN_URL`/`RUN_ATTEMPT_URL`, Wiki head/source
marker, viewer and permission, helper hash/reviews, request-contract hash/six
titles/40-operation scope, pipeline result,
disposition-review hash and 23-row eligibility matrix, visibility result, and
the count/state of targeted issues. If any precondition
is false, stop with zero issue mutations. If the user explicitly abandons an
ineligible Step 5–8 bootstrap, seal the reviewed bootstrap abandonment, remove
the worktree through the sealed isolation facility after exact revalidation,
and retire the receipt. If helper construction/review itself is interrupted,
perform no network mutation or manual deletion: either finish its hermetic
red-green review and take that same bootstrap-abandonment path, or retain both
the receipt and any facility-enumerated worktree mapping for resumption.

Present that exact read-only anchor to the user and ask for explicit approval
to perform Tasks 2–4's live issue creates/edits/comments/closures and explicit
confirmation of the quiet issue-operator window required by the external
mutation protocol. Record both in the approval's conversation reference by
exclusively sealing `approval.json` under global-then-receipt locks after the
closed sibling census proves no other active receipt; it is not an editable
Boolean.
Then invoke the absolute helper's `preflight` with slots
`anchor-existing-14` and `anchor-create-first-paint`, once for #14 and once for
the first exact creation title, and require the two sealed normalized matches.
Without approval, the plan stops successfully at a read-only audit; the
finish-branch selection alone is not mutation authority. If the user wants to
resume later, retain both exact paths. If the user explicitly abandons this
zero-mutation rollout, seal `abandonment.json`, remove the clean detached
rollout worktree through the same isolation facility after revalidating its
path/HEAD/detached/clean state, then invoke `retire-receipt` from the clean
durable primary root. The sole admitted body is the independently re-rendered
first-operation/no-operation-intent safe body in the abandonment schema; any
operation intent, unresolved decision, other body, or
partial-mutation receipt/worktree remains recovery state.

- [ ] **Step 10: Request the Task 1 anchor/recovery review**

Use `superpowers:requesting-code-review` for a read-only requirements, quality,
and recovery review of the exact helper/test hashes, anchor index, request and
disposition contracts, approval scope, two paired probe decisions, both locks,
and zero-mutation abandonment path. Process findings with
`superpowers:receiving-code-review`, rerun the helper fixtures and
`verification-before-completion`, and do not begin Task 2 until the review is
blocker-free. Review cannot broaden approval or edit an immutable receipt file.

### Task 2: Create the Three Verified Residual Issues First

**Files:** No tracked files. Issue bodies use mode-`0600` files inside the
temporary receipt and never enter the repository tree.

**Interfaces:**

- Consumes: the approved eligible rollout anchor, fully paginated local title
  set, frozen request contract, and immutable receipt state.
- Produces: three unique open `enhancement` issues and runtime variables
  `FIRST_PAINT_ISSUE`, `CREASE_KNEE_ISSUE`, and `NATIVE_LOAD_ISSUE`.

- [ ] **Step 1: Guard every exact title and define partial-create adoption**

Immediately before each creation, the absolute executor runs its creation
`preflight` under the same lock; it refreshes the complete paginated issue list
from the explicit host/repository and compares the exact contract title
locally, rechecks viewer permission is `WRITE|MAINTAIN|ADMIN`, and requires the
explicit repository label endpoint still returns the anchored ID/name/API URL
for `enhancement`. GitHub
search is not an authority because its index may lag. With no
pending receipt operation, require zero open or closed exact matches; any match
stops for user direction.

The reviewed executor seals an intent with exact argv/title, SHA-256 of
contract-rendered body bytes, label, viewer, anchor, and start time before it
spawns `gh issue create --repo "$GH_TARGET"`. If the command response is lost
or execution resumes later, do not create again.
Refresh the paginated API until its bounded consistency poll either finds one
issue whose exact title/body/label/actor/creation window/anchor all match that
pending receipt or reaches its deadline. Adopt exactly one verified match and
seal its returned number/URL observation; zero or multiple candidates stop for user
direction. An issue not tied to that pending receipt is never silently adopted.

- [ ] **Step 2: Create the first-paint readiness issue**

Title:

```text
Web smoke waits a fixed duration instead of observing first-paint readiness
```

Label: `enhancement`. Body:

```markdown
## Current owners

`test/web_smoke.sh` owns/defaults `SMOKE_WAIT` and passes the resulting seconds
to `test/web_probe.py`; `test/web_probe.py` consumes that `wait_s` value and
performs the fixed wait before browser observation.

## Missing externally visible proof

The initial `?demo` navigation still waits a fixed duration before judging the
first frame. A slow valid first paint and an early false-ready frame are not
distinguished by an observable readiness contract.

## Acceptance evidence

- Poll a concrete initial-navigation condition: expected location, loader
  removal, and stable non-black rendered evidence from the real browser.
- Keep one bounded timeout as failure containment; add no arbitrary sleep.
- Prove timeout, false-ready, loader-still-present, wrong-location, and deleted
  polling mutations fail.
- Preserve the existing later browser and raw G-channel assertions.

## Audit anchor

- Integrated main: `@@MAIN_SHA@@`
- Successful Actions run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@
```

Require the sealed contract row to contain the two audit-token sources and the
exact literal title above, then invoke only `execute-operation
create-first-paint`. The executor derives the exact `gh issue create` argv,
captures the returned canonical URL, validates its positive numeric basename as
`FIRST_PAINT_ISSUE`, seals observation before any next command, rereads it,
byte-compares the body, and requires state `OPEN`, exact title/actor, and
exactly the existing `enhancement` label before verification.

- [ ] **Step 3: Create the crease-knee agreement issue**

Title:

```text
MIN_SEP and the hearing shader's upper crease knee can drift apart
```

Label: `enhancement`. Body:

```markdown
## Current owners

`rust/src/render/labels.rs::MIN_SEP` owns semantic label clearance. The upper
crease `smoothstep` knee in `game/shaders/hearing_post.gdshader` owns when that
clearance reaches full rendered strength.

## Missing externally visible proof

The Rust allocator and shader are separately asserted, but no mechanism proves
that renderer narrowing cannot let their thresholds drift apart.

## Acceptance evidence

- Prove labels separated by the allocator's minimum reach full-strength
  rendered crease after every renderer/data-path narrowing step.
- Kill realistic drift on either the Rust or shader side.
- Do not add a gameplay mirror-constant assertion in a second language.
- Keep the owning pure Rust law and the rendered GPU boundary independently
  meaningful.

## Audit anchor

- Integrated main: `@@MAIN_SHA@@`
- Successful Actions run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@
```

Require the exact sealed contract row, then invoke only
`execute-operation create-crease-knee`; its locked internal preflight, create,
persist, and byte-verification protocol captures the returned positive issue
number as `CREASE_KNEE_ISSUE`.

- [ ] **Step 4: Create the native-load architecture issue**

Title:

```text
Cross-compilation does not prove native GDExtension loading on every desktop architecture
```

Label: `enhancement`. Body:

```markdown
## Current owners

`.github/workflows/test.yml`, `tools/bootstrap.sh`, `tools/bootstrap.ps1`, and
`game/unseeing.gdextension` define the supported desktop build/load boundary.

## Missing externally visible proof

Cross-compilation and artifact inspection do not prove a matching native Godot
process can load each declared GDExtension, import the project, and register the
engine classes from a clean checkout on clean hosts across every supported
desktop OS/architecture contract.

## Acceptance evidence

- Exercise real native loading on Linux x86_64 and arm64, both macOS
  architectures represented by the universal export, and Windows x86_64 and
  arm64, each on a matching clean host from a clean checkout.
- Run import and the complete registered-class census in each matching native
  Godot process; an artifact-only or cross-compiled check does not count.
- Report cross-compilation/artifact checks separately from native-load evidence.
- Kill missing-library, wrong-architecture, failed-import, and vacuous-census
  mutations.

## Audit anchor

- Integrated main: `@@MAIN_SHA@@`
- Successful Actions run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@
```

Require the exact sealed contract row, then invoke only
`execute-operation create-native-load`; its locked internal preflight, create,
persist, and byte-verification protocol captures and verifies the returned
positive issue number as `NATIVE_LOAD_ISSUE`.

- [ ] **Step 5: Read all three issues back together**

Require three distinct numbers/URLs, exact unique titles, `OPEN` state, and the
existing `enhancement` label. Require each exact body already contains
`MAIN_SHA`, the decimal `RUN_ATTEMPT`, and `RUN_ATTEMPT_URL`; do not add three
redundant audit comments or claim the audit implemented the residual work. Make
the third creation's mandatory
authoritative readback include all three created issues and use that immutable
snapshot for the combined assertion; each issue's own already sealed verified
record retains its per-operation readback and normalized `updated_at`. Do not
append to or rewrite any verified record.

- [ ] **Step 6: Request the Task 2 external-delta review**

Use `requesting-code-review` for a read-only review of the three decision/
preflight/operation/observation/verified chains, exact GitHub readbacks, title
uniqueness, residual necessity rows, locks, and recovery classification.
Process feedback with `receiving-code-review` and run
`verification-before-completion`; no reviewer or follow-up query may mutate an
issue, edit the receipt, or authorize replay. Do not begin Task 3 until the
review is blocker-free.

### Task 3: Rewrite the Three Issues That Remain Open

**Files:** None.

**Interfaces:**

- Consumes: current #14/#15/#38 state, actual new issue numbers, the frozen
  anchor, and the receipt's exact per-issue before-state.
- Produces: three bounded replacement bodies/titles, preserved labels, and
  verified `OPEN` state.

- [ ] **Step 1: Rewrite #14 around its sole rendered residual**

Retitle to:

```text
Pin the original jagged wall-junction artifact with a deterministic rendered regression oracle
```

Use this replacement body, substituting only validated opaque runtime tokens:

```markdown
## Current mechanism

Same-facing coplanar overlap is merged by
`rust/src/render/superface.rs`; `rust/src/render/paint_plan.rs` assigns one
atomic class/label plan; `rust/src/render/paint.rs` submits the mesh;
`rust/src/render/labels.rs` separates real creases and touching solids; and
`game/shaders/hearing_post.gdshader` consumes G-channel label differences.

Current contract:
https://github.com/cleveralbatraoz/unseeing/blob/@@MAIN_SHA@@/docs/current/mechanics/rendering.md

## Existing evidence

Pure Rust superface/label tests and Godot `CUSTOM0` mesh readback prove the
structural merge and separation contracts. They are useful diagnosis, but they
do not observe the original jagged wall-junction pixels.

## Sole remaining acceptance criterion

Add a deterministic rendered fixture and pixel oracle for the original jagged
wall-junction artifact. The oracle must fail when that visual defect is
reintroduced and pass on the current intended outline. Include a positive true
corner/crease control and require it to remain visible, so disabling crease
output or erasing every junction cannot satisfy the oracle.

Structural mesh or label evidence alone cannot close this issue.

## Audit anchor

- Integrated main: `@@MAIN_SHA@@`
- Successful Actions run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@
```

Require the sealed request row to contain this exact title/body and #14, then
invoke only `execute-operation edit-14`. Its internal preflight requires #14
still exactly match receipt `updated_at`/content. For audit, the sole derived
child argv is:

```sh
"$GH_BIN" issue edit 14 --repo "$GH_TARGET" --title "$TITLE" --body-file "$BODY_FILE"
```

Persist the response/readback before continuing.

- [ ] **Step 2: Read #14 back and prove it remains open**

Use only `edit-14`'s sealed authoritative verified readback; issue no extra
shell/API query. Require exact title/body, unchanged labels, state `OPEN`,
canonical rendering page link pinned to `MAIN_SHA`, and no retired
owner/mechanism text.

- [ ] **Step 3: Rewrite #15 to the bounded GPU-evidence scope**

Retitle to:

```text
Prove wall-crossing and hearing-post composition at the GPU boundary
```

Use this replacement body with actual runtime values:

```markdown
## Current owners

Rust reference behavior is `rust/src/sight.rs::crossings` for camera-side
crossings and `rust/src/sight.rs::crossings_from` for source-side crossings.
Their GLSL counterparts are `wall_crossings`/`wall_crossings_from` in
`game/shaders/pulse_pool.gdshaderinc`. `game/shaders/data_core.gdshaderinc`
uses source-side crossing count for kind-3
`pow(HUM_THROUGH, float(blocked))` surface reveal, where `blocked` comes from
`crossings_from`;
`game/shaders/hearing_post.gdshader` gives the visible kind-3 shell one
`HUM_THROUGH` factor at or behind the front scene surface rather than counting
walls.

Current contracts:

- https://github.com/cleveralbatraoz/unseeing/blob/@@MAIN_SHA@@/docs/current/mechanics/waves.md
- https://github.com/cleveralbatraoz/unseeing/blob/@@MAIN_SHA@@/docs/current/mechanics/rendering.md

## Missing GPU-boundary evidence

- Branch-sensitive Rust/GLSL parity for both `crossings` and `crossings_from`,
  including endpoint/source-wall cases, beyond the existing single-source
  rendered probe.
- Rendered composition of R reveal, G-label-derived crease, and
  B-distance-derived silhouette in the hearing post-pass.
- Visible shell-raytrace behavior at and behind the front scene surface,
  including the one-factor `HUM_THROUGH` expectation distinct from counted
  source-side surface reveal.
- Structured framebuffer facts sufficient to diagnose a failed oracle.

## Explicit exclusions

- Multi-source identity remains issue number `1`.
- Acoustic-image depth remains issue number `4`.
- Cross-target determinism remains issue number `5`.
- The wall-junction pixel oracle remains issue number `14`.
- Rust `MIN_SEP`/shader-knee agreement remains created residual issue number
  `@@CREASE_KNEE_ISSUE@@`.
- The separately deferred mood-layer policy conflict is not part of this issue.

The repaired gdUnit gate, retired production-GDScript/seed claims, MCP
installation, generic trace-capture wishlist, and observer-shipping discussion
are not acceptance criteria.

## Audit anchor

- Integrated main: `@@MAIN_SHA@@`
- Successful Actions run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@
```

Require the contract-bound positive crease-knee number and this exact
title/body, then invoke only `execute-operation edit-15`. Its internal
preflight requires #15's receipt state unchanged. For audit, the sole derived
child argv is:

```sh
"$GH_BIN" issue edit 15 --repo "$GH_TARGET" --title "$TITLE" --body-file "$BODY_FILE"
```

Persist exact readback before continuing.

- [ ] **Step 4: Read #15 back and prove it remains open**

Use only `edit-15`'s sealed authoritative verified readback. Require exact
bounded areas/exclusions, actual crease-knee issue number, current owners,
unchanged labels, and state `OPEN`; do not issue an unfenced follow-up query.

- [ ] **Step 5: Rewrite #38 to the remaining acquisition boundary**

Retitle to:

```text
Fresh hosts still require a separately installed pinned Godot editor
```

Use this replacement body with actual runtime values:

```markdown
## Current boundary

The POSIX and Windows bootstrap paths now install or select Rust, build the
GDExtension, import the Godot project, and run the registered-class census.

Current contract:
https://github.com/cleveralbatraoz/unseeing/blob/@@MAIN_SHA@@/docs/current/engineering/setup.md

## Remaining acceptance criterion

A fresh supported host can safely acquire the exact pinned Godot editor when it
is absent, verify that pin, and continue through the existing bootstrap without
a separately installed editor.

## Exclusions

- Native GDExtension load coverage on every desktop architecture is
  created residual issue number `@@NATIVE_LOAD_ISSUE@@`.
- Existing cross-compilation, engine build, import, and class-census behavior
  are already proved and are not reopened here.

## Audit anchor

- Integrated main: `@@MAIN_SHA@@`
- Successful Actions run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@
```

Require the contract-bound positive native-load number and this exact
title/body, then invoke only `execute-operation edit-38`. Its internal
preflight requires #38's receipt state unchanged. For audit, the sole derived
child argv is:

```sh
"$GH_BIN" issue edit 38 --repo "$GH_TARGET" --title "$TITLE" --body-file "$BODY_FILE"
```

Persist exact readback before continuing.

- [ ] **Step 6: Read #38 back and prove it remains open**

Use only `edit-38`'s sealed authoritative verified readback. Require the actual
native-load issue number, bounded acquisition scope, unchanged labels, and
state `OPEN`. Do not issue an unfenced aggregate query; `seal-after` later
captures and compares all three under its durable decision and global lock.

- [ ] **Step 7: Request the Task 3 external-delta review**

Use `requesting-code-review` for read-only requirements, quality, and recovery
review of the three exact replacement requests and their paired decision/
operation/readback chains. Verify actual non-linking referenced issue numbers and complete
unchanged-field comparisons from sealed records, process findings with
`receiving-code-review`, and run `verification-before-completion`. Do not begin
Task 4 until blocker-free; a review is not permission for another remote read
or mutation.

### Task 4: Close Only the Reverified Resolved Issues

**Files:** None.

**Interfaces:**

- Consumes: exact integrated owners/tests/pages/run and the current issue body.
- Produces: evidence comment plus `completed` closure for exactly
  #7, #12, #13, #16, #22, #30–#36, #39, #41, #42, #44, and #45.

- [ ] **Step 1: Revalidate every frozen implementation commit before use**

For each Task-1-resolved row below, reread the sealed literal full-SHA URLs,
require each commit remains reachable from `MAIN_SHA`, and inspect that commit
plus current owner. For #12, require the frozen value equals
`git log -1 --format=%H -- tools/setup-agents.sh`; for #13, do the same for
`rust/src/nodes/observer.rs`, `rust/src/reproduce/blob.rs`, and
`tools/restore_probe.sh`. Any difference is stale-target failure for the whole
rollout: seal an
`eligibility-withdrawn/stale-disposition-evidence` block with the immutable
failed owner/evidence reference before stopping. Never resolve a replacement
after approval or paste an abbreviation into a closure.

- [ ] **Step 2: Use this issue-specific evidence map**

| Issue | Implementation anchor | Current page | Strongest evidence |
| --- | --- | --- | --- |
| #7 | `7c7b85c` ArrayMesh migration | mechanics/rendering | `game/tests/cat_test.gd`, `game/tests/viewmodel_test.gd` mesh/label/winding cases |
| #12 | current `tools/setup-agents.sh` owner commit | engineering/agent-workflow | `test/setup_agents_test.sh`, `test/verify_superpowers_test.sh` |
| #13 | current observer/restore owner commits | engineering/debugging | observer/restore suites plus determinism and restore probes |
| #16 | `59efd5d` | mechanics/sound-sources | touching-radio seam and source role-label tests |
| #22 | `97530a4` | mechanics/levels-and-objects | law-shaped map/level fixtures and content-mutation evidence |
| #30 | `89a54f1`, `44f3145`, `b81b014` | engineering/editor-authoring | editor source probe |
| #31 | `2d8e60d`, `91003f5` | engineering/editor-authoring | editor-level probe and configuration-warning tests |
| #32 | `db5d333` | engineering/editor-authoring | `game/tests/icon_manifest_test.gd` |
| #33 | `268e423` | engineering/editor-authoring | Cargo `editor-docs` feature tests and CI check |
| #34 | `b6af7c2` | engineering/editor-authoring | `game/tests/knob_hint_test.gd` |
| #35 | `c790f37` | mechanics/levels-and-objects | nested censused-child world-box and placement regressions |
| #36 | `59efd5d` plus current allocator | mechanics/rendering | source starvation, graph-coloured role, and seam tests |
| #39 | `016002f` | mechanics/overview | `game/tests/game_root_test.gd` level-scene selection cases |
| #41 | `29b7b13` | mechanics/levels-and-objects | reusable-prefab and recursive-census cases |
| #42 | `fa7d666` | mechanics/levels-and-objects | WaveRun construction, persistence, and doorway-occluder cases |
| #44 | `268e423` | engineering/editor-authoring plus engineering/agent-workflow | `editor-docs` build/Inspector boundary plus byte-exact `CLAUDE.md` adapter gate |
| #45 | `0da2be8` | mechanics/levels-and-objects | slab-diagonal pure tests and shader-contract check |

- [ ] **Step 3: Build one exact closure comment per issue**

Each comment uses the issue's own row and runtime full SHAs. Implementation and
evidence fields accept one or more independently validated canonical URLs:

```markdown
Reverified against integrated main `@@MAIN_SHA@@`.

- Owning implementation: @@IMPLEMENTATION_URLS@@
- Current contract: @@CURRENT_PAGE_URLS@@
- Strongest executable evidence: @@EVIDENCE_URLS@@
- Successful integration run attempt @@RUN_ATTEMPT@@: @@RUN_ATTEMPT_URL@@

The current code and executable evidence implement this issue's requested
outcome. This closure is based on that implementation, not on the documentation
rewrite.
```

Revalidate that the sealed request-contract row already contains these exact
tokens and the preapproved literal full-SHA URL lists. No shell interpolation,
guessed, abbreviated, or mutable-branch link is permitted.

- [ ] **Step 4: Close each issue through a recoverable two-operation protocol**

GitHub cannot make “comment, then close” transactional. For one issue at a
time, verify owners/tests at `MAIN_SHA`, then invoke only
`execute-operation comment-$ISSUE`. The locked executor preflights, renders the
issue-specific contract comment, derives the exact repository-qualified
`gh issue comment` argv, records the returned positive comment ID/URL, actor,
exact body, and issue readback, and verifies it. If reconciliation finds that
exact receipt-owned comment, it never posts again.

Then invoke only `execute-operation close-$ISSUE`. Its new locked preflight
requires the post-comment expected state, derives the exact
`gh issue close ... --reason completed` argv, and requires normalized
`state=closed`, `state_reason=completed`, authoritative `updated_at` and
`closed_at`, viewer-matching `closed_by`, and unchanged title/body/labels/
assignees/milestone/lock/type/pin fields before verification. A failure
between operations leaves a precisely recorded
commented-open issue that resumes at closure only; a wrong comment/closure uses
the narrow recovery rules above. Never describe the pair as atomic and never
continue past an ambiguous or concurrently changed issue.

- [ ] **Step 5: Prove the operation prefix never targets a residual issue**

After every five closures and once at the end, derive status only from the
immutable receipt. Require the completed prefix contains no close/comment
operation for #14, #15, #38, or any of the three created issue numbers. Do not
make an unfenced periodic API query: the next target's decision preflight owns
its own stop condition, and Task 5's `seal-after` decision captures all six
residual issues plus the whole backlog under the global lock.

- [ ] **Step 6: Request the Task 4 closure/recovery review**

Use `requesting-code-review` for read-only review of all 34 comment/close
decision and operation chains, issue-specific evidence, full comment identity,
close metadata, exact prefix, and any recovery state. Process findings with
`receiving-code-review` and run `verification-before-completion`. The reviewer
may inspect sealed authoritative readbacks but may not query/mutate GitHub or
edit the receipt. Do not begin Task 5 until blocker-free.

### Task 5: Read Back the Whole Backlog and Detect Collateral Mutation

**Files:** None.

**Interfaces:**

- Consumes: Task 1's receipt-backed before-state and every immutable mutation
  response/comment ID.
- Produces: a fully paginated exact normalized issue/comment after-state proof
  and a recoverable receipt report. It proves the observed final normalized
  deltas, not provider timeline/event equality or absence of a
  transient remote title/body edit that an approved overwrite could have
  erased during GitHub's irreducible read/mutate window.

- [ ] **Step 1: Assert the intended closed set exactly**

Require `CLOSED` for:

```text
7 12 13 16 22 30 31 32 33 34 35 36 39 41 42 44 45
```

For each listed issue, require every normalized top-level field except `state`,
`state_reason`, `updated_at`, `closed_at`, `closed_by`, and `comments` equals
its complete before-state. Require final `state=closed`,
`state_reason=completed`, and `updated_at`/`closed_at`/`closed_by` byte-equal to
that issue's verified close readback; do not require
a strictly later timestamp because GitHub precision may collapse adjacent
operations. Require the full final comment set equals the complete before set
plus exactly one receipt-recorded closure comment whose complete normalized
`id`, `url`, `author`, `body`, `created_at`, and `updated_at` equal its verified
readback. That comment contains `MAIN_SHA`, decimal `RUN_ATTEMPT`,
`RUN_ATTEMPT_URL`, the issue-specific page/evidence, and
the not-docs-only sentence. Every complete pre-existing
comment record remains byte-equivalent after canonical serialization; an
extra, duplicate, edited, timestamp-changed, or missing comment is failure.

- [ ] **Step 2: Assert the intended open/rewritten/created set exactly**

For #14/#15/#38, require exact approved replacement title/body and the verified
edit's `updated_at`; require every other issue field equal before-state and the
entire complete normalized comment list unchanged. For each of the three
created issues, require its complete normalized object to equal the verified
creation readback: receipt-recorded number/canonical URL, exact
title/body/author, exact `created_at`/`updated_at`, sole `enhancement` label,
`state=open`, null `state_reason`, `locked=false`, null lock reason/type/pinned
comment/closed metadata, empty assignees, null milestone, and zero comments.
Require unique titles and exact positive non-linking issue-number references. No
targeted issue may contain an unrecorded new comment.

- [ ] **Step 3: Compare every other issue with before-state**

The locked `seal-after` helper fetches the same full paginated collection and
normalizes it with the same code as Task 1, including the full paginated
comments for every issue. For every issue
number not targeted by Tasks 2–4, require the complete normalized object—
number, URL, state/reason, title/body, lock/type/pin/closure metadata, labels,
assignees, milestone, author, creation/update times, and every complete comment record—to equal before-state
byte-for-byte after canonical serialization. Confirm set difference contains
exactly the three receipt-recorded new issue numbers and no additional issue.
An unrelated concurrent difference is reported as external divergence; this
rollout never restores it. The candidate exists only inside the helper's
decision/capture/install boundary; a shell cannot retain, replace, or discard
it before the immutable outcome.

- [ ] **Step 4: Prove prohibited issue classes are absent**

Search all new/updated titles and bodies for vague audio/phantom proposals,
speculative deployment hardening, and the mood-layer conflict. Require zero
unapproved creation. Also require #1/#4/#5 retained their pre-rollout state.

Invoke `seal-after --receipt "$RECEIPT_DIR"` with no candidate input. It
installs the decision intent before fetching, then applies every Step 1–4
assertion internally. If every comparison passes, require it to seal exactly
`after/issues.json` and report `TERMINAL`. If any comparison fails, require its
nonzero result and the exact `backlog-divergence` block, which embeds the
candidate and binds the expected/observed full-set hashes. Report the immutable
block/hash and stop read-only for a separately approved audit/recovery plan. A
read interruption becomes `interrupted-decision`; a later matching readback
cannot clear either evidence or authorize another `seal-after`.

- [ ] **Step 5: Report a complete recovery ledger**

If any command failed midway, report every receipt-recorded pending/completed
create/edit/comment/closure with returned IDs and current authoritative
readback. On resume, load the receipt, re-prove user/permission and the frozen
helper/local authority, then reconcile the sole pending operation by exact
body/actor/ID under both locks even if source or Wiki may have advanced; this
readback classifies an already-issued request and never replays it. Only the
next `UNSTARTED` operation's new decision reobserves user/permission and the
frozen source/Wiki anchor and either continues on an exact match or seals the
permanent advance/withdrawal block. If `blocked.json` records an advance after any possible mutation, do not
ordinary-resume: stop for the new explicit recovery plan required by the anchor
contract. An issue- or backlog-divergence block is equally permanent and never
ordinary-resumable; an eligibility-withdrawal block is also permanent for this
receipt. Never replay a completed/indeterminate mutation, assume a failed
network response meant no mutation, or discard the receipt while external
state remains partially changed.

- [ ] **Step 6: Remove the completed rollout worktree safely**

After and only after `after/issues.json` is sealed and `status` reports
`TERMINAL` with all 40 operations applied, re-read `meta.rollout_worktree` and
the hash-matched `anchor/worktree-isolation.json`; require their path,
facility, handle, primary root, and common directory agree. Require the path is
the exact detached worktree created by Task 1, has the same
common Git directory as the clean primary checkout, still names `MAIN_SHA`, is
clean, and is neither the primary nor the receipt directory. Remove it with
the same host/native isolation facility that created it (or exact
`git worktree remove` only if that facility used Git directly), then verify it
is absent from both exact facility enumeration and
`git worktree list --porcelain`. If interrupted, reload the sealed isolation
record: one still-valid mapping may be revalidated and removed, while proven
absence is success; a partial/wrong/multiple mapping refuses. Retain the receipt under the
common Git directory for Task 6 closeout. A pending/ambiguous/blocked partial
rollout retains both worktree and receipt unless a separately approved recovery
plan proves another exact cleanup boundary.

- [ ] **Step 7: Request the Task 5 terminal-state review**

Use `requesting-code-review` for read-only requirements, quality, collateral-
delta, crash-state, and recovery review of the final decision, complete
normalized before/after sets or immutable block, all 40 verified outcomes, and
worktree cleanup. Process findings with `receiving-code-review` and run
`verification-before-completion`. A block remains a valid reviewed task result
but never a success or permission to continue to Task 6; Task 6 starts only
from exact `TERMINAL` state and a blocker-free review.

### Task 6: Terminalize the Artifact Registry in a Fresh Worktree

**Files:**

- Modify: `docs/superpowers/README.md`
- Modify: `test/documentation_contract_test.py`

**Interfaces:**

- Consumes: actual three new issue numbers and completed external readback.
- Produces: terminal `shipped` rows for the design and both plans, plus another
  user-controlled finish-branch choice.

- [ ] **Step 1: Start a dedicated closeout worktree from current remote main**

Do not edit the durable primary checkout. From the clean durable primary
checkout—not the rollout worktree—invoke the receipt helper's derived closeout
state first. Resume only from `NO_CLOSEOUT`, `ISOLATION_PENDING`, `ISOLATED`,
`COMMITTED`, or `PROVED`; `WITHDRAWN` stops for a separate reviewed recovery
plan, `RETIRED` permits only final audit/reporting, while
`OBSERVATION_PENDING` must first reconcile locally to withdrawal or an
authenticated success counterpart. Never reconstruct a base, branch,
path, facility, or commit from shell memory. In `NO_CLOSEOUT`, under global
then receipt lock the helper validates local prerequisites, installs the next
`begin-closeout-isolation` observation intent, then fetches explicit remote
`main` and captures
one exact `CLOSEOUT_BASE_SHA`, which must equal `meta.main_sha`. It repeats the
workflow-bootstrap gitlink/lock/full-install proof against that base, requires
the rollout worktree is already absent, and revalidates the exact three
artifact OIDs, exact meta `(run_id, run_attempt, run_url, run_attempt_url)` and
its attempt-specific successful jobs, and Wiki equality. Under the same locks it fetches all 23
disposition subjects with complete comments, requires their canonical
projection equals `after/issues.json`, and embeds that projection in the
isolation intent. It then installs `closeout/isolation.intent.json`, referencing
the exact observation intent as its success counterpart, before any closeout
branch/worktree creation. Only after that durable intent may the worker invoke
`superpowers:using-git-worktrees` and its recorded facility.

At any terminal closeout substate, newly known issue-operator activity—even
when it restored identical normalized bytes—must invoke
`seal-closeout-withdrawal` with the immutable conversation/evidence reference
before any other closeout action. An independently known authority or local
integrity loss uses the same transition. Do not substitute a fresh equality
read for this knowledge.

In `ISOLATION_PENDING`, enumerate the recorded facility by exact path. One
matching branch/base/common-directory mapping is sealed as success; proven
absence permits the same recorded request to retry; multiple, partial, wrong,
or malformed mappings refuse. In `ISOLATED`, validate the mapping and inspect
the exact two-file diff/index state before resuming. In `COMMITTED`, never
recreate a worktree merely because finish-branch cleaned its branch/path;
resume only finish/readback/proof. In `PROVED`, perform only final validation
and retirement. The fresh-create commands below illustrate the already-sealed
intent and do not authorize alternative values:

```sh
PRIMARY_ROOT="$(pwd -P)"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_TERMINAL_PROMPT=0
unset GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT GIT_ASKPASS SSH_ASKPASS
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
unset GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_CEILING_DIRECTORIES
unset GIT_TRACE GIT_TRACE2 GIT_TRACE_PACKET GIT_TRACE_PERFORMANCE
unset GIT_TRACE_SETUP GIT_CURL_VERBOSE
test "$(git rev-parse --show-toplevel)" = "$PRIMARY_ROOT"
test -d "$PRIMARY_ROOT/.git"
test "$(git branch --show-current)" = main
test -z "$(git status --short)"
test "$(git remote get-url origin)" = "$CANONICAL_ORIGIN"
test "$(git remote get-url --push origin)" = "$CANONICAL_ORIGIN"
# CLOSEOUT_BASE_SHA is the exact base_sha already supplied from the
# helper-validated, sealed closeout/isolation.intent.json; do not fetch here.
CLOSEOUT_BRANCH=docs/ai-documentation-issue-closeout
CLOSEOUT_PATH="$PRIMARY_ROOT/.worktrees/ai-documentation-issue-closeout"
# The helper has already sealed these exact values in closeout/isolation.intent.json.
# Perform the pre-skill Superpowers proof and invoke using-git-worktrees now.
git -C "$PRIMARY_ROOT" check-ignore -q .worktrees/ai-documentation-issue-closeout
! git show-ref --verify --quiet "refs/heads/$CLOSEOUT_BRANCH"
test ! -e "$CLOSEOUT_PATH"
git worktree add -b "$CLOSEOUT_BRANCH" \
  "$CLOSEOUT_PATH" "$CLOSEOUT_BASE_SHA"
test "$(git -C "$CLOSEOUT_PATH" rev-parse HEAD)" = "$CLOSEOUT_BASE_SHA"
test "$(git -C "$CLOSEOUT_PATH" branch --show-current)" = "$CLOSEOUT_BRANCH"
test -z "$(git -C "$CLOSEOUT_PATH" status --short)"
```

Use a repository-native isolation facility only if its mapping and cleanup
handle satisfy the same durable query contract. Immediately call
`seal-closeout-isolation`; no repository edit is allowed before that success.
The base equals `meta.main_sha`; a later descendant is outside this receipt's
closed evidence and requires a separate reviewed closeout/recovery plan.
After intent, remote main must remain exactly that base until the user-selected
integration. A changed artifact or authority fact refuses before repository
edit, atomically seals the closeout withdrawal, and retains the immutable
`TERMINAL/WITHDRAWN` receipt for that separate plan. No receipt-level
`blocked.json` or replacement closeout intent is invented.

- [ ] **Step 2: Write the failing terminal-closeout assertion**

Modify the existing exact artifact-row assertion: the design, repository plan,
and issue-rollout plan must now be `shipped`, and each residual list must
consist of #14, #15, #38, and the three actual distinct positive new issue
references in ascending order. Run it and observe it fail against the
still-active registry rows. This unconditional test update intentionally
replaces Task 16's transient active-state expectation; leaving that old
expectation behind cannot produce a green closeout.

- [ ] **Step 3: Update only the three active registry rows**

Set the AI-documentation design, repository implementation plan, and this issue
plan to `shipped`. Task 5 has already made every directly authorized GitHub
Issues mutation terminal and independently read it back; the closeout commit
records that observed result and, by design, does not claim its branch is
integrated or mirrored. Set each residual cell to all six still-open issues—
#14, #15, #38, and the three actual new references—in ascending numeric order,
comma-separated with no duplicate. Do not rewrite the frozen artifact bodies.

- [ ] **Step 4: Run focused and complete verification**

Run the live documentation gate, renderer/publisher/workflow tests, repository
hygiene/archive tests, complete `ci/pipeline.sh`, and
`tools/probe_visibility.sh` with the pinned engine. Give the full pipeline a
fresh nonexistent `DEPLOY_DIR` and require its build-only message. Confirm no
issue/Wiki/game mutation occurs during verification.

- [ ] **Step 5: Review and commit the closeout**

Request requirements and quality review of the registry-only diff and actual
issue URLs. Review both the registry and its exact expectation change. Commit
with mandated identity, narrative subject/body, and no attribution. Confirm
`tools/superpowers` remains unchanged. After the commit and again immediately
before presenting the finish-branch choice, compare the same three artifact
blob OIDs at closeout `HEAD` with `meta.main_sha`; any change stops for renewed
review and uses `seal-closeout-withdrawal` with
`local-integrity-failure` instead of finishing an unapproved plan. Invoke
`seal-closeout-commit`;
require it derives the exact sole-parent, two-changed-path commit and reports
`COMMITTED`. No shell-recorded commit SHA is authoritative.

- [ ] **Step 6: Invoke finish-branch and stop for the second user choice**

Immediately repeat the exact gitlink/lock and installed-cache full verification;
a difference seals `local-integrity-failure` withdrawal and stops before skill
invocation. Then use
`superpowers:finishing-a-development-branch`. Do not merge or push the
closeout without the user's choice. Do not query Actions/Wiki readiness here
and do not publish manually; the fenced proof command owns the first such
observation after integration. Immediately
before any selected integration path, invoke `preflight-closeout-integration`
under global then receipt lock. The helper alone freshly fetches canonical
remote main under the closed Git environment, repeats the three-artifact blob
comparison at that remote SHA, refreshes all 23 disposition subjects with
complete comments, and requires the canonical projection still equals the one
sealed in the closeout intent. On equality it seals the same-sequence verified
counterpart; only that freshly completed pair authorizes the immediate
integration attempt. A changed base or projection atomically seals
the closeout withdrawal before integration and requires a separate explicitly
approved closeout/recovery plan; later restoration cannot re-enable this
receipt. Any pause requires another preflight.
Accept exactly one
integration shape during readback: fast-forward where the integrated SHA is the
closeout commit; merge where the integrated commit has exactly the ordered two
parents `[base_sha, closeout_commit_sha]` and its tree equals the closeout
commit tree; or squash/rebase where the integrated
commit's sole parent is the base and its exact two-path patch/blob result equals
the sealed closeout commit. No other ancestry claim is accepted.

- [ ] **Step 7: Seal the recovery receipt as an immutable retired audit record**

If the user keeps the closeout branch or opens an unmerged PR, do not begin a
proof observation; retain the receipt and report its absolute path. After a
user-selected integration, if the finish workflow did not already
remove the closeout worktree, revalidate its recorded path/branch/commit/common
directory and clean two-file committed state, then remove it through the same
sealed facility and prove absence by exact facility enumeration plus
`git worktree list --porcelain`; a crash resumes from those same immutable
records. After the closeout commit is integrated and the closeout worktree is
removed by its recorded facility, invoke `seal-closeout-proof` immediately
from the clean durable primary checkout without a caller-side Actions, Wiki,
or disposition-readiness query. It installs its
observation intent before the first read and closes it only with the exact
proof or a withdrawal; its bounded poll owns automatic-publication readiness.
This is the sole
planned post-rollout main advance: `meta.main_sha` is the rollout anchor,
`closeout.base_sha` is the revalidated pre-integration base, and
`proof.integrated_main_sha` is the user-selected closeout result. At proof time
remote main must equal that integrated result—not a later descendant—and its
Actions evidence must bind one explicit positive run attempt for that same SHA,
while Wiki evidence must name that SHA. Both planning artifact OID maps
remain equal to `meta.main_sha`, while the terminal registry/test blobs equal
the closeout commit result. The freshly normalized full-comment projection of
the 23 disposition issues must equal the corresponding projection from
`after/issues.json` and is embedded in the proof. Require `status` reports `TERMINAL/PROVED`, all 40
operations `applied`, sealed before/after sets, no block, no closeout
withdrawal or pending/ambiguous/temporary state, and both recorded worktrees
absent.
If the fenced readiness/proof observation reaches terminal failure or its
deadline, require `TERMINAL/WITHDRAWN`, retain the receipt, and stop for the
separate reviewed recovery plan; never issue an unfenced retry after inspecting
the remote result.

Then invoke `retire-receipt` with that exact primary root—never `rm` or another
cleanup command. Under global then receipt lock it installs a fresh
retirement observation intent, repeats every proof
readback, including the complete 23-issue/comment projection, and local absence
check before revalidating common-Git containment,
the complete roster, hashes, modes, inodes, and links. It seals the same-sequence
verified counterpart, revalidates the receipt, builds the exact manifest, and
atomically seals `retirement.json` without changing or deleting any prior
inode. A crash after verification requires authenticated tombstone-temp
finalization or a fresh retirement observation; the old completed pair alone
cannot be reused. A later remote-main or disposition-projection change atomically seals
the closeout withdrawal and retains the receipt for explicit audit; a busy
lock, missing proof, remaining worktree, or unexpected local shape refuses and
also retains it. Nothing silently re-anchors or retries a withdrawn receipt.
Require `status` reports `RETIRED`, then report the receipt's absolute path,
rollout ID, tombstone/manifest hashes, and continued read-only auditability.
The valid tombstone makes it inactive for future approvals; the receipt and
permanent global lock remain.

- [ ] **Step 8: Review the authoritative closeout and retired receipt**

After `status` reports `RETIRED` and before any completion claim, use
`superpowers:requesting-code-review` for one read-only requirements/authority
review and one read-only security/recovery review of the exact final receipt.
Reviewers inspect only immutable local records: all 40 verified operations, the
complete before/after sets, 23-subject terminal projection, closeout
isolation/commit/proof chain, explicit Actions run attempts, Wiki readback,
every observation counterpart, retirement manifest/tombstone, sibling census,
worktree absence, and the unchanged Superpowers boundary. They perform no new
GitHub/Wiki query, receipt write, integration, or cleanup. Process every
finding with `superpowers:receiving-code-review`; an authoritative flaw is a
stop for a separately approved recovery plan, never permission to edit or
retire the immutable evidence again. Run
`superpowers:verification-before-completion` over the reviewed hashes and the
helper's local `status`/self-test, require both reviews blocker-free, and record
their report references outside the receipt before continuing.

- [ ] **Step 9: Deliver the final report**

Report canonical-doc and mirror outcomes, exact issue creates/rewrites/closures,
automatic Wiki status, both integration SHAs/run attempts, full game verification,
clean primary/worktrees, unchanged Superpowers gitlink, no game deployment,
the deliberate absence of cross-target autolinks, the explicitly out-of-scope
target timeline/events/notification/reaction/project surfaces, the irreducible
remote-operator TOCTOU boundary, and the deliberately skipped mood-layer
policy conflict.
