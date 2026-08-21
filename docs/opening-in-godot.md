# Opening and running Unseeing in Godot

This is the complete first-time setup for opening Unseeing in the Godot editor,
editing a level, and running that level as the full game. The ordinary designer
workflow is intentionally code-free: levels and reusable objects live in Godot
scenes, while Rust supplies the hidden engine and the typed nodes shown in the
editor.

The one file Godot must open is `game/project.godot`. The repository root is
not itself a Godot project.

The authoring boundary stays visible throughout this guide:

| The designer controls in Godot | Rust supplies automatically |
| --- | --- |
| Level scenes and plain-root prefabs | Runtime composition and recursive discovery |
| Placement, rotation, wall endpoints, openings, and source knobs | Geometry, collision, sound waves, and validation |
| `WaveSpawn`, `WaveRun`, solids, sources, and room instances | Player setup, materials, per-face labels, source-role separation, and the hearing pass |
| The `UnseeingGame` **Level Scene** resource choice | Safe inject-before-add wiring of the selected `WaveLevel` |

Level design never requires editing Rust or writing GDScript.

## The short version

From a terminal in the checkout you intend to edit, use the native command for
your operating system.

macOS or Linux:

```sh
tools/bootstrap.sh
godot --editor --path "$PWD/game"
```

Windows PowerShell:

```powershell
.\tools\bootstrap.cmd
godot --editor --path "$PWD\game"
```

From Command Prompt, the equivalent paths are `tools\bootstrap.cmd` and
`godot --editor --path "%CD%\game"`.

Wait for the first command to end with `bootstrap: OK`.

To play the game rather than author it, there is one command and it needs no
editor open:

```sh
tools/run_game.sh            # add --windowed for a window instead of full screen
```

```powershell
.\tools\run_game.cmd --windowed
```

Windows accepts both spellings — `--windowed` and `-Windowed`, `--skip-build`
and `-SkipBuild` — so a command copied from either half of the documentation
works.

The bootstrap finds Godot under every name it normally installs under, including
`/Applications/Godot.app` and the official Windows archive's own filename. If
yours is somewhere else, name it:

```sh
GODOT=/Applications/Godot.app/Contents/MacOS/Godot tools/bootstrap.sh
```

```powershell
.\tools\bootstrap.cmd -Godot 'C:\path\to\Godot_v4.7.1-stable_win64_console.exe'
```

The remaining sections explain every step, including how to select the correct
checkout, what success looks like, and how to run level 02 without touching
code.

## 1. Choose the correct checkout

A Git worktree is another checkout of the same repository. During a feature
campaign, the newest editor work can be in a worktree while the ordinary
`main` checkout still contains the older project. Opening the wrong
`project.godot` therefore looks valid but silently hides the new scenes and
node types.

Open Terminal, change to any checkout of Unseeing, and list the worktrees:

```sh
git worktree list
```

Each row ends with its branch in square brackets. Change directory to the path
on the row for the work you intend to edit. While the editor-authoring campaign
has not been merged, that branch is `worktree-editor-authoring-campaign`. After
it is merged, use the normal `main` checkout.

For this in-flight campaign, the usual repository-relative checkout is
`.claude/worktrees/editor-authoring-campaign`. That is a temporary hidden
directory, not the durable `main` checkout. The absolute path printed by
`git worktree list` is authoritative if your host placed it elsewhere.

Verify the selected directory before continuing:

```sh
git branch --show-current
test -f game/project.godot && echo "Godot project found"
test -f game/scenes/level_02.tscn && echo "level 02 found"
```

Windows PowerShell:

```powershell
git branch --show-current
if (Test-Path game\project.godot) { 'Godot project found' }
if (Test-Path game\scenes\level_02.tscn) { 'level 02 found' }
```

Both `found` lines should print when using the completed editor-authoring
work. All commands below must run from this same directory.

## 2. Install the prerequisites

### Godot

The required editor version is recorded in `.godot-version`.

