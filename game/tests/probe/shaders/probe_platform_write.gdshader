shader_type spatial;
render_mode unshaded, cull_disabled, shadows_disabled, ambient_light_disabled, fog_disabled;

// PROBE ONLY — never referenced by the game.
//
// The WRITE half of a one-screenshot platform measurement. The desktop
// probes answer the same questions by writing a pair of values, reading
// them back in-game with `get_viewport().get_texture().get_image()`, and
// repeating that for every candidate step — dozens of frames and a readback
// per frame. On the web that path is the wrong shape: the browser gate
// cannot drive an in-game readback loop, and Godot's own readback on web is
// a different code path from the one under test.
//
// So the whole ladder is laid out SPATIALLY and answered by one screenshot.
// The screen is divided into NBANDS vertical bands; band b writes a base
// value in its left half and base + STEPS[b] in its right half, and the
// READ shader compares each right-hand pixel against its partner on the
// left. The base sweeps with y, so a whole column of bases is tested at
// once — which is the part that matters: a single base sits at one
// arbitrary place on the quantisation grid, and that alone moved the
// desktop answer by a full bit before it was swept.
//
// WHY THE LADDER IS NOT POWERS OF TWO. It was: band b tested 1/(2^(8+b) - 1)
// and the verdict was the last whole power that survived. That can only ever
// answer 512 or 1024, so a channel that resolves 1/1023 at 99.7% of bases
// and not at the rest reads as 512 — a factor of two pessimism in the one
// number the B-channel reconstruction guard turns on. What that guard needs
// is the WORST-CASE QUANTUM, so the ladder sweeps multiples of a 10-bit step
// instead and the verdict is the smallest multiple that survives everywhere.
// The extremes are the probe's own sanity checks: 0.5 must fail on any
// 10-bit buffer and 2.0 must survive on one, so a ladder that comes back all
// white or all black has measured nothing.

const int NBANDS = 9;
const float TEN_BIT = 1.0 / 1023.0;
// Multiples of one 10-bit step, ascending — PUSHED by platform_probe.gd,
// which owns them, because that file computes the verdict by indexing its
// own copy against the pixels this one wrote. Two copies bound by a comment
// meant a single drifted entry mislabelled which step survived, silently,
// and that verdict is the sole source of render::channel::WORST_STEP_CODES.
//
// The default is all zeros and is deliberately unusable: an unpushed ladder
// collapses every band to no step at all, which is the probe's own
// documented "all black means it measured nothing" failure, rather than a
// plausible ladder nobody chose.
uniform float u_steps[9] = float[9](0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
const float BASE_LO = 0.05;
const float BASE_HI = 0.95;

void fragment() {
	int band = int(clamp(floor(SCREEN_UV.x * float(NBANDS)), 0.0, float(NBANDS - 1)));
	float local = fract(SCREEN_UV.x * float(NBANDS));
	float step_size = u_steps[band] * TEN_BIT;
	float base = mix(BASE_LO, BASE_HI, SCREEN_UV.y);
	// left half: the base. right half: the base plus one candidate step.
	float v = local < 0.5 ? base : base + step_size;
	ALBEDO = vec3(v);
}
