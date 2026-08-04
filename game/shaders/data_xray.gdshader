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
// The object id still packs into G through the shared core (u_oid), so a
// source reads as one clean silhouette exactly like the world skin — the
// only differences are the faked depth and the standing floor.
//
// v_world/v_nworld stay TRUE world values while the rasterized depth is
// faked — B-channel distance and G normals never lie.

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
	v_nworld = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	DEPTH = ALWAYS_ON_TOP;
	float reveal = max(reveal_at(v_world), u_source_floor);
	ALBEDO = pack_data(reveal, v_world, v_nworld, CAMERA_POSITION_WORLD);
}