Install it however your platform prefers. Every tool here finds the editor
through one shared search (`tools/lib/engine.sh`, and its PowerShell twin in
`tools/bootstrap.ps1`), which knows the names Godot actually installs under:

- `godot`, `godot4`, `godot-4`, `godot-editor`, `Godot` on `PATH`
- Homebrew (`/opt/homebrew/bin`, `/usr/local/bin`), `/usr/bin`, `~/bin`
- `/Applications/Godot.app` and `~/Applications/Godot.app`
- Scoop, WinGet and `%LOCALAPPDATA%\Programs\Godot` on Windows
- the official archive under its own shipped filename —
  `Godot_v4.7.1-stable_linux.x86_64`, `Godot_v4.7.1-stable_win64_console.exe` —
  anywhere on `PATH`, beside the checkout, or in `godot-bin/`
- a repository-local `godot-bin/godot`

So on macOS `brew install godot`, on Windows `scoop install godot`, and on Linux
unzipping the official build somewhere on `PATH` all work without renaming
anything.

**The version must match `.godot-version` exactly** — another 4.x release is not
an equivalent substitute, because the extension ABI and editor behaviour are
version-sensitive. A Mono/.NET build of the pinned version *is* accepted: the pin
constrains the version, not the build flavour.

Search is version-aware, so a machine holding several editors gets the right
one: the first candidate that satisfies the pin wins, not the first that exists.

```sh
cat .godot-version           # what this checkout requires
godot --version              # what you installed, if it is on PATH
```

If your editor lives somewhere the search does not reach, name it — and it will
still be version-checked, never trusted blindly:

```sh
GODOT=/path/to/godot tools/bootstrap.sh
```

```powershell
.\tools\bootstrap.cmd -Godot C:\path\to\Godot_v4.7.1-stable_win64_console.exe
```

### Native build tools

Unseeing's framework is a Rust GDExtension. The bootstrap script installs the
pinned Rust toolchain when necessary, but Rust still needs a native C linker.

On macOS, install Apple's command-line tools if they are absent:

```sh
xcode-select --install
```

On Debian or Ubuntu Linux, the equivalent prerequisite is:

```sh
sudo apt install build-essential curl
```

The first bootstrap may use the network to install Rust and download Rust
dependencies. Node.js is not needed to open or run the game; it is needed only
for the optional MCP setup near the end of this guide.

### Windows native tools

Install Visual Studio 2022 Build Tools with **Desktop development with C++**,
the Windows SDK, and the C++ tools matching the Godot editor's x64 or ARM64
architecture (install both when you use editors of both architectures). The
bootstrap installs rustup itself when rustup is absent, detects whether the
Godot editor is x86_64 or ARM64, and builds the matching target automatically:

```powershell
.\tools\bootstrap.cmd
```

You do not need to choose a target; automatic selection is the supported
designer workflow. The same command validates the Godot pin, imports the
project, and must report the exact class count from `ci/engine_class_count`
before `bootstrap: OK`. If the MSVC
linker is missing, its failure names the Build Tools components to install.

## 3. Build the editor engine before opening Godot

Fully quit Godot if this project is already open. Then, from the selected
checkout, run the platform entry point:

```sh
tools/bootstrap.sh
```

```powershell
.\tools\bootstrap.cmd
```

The script performs four useful checks in one operation:

1. It installs or locates the repository-pinned Rust toolchain.
2. It builds the release GDExtension with Inspector documentation enabled.
3. It verifies the exact Godot version and imports the project.
4. It asks Godot's `ClassDB` to prove that all 19 engine classes registered.

The final output should include:

```text
probe: PASS (19 checks)
bootstrap: OK
```

Do not open the editor before this build on a fresh checkout. If Godot tries to
load a missing extension once, that editor process does not retry it after the
library appears. When this happens, quit every Godot window, rerun the
bootstrap, and launch a fresh editor process.

Run the bootstrap again after pulling Rust engine changes, creating a fresh
worktree, or seeing custom nodes load as `MissingNode`. Scene-only edits do not
need a Rust rebuild.

