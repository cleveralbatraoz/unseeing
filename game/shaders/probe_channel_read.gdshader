shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// PROBE ONLY. Samples the screen texture on both sides of the split and
// amplifies their difference enormously: if the buffer preserved a step
// smaller than 1/255, the two halves differ and this glows; if the buffer
// quantised them together, it stays black.
uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
// The DEPTH texture, measured alongside the colour one. data_core has long
// carried the claim that it is "unreliable on WebGL2/Compatibility"; this
// makes that a measurement. It matters because an exact layer test — is
// this pixel an acoustic image? — would fall straight out of it: the source
// skin is opaque and writes DEPTH inside the always-on-top band, so
// `depth >= ALWAYS_ON_TOP - SOURCE_BAND` would identify a source with no
// inference, no tables and no reconstruction at all.
uniform float u_gain = 4000.0;
// 0 = report the colour-channel verdict, 1 = report the depth texture
uniform int u_mode = 0;

void vertex() {
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

void fragment() {
	float a = texture(screen_tex, vec2(0.25, 0.5)).r;
	float b = texture(screen_tex, vec2(0.75, 0.5)).r;
	ALBEDO = vec3(clamp(abs(a - b) * u_gain, 0.0, 1.0));
}
