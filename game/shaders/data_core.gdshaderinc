// DATA CORE — the shared fragment machinery of every data-writing skin.
// The world is never drawn: geometry renders as DATA for the hearing
// pass to read from the screen texture:
//   R — reveal: how recently/strongly a wave swept this point
//   G — per-vertex face or semantic-role label: crease edge detection
//   B — camera distance / DIST_PACK_RANGE (silhouette edges + wave occlusion)
// Distance is packed here on purpose, and the reason that used to be given
// for it is gone. The depth texture IS live in desktop Compatibility —
// game/tests/probe/depth_texture_probe.gd reads real reversed-Z depth out
// of it, 0.0158 at three metres against an analytic 0.01585 — and it is
// live on WebGL2 too, measured at 0.9490 against an analytic 0.9508 under
// SwiftShader AND under a GPU-backed ANGLE/Apple-Metal driver. The old
// blanket claim that it is unreliable was wrong on both targets. B carries
// distance for the one reason that survives measurement: the outline pass
// needs distance in a channel it can take a Laplacian of across five taps,
// not a value it must unproject per sample.
//
// Skins differ only in render_mode, in their rasterized depth (the world
// keeps true depth; the acoustic image rides always_on_top) and in the
// standing floor they max the wave reveal with before packing. Builtins
// cross as parameters — an include cannot touch them itself.

#include "res://shaders/pulse_pool.gdshaderinc"

uniform float u_base = 0.0;  // standing reveal floor; 0 = waves only

// Every drawn mesh supplies a per-VERTEX label in CUSTOM0. WaveLevel's
// derive-time paint pass (rust/src/render/paint.rs) assigns per-face
// superface labels to world solids (with slabs anchored to fixed roles).
// Source builders bake the per-instance numeric labels derived for their
// semantic roles; creature and viewmodel builders bake fixed role labels.
// Both skins' vertex() functions carry either kind straight
// through as this varying. Overlapping world solids sharing a label
// bit-for-bit melt into one silhouette; labels that must differ stay at
// least MIN_SEP apart. Replaces the old per-instance object-id uniform and
// its normal-derived fallback outright — every G value comes from geometry,
// never a per-object override.
varying float v_label;

varying vec3 v_world;

// Where in the acoustic-image band a source fragment `dist` metres from the
// eye writes its depth.
float source_depth(float dist) {
	return ALWAYS_ON_TOP - SOURCE_BAND * clamp(dist / DIST_PACK_RANGE, 0.0, 1.0);
}

// How much a wave still reveals `world`, given the WALLS between it and
// its source (props are transparent to waves — only walls obstruct). A
// wall is a BARRIER, not a muffle: the wave flows through openings and
// lights the next room through a doorway, and stops dead at a wall. Kind
// does not enter — a cane tap, its echoes, a footstep and a world
// source's hum are all cut crisp alike, which is why this takes no `typ`.
// EVERY wave obeys it, echoes included: an echo is a reflected wave, and
// a reflection is stopped by a wall like any sound — the reflection ray
// only decides where the echo is BORN, never that its reveal may pass
// through a wall.
// wall_blocked_from skips the wall the source is born inside, so a tap or
// an echo struck ON a wall still lights that wall's own near face.
// rust/src/sight.rs::reveal_visibility is the cargo-pinned reference this
// transliterates, over rust/src/sight.rs::blocked_from's verdict.
// hearing_post.gdshader's shell loop asks the SAME predicate of the same
// origin, so the ring in the air and the surfaces it lights stop at one
// wall together.
float source_reveal_vis(vec3 src, vec3 world) {
	return wall_blocked_from(src, world) ? 0.0 : 1.0;
}

// How strongly a wave still reveals a point `since_front` seconds after its
// own front passed there, given the tail its kind is granted.
// rust/src/render/reveal.rs::flare is the cargo-pinned reference, and
// game/tests/shader_contract_test.gd reads the four constants below straight
// out of this text and evaluates them against it — a substring match cannot
// tell 1.3 from 1.0, and this shape is the whole look of the game.
//
// The shape is a fast strike flash plus a slow lingering half. A sum of
// exponentials never reaches zero, so on its own it left 0.068 of peak (a
// tap) or 0.257 (a hum) on every surface a wave ever swept, for as long as
// the pool slot lasted — the wave's visible end was decided by the slot
// allocator.
//
// The CLOSING WINDOW ends it, and a window is the point rather than a
// downward shift: the decay is the acoustic law and the tail is only the
// budget. Subtracting the shape's own tail value would reach zero too, but
// it would darken the whole curve by a kind-dependent amount and make a
// cane tap and its own echo read differently on one surface at one age. The
// window holds at 1.0 for the first three quarters of every wave's life —
// so every kind decays by ONE law until then — and smoothsteps to exactly
// zero at the tail.
const float CLOSE_FRACTION = 0.25;