## 4. Open the project

### Recommended: open it from the terminal

This method cannot accidentally select the repository root or a different
worktree:

```sh
godot --editor --path "$PWD/game"
```

For the macOS application bundle, use:

```sh
/Applications/Godot.app/Contents/MacOS/Godot --editor --path "$PWD/game"
```

On Linux, if Godot is outside `PATH`, launch the same explicit binary supplied
to bootstrap:

```sh
GODOT=/path/to/godot tools/bootstrap.sh
/path/to/godot --editor --path "$PWD/game"
```

### Alternative: use Godot's Project Manager

1. Launch Godot without opening a project.
2. Select **Import** in the Project Manager.
3. Browse to the selected checkout and choose `game/project.godot`.
4. Select **Import & Edit**.

If the worktree is inside a hidden directory on macOS, press
**Command+Shift+G** in the file chooser and paste the worktree path, or press
**Command+Shift+.** in Finder to show hidden folders. Select `project.godot`,
not the repository directory and not an individual `.tscn` scene.

The first import can take a little longer while Godot fills its local
`.godot/` cache.

## 5. Confirm that the editor loaded correctly

The editor title should say **Unseeing**. The renderer named near the upper
right should be **Compatibility**; the project intentionally uses Godot's GL
Compatibility renderer because the same project also exports to the web.

In the **FileSystem** dock, confirm that these files exist:

- `scenes/level_01.tscn`
- `scenes/level_02.tscn`
- `scenes/props/chair.tscn`
- `scenes/props/table.tscn`
- `scenes/rooms/doorway_8m.tscn`
- `scenes/rooms/room_16x16.tscn`

Double-click `scenes/level_02.tscn`. Its root should be a `WaveLevel`, and its
children should include `WaveSpawn`, `SoundFan`, and `WaveRun` nodes. They must
not say `MissingNode`. The custom node icons, fan blueprint, generated wall
segments, chair, and room should be visible in the 3D viewport.

The four editor areas used most often are:

- **FileSystem**, usually at lower left: scene and prefab files.
- **Scene**, usually at upper left: nodes inside the open scene.
- **3D viewport**, in the centre: placement, movement, and rotation.
- **Inspector**, on the right: the selected node's editable properties.

## 6. Edit a level without code

Open a level scene, select a node in the Scene dock, and edit it with the 3D
gizmos or the Inspector. Save with **Command+S** on macOS or **Ctrl+S** on
Windows and Linux. Do not attach a GDScript to a level: the supported authoring
vocabulary is already exposed as typed Godot nodes.

### Move a chair visually

This is a complete first edit using the shipped chair in level 01:

1. Double-click `scenes/level_01.tscn` in FileSystem.
2. In the Scene dock, select the top-level **Chair** instance. Select the
   plain prefab root, not its `Seat`, `Leg`, or `Back` pieces; moving the root
   keeps the whole chair together.
3. Move the mouse over the 3D viewport and press **F** to centre the view on
   the selected chair.
4. Press **W** to enter Move mode. Drag the red arrow for X, the blue arrow
   for Z, or the red/blue plane handle to move across the floor. The green
   arrow is Y (height), so leave it alone when the chair should stay grounded.
5. For an exact placement, expand **Transform** in the Inspector and type the
   desired **Position** values. Godot uses X/Z for the floor and Y for height;
   a floor-standing chair keeps Y at `0`.
6. Save the level. **Command+Z** on macOS or **Ctrl+Z** on Windows/Linux undoes
   an unwanted move.

Only this chair instance moves; the reusable `chair.tscn` remains unchanged.
To inspect the text change from the same checkout, run:

```sh
git diff -- game/scenes/level_01.tscn
```

Godot may also normalize scene syntax or mint `unique_id` values when it
saves. Review the whole diff and keep the intended `Chair` position change;
do not treat unrelated serialized noise as part of the design edit.

### Change the 3D point of view

With the default Godot navigation scheme and the pointer over the 3D viewport:

