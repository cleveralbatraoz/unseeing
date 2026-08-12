// DATA CORE — the shared fragment machinery of every data-writing skin.
// The world is never drawn: geometry renders as DATA for the hearing
// pass to read from the screen texture:
//   R — reveal: how recently/strongly a wave swept this point
//   G — superface label: crease edge detection
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

// The per-VERTEX superface label: the derive-time paint pass
// (rust/src/render/paint.rs) bakes one label per face into each mesh's
// CUSTOM0 channel, and both skins' vertex() functions carry it straight
// through as this varying. Two overlapping solids sharing a label
// bit-for-bit melt into one silhouette; where they must differ the paint
// pass has already kept them >= MIN_SEP apart. Replaces the old
// per-instance object-id uniform and its normal-derived fallback outright
// — every object's G channel now comes from its own geometry, never a
// per-object override.
varying float v_label;

varying vec3 v_world;

// The viewmodel depth hack, written from the FRAGMENT: DEPTH takes the
// window depth of the near plane (~1.0 under Godot's reversed-Z), so
// every fragment passes the GEQUAL depth test and opaque draw order
// alone decides the pixel — the perceptual layering, as render_priority.
// It must NOT be a vertex POSITION.z override: faking clip z defeats
// near-plane clipping, and geometry crossing the camera plane (a source
// walked up to) rasterizes as projective full-screen sheets, stamping
// garbage data over the whole buffer.
const float ALWAYS_ON_TOP = 0.999999;

// ...but a CONSTANT depth only works while the world holds one source. Two
// acoustic images writing the same value resolve against each other by
// opaque draw order alone, and Godot sorts opaque surfaces near-to-far —
// so the NEARER source draws first and the FARTHER one, passing GEQUAL on
// an equal value, overwrites it. A dim, distant ghost would punch a hole
// through a near, loud one.
//
// So the layer gets a BAND instead of a point: a source's depth is the
// always-on-top value minus a hair proportional to its true distance. Every
// source still beats every world fragment (a world surface would have to be
// inside the near plane to reach this band), while sources — and a single
// source's own overlapping limbs — resolve by distance like everything
// else. The band is far narrower than any depth step the world can produce.
const float SOURCE_BAND = 1.0e-5;

// Where in the acoustic-image band a source fragment `dist` metres from the
// eye writes its depth.
float source_depth(float dist) {
	return ALWAYS_ON_TOP - SOURCE_BAND * clamp(dist / DIST_PACK_RANGE, 0.0, 1.0);
}

// How much a wave of this kind still reveals `world`, given the WALLS
// between it and its source (props are transparent to waves — only walls
// obstruct). The wave flows through openings and lights the next room
// through a doorway, but a wall stops it. EVERY wave obeys this, echoes
// included: an echo is a reflected wave, and a reflection is stopped by a
// wall like any sound — the reflection ray only decides where the echo is
// BORN, never that its reveal may pass through a wall.
//   - a cane tap, its echoes, and footsteps (0, 1, 2) are cut CRISP at a
//     wall — a player sound does not pass, and their shells die at the
//     world too;
//   - a world SOURCE's wave (3 — the fan's hum, the radio) is MUFFLED,
//     not silenced: HUM_THROUGH per wall,
//     the same factor its shell passes by, so a far surface dims instead
//     of turning black against a ring still travelling through the wall.
// crossings_from skips the wall the source is born inside, so a tap or an
// echo struck ON a wall still lights that wall's own near face.
float source_reveal_vis(float typ, vec3 src, vec3 world) {
	int blocked = wall_crossings_from(src, world);
	if (blocked == 0) { return 1.0; }
	if (typ > 2.5) { return pow(HUM_THROUGH, float(blocked)); }
	return 0.0;  // tap, echo, footstep: cut crisp at a wall
}

// How strongly the waves in flight reveal this world point: the struck
// spot lights and falls off fast from the source, occluded by the walls
// between the source and the lit surface (source_reveal_vis) — a source
// lights the next room through a doorway, never through a wall.
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
		// the beam gate comes first: a fragment outside a DIRECTED wash's
		// cone reveals nothing, so skip the wall-occlusion loop there. This
		// spares a DIRECTED source's narrow sweep only (the fan; a radio's
		// even sphere pays like any other wave) — the hero's own taps, echoes
		// and footsteps are omni (cone == 1), so they pay the per-fragment
		// wall loop wherever their radius still reaches, which is correct:
		// those sounds ARE cut at walls. The cost is O(live pulses × walls)
		// per fragment, bounded per pulse by the radius gate above; an
		// angular/polar shadow map is the documented next step IF a weak-GPU
		// measurement ever demands it. The browser-smoke gate proves the
		// occlusion GLSL runs on WebGL2; profiling weak hardware is deferred.
		float cone = pulse_cone(u_pdir[i], -to_src);
		if (cone <= 0.0) { continue; }
		float atten = 1.0 / (1.0 + dist * dist * 0.06);
		float ga = age - dist / d.z;
		float flare = 1.3 * exp(-ga / 0.25) + 0.5 * exp(-ga / 3.0);
		// EXACT early-out, and it is the loop's whole affordability. The
		// accumulator is a max, and source_reveal_vis only ever returns a
		// value in [0, 1] (1, 0, or HUM_THROUGH^walls) — so `bound` below,
		// computed with that factor at its maximum, is an upper bound on
		// what this pulse could contribute. A pulse that cannot beat what is
		// already accumulated changes nothing, and is dropped BEFORE the
		// wall loop rather than after it. No pixel differs; only the cost.
		//
		// This matters most for an EVEN source: a directed wash is thrown
		// out by the cone gate above over most of the screen, but a radio's
		// sphere passes the cone gate everywhere, so without this its
		// permanently-live waves would each pay the full per-fragment wall
		// walk in every direction, forever.
		float bound = min(flare, 1.0) * atten * cone * gain * peak;
		if (bound <= reveal) { continue; }
		reveal = max(reveal, bound * source_reveal_vis(typ, u_ppos[i], world));
	}
	return reveal;
}

// Pack (reveal, superface label, camera distance) into the data channels:
// nervous-light modulation and distance fade on R; the per-vertex label
// straight off CUSTOM0 (v_label) on G; packed camera distance on B.
vec3 pack_data(float reveal, vec3 world, vec3 cam) {
	float vd = length(world - cam);
	reveal *= (0.9 + 0.1 * u_flick) * exp(-vd * 0.05);
	return vec3(clamp(reveal, 0.0, 1.0), v_label, clamp(vd / DIST_PACK_RANGE, 0.0, 1.0));
}
