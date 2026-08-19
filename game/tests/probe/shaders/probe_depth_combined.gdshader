shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// PROBE ONLY. Declares BOTH the screen texture and the depth texture, to
// measure what that combination costs in the Compatibility renderer.
uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	// The screen read, from a shader that ALSO declares a depth texture and
	// genuinely samples it. Both halves matter: an unsampled uniform can be
	// optimised away, and a value folded back out (`min(c + d, c)`, which
	// this file used to do) makes the comparison against the depth-free
	// shader an algebraic identity that cannot fail whatever the platform
	// does.
	//
	// The depth sample is mixed in at a weight small enough to leave the
	// verdict legible but large enough that the compiler cannot drop it.
	float c = texture(screen_tex, vec2(0.5, 0.5)).r;
	float d = texture(depth_tex, vec2(0.5, 0.5)).r;
	ALBEDO = vec3(c + d * 0.001);
}