- Press **F** to focus the selected node.
- Hold the **middle mouse button** and drag to orbit.
- Hold **Shift+middle mouse button** and drag to pan.
- Use the mouse wheel to zoom; **Ctrl+middle mouse button** and drag is the
  continuous zoom alternative.
- Hold the **right mouse button** for freelook. Move the mouse to look; use
  **W A S D** to fly, **E** to rise, and **Q** to descend. Release the button
  to leave freelook.
- Drag the orientation gizmo at the viewport's upper right to orbit, or click
  one of its coloured circles for an exact orthogonal side, top, or front
  view. **Keypad 5** toggles perspective and orthogonal projection.

These keys affect the editor camera only while the 3D viewport has focus; they
do not move the game character. For a trackpad-oriented scheme, open **Editor
Settings → Editors → 3D → Navigation** and select **Tablet/Trackpad**.

To add one of those nodes, select its intended parent in the Scene dock, select
the **+** button (**Add Child Node**), type the class name such as `WaveSpawn`
or `WaveRun`, select the matching result, and select **Create**. If a custom
class does not appear in that search, stop and repair the GDExtension rather
than substituting a similarly named built-in node.

The normal building blocks are:

- `WaveSpawn`: the player's start position and facing direction. Move it and
  rotate it around Y. The first typed `WaveSpawn` in scene order wins; duplicate
  spawns display warnings. A plain `Marker3D` merely named `SpawnPoint` is not
  a spawn.
- `WaveWall`: one short, solid wall. Change **Length** in the Inspector. It is
  an axis-aligned building block: a freehand rotation snaps live to the nearest
  quarter turn, and inherited scale is discarded so its drawn, struck,
  painted, and occluding shapes remain one object. `WaveWall` is the editable
  datum: use its exported **Length** and collision properties and connect its
  relayed collision signals. Its `WaveBody`, mesh, and collider are private
  generated limbs; never edit them. Make wall edits in the editor; runtime wall
  geometry is fixed after the scene enters the tree.
- `WaveRun`: a long wall or doorway described by endpoints and openings. It
  generates its `RunSeg1...N` wall children; do not edit those generated
  children directly. Make endpoint, opening, and transform edits in the editor;
  after runtime ready those authoring writes are ignored so the generated
  walls cannot drift from the level's retained paint and occlusion snapshot.
- `WaveProp`, `WaveColumn`, and `WaveWedge`: solid object pieces discovered by
  the level regardless of how deeply they are nested under a prefab root.
- `SoundFan` and `SoundRadio`: visible sound-source blueprints with designer
  knobs such as **Volume**, **Cadence**, and **Wave Speed**.
- `WaveCat`: the companion blueprint.

To place a reusable object, drag one of these scene files from FileSystem into
the 3D viewport or Scene dock:

- `scenes/props/chair.tscn`
- `scenes/props/table.tscn`
- `scenes/rooms/doorway_8m.tscn`
- `scenes/rooms/room_16x16.tscn`

Move or rotate the prefab's plain `Node3D` root. To change every instance of a
prefab, open the prefab scene itself and edit its typed children. The preview
limbs, `WaveBody`, and `RunSeg` nodes generated by Rust are intentionally not
saved into the scene; the engine rebuilds them.

### Create an inherited room variant

Use a Godot inherited scene when a room should keep its base layout while one
variant changes Inspector values or adds gameplay objects:

1. In FileSystem, right-click the base room scene and choose **New Inherited
   Scene**.
2. Save the inherited variant under `game/scenes/rooms/`.
3. Select an inherited `SoundFan` node and override an exported authored
   property such as **Volume** in the Inspector.
4. Add new typed authored children, such as `SoundRadio`, to the inherited
   root, place them, and save the variant.
5. Instance the variant in a `WaveLevel`. It may sit beneath any plain `Node3D`
   grouping root; moving or turning that group composes the authored room in
   world space.