// WHY THIS CARRIES NO NON-FINITE ARM, when its cargo-pinned reference
// `render::reveal::flare` opens with one. `flare` is a pure function whose
// declared domain is every f64, so totality obliges it. These arguments are
// not open: `since_front` is `age - dist / d.z`, and every term comes from
// the pulse pool, which refuses a non-finite origin, radius or speed at
// `PulsePool::emit` — the boundary where a designer's `#[export]` actually
// enters. Nothing here can manufacture a NaN either, unlike `wall_entry`,
// whose min/max fold genuinely can and which therefore does carry a guard.
// An arm no input can reach is the dead defence this renderer has been
// removing, so the divergence is deliberate and recorded rather than an
// oversight in the transliteration.
float pulse_flare(float since_front, float tail) {
	if (since_front < 0.0 || since_front >= tail) { return 0.0; }
	float shape = 1.3 * exp(-since_front / 0.25) + 0.5 * exp(-since_front / 3.0);
	float opens = tail * (1.0 - CLOSE_FRACTION);
	float closing = 1.0 - smoothstep(opens, tail, since_front);
	return clamp(shape * closing, 0.0, 1.0);
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
		// Seconds since THIS pulse's front passed THIS point — the one
		// coordinate the fade law is written against, so a surface at the
		// wave's full reach fades exactly as the struck spot did, merely
		// later. The radius gate above guarantees it is non-negative.
		float ga = age - dist / d.z;
		float tail = pulse_fade_tail(typ);
		// The wave is OVER here. Not merely dim: gone, at the same moment
		// the pool retires this pulse's slot. Hoisted above the cone gate
		// and the exponentials because a dead pulse must cost one subtract
		// and one compare, not a trace. Without it the loop had NO end
		// condition at all: `dist > min(radius, d.y)` freezes into the
		// static `dist > max_r` once the front has run its course, so every
		// point the wave ever reached kept its reveal until the slot was
		// reused — the visible life of a sound was a property of the slot
		// allocator, not of the sound.
		if (ga >= tail) { continue; }
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
		float flare = pulse_flare(ga, tail);
		// EXACT early-out. The accumulator is a max, and source_reveal_vis
		// only ever returns a value in [0, 1] (1 with a clear line, 0
		// behind a wall) — so `bound` below, computed with that factor at
		// its maximum, is an upper bound on what this pulse could
		// contribute. A pulse that cannot beat what is already accumulated
		// changes nothing, and is dropped BEFORE the wall loop rather than
		// after it. No pixel differs; only the cost.
		//
		// It is NOT the loop's whole affordability, and the comment that
		// said so was left behind by the barrier campaign: once
		// source_reveal_vis became a 0/1 gate, `reveal` stays exactly 0.0
		// on any fragment every in-range pulse is walled off from, and an
		// early-out against 0 can never fire. The fragments that pay the
		// full walk are precisely the ones in the room next door. What
		// actually bounds this loop is the radius gate and the death gate
		// above; this saves the walk only where something is already lit.
		// pulse_flare already clamps to [0, 1], so no min() is needed here.
		float bound = flare * atten * cone * gain * peak;
		if (bound <= reveal) { continue; }
		reveal = max(reveal, bound * source_reveal_vis(u_ppos[i], world));
	}
	return reveal;
}

// Pack (reveal, face-or-role label, camera distance) into the data channels:
// nervous-light modulation and distance fade on R; the per-vertex label
// straight off CUSTOM0 (v_label) on G; packed camera distance on B.
vec3 pack_data(float reveal, vec3 world, vec3 cam) {
	float vd = length(world - cam);
	reveal *= (0.9 + 0.1 * u_flick) * exp(-vd * 0.05);
	// G is clamped like R and B. Every label a level ALLOCATES already lies
	// in [0.15, 0.96], so this changes no shipped pixel — but the value
	// reaching here is not always an allocated one: a solid whose paint pass
	// was rejected keeps the placeholder ordinals nodes::solid::BOX_ORDINALS
	// bakes at build time, which run 0..5 and leave the channel entirely.
	// Writing those unclamped hands the target a value the rest of the
	// pipeline has no defined reading of. It is still a broken picture — the
	// deeper repair is a reserved in-band sentinel the hearing pass can
	// RECOGNISE, so an unpainted world announces itself instead of merely
	// looking seamless — but a broken picture with defined arithmetic.
	return vec3(
			clamp(reveal, 0.0, 1.0),
			clamp(v_label, 0.0, 1.0),
			clamp(vd / DIST_PACK_RANGE, 0.0, 1.0));
}
