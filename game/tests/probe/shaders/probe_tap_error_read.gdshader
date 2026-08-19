shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// PROBE ONLY — never referenced by the game.
//
// WHERE THE REPRESENTATIVE SITS INSIDE A WIDENED CELL, which is the one
// unmeasured number the whole B-channel error budget rests on.
//
// `platform_probe` measures a gap WIDTH: the smallest step that survives at
// every base, 1.25 nominal codes on Mesa/AMD. `render::channel::recon_eps`
// halves that and calls the result the worst error in a single value read
// back — which holds for a quantiser that rounds to the nearest cell
// CENTRE, and the 1.25 figure exists precisely because this pipeline is not
// a plain quantiser. If the value it hands back sits at a cell EDGE, every
// budget derived from the half-gap is out by two: `sight::RECT_SHRINK` has
// to clear 48.9 mm rather than 24.4 mm.
//
// So this measures the error DIRECTLY, and measures nothing else. The write
// pass puts ONE constant on the whole screen (probe_channel_write with
// u_lo == u_hi), this reads one texel of it back, and the difference is
// taken against the same constant as a uniform. No spatial layout, no base
// swept down a column, no second tap: an earlier version of this probe did
// sweep the base with y and reported a mean error of -0.578 codes that was
// mostly the base moving 1.28 codes per screen row, which is a reading
// about sampling wearing a reading about the channel's clothes.
//
// The difference is taken and amplified INSIDE the shader, where the
// precision still exists, and only the signed result is screenshotted:
// `GAIN` codes either side of zero mapped onto [0, 1], so 0.5 means the
// value came back exactly.
uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
uniform float u_ref = 0.5;
/// nominal 10-bit codes represented either side of zero
uniform float u_span = 4.0;

void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	float stored = texture(screen_tex, vec2(0.25, 0.5)).r;
	float codes = (stored - u_ref) * 1023.0;
	// NO `return` anywhere: Godot's shading language forbids it in
	// fragment(), and a shader that fails to compile does not render at all.
	ALBEDO = vec3(clamp(codes / (2.0 * u_span) + 0.5, 0.0, 1.0));
}