6. Edit only nodes represented by the authored scenes: the room and grouping
   roots, typed gameplay nodes, and their exported properties. Never edit or
   take ownership of `RunSeg*`, `WaveBody`, `WaveSkin`, `WaveCollider`, source
   or cat blueprint limbs, `WaveFloor`, or `WaveCeiling`.
7. The measured duplication contract is
   `Node.duplicate(Node.DUPLICATE_USE_INSTANTIATION)`: after that programmatic
   operation, reload, or play, expect Rust to remove stale generated limbs and
   rebuild one ownerless set from the authored data. GUI **Ctrl+D** is not
   covered by this regression and is not claimed here.

The ownership boundary is deliberate. Every non-root authored node has a scene
owner and survives saving. Each authored scene root anchors its own scene
artifact and is saved as that artifact's root even though its `Node.owner` is
null. Generated nodes are live engine data with no scene owner, so Godot omits
them from the saved `SceneState`; seeing them in the viewport does not make
them authored content. `rust/src/nodes/level.rs::collect` owns the recursive
live discovery through plain groups, nested scene instances, and inherited
scenes. `WaveRun`, the wall/solid/source/cat builders, and
`WaveLevel::build_slabs` own their generated limbs. The executable regression
contract lives in `game/tests/scene_composition_test.gd` and
`game/tests/probe/editor_prefab_probe.gd`.

### Editing a WaveRun opening

`WaveRun.From` and `WaveRun.To` are `(X, Z)` coordinates in the parent's local
space. The dominant changing axis is the run's axis; X wins an exact tie. Keep
runs axis-aligned so a diagonal does not have to be folded with a warning.

Godot's generic `Vector2` editor labels those two boxes **x** and **y**. For a
WaveRun endpoint, read them as **parent-local X** and **parent-local Z**: the
displayed `y` box is horizontal Z, not elevation. WaveRun is planar and has no
endpoint height field.

Moving or rotating the `WaveRun` node with the viewport gizmo is also supported
while editing. The engine folds that node's planar transform into **From**,
**To**, and **Openings**, then resets the node transform to identity so there
is still one source of authored truth. A transform on an ancestor room prefab
remains ordinary composition. Y translation or tilt cannot be represented by
this planar vocabulary, so it is discarded with a warning. A running level is
different: ready has already derived paint, occlusion, and retained wall
handles, so post-ready endpoint, opening, and WaveRun-transform writes are
ignored and the ready-time generation remains exact.

Each element of **Openings** is a `Vector2` whose displayed fields mean:

- displayed **x**: absolute start coordinate on the run's selected axis;
- displayed **y**: opening width.

In the Inspector, expand **Openings**, increase its **Size** to add an element,
then expand that element and edit its x/y boxes. Reduce **Size** to remove
elements from the end.

For example, a run from `(0, 0)` to `(16, 0)` with opening `(6.5, 3)` leaves a
gap from X=6.5 through X=9.5. The first value is not an offset from `From`.
Although negative widths are safely treated as magnitudes, positive widths are
clearer for authored content.

### Read and clear warnings

A yellow warning triangle in the Scene dock is an authoring fault, not
decoration. Hover it to read the message. Typical causes include duplicate
spawns, overlapping or unfloored solids, an invalid run, a singular,
non-finite, or unrepresentably large wall ancestor, a non-finite wall-local
transform, an invalid wall **Length** or **Collision Priority**, or too many
mutually separated face/source-role classes for the five-label palette. Fix the
placement or Inspector value and give the editor a frame to re-evaluate; the
warning should clear by itself.

## 7. Run the game or a selected level

Outside the editor, `tools/run_game.sh` (`.\tools\run_game.cmd` on Windows)
builds the engine and plays the world in one command:

```sh
tools/run_game.sh                                  # full screen, as it ships
tools/run_game.sh --windowed                       # 1280x720 window
tools/run_game.sh --windowed 1920x1080 --seed 1    # a reproducible world
tools/run_game.sh --skip-build --scene res://scenes/level_02.tscn
```

