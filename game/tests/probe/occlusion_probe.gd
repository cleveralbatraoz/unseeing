extends Node
## Regression probe for the wave-through-wall law, reproducing the report:
## walk UP TO the divider the fan is behind and tap it. Boots main and
## checks:
##   1. SOURCE REVEAL (hearing pass OFF): what the fan's voice ADDS to the
##      divider's far face must be nothing. A DELTA across the fan's own
##      `volume` rather than an absolute darkness reading, because the fan
##      is a SWEPT beam and data_core gates on the cone BEFORE it consults
##      the wall table — an absolute check reads dark for a stretch of
##      every oscillation whatever transmission the wall law grants. The
##      delta also cancels every other emitter in the room;
##   2. THE SAME VOICE, THE SAME WALL, THE OTHER SIDE (hearing pass OFF):
##      the divider's east face, 2 m in front of the hub with nothing
##      between. It must be LIT. Without it every check here is one-sided —
##      each asserts a reading stays dark, which is satisfied just as well
##      by an occluder that swallowed the level whole, or by a fan that
##      stopped humming, as by a correct barrier;
##   3. THE SHELL (hearing pass ON): the same delta at case 1's points with
##      the quad up. Every other reading runs with it hidden and so cannot
##      see a ring at all; this is the only check anywhere that measures
##      whether a source's shell crosses a wall in the air. Proven to fail
##      against the pre-fix law (0.043 against a 0.02 floor);
##   4. TAP CONTROL (hearing pass OFF): the hero's own strike must light
##      the face it lands on — the second two-sided check, and the one
##      that catches a reveal gate inverted wholesale;
##   5. FINAL IMAGE (hearing pass ON): tapping the wall must not brighten
##      the fan behind it — not its shell wash, not its OUTLINE borrowing
##      the lit wall. Sampled on the fan's OUTLINE (guard ring), where the
##      flare showed;
##   6. REVEAL (hearing pass OFF): the same tap — its direct pulse AND its
##      echoes — must not reach the fan's own data reveal. Sampled on the
##      fan's SOLID interior only; the outline points sit at the fan/wall
##      boundary and would read the wall the tap rightly lights.
##
##      Cases 5 and 6 are RATIOS of the fan's own standing image, not
##      absolute deltas. As absolutes they silently became unfailable the
##      moment the wall muffle turned into a multiplier over the whole
##      acoustic image: every reading on the fan shrank 3.3x while the
##      floors did not, and a full-strength leak measured 0.065 against a
##      0.08 floor. A ratio is scale-free and cannot rot that way;
##   7. THE MUFFLE ARRIVES (hearing pass OFF): the fan's own body, read as
##      an ABSOLUTE through one wall, must land in a hand-derived window.
##      Every other case here is a delta or a ratio and so is blind to a
##      factor common to both halves — including the whole standing image
##      going missing. This one catches an instance uniform that never
##      reached the GPU, which no unit test can: the suites and the
##      observer read those uniforms back by the same names that would
##      have been renamed.
##
## Windowed, real GPU; run by tools/probe_visibility.sh, not in headless.
##
## THREE WAYS THIS PROBE CAN LIE, all three met by hand:
##   - `_peak_r` CLAMPS an off-screen sample to the black image border, so a
##     mis-aimed check passes while measuring nothing. The camera aim is
##     therefore re-applied every frame (`_aim`), because the player rewrites
##     its own camera rotation each tick and a single `look_at` drifts off
##     target over a long window;
##   - a reading of a SWEPT source must outlast its sweep, or the same
##     correct build measures 0.392, 0.000 and -0.020 on consecutive runs.
##     The window is therefore a DURATION on the simulated clock
##     (`SWEEP_SECONDS`), never a frame count: the probe reads the whole
##     framebuffer back every frame, so its frame rate — and with it the
##     slice of the 11.42 s sweep any fixed frame count covers — depends on
##     the machine;
##   - a delta is only honest if the world does not change between its two
##     halves, so the hero never MOVES inside one (a teleport emits a
##     footstep) and the level's creatures are silenced for the run.
const MAIN := preload("res://scenes/main.tscn")
## The hero right at the divider, looking at the fan behind it.
const AT_WALL := Vector3(5.8, 0.9, 4.0)
const FAN := Vector3(8.6, 1.15, 4.4)
## The fan's OUTLINE — the guard ring is where the borrowed-outline flare
## showed; used for the final-image check (hearing on).
const FAN_EDGE: Array[Vector3] = [
	Vector3(8.6, 1.58, 4.35),  # guard ring top
	Vector3(8.6, 0.72, 4.35),  # guard ring bottom
	Vector3(8.6, 1.15, 4.4),  # motor hub
	Vector3(8.6, 0.6, 4.4),  # pole
]
## The fan's SOLID motor box — reading this in the DATA pass gives the
## fan's own reveal, never the divider around it (the thin pole/base would
## let the pixel tolerance spill onto the lit wall); used for reveal.
const FAN_CORE: Array[Vector3] = [
	Vector3(8.6, 1.15, 4.4),  # motor hub — a solid box, tolerance stays on it
]
const WALL_TAP := Vector3(6.25, 1.5, 4.06)  # the aimed strike on the divider face
## The struck face itself, read as the positive control. The strike point
## is its own brightest spot, and it is the one surface whose lighting the
## hero's tap OWNS under the shipped law: the fan is a wall away from it.
const WALL_FACE: Array[Vector3] = [WALL_TAP]

