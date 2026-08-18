// The pulse-pool contract shared by data_pass and hearing_post.
// The two shaders are one system communicating through packed channels;
// this include is the single definition of that protocol.
//
// Slot layout (mirrors pulses.gd):
//   u_ppos[i] — world origin of the sound
//   u_pdat[i] — (birth time, max radius, speed m/s, type*10 + gain*9)
//   u_pdir[i] — beam direction xyz + cos(half-angle); w = -2 means omni

const int MAXP = 64;

// Camera distance is packed into the data pass's BLUE channel divided by
// this range; the hearing pass multiplies it back. Camera far (60) exceeds
// it, which is safe only because the map is 28 m across, keeping the
// floor-and-ceiling slab pair's own diagonal under this range.
const float DIST_PACK_RANGE = 40.0;

uniform float u_time = 0.0;
uniform float u_flick = 1.0;
uniform int u_count = 0;
uniform vec3 u_ppos[MAXP];
uniform vec4 u_pdat[MAXP];
uniform vec4 u_pdir[MAXP];

// The level's walls as world XZ occluder rects (min_x, min_z, max_x,
// max_z), each swept through its own y span (u_wall_y) — pushed by the
// level. Both wave
// passes read them through the SAME law, whatever kind made the wave: the
// data core refuses a source's REVEAL past a wall (a wall stops a wave
// from lighting a surface), and the hearing pass refuses its SHELL past
// one (the ring in the air does not pass a wall either). Both ask
// wall_blocked_from, keyed to the SOUND'S OWN ORIGIN — a depth test
// against the eye is a different question and answers it correctly only
// while the sound was made on the camera's side of the world.
//
// The CAMERA occluder (wall_any_crossing) is the separate law: it dims a
// source's SILHOUETTE per wall and drops a ring over an x-rayed source,
// and it must test the walls analytically rather than lean on depth,
// because the always-on-top skins corrupt the packed depth at their own
// pixels.
// rust/src/sight.rs is the cargo-pinned reference this GLSL transliterates.
const int MAXW = 32;
uniform int u_wall_count = 0;
uniform vec4 u_walls[MAXW];  // world XZ occluder rects (min_x, min_z, max_x, max_z)
// Each wall's OWN world y sweep, (bottom, top) — sight.rs::Occluder::span.
//
// Two arrays because std140 forces two, but they are projections of one
// Vec<Occluder> on the Rust side and cannot desync in order or length. This
// used to be a single `u_wall_top` float, with every occluder swept
// [0, WALL_H] wherever its wall actually stood — so a wall lifted with the
// gizmo left a phantom barrier in the air beneath it and an unoccluded strip
// across its raised top, and a level root lifted bodily put every occluder
// below the map, failing the barrier law open everywhere in silence.
//
// The default is deliberately a ZERO-HEIGHT span at y = 0, not the shipped
// (0, 3): a material nobody pushed must lose the barrier law LOUDLY. A
// default that happened to be right on the shipped map would ship the bug
// invisibly.
uniform vec2 u_wall_y[MAXW];

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
// always-on-top value minus a hair proportional to its true distance, so
// sources — and a single source's own overlapping limbs — resolve by
// distance like everything else.
//
// THE WIDTH IS DERIVED, not chosen. rust/src/render/depth.rs states both
// bounds as functions and cargo-tests them against the shipped camera and
// the shipped fan:
//   - wide enough to ORDER. The band is quantised by the 24-bit depth
//     buffer and, identically, by f32's own 2^-24 ULP near 1.0, so it
//     carries only band * 2^24 distinct values across DIST_PACK_RANGE. The
//     old 1.0e-5 carried 168 of them over 40 m — one code every 0.238 m,
//     coarser than a fan is big, so its housing, guard and blades all wrote
//     the SAME depth and resolved by opaque draw order. Since a blade's
//     sort key moves as it spins, the crease pattern on the fan head
//     reshuffled instead of rotating. 1.0e-3 resolves 2.4 mm, comfortably
//     under the 12 mm between that guard and those blades;
//   - narrow enough to stay UNREACHABLE. Under reversed-Z with near 0.05
//     and far 60, a world surface must stand within 0.05005 m of the eye to
//     reach a 1.0e-3 band — 0.05 mm past a near plane that already clips
//     everything closer.
const float SOURCE_BAND = 1.0e-3;


