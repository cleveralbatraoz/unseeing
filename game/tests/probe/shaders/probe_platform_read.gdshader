shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// PROBE ONLY — never referenced by the game.
//
// The READ half of the one-screenshot platform measurement. Three regions,
// so a single decoded PNG answers three questions:
//
//   x < 0.70  — THE CHANNEL LADDER. Each of NBANDS vertical bands asks
//               whether a step of 1/(2^(8+b) - 1) survived the screen
//               texture, comparing each right-hand pixel against its
//               partner in the left half of the same band. White means the
//               two values stayed distinct; black means the channel
//               quantised them together. The base sweeps with y, so a
//               band is only honestly "surviving" if the WHOLE column is
//               white — one base alone can sit between two codes and
//               separate half a step, which is how a fixed-base version of
//               this measurement reported two different depths for one
//               buffer.
//
//   0.70..0.85 — THE DEPTH TEXTURE, amplified so a world fragment's ~0.016
//               is visible in an 8-bit screenshot. Black here means the
//               depth texture is dead on this platform, which is what the
//               shipped shaders assumed for months and what desktop GL
//               disproved.
//
//   x >= 0.85 — THE CONTROL: the screen texture read straight back. If this
//               is not the value the write pass put there, the probe is
//               measuring nothing and every verdict to its left is void.
uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
uniform float u_depth_gain = 60.0;
uniform float u_control_expect = 0.5;

const int NBANDS = 9;

void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	vec2 uv = SCREEN_UV;
	vec3 out_colour;

	if (uv.x >= 0.85) {
		// CONTROL: the write pass put mix(0.05, 0.95, 0.5) = 0.5 at
		// (0.05, 0.5) — band 0's left half, halfway up. Reading anything
		// else back means the screen texture is not carrying this frame's
		// geometry and every verdict to the left is void.
		out_colour = vec3(texture(screen_tex, vec2(0.05, 0.5)).r);
	} else if (uv.x >= 0.70) {
		out_colour = vec3(clamp(texture(depth_tex, vec2(0.5, 0.5)).r * u_depth_gain, 0.0, 1.0));
	} else {
		// The ladder is DISPLAYED across x in [0, 0.70) but the write pass
		// laid it across the whole width, so the sample coordinates below
		// are in the write's own layout, not this one. Getting that mapping
		// wrong is the obvious way to make this probe answer confidently
		// about the wrong band.
		float band = floor((uv.x / 0.70) * float(NBANDS));
		float left_u = (band + 0.25) / float(NBANDS);
		float right_u = (band + 0.75) / float(NBANDS);
		float a = texture(screen_tex, vec2(left_u, uv.y)).r;
		float b = texture(screen_tex, vec2(right_u, uv.y)).r;
		out_colour = vec3(a == b ? 0.0 : 1.0);
	}

	// NO `return` anywhere above: Godot's shading language forbids it in
	// fragment(), and a shader that fails to compile does not render at
	// all — which let this probe's own control read the WRITE pass
	// directly and report a confident 0.502 while measuring nothing.
	ALBEDO = out_colour;
}