## The hero at the spawn marker, in the room WEST of the Divider (x = 6.4,
## whose doorway spans z in [8, 12.4]). The fan at (8.6, 4.4) is east of
## it, and the fan's sight line to every point below crosses SOLID divider
## well clear of that doorway. Nothing the fan emits may light any of them.
const AT_SPAWN := Vector3(3.0, 0.9, 4.0)
## What the hero looks at from the spawn: the divider face, 2.4 m along +z
## from the tap point, so the later strike cannot be mistaken for a leak.
const SPAWN_AIM := Vector3(6.25, 1.0, 6.5)
## Spawn-room surfaces the fan must never reveal. Both are chosen so the
## fan-to-point line pierces the divider: (8.6, 4.4) -> (5.0, 6.0) crosses
## x = 6.4 at z ~= 5.38, inside the divider's z in [0.6, 8] run.
const SPAWN_SIDE: Array[Vector3] = [
	Vector3(6.25, 1.5, 6.5),  # the divider's WEST face, fan directly behind it
	Vector3(5.0, 0.0, 6.0),  # spawn-room floor
]
## The POSITIVE half of the same law, on THE SAME WALL: the divider's EAST
## face, which the fan stands 2.08 m in front of and directly faces. Zero
## crossings from the hub (`explain_ray` reports `wave_transmission` 1.00
## against 0.00 for both SPAWN_SIDE points), and inside the fan's swept wash
## for 91.8% of its oscillation, so the same voice that must light NOTHING
## on the hero's side must light THIS brightly.
##
## Without it every check here is one-sided — each asserts a reading stays
## BELOW a floor, which an occluder that swallowed the level whole satisfies
## perfectly, and so does a fan that simply stopped humming. It is what
## proves the toggle these deltas are built on does anything at all.
##
## x = 6.55 sits 0.02 m OUTSIDE the divider's occluder rect (x ∈ [6.27,
## 6.53] after RECT_SHRINK), exactly as WALL_TAP does on the far side, so
## the hub's line to it never enters the rect.
##
## NOT a doorway: the fan cannot reach the spawn room through one. Its room
## is closed by the Divider to the west AND `FanRoomSouth` at z = 8 running
## x ∈ [6.4, 14], and the divider's opening (z ∈ [8, 12.4]) lies on the far
## side of that second wall. A doorway control was tried at (5.5, 0, 11.0)
## and `explain_ray` named `FanRoomSouth` as the wall it crossed.
const FAN_SIDE: Array[Vector3] = [Vector3(6.55, 1.5, 4.4)]
## Where the hero stands to read FAN_SIDE — inside the fan's own room,
## offset in z so the fan's own body is not between the eye and the face.
const IN_FAN_ROOM := Vector3(10.5, 0.9, 6.5)

