shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, depth_test_disabled, fog_disabled;

// HEARING PASS — a fullscreen quad pinned to the camera. Everything the
// player ever sees is produced here, from two ingredients:
//   1. thin white OUTLINES where geometry breaks (silhouettes via a distance
//      Laplacian, creases via per-vertex face-or-role LABEL differences),
//      gated by the per-pixel "reveal" the data pass wrote — outlines exist
//      only where a wave swept;
//   2. the WAVES themselves: expanding spherical shells ray-traced in air,
//      limb-brightened into rings, occluded by the world (walls carve bites
//      out of a ring — absence of wave IS the shape of the world).
// Plus the mood layer: breathing vignette and film grain.

#include "res://shaders/pulse_pool.gdshaderinc"

uniform sampler2D screen_tex : hint_screen_texture, filter_nearest;
// The depth buffer, read to answer one question exactly that the wall table
// could only guess at: is the surface at this pixel an acoustic image drawn
// over the world, or a real surface? See depth_is_acoustic_image in the
// pool include for the measurement behind it, and for why it is ORed with
// the old inference rather than replacing it.
uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
uniform float u_breath = 1.0;
// The crease knee: a label difference of u_crease_knee.y draws a seam at
// full strength, and one below u_crease_knee.x draws none.
//
// It is DERIVED in Rust from render::labels::MIN_SEP, the very separation
// the colouring allocates against, and pushed by the composition root —
// rust/src/render/crease.rs owns it. The two are the same law seen from two
// ends and they used to be two literals that never met: lowering MIN_SEP to
// fit a starved label band kept every cargo test green while this shader
// went on fading over a knee it no longer matched, drawing the seams the
// allocator had just approved at a fraction of strength.
//
// The default is deliberately NOT the shipped (0.04, 0.08). A post material
// nobody pushed draws every seam at full strength regardless of label
// distance, which is loudly wrong; a default that happened to be right
// would hide a broken push forever.
uniform vec2 u_crease_knee = vec2(0.0, 1.0);
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

	vec3 ndc_far = vec3(uv * 2.0 - 1.0, 1.0);
	vec4 vf = INV_PROJECTION_MATRIX * vec4(ndc_far, 1.0);
	vf.xyz /= vf.w;
	vec3 world_far = (INV_VIEW_MATRIX * vec4(vf.xyz, 1.0)).xyz;
	vec3 rd = normalize(world_far - cam);
	float scene_d = c_c.b * DIST_PACK_RANGE;
	// the surface visible at this pixel: if it lies BEHIND a wall from the
	// eye it is an always-on-top skin x-rayed through the wall (a sound
	// source) — no player-made ring may wash it. A real-depth surface is
	// never behind a wall (the wall would hide it), so this fires only on
	// the x-rayed sources.
	// TWO tests, ORed, and the pair is deliberate.
	//
	// The depth read is EXACT: a fragment in the always-on-top band is a
	// source, full stop. The wall-table inference underneath it is not —
	// it rebuilds a world point from B and asks which walls stand there,
	// and WaveProp, WaveColumn and WaveWedge are in no occluder table
	// anywhere, so a source hidden behind a pillar was simply missed: its
	// rings were never cut and its outline borrowed the pillar's
	// brightness. The depth test covers exactly that case.
	//
	// The inference stays because the depth texture is measured on desktop
	// GL and unmeasured on WebGL2. If it is dead there, the first term is
	// false everywhere and this pass behaves precisely as it did before —
	// which is why the OR is the safe shape and a replacement would not be.
	vec3 seen_pt = cam + rd * scene_d;
	bool seen_walled = depth_is_acoustic_image(texture(depth_tex, uv).r)
			|| wall_crossings(cam, seen_pt) > 0;

	// thin white outlines: silhouettes (Laplacian of the packed distance in B)
	// + creases (label differences in G — the per-vertex CUSTOM0 label:
	// derive-painted face labels for world solids, per-instance labels for
	// semantic source roles, and fixed role labels for creatures/viewmodel),
	// visible only where a wave has
	// swept (max of the five reveal taps in R). Two touching world faces
	// sharing the SAME label draw no seam at all: see
	// rust/src/render/labels.rs (MIN_SEP) for the 0.08 separation the
	// smoothstep below demands.
	float lap = abs(c_l.b + c_r.b + c_u.b + c_d.b - 4.0 * c_c.b);
	float nrm = abs(c_l.g - c_r.g) + abs(c_u.g - c_d.g);
	float edge = max(smoothstep(0.012, 0.03, lap), smoothstep(u_crease_knee.x, u_crease_knee.y, nrm));
	float reveal = max(c_c.r, max(max(c_l.r, c_r.r), max(c_u.r, c_d.r)));
	// an x-rayed source's OUTLINE must never borrow the lit wall behind it.
	// Its silhouette is a distance edge shared with that wall, so BOTH
	// sides — the source's pixels AND the wall's pixels touching it — must
	// be capped to the source's OWN dim reveal. Only at edge pixels
	// (cheap): check whether any of the five taps is a surface seen through
	// a wall (an x-rayed source), and if so clamp the reveal to that tap's.
	if (edge > 0.0) {
		float src_r = -1.0;
		if (seen_walled) { src_r = c_c.r; }
		// the same pair at each neighbour: the exact depth test first, the
		// wall-table inference as the platform fallback behind it
		if (depth_is_acoustic_image(texture(depth_tex, uv - vec2(px.x, 0.0)).r)
				|| wall_crossings(cam, cam + rd * (c_l.b * DIST_PACK_RANGE)) > 0) { src_r = max(src_r, c_l.r); }
		if (depth_is_acoustic_image(texture(depth_tex, uv + vec2(px.x, 0.0)).r)
				|| wall_crossings(cam, cam + rd * (c_r.b * DIST_PACK_RANGE)) > 0) { src_r = max(src_r, c_r.r); }
		if (depth_is_acoustic_image(texture(depth_tex, uv + vec2(0.0, px.y)).r)
				|| wall_crossings(cam, cam + rd * (c_u.b * DIST_PACK_RANGE)) > 0) { src_r = max(src_r, c_u.r); }
		if (depth_is_acoustic_image(texture(depth_tex, uv - vec2(0.0, px.y)).r)
				|| wall_crossings(cam, cam + rd * (c_d.b * DIST_PACK_RANGE)) > 0) { src_r = max(src_r, c_d.r); }
		if (src_r >= 0.0) { reveal = min(reveal, src_r); }
	}
	vec3 col = vec3(edge * reveal);

	// the waves themselves: rings in the air, cut where the world stands

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
			vec3 hp = cam + rd * t;
			// Two DIFFERENT laws cut a ring, and both are needed.
			//
			// First the CAMERA's: the eye cannot see air the world stands
			// in front of, and none of it washes an x-rayed source seen
			// through a wall. seen_walled is not redundant with the depth
			// test — the always-on-top source skins corrupt packed depth at
			// their own pixels, so scene_d alone cannot be trusted on a ray
			// that reaches a source through a wall.
			if (t >= scene_d || seen_walled) { continue; }
			vec3 sn = (hp - u_ppos[i]) / radius;
			float cone = pulse_cone(cn, sn);
			if (cone <= 0.0) { continue; }
			// ...and then the WAVE's, which the depth test above cannot
			// stand in for. A wall is a barrier no sound crosses, so this
			// ring only exists here if the sound could REACH here from its
			// own origin — the same wall_blocked_from, keyed to the same
			// u_ppos[i], that data_core's source_reveal_vis asks of the
			// surfaces this ring lights.
			//
			// The depth test agrees with it only while the sound was made
			// on the camera's side of the world. It never is for a world
			// source in another room: that sphere, once it has grown past
			// the wall, meets the view ray at a NEAR root in front of the
			// wall — inside the hero's own air, where t < scene_d — so the
			// one part of the shell the depth test still draws is exactly
			// the part that leaked through. Kind does not enter here any
			// more than it does in the reveal.
			//
			// Paid LAST on purpose: this is a per-fragment wall walk per
			// live pulse per sphere root, so it sits behind the disc, the
			// t, the depth and the cone rejections, which are all arithmetic
			// on values already in hand.
			if (wall_blocked_from(u_ppos[i], hp)) { continue; }
			float grz = 1.0 - abs(dot(rd, sn));
			// a hum's shell has a diffuse body, not just a grazing ring — a
			// constant source is a throb the whole room can feel
			float body = typ > 2.5 ? 0.22 : 0.05;
			float ink = 0.55 + 0.45 * n2(hp.xz * 1.7 + hp.y * 0.9);
			col += vec3(env * cone * (body + (1.0 - body) * pow(max(grz, 0.0), 4.0)) * ink * exp(-t * 0.09));
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
