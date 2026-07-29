shader_type spatial;
render_mode unshaded, cull_disabled, shadows_disabled, ambient_light_disabled, fog_disabled;

// DATA PASS — the world is never drawn. This material renders geometry as
// DATA for the hearing pass to read from the screen texture:
//   R — reveal: how recently/strongly a wave swept this point
//   G — normal id: axis-weighted normal encoding (crease edge detection)
//   B — camera distance / DIST_PACK_RANGE (silhouette edges + wave occlusion)
// Distance is packed here on purpose: the depth texture is unreliable on
// WebGL2/Compatibility, and the web build is a first-class target.

#include "res://shaders/pulse_pool.gdshaderinc"
#include "res://shaders/materials.gdshaderinc"

uniform float u_base = 0.0;     // standing reveal; >0 only for the hero's cane
uniform int u_material = 0;     // Materials.Kind — chooses the reveal signature

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

		float typ = pulse_type(d);
		float gain = pulse_gain(d);
		float peak = typ < 0.5 ? 1.0 : (typ < 1.5 ? 0.9 : 0.35);

		float dist = distance(v_world, u_ppos[i]);
		float radius = d.z * age;
		if (dist > min(radius, d.y)) { continue; }
		vec3 to_src = (u_ppos[i] - v_world) / max(dist, 0.001);
		float cone = pulse_cone(u_pdir[i], -to_src);
		// only the struck spot lights: ignition falls off fast from the source
		float atten = 1.0 / (1.0 + dist * dist * 0.06);
		float ga = age - dist / d.z;
		float flare = 1.3 * exp(-ga / 0.25) + 0.5 * exp(-ga / 3.0);
		reveal = max(reveal, min(flare, 1.0) * atten * cone * gain * peak);
	}
	reveal = max(reveal, u_base);
	float vd = length(v_world - CAMERA_POSITION_WORLD);
	reveal *= (0.9 + 0.1 * u_flick) * exp(-vd * 0.05);
	// normal id: distinct levels for the six axis directions, continuous for
	// curved surfaces — one channel is enough for crease detection
	float nid = 0.5 + v_nworld.x * 0.30 + v_nworld.y * 0.17 + v_nworld.z * 0.09;
	// material glyphs are ENGRAVED into the distance channel: the hearing
	// pass's edge detector draws them itself, in the same thin stroke as the
	// outlines, gated by the same reveal — the fill stays fully transparent
	float glyph = material_pattern(v_world, v_nworld, u_material);
	ALBEDO = vec3(clamp(reveal, 0.0, 1.0), nid,
			clamp(vd / DIST_PACK_RANGE + 0.028 * glyph, 0.0, 1.0));
}
