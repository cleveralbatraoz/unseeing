// DATA CORE — the shared fragment machinery of every data-writing skin.
// The world is never drawn: geometry renders as DATA for the hearing
// pass to read from the screen texture:
//   R — reveal: how recently/strongly a wave swept this point
//   G — normal id / object id: crease edge detection
//   B — camera distance / DIST_PACK_RANGE (silhouette edges + wave occlusion)
// Distance is packed here on purpose: the depth texture is unreliable on
// WebGL2/Compatibility, and the web build is a first-class target.
//
// Skins differ only in render_mode, in their rasterized depth (the world
// keeps true depth; the acoustic image rides always_on_top) and in the
// standing floor they max the wave reveal with before packing. Builtins
// cross as parameters — an include cannot touch them itself.

#include "res://shaders/pulse_pool.gdshaderinc"

uniform float u_base = 0.0;  // standing reveal floor; 0 = waves only

// One flat OBJECT ID per instance: the post pass diffs the G channel, so a
// constant id across a whole object draws its unified silhouette with NO
// interior component seams. < 0 (the default) falls back to the normal-id
// crease encoding, so an object opts in the moment it sets this per-instance.
instance uniform float u_oid = -1.0;

varying vec3 v_world;
varying vec3 v_nworld;

// The viewmodel depth hack, written from the FRAGMENT: DEPTH takes the
// window depth of the near plane (~1.0 under Godot's reversed-Z), so
// every fragment passes the GEQUAL depth test and opaque draw order
// alone decides the pixel — the perceptual layering, as render_priority.
// It must NOT be a vertex POSITION.z override: faking clip z defeats
// near-plane clipping, and geometry crossing the camera plane (a source
// walked up to) rasterizes as projective full-screen sheets, stamping
// garbage data over the whole buffer.
const float ALWAYS_ON_TOP = 0.999999;

// How strongly the waves in flight reveal this world point: the struck
// spot lights and falls off fast from the source. A source's hum reveals
// only its own room today (hum_room_mask); the analytic wall occlusion
// that replaces the rectangle rides the shared wall table in the pool.
float reveal_at(vec3 world) {
	float reveal = 0.0;
	for (int i = 0; i < MAXP; i++) {
		if (i >= u_count) { break; }
		vec4 d = u_pdat[i];
		if (d.x < 0.0) { continue; }
		float age = u_time - d.x;
		if (age <= 0.0) { continue; }

		float typ = pulse_type(d);
		float gain = pulse_gain(d);
		float peak = typ < 0.5 ? 1.0 : (typ < 1.5 ? 0.9 : (typ < 2.5 ? 0.35 : 0.8));

		float dist = distance(world, u_ppos[i]);
		float radius = d.z * age;
		if (dist > min(radius, d.y)) { continue; }
		vec3 to_src = (u_ppos[i] - world) / max(dist, 0.001);
		float cone = pulse_cone(u_pdir[i], -to_src) * hum_room_mask(typ, world);
		float atten = 1.0 / (1.0 + dist * dist * 0.06);
		float ga = age - dist / d.z;
		float flare = 1.3 * exp(-ga / 0.25) + 0.5 * exp(-ga / 3.0);
		reveal = max(reveal, min(flare, 1.0) * atten * cone * gain * peak);
	}
	return reveal;
}

// Pack (reveal, normal/object id, camera distance) into the data channels:
// nervous-light modulation and distance fade on R; the per-object id when
// assigned, else distinct normal-id levels for the six axis directions
// (continuous for curved surfaces) on G; packed camera distance on B.
vec3 pack_data(float reveal, vec3 world, vec3 nworld, vec3 cam) {
	float vd = length(world - cam);
	reveal *= (0.9 + 0.1 * u_flick) * exp(-vd * 0.05);
	float nid = u_oid >= 0.0
			? u_oid
			: (0.5 + nworld.x * 0.30 + nworld.y * 0.17 + nworld.z * 0.09);
	return vec3(clamp(reveal, 0.0, 1.0), nid, clamp(vd / DIST_PACK_RANGE, 0.0, 1.0));
}
