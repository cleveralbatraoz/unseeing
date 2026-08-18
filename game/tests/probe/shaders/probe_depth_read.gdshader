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

// The acoustic-image band, hand-transcribed from data_core.gdshaderinc so
// the probe does not read its expectation out of the file under test.
const float ALWAYS_ON_TOP = 0.999999;
const float SOURCE_BAND = 1.0e-3;

// 0 — amplified raw depth (is the texture alive at all?)
// 1 — the SHIPPED predicate, as 1.0 or 0.0
// 2 — signed margin against the band floor, 1e-3 either side mapped onto
//     [0,1], so a near miss is legible instead of saturating
uniform int u_mode = 0;

void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	float d = texture(depth_tex, vec2(0.5, 0.5)).r;
	// Sampled where geometry certainly stands. Under Godot's reversed-Z
	// with near 0.05 and far 60, a surface 3 m from the eye writes 0.0159 —
	// small, but emphatically not zero. A working depth texture cannot read
	// 0.0 here; a dead one reads nothing anywhere. Amplified so an 8-bit
	// screenshot can see a value that small.
	float raw = clamp(d * u_gain, 0.0, 1.0);
	// WHY THE BAND IS NOT READ AS A LEVEL. The band is 1e-3 wide and an
	// 8-bit screenshot resolves 1/255 = 3.9e-3, so a readback at unit gain
	// cannot see it AT ALL: only code 255 could ever satisfy the test, and
	// every true depth from 0.998039 up reads back as exactly 1.0 — a
	// driver answering 0.9985 for an always-on-top fragment would PASS
	// here while the shipped predicate returned false for it at every
	// pixel, silently dropping the depth term of the outline cap. So the
	// comparison is made HERE, where the precision still exists, and only
	// its verdict is screenshotted.
	float verdict = d >= ALWAYS_ON_TOP - SOURCE_BAND ? 1.0 : 0.0;
	float margin = clamp((d - (ALWAYS_ON_TOP - SOURCE_BAND)) * 500.0 + 0.5, 0.0, 1.0);
	ALBEDO = vec3(u_mode == 0 ? raw : (u_mode == 1 ? verdict : margin));
}
