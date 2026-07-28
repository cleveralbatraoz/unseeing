# Unseeing

A first-person blind-person simulator. The hero cannot see — the player perceives
the world only through sound waves: cane taps, footsteps, and the thin white
outlines they briefly reveal. Black and white, outlines only, everything fades.

**Play:** http://dggrus.hlab.kz (best in Chrome; click the dark to begin)

## Controls

- `W A S D` — walk (physical key positions; works on any keyboard layout)
- mouse — face direction (pointer lock when available, cursor-steering otherwise)
- click — tap the cane: strikes walls at the aimed height, taps the floor when
  aiming down, swishes silently through empty air

## Development

Everything is one self-contained file: `index.html` (WebGL2, no dependencies).

- `test/run.sh` — runs the full suite in headless Chrome (GL-error tracing,
  scripted scenarios with pixel assertions, unit tests for raycasts/collision/
  projection). Exits non-zero on any failure.
- `deploy.sh` — test-gated deploy to the droplet with served-bytes verification.
- CI runs the same suite on every push via GitHub Actions.
