shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// PROBE ONLY. Declares BOTH the screen texture and the depth texture, to
// measure what that combination costs in the Compatibility renderer.
uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
uniform bool u_want_depth = false;

void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	// u_want_depth only decides whether the depth sample is REACHED; the
	// uniform is declared either way, which is the thing being measured.
	float c = texture(screen_tex, vec2(0.5, 0.5)).r;
	if (u_want_depth) {
		c = min(c + texture(depth_tex, vec2(0.5, 0.5)).r, c);
	}
	ALBEDO = vec3(c);
}
