shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// PROBE ONLY — never referenced by the game.
//
// A plain screen-texture read, declaring NOTHING else. It exists to be the
// control half of one comparison: probe_depth_combined declares a depth
// texture beside the screen texture, and the two readings together answer
// whether that declaration costs the screen read anything in the
// Compatibility renderer. hearing_post depends on the answer, since that
// pass IS the screen texture and now declares a depth uniform beside it.
uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;

void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	ALBEDO = vec3(texture(screen_tex, vec2(0.5, 0.5)).r);
}