## Frames a reading is a running MAX over. The default is enough for a
## surface whose lighting does not come and go.
const WINDOW := 26
## ...but a reading of the FAN'S OWN wash must outlast its sweep, and this
## is the trap that cost three runs to find. `pivot_angle` is
## `sin(t * PIVOT_SPEED) * PIVOT_RANGE`, so the head's oscillation has a
## period of 2*PI / 0.55 = 11.42 s, and the beam leaves any given point for
## a stretch of it — for FAN_SIDE, |theta| > 0.78 rad, which lasts about
## 1.5 s and comes round twice a cycle. A 26-frame window is roughly 1.8 s
## of wall clock here, so it can land entirely inside one of those
## stretches: the same correct build measured 0.392, then 0.000, then
## -0.020 on consecutive runs. Reading over a window longer than the whole
## period makes the max independent of which phase the run started in.
##
## Do NOT "fix" this with --fixed-fps instead; see tools/probe_visibility.sh
## for why that breaks the tap cases.
## One full sweep of the fan's head, in SIMULATED seconds, plus margin.
##
## The head oscillates as sin(t * PIVOT_SPEED) with PIVOT_SPEED = 0.55
## (rust/src/fan_wave.rs), so its period is 2*PI/0.55 = 11.42 s. Any window
## shorter than that can land wholly in a phase where the beam points
## somewhere else, and a peak taken over it reads nothing through no fault
## of the law under test.
##
## It is a DURATION and not a frame count, and that is the fix rather than a
## detail. This was 200 frames, which is 3.3 s at 60 fps — 29% of a cycle —
## and the probe's own frame rate is set by the full-framebuffer readback
## `_peak_r` performs every single frame, so the same 200 frames is a
## different slice of the sweep on every machine and every driver. Measured
## consequence: check 4, a POSITIVE control, read 0.000 on one boot and
## 0.322 and 0.310 on the next two.
const SWEEP_SECONDS := 12.0

## Frames a windowed read may burn before it gives up waiting for the
## simulated clock. A bound, not a budget: without it a stalled clock would
## hang the probe instead of failing it.
const WINDOW_FRAME_LIMIT := 4000

var _checks := 0
var _failed := 0
## Where the camera is currently supposed to be looking. Re-applied EVERY
## frame a reading is taken, because the player owns its camera and rewrites
## that rotation each tick — a `look_at` issued once drifts off target over
## a long window. With a 26-frame read the drift was small enough to hide;
## at 200 frames the sample point left the frame entirely and `_peak_r`
## clamped it to the black border, which reads as a silent PASS on a
## darkness check and a silent FAIL on a positive one. Both were observed.
var _aim := Vector3.ZERO
## The pose `_look` last placed the hero at, re-applied every sampled frame
## alongside the aim. The hero is a CharacterBody3D running its own physics:
## it settles, slides and drifts, which a short window never noticed and a
## window long enough to outlast the fan's 11.42 s sweep certainly does.
## Measured, holding only the aim: a 12 s window left the tap control reading
## 0.000 and the fan's own body reading 0.000 while an outline point
## saturated at 1.000 — the camera was no longer where the check believed.
var _pose := Vector3.ZERO