// Is the fragment at `depth` an ACOUSTIC IMAGE rather than a real surface?
//
// One comparison, and it is exact. The source skin is opaque and writes its
// DEPTH inside the always-on-top band above, so any fragment whose depth
// lands in that band IS a source — no table, no reconstruction, no
// inference. Measured on desktop GL by
// game/tests/probe/depth_texture_probe.gd: an always-on-top fragment reads
// back at 1.0000 against a band floor of 0.9990, while a world surface 3 m
// from the eye reads 0.0158 — the analytic reversed-Z value to four
// decimals.
//
// WHY IT IS STILL NOT THE ONLY TEST. Both shipped targets are now measured
// and both agree: the depth texture is live on desktop GL and under the
// browser (tools/measure_web_platform.sh reads 0.9490 against an analytic
// 0.9508 in each). data_core had carried the claim that it was unreliable
// on WebGL2 for months, and that claim was simply wrong.
//
// The web reading has since been taken twice more: once under SwiftShader,
// the software rasteriser the browser gate uses, and once under ANGLE on
// an Apple Metal GPU, which answers 0.9490 as well. Three drivers, two
// targets, one number.
//
// hearing_post nonetheless ORs this against the older wall-table inference
// rather than replacing it, for the one reason no measurement can retire:
// three drivers are not every driver. The OR costs one comparison and
// cannot regress anywhere — if some future driver returns nothing, this is
// false everywhere and the pass behaves exactly as it did before the depth
// read existed.
bool depth_is_acoustic_image(float depth) {
	return depth >= ALWAYS_ON_TOP - SOURCE_BAND;
}

// sight.rs::near — the EXACT cheap refusal in front of the slab test: a
// segment that crosses the rect has a point inside it, so its own XZ
// bounding box must overlap the rect. Four comparisons throw out a wall
// across the map before the slab test spends three divisions on it, which
// is what keeps the per-fragment sight loop affordable as the map grows.
bool wall_near(vec3 from, vec3 to, vec4 rect) {
	return min(from.x, to.x) <= rect.z && max(from.x, to.x) >= rect.x
			&& min(from.z, to.z) <= rect.w && max(from.z, to.z) >= rect.y;
}

// sight.rs::Occluder::is_empty — an occluder describing no volume must stop
// nothing, and the slab arithmetic alone does NOT deliver that: an inverted
// interval still yields a valid parametric window (lo = 1, hi = -1 against a
// segment from -5 to +5 gives ta = 1, tb = -1, so t0 stays 0.001 and t1
// stays 0.999 and no axis rejects).
//
// Written !(a <= b) and never a > b. Every GLSL comparison against NaN is
// false, so the `>` form reads a NaN lane as ORDERED and hands it to a slab
// test where t0 > t1 is also false — an occluder that swallows the level.
bool wall_empty(vec4 rect, vec2 yspan) {
	return !(rect.x <= rect.z) || !(rect.y <= rect.w) || !(yspan.x <= yspan.y);
}

// sight.rs::crosses — the three-slab segment-vs-AABB test over the wall's
// OWN y sweep, total on axis-parallel directions, both parametric endpoints
// ignored inside the graze window.
// A miss, as a fraction past the far end of the segment. Any value > 1.0
// would do; 2.0 is chosen so a min() fold over a whole table stays finite
// and readable in a debugger.
const float WALL_MISS = 2.0;

// sight.rs::entry — WHERE the segment first enters this wall, as a fraction
// of itself, or WALL_MISS if it never does. The same three-slab arithmetic
// that used to answer only yes-or-no, now reporting the t0 it always had.
float wall_entry(vec3 from, vec3 to, vec4 rect, vec2 yspan) {
	if (wall_empty(rect, yspan)) { return WALL_MISS; }
	if (!wall_near(from, to, rect)) { return WALL_MISS; }
	vec3 d = to - from;
	vec3 lo = vec3(rect.x, yspan.x, rect.y);
	vec3 hi = vec3(rect.z, yspan.y, rect.w);
	float t0 = 0.001;
	float t1 = 0.999;
	for (int k = 0; k < 3; k++) {
		if (abs(d[k]) < 1e-6) {
			if (from[k] < lo[k] || from[k] > hi[k]) { return WALL_MISS; }
		} else {
			float ta = (lo[k] - from[k]) / d[k];
			float tb = (hi[k] - from[k]) / d[k];
			t0 = max(t0, min(ta, tb));
			t1 = min(t1, max(ta, tb));
			if (t0 > t1) { return WALL_MISS; }
		}
	}
	return t0;
}

// sight.rs::crosses — now a READING of wall_entry, so the two can never
// disagree about whether a wall is in the way while disagreeing about
// where it is.
//
// THE NEGATION IS LOAD-BEARING. `!(x > 1.0)` and `x <= 1.0` differ on NaN:
// every comparison against NaN is false, so the first answers TRUE (this
// wall blocks) and the second answers FALSE (it does not). GLSL leaves
// max/min with a NaN operand implementation-defined, so a NaN really can
// reach here — and wall_blocked_from calls this to stop WAVES. Failing
// toward "blocks" is the direction settled law 3 requires and the one
// wall_empty's own `!(a <= b)` discipline exists to hold. (The Rust twin
// cannot produce the NaN at all: Rust's max/min suppress it. The
// asymmetry is real, measured, and recorded in sight.rs::entry.)
bool wall_crosses(vec3 from, vec3 to, vec4 rect, vec2 yspan) {
	return !(wall_entry(from, to, rect, yspan) > 1.0);
}

