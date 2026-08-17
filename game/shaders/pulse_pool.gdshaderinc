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
// max_z), swept y in [0, u_wall_top] — pushed by the level. Both wave
// passes read them through the SAME law, whatever kind made the wave: the
// data core refuses a source's REVEAL past a wall (a wall stops a wave
// from lighting a surface), and the hearing pass refuses its SHELL past
// one (the ring in the air does not pass a wall either). Both ask
// wall_blocked_from, keyed to the SOUND'S OWN ORIGIN — a depth test
// against the eye is a different question and answers it correctly only
// while the sound was made on the camera's side of the world.
//
// The CAMERA occluder (wall_crossings) is the separate law: it dims a
// source's SILHOUETTE per wall and drops a ring over an x-rayed source,
// and it must test the walls analytically rather than lean on depth,
// because the always-on-top skins corrupt the packed depth at their own
// pixels.
// rust/src/sight.rs is the cargo-pinned reference this GLSL transliterates.
const int MAXW = 32;
uniform int u_wall_count = 0;
uniform vec4 u_walls[MAXW];  // world XZ occluder rects (min_x, min_z, max_x, max_z)
uniform float u_wall_top = 3.0;

// sight.rs::near — the EXACT cheap refusal in front of the slab test: a
// segment that crosses the rect has a point inside it, so its own XZ
// bounding box must overlap the rect. Four comparisons throw out a wall
// across the map before the slab test spends three divisions on it, which
// is what keeps the per-fragment sight loop affordable as the map grows.
bool wall_near(vec3 from, vec3 to, vec4 rect) {
	return min(from.x, to.x) <= rect.z && max(from.x, to.x) >= rect.x
			&& min(from.z, to.z) <= rect.w && max(from.z, to.z) >= rect.y;
}

// sight.rs::crosses — the three-slab segment-vs-AABB test over
// y in [0, top], total on axis-parallel directions, both parametric
// endpoints ignored inside the graze window.
bool wall_crosses(vec3 from, vec3 to, vec4 rect, float top) {
	if (!wall_near(from, to, rect)) { return false; }
	vec3 d = to - from;
	vec3 lo = vec3(rect.x, 0.0, rect.y);
	vec3 hi = vec3(rect.z, top, rect.w);
	float t0 = 0.001;
	float t1 = 0.999;
	for (int k = 0; k < 3; k++) {
		if (abs(d[k]) < 1e-6) {
			if (from[k] < lo[k] || from[k] > hi[k]) { return false; }
		} else {
			float ta = (lo[k] - from[k]) / d[k];
			float tb = (hi[k] - from[k]) / d[k];
			t0 = max(t0, min(ta, tb));
			t1 = min(t1, max(ta, tb));
			if (t0 > t1) { return false; }
		}
	}
	return true;
}

// sight.rs::crossings — the CAMERA occluder: every wall the sight line
// pierces (the source-silhouette muffle, and the x-rayed-source ring cut).
int wall_crossings(vec3 from, vec3 to) {
	int n = 0;
	for (int i = 0; i < MAXW; i++) {
		if (i >= u_wall_count) { break; }
		if (wall_crosses(from, to, u_walls[i], u_wall_top)) { n++; }
	}
	return n;
}

// sight.rs::contains — the point lies inside the wall box, floor to ceiling.
bool wall_contains(vec4 rect, vec3 p, float top) {
	return p.x >= rect.x && p.x <= rect.z && p.z >= rect.y && p.z <= rect.w
			&& p.y >= 0.0 && p.y <= top;
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
		if (wall_contains(u_walls[i], from, u_wall_top)) { continue; }
		if (wall_crosses(from, to, u_walls[i], u_wall_top)) { return true; }
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