func _ready() -> void:
	# keep the window on top and foregrounded — an occluded window is
	# throttled by the OS to a frame a second, and the per-frame GPU
	# readback below then starves; on top it renders at full rate.
	DisplayServer.window_set_flag(DisplayServer.WINDOW_FLAG_ALWAYS_ON_TOP, true)
	DisplayServer.window_move_to_foreground()
	var main: UnseeingGame = MAIN.instantiate() as UnseeingGame
	add_child(main)
	await _settle(35)
	Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	main.hero.visible = false
	# The level's cat lives in the spawn room and speaks: kind-2 paw pulses
	# (PAW_RANGE 1.3 m) and a 1.6 s presence beat, from inside its 3.6x4.4
	# roam at (2.8, 7.6) — close enough to reach the floor sample below. It
	# is a wave this probe did not queue, so it goes, the way the hero's own
	# body does. Everything after this point is the sources and the probe.
	_silence_the_cat(main)
	_look(main, AT_SPAWN, SPAWN_AIM)
	await _settle(30)
	_set_quad(main, false)
	# 1 — SOURCE REVEAL, as a DELTA ACROSS THE FAN'S OWN VOICE rather than
	# an absolute darkness reading, and each word of that is load-bearing.
	#
	# The fan is a SWEPT beam: data_core gates on pulse_cone BEFORE it ever
	# consults the wall table, so both sample points fall outside the wash
	# for part of every 11.42 s oscillation. An absolute `leak < floor`
	# therefore reads dark for a stretch of every cycle no matter WHAT
	# transmission the wall law grants, and nothing pins which stretch the
	# 26-frame window lands in. Silencing the fan and measuring what its
	# voice ADDS attributes the reading to the fan itself, and cannot be
	# satisfied by a phase that happened to be dark — the baseline was dark
	# for that reason too.
	#
	# It also cancels every other emitter by construction: the radio, the
	# hero, anything a future level adds to this room appears in both
	# readings and subtracts out.
	# Both halves are read on ONE WALL, from its two sides, across one voice
	# change: the divider's west face must not brighten when the fan speaks,
	# and its east face — which the fan stands 2 m in front of — must.
	#
	# NEVER MOVE THE HERO BETWEEN A BASELINE AND ITS MEASUREMENT. Teleporting
	# the body emits a footstep, and a footstep lights the room it lands in:
	# reading the fan-room baseline right after the walk in measured 0.169 on
	# a wall the silenced fan could not have touched, which then subtracted a
	# real 0.102 down to a NEGATIVE delta. Each pose therefore takes its own
	# silence/restore pair, and the settle after the walk is what buries the
	# footstep before the baseline is read.
	var fan: SoundFan = main.level.get_node("Fan") as SoundFan
	var voice: float = fan.volume
	var leak := await _voice_delta(main, fan, voice, AT_SPAWN, SPAWN_AIM, SPAWN_SIDE, SWEEP_SECONDS)
	var lit := await _voice_delta(
		main, fan, voice, IN_FAN_ROOM, FAN_SIDE[0], FAN_SIDE, SWEEP_SECONDS
	)
	print(
		(
			"# occlusion @spawn: fan lifts the divider's far face %.3f ; its own face %.3f"
			% [leak, lit]
		)
	)
	_check(
		"the fan does NOT reveal the spawn room through the divider (%.3f < 0.02)" % leak,
		leak < 0.02
	)
	# ...and the same voice DOES light the same wall's other side. This is
	# the check that fails if the wall law ever swallows the level whole, or
	# if the fan simply stopped speaking — both of which every darkness
	# assertion here would report as success.
	_check("the same voice DOES light the divider's own face (%.3f > 0.05)" % lit, lit > 0.05)
	_set_quad(main, true)
	# 1b — THE SHELL, the same law asked of the ring in the air rather than
	# the surfaces it lights, and the half nothing had ever measured: every
	# reading above runs with the hearing quad HIDDEN, so none of them can
	# see a shell at all. Hearing pass ON, same delta across the same voice,
	# same sample points — with the quad up, a ring crossing the divider
	# into the hero's air adds its brightness at exactly these pixels.
	var shell := await _voice_delta(
		main, fan, voice, AT_SPAWN, SPAWN_AIM, SPAWN_SIDE, SWEEP_SECONDS
	)
	# ...and its positive half, because the NEW failure mode of a per-fragment
	# source-keyed cut is OVER-blocking, and a darkness delta cannot see that.
	# `float env = 0.0;` in hearing_post deletes every ring in the game while
	# every text pin still matches and this case still reads 0 - 0 = 0 and
	# prints "ok". The plan named this risk directly — "if the probe shows a
	# source's shell wrongly vanishing inside the hero's own room" — and
	# without the reading below the probe could never produce that evidence.
	var shell_lit := await _voice_delta(
		main, fan, voice, IN_FAN_ROOM, FAN_SIDE[0], FAN_SIDE, SWEEP_SECONDS
	)
	print(
		(
			"# occlusion @spawn: fan's SHELL lifts the walled image %.3f ; its own room %.3f"
			% [shell, shell_lit]
		)
	)
	_check(
		"the fan's ring does NOT cross the divider into the hero's air (%.3f < 0.02)" % shell,
		shell < 0.02
	)
	_check(
		"the fan's ring IS drawn inside its own room (%.3f > 0.05)" % shell_lit, shell_lit > 0.05
	)
	_look(main, AT_WALL, FAN)
	await _settle(20)
	# 2 — POSITIVE CONTROL, and it must be a DELTA. Every other check here
	# is one-sided: each asserts a reading stays BELOW a darkness floor,
	# which holds trivially once everything legitimate has gone dark too.
	# So a reveal gate inverted wholesale — data_core.gdshaderinc's
	# `wall_crossings_from(src, world) == 0` flipped to `!= 0`, lighting
	# only what sits BEHIND a wall — passed every one of them.
	#
	# An ABSOLUTE brightness reading here does not catch it either, and the
	# measurement says why: the tap is queued AT the strike point, so the
	# segment fed to the wall test is near-degenerate (from == to), and
	# WALL_TAP (x = 6.25) sits 0.02 m OUTSIDE the divider's occluder rect —
	# shrunk by RECT_SHRINK (rust/src/sight.rs) from the wall's real
	# half-thickness — so it reads ZERO crossings to its own face and an
	# inverted gate gives it nothing — but the FAN reaches the same face
	# through one wall, which an inverted gate rewards with full
	# brightness, and reveal is a MAX over live pulses. The face stays lit
	# by the wrong source (0.675 measured).
	#
	# The DIFFERENCE the tap makes is what separates them. Baseline first,
	# before any player sound exists: under the shipped law the fan cannot
	# light this face at all, so it is dark until the tap lands and the
	# delta is the whole reading. Under an inverted gate the fan holds the
	# face at a steady glow before AND after, while the tap adds nothing,
	# so the delta collapses to zero.
	_set_quad(main, false)
	var wall_base := await _peak_r(main, WALL_FACE, 12)
	main.player.queue_wave(0, WALL_TAP, 6.0, 5.5, 1.0, 6, Vector3(-1, 0, 0))
	var wall_lit := await _peak_r(main, WALL_FACE, 26) - wall_base
	print("# occlusion @wall: tap lights its own struck face by %.3f" % wall_lit)
	_check(
		"the tap DOES light the wall's own face at the strike point (%.3f > 0.15)" % wall_lit,
		wall_lit > 0.15
	)
	_set_quad(main, true)
	await _settle(40)
	# 3 — FINAL IMAGE: tapping the wall must not FLARE the fan's outline
	# (the borrowed-outline + shell-wash bug the reporter saw)
	var base_img := await _peak_r(main, FAN_EDGE, 12)
	main.player.queue_wave(0, WALL_TAP, 6.0, 5.5, 1.0, 6, Vector3(-1, 0, 0))
	var flare := await _peak_r(main, FAN_EDGE, 26) - base_img
	await _settle(40)
	# 4 — REVEAL: the same tap (its direct pulse AND its echoes) must not
	# light the fan's own data reveal — read on the fan's solid interior,
	# hearing quad out of the way
	_set_quad(main, false)
	var base_r := await _peak_r(main, FAN_CORE, 12)
	main.player.queue_wave(0, WALL_TAP, 6.0, 5.5, 1.0, 6, Vector3(-1, 0, 0))
	var reveal := await _peak_r(main, FAN_CORE, 26) - base_r
	# BOTH leak readings are expressed as a FRACTION of the fan's own
	# standing image at this pose, never as an absolute delta, and the
	# reason is a bug this probe already shipped once.
	#
	# The fan here is one wall from the eye, so every pixel of it is
	# multiplied by its wall muffle (SOURCE_THROUGH = 0.3 per crossing,
	# rust/src/level_plan.rs). When the muffle became a multiplier over the
	# whole acoustic image rather than a floor under it, every reading on
	# these points shrank by 3.3x while the floors 0.12 and 0.08 stayed
	# where they were — and a full-strength leak, the exact bug this probe
	# exists to catch, then measured 0.867 * (0.30 - 0.225) = 0.065, under
	# BOTH floors. The checks could no longer fail.
	#
	# A ratio cannot rot that way: it asks "how much did the tap add,
	# against how bright this fan already was", which is scale-free and
	# survives any future change to how the image is composed. Hand-derived
	# floor: a leak at full wave strength lifts max(wave, volume) from
	# volume (0.75) to 1.0, a ratio of 0.333, so 0.10 catches it with 3.3x
	# margin while sitting far above the measured noise (0.00-0.02).
	var flare_ratio := flare / maxf(base_img, 0.001)
	var reveal_ratio := reveal / maxf(base_r, 0.001)
	print(
		(
			"# occlusion @wall: tap flares outline %.3f of %.3f ; lifts fan reveal %.3f of %.3f"
			% [flare, base_img, reveal, base_r]
		)
	)
	# non-vacuity for both ratios: a fan that had gone dark would divide a
	# nothing by a nothing and pass every leak check ever written
	_check(
		"the fan IS drawn at all, so the leak ratios mean something (%.3f > 0.05)" % base_r,
		base_r > 0.05
	)
	_check(
		(
			"tapping the wall does NOT flare the fan behind it (%.3f of its own image < 0.10)"
			% flare_ratio
		),
		flare_ratio < 0.10
	)
	_check(
		(
			"the tap's reveal (with echoes) does NOT reach the fan (%.3f of its own image < 0.10)"
			% reveal_ratio
		),
		reveal_ratio < 0.10
	)

	# 7 — THE MUFFLE REACHES THE GPU AT ALL, read as an absolute.
	#
	# Everything above is a delta or a ratio, and none of them can see a
	# source's standing image vanish or double: a factor common to both
	# halves cancels. This one reads the fan's own body as an absolute
	# number and holds it to a hand-derived window.
	#
	# Derivation. The eye stands one wall from the hub (asserted, not
	# assumed), so the muffle is SOURCE_THROUGH^1 = 0.3
	# (rust/src/level_plan.rs). FAN_CORE is the motor housing, which sits
	# BEHIND the hub and so outside the fan's own forward cone — its wave
	# term stays under the standing volume, leaving
	# 0.3 * 0.75 = 0.225. pack_data then applies
	# (0.9 + 0.1 * flicker) * exp(-vd * 0.05) with vd = 2.86 m: between
	# 0.78 and 0.87. So the shipped reading is 0.176 to 0.195, and the
	# window below is [0.13, 0.30].
	#
	# What it catches, and it is the failure mode nothing else could see:
	# if u_source_muffle never reaches the limbs — a renamed constant, a
	# push that stopped happening — the shader falls back to its declared
	# default of 1.0 and this reads 0.75 * 0.867 = 0.65. If u_source_volume
	# is the one that goes missing, its default is 0.0 and this reads
	# nothing. Both are outside the window; both leave every unit test in
	# the tree agreeing with itself, because the suites and the observer
	# read the uniforms back through the SAME constants that would have
	# been renamed.
	#
	# What it deliberately does NOT claim: this cannot tell
	# muffle * max(wave, volume) from max(wave, volume * muffle). Those two
	# differ only where a source's own wave washes its own body, and on the
	# shipped fan that is the guard ring and the blades — a 5-pixel torus
	# and a set of paddles that spin out from under any fixed world point,
	# both sitting on the fan/wall silhouette where _peak_r's tolerance
	# reads the wall instead (measured: 0.753, the wall's own unmuffled
	# reveal, not the fan's). The composition law is held by
	# rust/src/render/reveal.rs's cargo tests; this holds its delivery.
	var walls_to_fan: int = (
		main.observer.explain_ray(main.player.camera.global_position, FAN)["camera_crossings"]
	)
	print(
		(
			"# occlusion @wall: the fan's own body reads %.3f through %d wall(s)"
			% [base_r, walls_to_fan]
		)
	)
	_check(
		"the fan really is ONE wall from the eye here (%d == 1)" % walls_to_fan, walls_to_fan == 1
	)
	_check(
		"the fan's image arrives MUFFLED, not whole (0.13 < %.3f < 0.30)" % base_r,
		base_r > 0.13 and base_r < 0.30
	)
	_report()