// sight.rs::first_entry — the NEAREST wall entry along the segment, as a
// fraction of it, or WALL_MISS if the line is clear.
float wall_first_entry(vec3 from, vec3 to) {
	float first = WALL_MISS;
	for (int i = 0; i < MAXW; i++) {
		if (i >= u_wall_count) { break; }
		first = min(first, wall_entry(from, to, u_walls[i], u_wall_y[i]));
	}
	return first;
}

// Does ANY wall stand on this sight line? The CAMERA occluder asked as the
// question its callers actually have.
//
// This was `wall_crossings`, returning a COUNT, and all four of its call
// sites compared it `> 0` and threw the count away — so every outline pixel
// in the game paid `4 * u_wall_count` full slab tests, 104 at the shipped
// 26 walls, to answer four booleans the first crossing already settles.
// Returning on the first hit is the shape `wall_blocked_from` below already
// uses, for the reason its own comment gives.
//
// NO birth-wall skip: that is `wall_blocked_from`'s separate law — a sound
// born flush on a wall lights its own near face — and inheriting it here
// would quietly change which walls cut a ring. Rust's `sight::crossings`
// still counts, because the muffle ladder on the CPU genuinely needs the
// number; nothing in GLSL does.
bool wall_any_crossing(vec3 from, vec3 to) {
	for (int i = 0; i < MAXW; i++) {
		if (i >= u_wall_count) { break; }
		if (wall_crosses(from, to, u_walls[i], u_wall_y[i])) { return true; }
	}
	return false;
}

// sight.rs::contains — the point lies inside the wall box, over that wall's
// OWN sweep. This half failed in the DANGEROUS direction under one global
// top: a source standing in open air UNDER a lifted wall was judged born
// inside it, so the birth-wall skip disabled that wall for that source in
// every direction at once.
bool wall_contains(vec4 rect, vec3 p, vec2 yspan) {
	if (wall_empty(rect, yspan)) { return false; }
	return p.x >= rect.x && p.x <= rect.z && p.z >= rect.y && p.z <= rect.w
			&& p.y >= yspan.x && p.y <= yspan.y;
}

// sight.rs::blocked_from — the SOURCE occluder, asked as the PREDICATE it
// really is: does ANY wall stand between the source and the point, except
// the one the source is born inside (a tap struck flush on a wall lights
// its own near face)?
//
// A wall is a barrier and not a fade, so neither reader needs a count —
// one wall extinguishes a wave exactly as ten do. Returning on the FIRST
// wall found instead of walking all MAXW slots is what keeps the law
// affordable in the hearing pass, which pays it per fragment per live
// pulse per sphere root where the data core pays it once per fragment.
bool wall_blocked_from(vec3 from, vec3 to) {
	for (int i = 0; i < MAXW; i++) {
		if (i >= u_wall_count) { break; }
		if (wall_contains(u_walls[i], from, u_wall_y[i])) { continue; }
		if (wall_crosses(from, to, u_walls[i], u_wall_y[i])) { return true; }
	}
	return false;
}

float pulse_type(vec4 d) {
	return floor(d.w / 10.0);
}

float pulse_gain(vec4 d) {
	return mod(d.w, 10.0) / 9.0;
}

// rust/src/pulse_pool.rs::fade_tail — seconds a point keeps hearing a wave
// after the front passed it, and the very number the CPU retires the slot
// with (emit() sets end = now + max_r/speed + fade_tail). Both readers must
// agree or the wave outlives its own data: echoes and footsteps are granted
// less than taps precisely so their slots free fast.
//
// The chain matches the Rust `match`'s wildcard arm exactly: kind 0 and any
// kind outside 0..3 — which a tampered or future emit could pack — take the
// tap's long tail rather than falling through to an undefined value.
float pulse_fade_tail(float typ) {
	if (typ < 0.5) { return 6.0; }
	if (typ < 1.5) { return 3.5; }
	if (typ < 2.5) { return 2.5; }
	if (typ < 3.5) { return 2.0; }
	return 6.0;
}

// Directed sound: only the cone the sound was aimed into carries information.
// dir_from_src must be the unit vector from the pulse origin toward the point.
float pulse_cone(vec4 cn, vec3 dir_from_src) {
	return smoothstep(cn.w - 0.15, cn.w + 0.05, dot(dir_from_src, cn.xyz));
}
