shader_type spatial;
render_mode unshaded, cull_back, shadows_disabled, ambient_light_disabled, fog_disabled;

// PROBE ONLY. The acoustic-image skin's depth trick in miniature: a
// fragment that writes DEPTH inside the always-on-top band, so the probe can
// ask whether the depth texture reads it back there — the one thing that
// decides whether the hearing pass can identify a source by depth alone.
uniform float u_lo = 0.5;
uniform float u_hi = 0.5;

const float ALWAYS_ON_TOP = 0.999999;

void fragment() {
	DEPTH = ALWAYS_ON_TOP;
	ALBEDO = vec3(u_hi);
}