## Peak brightness over `pts` (each with a small pixel tolerance, to catch
## a thin outline) across `frames` frames.
func _peak_r(main: UnseeingGame, pts: Array[Vector3], frames: int) -> float:
	var cam := main.player.camera
	var view := cam.get_viewport().get_visible_rect().size
	var peak := 0.0
	for _i: int in frames:
		await get_tree().process_frame
		# hold the POSE and the aim: the player rewrites its camera rotation
		# every tick and its body keeps running physics, so without both the
		# sample point walks out of the frame mid-window — and the longer the
		# window, the further it walks
		if _aim != Vector3.ZERO:
			main.player.position = _pose
			main.player.velocity = Vector3.ZERO
			cam.look_at(_aim, Vector3.UP)
		await RenderingServer.frame_post_draw
		var img := get_viewport().get_texture().get_image()
		for pt: Vector3 in pts:
			var px := cam.unproject_position(pt)
			var cx := roundi(px.x * img.get_width() / view.x)
			var cy := roundi(px.y * img.get_height() / view.y)
			for dy: int in range(-2, 3):
				for dx: int in range(-2, 3):
					var x := clampi(cx + dx, 0, img.get_width() - 1)
					var y := clampi(cy + dy, 0, img.get_height() - 1)
					peak = maxf(peak, img.get_pixel(x, y).r)
	return peak


