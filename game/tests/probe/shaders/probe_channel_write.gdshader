shader_type spatial;
render_mode unshaded, cull_disabled, shadows_disabled, ambient_light_disabled, fog_disabled;

// PROBE ONLY — never referenced by the game. Writes a known pair of values
// into the data channels so a second pass can measure what the screen
// texture actually preserved.
uniform float u_lo = 0.5;
uniform float u_hi = 0.5;

void fragment() {
	// left half writes u_lo, right half writes u_hi
	float v = SCREEN_UV.x < 0.5 ? u_lo : u_hi;
	ALBEDO = vec3(v, v, v);
}
