# Unseeing — Godot project

The Godot 4 port of the game. `web-reference/` at the repo root is the frozen,
fully-tested WebGL version that serves as the playable design spec; this
project ports it system by system.

## Architecture

- `scenes/main.tscn` — one root node; everything else is built in code.
- `scripts/main.gd` — composition root: input actions (physical keycodes),
  materials, world build, player, the fullscreen hearing quad, per-frame
  globals (clock, flicker).
- `scripts/map_builder.gd` — wall centerlines → box meshes + colliders.
- `scripts/pulses.gd` — the 64-slot wave pool shared with both shaders.
- `scripts/player.gd` — movement, mouse look, cane tap modes, footsteps.
- `shaders/data_pass.gdshader` — world rendered as data (reveal/normals).
- `shaders/hearing_post.gdshader` — outlines + wave shells; the only pass
  the player ever sees.

Renderer is `gl_compatibility` — mandatory for the Web (wasm) export.

## Porting status

| System | Status |
| --- | --- |
| Map, collision, movement, mouse look | done |
| Wave pool + data/hearing shaders | done |
| Cane tap modes (wall / floor / silent air) | done |
| Footstep ripples | done (from feet; shoe-accurate after body port) |
| Hero body + cane viewmodel (bob/sway/strike) | TODO |
| Audio ticks (wall/floor/phantom) | TODO |
| Phantom sounds | TODO |
| gdUnit test port of web-reference scenarios | TODO |
| Web (wasm) export preset + droplet deploy | TODO |