It never opens the editor. `--windowed` works by writing `game/override.cfg`,
because Godot's own window flags lose to the project setting; the file is
removed however the run ends, and a run refuses to start if one already exists.

Inside the editor there are two different commands, and the distinction matters.

### Run the shipped default with F5

Press **F5** or select **Run Project**. Godot runs `scenes/main.tscn`, whose
root is `UnseeingGame`. When its **Level Scene** property is empty, the engine
uses the exact `level_01.tscn` fallback.

### Run level 02, or another authored level, with F6

A raw `WaveLevel` is content, not the full game. Pressing F6 while
`level_02.tscn` itself is open runs only that content root: there is no player,
hearing pass, wave pool, or material injection. It is useful for editor layout,
but it is not a playable preview.

Create a code-free runner scene instead:

1. In FileSystem, right-click `scenes/main.tscn` and choose **Duplicate**.
2. Name the copy something clear, such as `play_level_02.tscn`.
3. Double-click the copy and select its `Main` (`UnseeingGame`) root.
4. In the Inspector, find **Level Scene**.
5. Drag `scenes/level_02.tscn` from FileSystem onto that resource field.
6. Save the runner scene.
7. With the runner tab active, press **F6** or select **Run Current Scene**.

The `UnseeingGame` root now injects the hidden framework before adding the
selected `WaveLevel`, exactly as it does for the shipped game. The runner
contains only a Godot resource selection; no script or Rust change is needed.
You can make one runner per level or reuse one by changing **Level Scene**.

If you deliberately want F5 to open a different level for everyone, set the
same **Level Scene** property in `main.tscn` and save it. Leaving it empty
always restores the level-01 fallback.

On compact keyboards, the function keys may require **Fn+F5**, **Fn+F6**, or
**Fn+F8**. The desktop game starts fullscreen by design. Press **Esc** to
release the mouse and **F8** to stop the running scene.

Once running:

- **W A S D** moves.
- The mouse looks around.
- A click taps the cane.
- **Esc** releases the mouse.

The world begins black because the hero is blind. Movement, footsteps, sound
sources, and cane taps reveal outlines; a black first frame is not by itself a
load failure.

## 8. Create a new level or prefab

To create a new level entirely in the editor:

1. Select **Scene → New Scene**.
2. Select **Other Node**, search for `WaveLevel`, select it, and select
   **Create**. If it is absent, rerun the bootstrap instead of using `Node3D`.
3. Save the scene under the project's `scenes/` folder with a descriptive
   `.tscn` name.
4. Select the `WaveLevel` root, select **+**, add a `WaveSpawn`, and place and
   rotate it where the player should begin.
5. Add typed nodes with **+**, and drag reusable prop or room scenes from the
   FileSystem dock into the level.
6. Save, create or update an `UnseeingGame` runner, and press F6 from its tab.

Every playable level needs a `WaveSpawn`. The root's **Extents** describes its
planar size, but the placed typed nodes are what define its actual walls,
solids, sources, and spawn.

For a new reusable prop or room:

1. Select **Scene → New Scene**, choose **3D Scene** to create a plain `Node3D`
   root, and rename it for the object.
2. Add typed solid children such as `WaveProp`, `WaveColumn`, `WaveWedge`,
   `WaveWall`, or `WaveRun` below it.
3. Save it under `game/scenes/props/` or `game/scenes/rooms/`.
4. Drag instances of that scene into levels and transform the plain root.

This boundary is deliberate. Designers compose scenes, transforms, prefabs,
and Inspector values in Godot. Rust owns rendering, physics, wave propagation,
material injection, superface and semantic-role label derivation, and
validation. Within Rust, `rust/src/render/paint_plan.rs` makes the complete
paint decision atomically, while `rust/src/render/paint.rs` is only the Godot
`ArrayMesh` submission/layout boundary. The planner derives solid touch bounds
from `render::faces::Shape`, validates grown source sweeps, and rejects requests
above its named entry/source/palette/role ceilings before quadratic graph work.
GDScript in this repository is reserved for automated
tests and editor probes, not level content.

