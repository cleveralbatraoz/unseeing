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
// The default is deliberately NOT the shipped (0.04, 0.08), so a post
// material nobody pushed is visibly wrong rather than accidentally right.
// It is wrong in the DIM direction, not the bright one: smoothstep(0, 1, x)
// at a genuine MIN_SEP seam of 0.08 gives 0.0182, so creases all but
// vanish while silhouettes carry on drawing. An earlier version of this
// comment claimed the opposite — every seam at full strength — which is not
// what the arithmetic does.
uniform vec2 u_crease_knee = vec2(0.0, 1.0);
// Where a swept ACOUSTIC IMAGE stops telling you WHAT it is while still
// telling you THAT it is there — the knee applies to image fragments
// only; the world's creases never pass through it (see the composition
// below for the playtest that scoped it). Derived in Rust from the wall
// factor (render::detail::DetailKnee::shipped) and pushed by the
// composition root. Defaults to (0.0, 1.0) — a fade that runs the whole
// range and so dims every source's creases — because an unpushed post
// material must be visibly wrong rather than accidentally right, exactly
// as u_crease_knee's default is.
uniform vec2 u_detail_knee = vec2(0.0, 1.0);
// The silhouette knee, in METRES of depth step: a break of u_sil_knee.y
// draws a full outline and one below u_sil_knee.x draws none.
//
// It used to be the literal pair (0.012, 0.03) in CHANNEL units, which made
// the outline a hidden function of DIST_PACK_RANGE — and
// level_plan::pack_range_budget exists to tell a designer to raise
// DIST_PACK_RANGE when the map outgrows it. Following that from 40 m to 60 m
// would have taken a 0.8 m step from drawing at 42% to drawing at 1.6%.
// rust/src/render/silhouette.rs owns the pair and the composition root
// pushes it.
//
// The default draws NOTHING, which is a long way outside any depth step a
// room contains. An unpushed post material must be visibly wrong rather than
// accidentally right, and it fails DIM like every other uniform on this
// pass: a black screen is unmissable where a slightly-too-eager outline is
// not.
uniform vec2 u_sil_knee = vec2(1.0e4, 2.0e4);

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
	float scene_d = unpack_distance(c_c.b);
	// the surface visible at this pixel: if it lies BEHIND a wall from the
	// eye it is an always-on-top skin x-rayed through the wall (a sound
	// source) — no player-made ring may wash it. A real-depth surface is
	// never behind a wall (the wall would hide it), so this fires only on
	// the x-rayed sources.
	// TWO DIFFERENT QUESTIONS, and they must not be conflated — conflating
	// them is a bug this pass shipped for exactly one commit.
	//
	//   seen_walled — is there something BETWEEN THE EYE AND the surface at
	//   this pixel? Only the wall table can answer that. A depth read
	//   cannot: at an x-rayed pixel the depth buffer holds the SOURCE's
	//   faked always-on-top value, not the occluder's, so it knows nothing
	//   about what stands in front.
	//
	//   seen_image — is the surface at this pixel an acoustic image at all?
	//   Here the depth read is exact and the wall table is only a proxy.
	//
	// The ring cut below needs the FIRST. Feeding it the second drops
	// player rings over every source pixel in the game, including a fan
	// standing in the open with nothing between it and the eye, because
	// every source fragment is an acoustic image by definition.
	//
	// The outline cap needs the SECOND, and that is where the depth read
	// earns its place: the wall table misses a source hidden behind a solid
	// the admission rule REFUSED — a crate, a barrel, a knee-high wedge — so
	// its outline borrowed that solid's brightness. ORed, not substituted,
	// because three measured drivers are not every driver: where the depth
	// texture is dead the term is false everywhere and the cap degrades to
	// precisely its former behaviour.
	//
	// WHAT IS STILL NOT FIXED, stated so nobody has to rediscover it: the
	// prop gap is closed for the outline cap and OPEN for the ring cut. A
	// player's ring still washes over a source hidden behind a NON-SPANNING
	// solid, because that needs to know something stands in FRONT of the
	// source and neither surface can say so — the depth buffer holds the
	// source's own faked value there, and only spanning solids reach
	// `u_walls`. It is a smaller gap than it was: a source behind a PILLAR
	// is already covered, because level_plan::spans_the_corridor admits
	// pillars and `wall_first_entry` below finds them. Closing the rest
	// means giving the CAMERA occluder a table that includes the refused
	// solids — a different law from the wave occluder, which admits by
	// geometry rather than by class — and a per-fragment cost on the
	// hottest path in the game.
	// The endpoint every wall test gets is the BIASED probe, never the raw
	// reading: probe_distance pulls it one reconstruction gap toward the
	// eye, because a reading that comes back a hair high places this
	// surface inside its own wall and the pixel then reads as a source
	// seen THROUGH it — see the include. The ring cut keeps the unbiased
	// scene_d: rings die at the surface itself, not nine centimetres in
	// front of it.
	float probe_d = probe_distance(c_c.b);
	vec3 seen_pt = cam + rd * probe_d;
	// sight.rs::visible_air — HOW FAR the eye can see air along this ray:
	// the whole way to the surface, or only as far as the nearest wall it
	// enters. A DISTANCE, never a flag; see the ring cut below for why that
	// distinction shipped as a visible defect.
	float wall_t = wall_first_entry(cam, seen_pt);
	bool seen_walled = !(wall_t > 1.0);
	// The inner guard is the GLSL twin of visible_air's non-finite arm, and
	// it is NOT redundant here the way it is in Rust: GLSL leaves max/min
	// with a NaN operand implementation-defined, so wall_t really can come
	// back NaN. Left unguarded, air_d would be NaN, `t >= air_d` would be
	// false for EVERY root, and one bad pixel would draw every ring in the
	// level straight through every wall.
	float air_d = seen_walled ? (wall_t >= 0.0 ? wall_t * probe_d : 0.0) : scene_d;
	bool seen_image = depth_is_acoustic_image(texture(depth_tex, uv).r) || seen_walled;

	// thin white outlines: silhouettes (Laplacian of the packed distance in B)
	// + creases (label differences in G — the per-vertex CUSTOM0 label:
	// derive-painted face labels for world solids, per-instance labels for
	// semantic source roles, and fixed role labels for creatures/viewmodel),
	// visible only where a wave has
	// swept (max of the five reveal taps in R). Two touching world faces
	// sharing the SAME label draw no seam at all: see
	// rust/src/render/labels.rs (MIN_SEP) for the 0.08 separation the
	// smoothstep below demands.
	// SHAPE, in METRES. The five coefficients sum to zero, so the packed
	// floor cancels exactly and the whole of the packing that survives here
	// is its gain — which is why the knee below can be stated in metres of
	// depth step and stop depending on DIST_PACK_RANGE.
	// rust/src/render/silhouette.rs::outline is the cargo-pinned reference.
	float lap = abs(c_l.b + c_r.b + c_u.b + c_d.b - 4.0 * c_c.b) * DIST_UNPACK_SCALE;
	float nrm = abs(c_l.g - c_r.g) + abs(c_u.g - c_d.g);
	// SHAPE and DETAIL are two different facts and get two different laws.
	// They were fused here — one max(), one shared reveal — and the whole
	// perception model was inside the distinction that fusing threw away.
	// rust/src/render/detail.rs carries the derivation and the theorem.
	float sil = smoothstep(u_sil_knee.x, u_sil_knee.y, lap);
	float crease = smoothstep(u_crease_knee.x, u_crease_knee.y, nrm);
	// the cap below must still fire on EXACTLY the pixels it fired on
	// before the split, so it keeps asking the fused question
	float edge = max(sil, crease);
	float reveal = max(c_c.r, max(max(c_l.r, c_r.r), max(c_u.r, c_d.r)));
	// an x-rayed source's OUTLINE must never borrow the lit wall behind it.
	// Its silhouette is a distance edge shared with that wall, so BOTH
	// sides — the source's pixels AND the wall's pixels touching it — must
	// be capped to the source's OWN dim reveal. Only at edge pixels
	// (cheap): check whether any of the five taps is a surface seen through
	// a wall (an x-rayed source), and if so clamp the reveal to that tap's.
	if (edge > 0.0) {
		float src_r = -1.0;
		if (seen_image) { src_r = c_c.r; }
		// the same pair at each neighbour: the exact depth test first, the
		// wall-table inference as the platform fallback behind it
		if (depth_is_acoustic_image(texture(depth_tex, uv - vec2(px.x, 0.0)).r)
				|| wall_any_crossing(cam, cam + rd * probe_distance(c_l.b))) { src_r = max(src_r, c_l.r); }
		if (depth_is_acoustic_image(texture(depth_tex, uv + vec2(px.x, 0.0)).r)
				|| wall_any_crossing(cam, cam + rd * probe_distance(c_r.b))) { src_r = max(src_r, c_r.r); }
		if (depth_is_acoustic_image(texture(depth_tex, uv + vec2(0.0, px.y)).r)
				|| wall_any_crossing(cam, cam + rd * probe_distance(c_u.b))) { src_r = max(src_r, c_u.r); }
		if (depth_is_acoustic_image(texture(depth_tex, uv - vec2(0.0, px.y)).r)
				|| wall_any_crossing(cam, cam + rd * probe_distance(c_d.b))) { src_r = max(src_r, c_d.r); }
		if (src_r >= 0.0) { reveal = min(reveal, src_r); }
	}
	// DETAIL is the ACOUSTIC IMAGE's law, gated on the FINAL reveal, after
	// the x-ray cap above has had its say. Both halves of that sentence are
	// load-bearing and each was learned the hard way:
	//
	// THE SCOPE. The knee fades an IMAGE's creases only; the world's pass
	// through at full strength. Gating every crease in the game shipped
	// first, and a playtest rejected it in one sentence per symptom — a
	// footstep's reveal peaks at 0.330 one metre out, under the knee's own
	// 0.30 floor, so a walking hero swept a room whose corners, floor
	// seams and face edges never drew, and a tap's crease field tore off
	// at the 0.30 reveal contour mid-object. A world surface behind a wall
	// already reveals NOTHING (source_reveal_vis is a hard gate), so
	// world-side gating bought no theorem and cost the whole picture.
	// rust/src/render/detail.rs::gate is the cargo-pinned twin.
	//
	// THE ORDERING. `reveal` is rewritten inside the cap block, so
	// computing `detail` any earlier would gate identity on a number the
	// cap is about to lower — a source's creases would survive precisely
	// because it is being dimmed for standing behind a wall.
	//
	// The knee opens at SOURCE_THROUGH, which is the ceiling on a
	// once-walled source's reveal, so the composition below is a theorem
	// rather than a tuning: a source behind a wall CANNOT draw a crease.
	// You always know it is sounding in there; you stop knowing it is a
	// radio. rust/src/render/detail.rs::the_first_wall_takes_a_sources_identity.
	float detail = seen_image ? smoothstep(u_detail_knee.x, u_detail_knee.y, reveal) : 1.0;
	// With detail == 1 — every world fragment — this is
	// max(sil, crease) * reveal, bit-for-bit the fused line it replaces.
	vec3 col = vec3(max(sil * reveal, crease * detail * reveal));

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
			// The CAMERA's law: the eye cannot see air that something
			// stands in front of. ONE comparison, against a DISTANCE.
			//
			// This shipped as the surface compare ORed with a walled FLAG,
			// and the OR was a real defect. A boolean is fragment-constant:
			// it killed
			// EVERY root at the pixel, including rings physically nearer
			// than the wall that set it. And because an x-rayed source's
			// skin takes the pixel from the wall behind it, the flag was
			// true across that source's whole screen-space silhouette — so
			// a source seen through a wall punched a source-shaped HOLE in
			// rings that had every right to be drawn, in the hero's own air.
			//
			// air_d folds every ender of the eye's view — the visible
			// surface, and the nearest wall the ray enters — with a
			// minimum. A root nearer than all of them survives, which is
			// the whole point.
			if (t >= air_d) { continue; }
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
