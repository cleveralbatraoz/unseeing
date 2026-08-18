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
// The screen is divided into NBANDS vertical bands; band b tests whether a
// step of 1/(2^(8+b) - 1) survives the screen texture. Within a band the
// left half writes a base value and the right half writes base + step, and
// the READ shader compares each right-hand pixel against its partner on the
// left. The base sweeps with y, so a whole column of bases is tested at
// once — which is the part that matters: a single base sits at one
// arbitrary place on the quantisation grid, and that alone moved the
// desktop answer by a full bit before it was swept.

const int NBANDS = 9;  // bits 8..16
const float BASE_LO = 0.05;
const float BASE_HI = 0.95;

void fragment() {
	float band = floor(SCREEN_UV.x * float(NBANDS));
	float local = fract(SCREEN_UV.x * float(NBANDS));
	float bits = 8.0 + band;
	float step_size = 1.0 / (pow(2.0, bits) - 1.0);
	float base = mix(BASE_LO, BASE_HI, SCREEN_UV.y);
	// left half: the base. right half: the base plus one candidate step.
	float v = local < 0.5 ? base : base + step_size;
	ALBEDO = vec3(v);
}
