shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// PROBE ONLY — never referenced by the game.
//
// Is hint_depth_texture usable in the Compatibility renderer? The question
// is not academic: hearing_post has to know whether the surface at a pixel
// is an acoustic image drawn over the world or a real-depth surface, and it
// infers that today by rebuilding a world point from the B channel and
// asking the wall table — an inference that is incomplete, because props
// are in no occluder table anywhere. A depth texture would settle it
// exactly, since the source skin is opaque and writes DEPTH inside the
// always-on-top band.
//
// This shader declares the depth texture ALONE so the measurement isolates
// it. Whether it may be declared BESIDE hint_screen_texture is a separate
// question and is measured separately, by probe_depth_combined — it may,
// and that is what licenses hearing_post to do so. An earlier draft of this
// comment asserted the opposite as a measured fact; it was not measured, it
// was a misread of an unrelated edit, and the game now ships the
// combination it called impossible.
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
// Amplification, because a world fragment's depth is ~0.016 and an 8-bit
// frame cannot resolve it. Set to 1.0 when the value of interest is near
// the top of the range instead — at any gain above 1, everything in the
// acoustic-image band saturates and stops being distinguishable from
// anything else that saturates.
uniform float u_gain = 60.0;

void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	// Sampled where geometry certainly stands. Under Godot's reversed-Z
	// with near 0.05 and far 60, a surface 3 m from the eye writes 0.0159 —
	// small, but emphatically not zero. A working depth texture cannot read
	// 0.0 here; a dead one reads nothing anywhere. Amplified so an 8-bit
	// screenshot can see a value that small.
	ALBEDO = vec3(clamp(texture(depth_tex, vec2(0.5, 0.5)).r * u_gain, 0.0, 1.0));
}