## 9. Optional: connect an assistant through Godot MCP

MCP is a developer convenience, not a requirement for designers or builds. It
lets one connected assistant drive a running editor, inject input, step time,
and read structured game state.

Install Node.js 20 or newer, then run from the repository root:

```sh
tools/setup-mcp.sh
```

If Homebrew installed an unlinked `node@22` keg on macOS, this one-command form
provides it to the installer without changing the rest of the shell setup:

```sh
PATH="/opt/homebrew/opt/node@22/bin:$PATH" tools/setup-mcp.sh
```

Keep the Godot editor open, then select **Project → Project Settings → Plugins
→ Godot MCP → Enable**. Restart or reconnect the MCP client after enabling the
plugin. Only one MCP client can control the editor at a time.

The addon lives under the ignored `game/addons/godot_mcp/` directory and must
never be committed or shipped. Enabling it can produce a local
`game/project.godot` change; do not include the Godot MCP plugin entry in a
commit. The complete debugging loop is documented in
[`superpowers/mcp/godot-mcp-loop.md`](superpowers/mcp/godot-mcp-loop.md).

## Troubleshooting

### The custom nodes say MissingNode

The release GDExtension was absent or failed to load. Quit every Godot process,
run the platform bootstrap again, confirm it ends with `bootstrap: OK`, and
reopen the editor. Merely closing and reopening the scene tab is not enough.

### Bootstrap says the Godot version is wrong

Install the exact version printed from `.godot-version`, or point the command
at a matching executable:

```sh
GODOT=/path/to/matching/godot tools/bootstrap.sh
```

On Windows:

```powershell
.\tools\bootstrap.cmd -Godot 'C:\path\to\matching\Godot_console.exe'
```

Do not bypass the version check with a nearby Godot release. A Mono/.NET build
of the *pinned* version is fine — the pin constrains the version, not the build
flavour — and the refusal prints the version it actually found, so you can tell
the two cases apart. If a machine holds several editors, the search takes the
one that satisfies the pin rather than the first it happens to meet.

### Bootstrap says an editor reported no version

A GUI-subsystem Godot has no console to answer on. Point the command at its
`_console.exe` sibling, which ships in the same archive.

### Bootstrap cannot find a linker

On macOS run `xcode-select --install`. On Linux install the distribution's C
build toolchain, commonly `build-essential`, then rerun the bootstrap.

### Level 02 or the prefab folders are missing

The editor opened an older checkout. Quit Godot, repeat `git worktree list`,
change to the intended worktree, and open that worktree's `game/project.godot`.

### F6 reports that materials or pulses were not injected

The active tab is a raw `WaveLevel`. This is an expected refusal from the
content layer. Open or create an `UnseeingGame` runner, assign the level to its
**Level Scene** property, and press F6 from the runner tab.

### F5 always opens level 01

F5 runs `main.tscn`, not whichever level tab is visible. Its empty **Level
Scene** picker intentionally means level 01. Use the configured runner and F6,
or deliberately assign the desired scene in `main.tscn`.

### The game window appears entirely black

Tap the cane, walk, or wait near a sound source. Black is the game's resting
visual state. If there is also an injection error in Godot's Output panel,
check that the running scene is an `UnseeingGame` runner rather than a raw
level.

### Godot MCP does not appear in the Plugins list

Run `tools/setup-mcp.sh` with Node.js 20+, confirm that
`game/addons/godot_mcp/plugin.cfg` exists locally, and reopen Project Settings.
The addon is intentionally not part of Git.

## Reopening the project later

For ordinary scene-authoring sessions, return to the same checkout and run:

```sh
godot --editor --path "$PWD/game"
```

On Windows, use `godot --editor --path "$PWD\game"` instead.

Bootstrap again only after a fresh checkout, a Rust framework change, a Godot
version change, or a missing-class failure. Everything else in the daily level
design loop stays inside the Godot editor.