## Peak brightness over `pts` across at least `seconds` of SIMULATED time —
## the same clock the fan's sweep and every wave in the pool run on, so a
## window stated here means the same slice of the world's motion whatever
## rate the probe happens to render at.
func _peak_r_over(main: UnseeingGame, pts: Array[Vector3], seconds: float) -> float:
	var until: float = main.now + seconds
	var peak := 0.0
	var frames := 0
	while main.now < until and frames < WINDOW_FRAME_LIMIT:
		frames += 1
		peak = maxf(peak, await _peak_r(main, pts, 1))
	if frames >= WINDOW_FRAME_LIMIT:
		push_warning("probe: windowed read hit its frame bound before the clock advanced")
	return peak


## How much of `pts`' brightness belongs to THIS source, read from one
## fixed pose: stand the hero, let the walk-in's own footstep die, silence
## the source, read, give the voice back, read again. The difference is the
## source's contribution and nothing else's — every other emitter in the
## world, and every phase of a swept beam that happened to be dark, appears
## in both halves and cancels.
##
## The pose is set ONCE, before the baseline, and never touched again until
## the pair is complete. That ordering is the whole point: see the note at
## the call site.
func _voice_delta(
	main: UnseeingGame,
	fan: SoundFan,
	voice: float,
	where: Vector3,
	at: Vector3,
	pts: Array[Vector3],
	seconds: float
) -> float:
	_look(main, where, at)
	fan.volume = 0.0
	# outlast fade_tail(SOURCE_KIND) = 2 s of live hum AND the footstep the
	# walk-in just made
	await _settle(150)
	var muted := await _peak_r_over(main, pts, seconds)
	fan.volume = voice
	await _settle(150)
	var voiced := await _peak_r_over(main, pts, seconds)
	return voiced - muted


