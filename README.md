# Unseeing

A first-person blind-person simulator built in Godot 4. The hero cannot see —
the player perceives the world only through sound: cane taps and footsteps send
visible waves through the dark, and thin white outlines flare where a wave
strikes geometry, then fade. Black and white, outlines only, everything fades.

**Play:** http://dggrus.hlab.kz (Web/wasm build; best in a Chromium browser)

## Controls

- `W A S D` — walk (physical key positions; works on any keyboard layout)
- mouse — look
- click — tap the cane: strikes walls at the aimed height, taps the floor when
  aiming down, swishes silently through empty air (air reflects nothing)
- `Esc` — release the mouse

## Project layout

- `game/` — the Godot 4 project (single source of truth; see `game/README.md`
  for architecture and porting status)
- `ci/pipeline.sh` — boot-check gate → Web export → deploy; the same POSIX
  script runs locally, on the droplet, and in cloud CI
- `deploy.sh` — local fast gate, then `git push production` (the droplet's
  post-receive hook runs the pipeline server-side)

## Development

Open `game/project.godot` in Godot 4.7+ and press play. Renderer is
`gl_compatibility` — required for the Web export; keep it that way.
