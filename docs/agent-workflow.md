# Agent workflow

The repository supports Claude Code and Codex App/CLI with one project policy
and one reviewed Superpowers release. Codex IDE is out of scope because it does
not currently load Codex plugins.

## Clone and activate

Clone recursively, or initialize the pin after cloning:

```sh
git clone --recurse-submodules https://github.com/cleveralbatraoz/unseeing.git
cd unseeing
git submodule update --init --depth 1 -- tools/superpowers
```

From this durable primary checkout—not a disposable linked worktree—install
for one host or both:

```sh
tools/setup-agents.sh claude
tools/setup-agents.sh codex
tools/setup-agents.sh all
```

The script validates the gitlink and upstream manifests, registers the pinned
checkout's `superpowers-dev` local marketplace, installs and enables
`superpowers@superpowers-dev`, and compares the installed skill tree with the
repository pin. It does not silently replace another Superpowers selector.
Follow its exact removal commands, or use `--migrate` to remove only competing
Superpowers selectors and stale `superpowers-dev` registrations while leaving
all unrelated plugins and marketplaces untouched.

Restart Claude Code after installation. Begin a new Codex session; an existing
session retains the plugins and instructions it loaded at startup. Claude reads
the small `CLAUDE.md` adapter, which imports `AGENTS.md`; Codex reads
`AGENTS.md` directly.

## Diagnose conflicts

Inspect host state without changing it:

```sh
claude plugin list --json
claude plugin marketplace list --json
codex plugin list --available --json
codex plugin marketplace list --json
ci/verify-superpowers.sh full
```

There must be one enabled `superpowers@superpowers-dev` at version 6.3.0 and
its marketplace root must be this checkout's `tools/superpowers`. If two
durable clones compete for the same host registration, choose the clone whose
pin should govern and run its setup with `--migrate`.

## Upgrade the pin

Never follow upstream `main`. On a clean isolated feature branch run:

```sh
tools/update-superpowers.sh vX.Y.Z
```

The updater fetches only that canonical tag, verifies its peeled commit and
manifest versions, checks out the candidate detached, and displays the commit
and diff summary. Review the supply-chain change before staging the
`tools/superpowers` gitlink. Run full verification and repository tests, merge
through the normal review workflow, then rerun `tools/setup-agents.sh` from the
durable primary checkout and restart the hosts.

## Build and deployment boundary

Superpowers is developer tooling only. `.gitmodules` and `tools/superpowers`
are `export-ignore`; game builds, ordinary local/production pipelines, and
deployment archives perform metadata checks but never fetch or execute it.
GitHub CI first checks parent metadata with checkout credentials disabled,
then initializes exactly `tools/superpowers` and runs full verification.
