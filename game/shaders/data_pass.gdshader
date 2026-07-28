shader_type spatial;
render_mode unshaded, cull_disabled, shadows_disabled, ambient_light_disabled, fog_disabled;

// DATA PASS — the world is never drawn. This material renders geometry as
// DATA for the hearing pass to read from the screen texture:
//   R — reveal: how recently/strongly a wave swept this point
//   G, B — world normal xy (for crease edge detection)
//   (depth comes from the depth texture; the web version packed it in alpha)
// Ported from web-reference/index.html scene fragment shader.

const int MAXP = 64;

uniform float u_time = 0.0;
uniform float u_flick = 1.0;
uniform float u_base = 0.0;   // standing reveal; >0 only for the hero's cane
uniform int u_count = 0;
uniform vec3 u_ppos[MAXP];
uniform vec4 u_pdat[MAXP];    // birth, maxR, speed, type*10 + gain*9
uniform vec4 u_pdir[MAXP];    // beam dir xyz + cos(half-angle); w = -2 = omni

varying vec3 v_world;
varying vec3 v_nworld;

void vertex() {
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_nworld = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	float reveal = 0.0;
	for (int i = 0; i < MAXP; i++) {
		if (i >= u_count) { break; }
		vec4 d = u_pdat[i];
		if (d.x < 0.0) { continue; }
		float age = u_time - d.x;
		if (age <= 0.0) { continue; }

		float typ = floor(d.w / 10.0);
		float gain = mod(d.w, 10.0) / 9.0;
		float peak = typ < 0.5 ? 1.0 : (typ < 1.5 ? 0.9 : 0.35);

		float dist = distance(v_world, u_ppos[i]);
		float radius = d.z * age;
		if (dist > min(radius, d.y)) { continue; }
		vec3 to_src = (u_ppos[i] - v_world) / max(dist, 0.001);
		// directed sound: only the cone the sound was aimed into carries info
		vec4 cn = u_pdir[i];
		float cone = smoothstep(cn.w - 0.15, cn.w + 0.05, dot(-to_src, cn.xyz));
		// only the struck spot lights: ignition falls off fast from the source
		float atten = 1.0 / (1.0 + dist * dist * 0.06);
		float ga = age - dist / d.z;
		float flare = 1.3 * exp(-ga / 0.25) + 0.5 * exp(-ga / 3.0);
		reveal = max(reveal, min(flare, 1.0) * atten * cone * gain * peak);
	}
	reveal = max(reveal, u_base);
	float vd = length(v_world - CAMERA_POSITION_WORLD);
	reveal *= (0.9 + 0.1 * u_flick) * exp(-vd * 0.05);
	ALBEDO = vec3(clamp(reveal, 0.0, 1.0), v_nworld.x * 0.5 + 0.5, v_nworld.y * 0.5 + 0.5);
}
