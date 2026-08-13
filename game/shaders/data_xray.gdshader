shader_type spatial;
render_mode unshaded, cull_back, shadows_disabled, ambient_light_disabled, fog_disabled;

// DATA X-RAY — the ACOUSTIC IMAGE skin, worn by EVERY world sound source:
// the fan, the radio, whatever comes next. Its fragments write the
// ALWAYS_ON_TOP depth, so the hardware depth test can never occlude it —
// the acoustic image passes over the world, felt through walls. A wall does
// not hide a source, only DIMS it. cull_back is mandatory: under an
// always-pass depth test a mesh's own back faces would stomp its front
// faces in draw order.
//
// Every limb in one semantic role shares the numeric label WaveLevel derives
// for this source instance. That CUSTOM0 value packs into G through the shared
// core, so source silhouettes and intended role creases use the same outline
// machinery as the world skin — the only differences are the faked depth and
// the standing floor.
//
// v_world stays a TRUE world value while the rasterized depth is faked —
// B-channel distance never lies.

#include "res://shaders/data_core.gdshaderinc"

// How strongly THIS source's standing image is felt right now: its own
// volume, dimmed by SOURCE_THROUGH per wall between the camera and its hub.
// The level computes it per source, per frame, on the CPU (WaveLevel's
// tick_sources) and pushes it here.
//
// It is an INSTANCE uniform, and both halves of that matter:
//   - per OBJECT rather than per fragment, so a source dims as a coherent
//     whole instead of splitting bright/dim along a wall's screen edge,
//     which is exactly how a per-fragment sight test would tear it;
//   - per INSTANCE rather than per material, because every source in the
//     world shares this one skin — a material uniform would make the quiet
//     fan and the loud radio brighten and dim as one object.
// (The reveal from waves in flight is still per-fragment wall-occluded in
// reveal_at, shared in the core.)
instance uniform float u_source_floor = 0.0;

void vertex() {
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_label = CUSTOM0.x;
}

void fragment() {
	// inside the acoustic-image band, ordered by TRUE distance: over every
	// world fragment, but never over a nearer source than itself
	DEPTH = source_depth(length(v_world - CAMERA_POSITION_WORLD));
	float reveal = max(reveal_at(v_world), u_source_floor);
	ALBEDO = pack_data(reveal, v_world, CAMERA_POSITION_WORLD);
}