## Stand the hero at `where` and aim the camera at `at`. Synchronous on
## purpose: the caller awaits its own settle, so a forgotten `await` here
## cannot leave the pose half-applied while a readback is already running.
func _look(main: UnseeingGame, where: Vector3, at: Vector3) -> void:
	main.player.position = where
	_pose = where
	_aim = at
	main.player.camera.look_at(at, Vector3.UP)


## Silence every creature in the level. They emit waves this probe did not
## queue, into the very room it measures — the same reason the hero's own
## body is hidden above.
##
## STOPPING the physics step, not freeing the node: WaveLevel holds each
## creature as a typed `cat_children: Vec<Gd<WaveCat>>` handle and the
## composition root drives it every frame, so a freed cat dangles that
## handle and the next tick panics converting it back
## (`FromGodot::from_variant() failed -- variant holds object which is no
## longer alive`). Both of a cat's voices — the per-stride paw pulse and the
## 1.6 s presence beat — are emitted from `physics_process`, so this stops
## the sound at its source while the handle stays valid. Hidden as well, so
## the body cannot stand between the camera and a sample point.
func _silence_the_cat(main: UnseeingGame) -> void:
	for c: Node in main.level.get_children():
		if c is WaveCat:
			var cat := c as WaveCat
			cat.set_physics_process(false)
			cat.visible = false


func _set_quad(main: UnseeingGame, shown: bool) -> void:
	for c: Node in main.player.camera.get_children():
		if c is MeshInstance3D and (c as MeshInstance3D).material_override == main.post_mat:
			(c as MeshInstance3D).visible = shown


func _check(what: String, ok: bool) -> void:
	_checks += 1
	print(("ok %d - %s" if ok else "not ok %d - %s") % [_checks, what])
	if not ok:
		_failed += 1


func _report() -> void:
	print("1..%d" % _checks)
	var verdict := (
		"PASS (%d checks)" % _checks if _failed == 0 else "FAIL (%d of %d)" % [_failed, _checks]
	)
	print("probe: %s" % verdict)
	get_tree().quit(1 if _failed > 0 else 0)


func _settle(n: int) -> void:
	for _i: int in n:
		await get_tree().process_frame
