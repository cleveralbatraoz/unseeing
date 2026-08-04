shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// HEARING PASS — a fullscreen quad pinned to the camera. Everything the
// player ever sees is produced here, from two ingredients:
//   1. thin white OUTLINES where geometry breaks (silhouettes via a distance
//      Laplacian, creases via normal-id differences), gated by the per-pixel
//      "reveal" the data pass wrote — outlines exist only where a wave swept;
//   2. the WAVES themselves: expanding spherical shells ray-traced in air,
//      limb-brightened into rings, occluded by the world (walls carve bites
//      out of a ring — absence of wave IS the shape of the world).
// Plus the mood layer: breathing vignette and film grain.

#include "res://shaders/pulse_pool.gdshaderinc"

uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
uniform float u_breath = 1.0;
uniform float u_grain_amp = 0.034;
uniform float u_grain_t = 0.0;

void vertex() {
	// pin the unit quad to the whole screen, ignoring its 3D placement
	POSITION = vec4(VERTEX.xy * 2.0, 1.0, 1.0);
}

float shash(vec2 p) {
	return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

float n2(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = shash(i);
	float b = shash(i + vec2(1, 0));
	float c = shash(i + vec2(0, 1));
	float d = shash(i + vec2(1, 1));
	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

void fragment() {
	vec2 uv = SCREEN_UV;
	vec2 px = 1.0 / VIEWPORT_SIZE;
	vec3 cam = CAMERA_POSITION_WORLD;

	vec4 c_c = texture(screen_tex, uv);
	vec4 c_l = texture(screen_tex, uv - vec2(px.x, 0.0));
	vec4 c_r = texture(screen_tex, uv + vec2(px.x, 0.0));
	vec4 c_u = texture(screen_tex, uv + vec2(0.0, px.y));
	vec4 c_d = texture(screen_tex, uv - vec2(0.0, px.y));

	// thin white outlines: silhouettes (Laplacian of the packed distance in B)
	// + creases (object-id differences in G — a flat per-instance id, or a
	// normal-derived one where the level tagged nothing), visible only where
	// a wave has swept (max of the five reveal taps in R). Two touching
	// objects with the SAME id draw no seam at all: see rust/src/oid_palette.rs
	// for the 0.08 separation the smoothstep below demands.
	float lap = abs(c_l.b + c_r.b + c_u.b + c_d.b - 4.0 * c_c.b);
	float nrm = abs(c_l.g - c_r.g) + abs(c_u.g - c_d.g);
	float edge = max(smoothstep(0.012, 0.03, lap), smoothstep(0.04, 0.08, nrm));
	float reveal = max(c_c.r, max(max(c_l.r, c_r.r), max(c_u.r, c_d.r)));
	vec3 col = vec3(edge * reveal);

	// the waves themselves: rings in the air, cut where the world stands
	vec3 ndc_far = vec3(uv * 2.0 - 1.0, 1.0);
	vec4 vf = INV_PROJECTION_MATRIX * vec4(ndc_far, 1.0);
	vf.xyz /= vf.w;
	vec3 world_far = (INV_VIEW_MATRIX * vec4(vf.xyz, 1.0)).xyz;
	vec3 rd = normalize(world_far - cam);
	float scene_d = c_c.b * DIST_PACK_RANGE;

	for (int i = 0; i < MAXP; i++) {
		if (i >= u_count) { break; }
		vec4 d = u_pdat[i];
		if (d.x < 0.0) { continue; }
		float age = u_time - d.x;
		if (age <= 0.0) { continue; }
		float radius = d.z * age;
		float prog = radius / d.y;
		if (prog >= 1.0) { continue; }
		float typ = pulse_type(d);
		float gain = pulse_gain(d);
		float peak = typ < 0.5 ? 0.5 : (typ < 1.5 ? 0.45 : (typ < 2.5 ? 0.18 : 0.5));
		vec3 oc = cam - u_ppos[i];
		float b = dot(oc, rd);
		float disc = b * b - (dot(oc, oc) - radius * radius);
		if (disc <= 0.0) { continue; }
		float sq = sqrt(disc);
		float env = pow(1.0 - prog, 1.2) * min(age * 10.0, 1.0) * peak * gain * u_flick;
		vec4 cn = u_pdir[i];
		for (int s = 0; s < 2; s++) {
			float t = -b + (s == 0 ? -sq : sq);
			if (t <= 0.05) { continue; }
			// player-made sounds are cut by the world; a source hum (type 3)
			// passes through walls muffled — a constant source is FELT from
			// another room long before it is ever tapped
			float mute = 1.0;
			if (t >= scene_d) {
				if (typ < 2.5) { continue; }
				mute = 0.55;
			}
			vec3 hp = cam + rd * t;
			vec3 sn = (hp - u_ppos[i]) / radius;
			float cone = pulse_cone(cn, sn);
			if (cone <= 0.0) { continue; }
			float grz = 1.0 - abs(dot(rd, sn));
			// a hum's shell has a diffuse body, not just a grazing ring — a
			// constant source is a throb the whole room can feel
			float body = typ > 2.5 ? 0.22 : 0.05;
			float ink = 0.55 + 0.45 * n2(hp.xz * 1.7 + hp.y * 0.9);
			col += vec3(env * mute * cone * (body + (1.0 - body) * pow(max(grz, 0.0), 4.0)) * ink * exp(-t * 0.09));
		}
	}

	// the void is not quite empty
	col += vec3(0.006);
	// breathing vignette
	float vig = 1.0 - smoothstep(0.38, 0.92, length(uv - 0.5) * 1.35 * u_breath);
	col *= mix(0.45, 1.0, vig);
	// film grain
	col += (shash(FRAGCOORD.xy + u_grain_t) - 0.5) * u_grain_amp;

	ALBEDO = max(col, vec3(0.0));
}
