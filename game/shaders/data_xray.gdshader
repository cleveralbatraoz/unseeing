shader_type spatial;
render_mode unshaded, cull_back, shadows_disabled, ambient_light_disabled, fog_disabled;

// DATA X-RAY — the ACOUSTIC IMAGE skin: the sound sources (the fan). Its
// fragments write the ALWAYS_ON_TOP depth, so the hardware depth test can
// never occlude it — the acoustic image passes over the world, felt
// through walls. A wall does not hide the source, only DIMS it: the
// composition root muffles its standing floor per OBJECT (u_source_muffle,
// SOURCE_THROUGH per wall between the camera and the source), so the whole
// fan dims together. cull_back is mandatory: under an always-pass depth
// test a mesh's own back faces would stomp its front faces in draw order.
//
// The object id still packs into G through the shared core (u_oid), so a
// source reads as one clean silhouette exactly like the world skin — the
// only difference is the faked depth and the muffled floor.
//
// v_world/v_nworld stay TRUE world values while the rasterized depth is
// faked — B-channel distance and G normals never lie.

#include "res://shaders/data_core.gdshaderinc"

// The source's standing floor, muffled through walls as ONE object: the
// composition root counts the walls between the camera and the source and
// hands SOURCE_THROUGH^that here, per frame. So the fan dims as a coherent
// whole instead of splitting bright/dim along a wall's screen edge — a
// per-object muffle where a per-fragment sight test would tear. (The
// reveal from waves in flight is still per-fragment wall-occluded in
// reveal_at, shared in the core.)
uniform float u_source_muffle = 1.0;

void vertex() {
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_nworld = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	DEPTH = ALWAYS_ON_TOP;
	float reveal = max(reveal_at(v_world), u_base * u_source_muffle);
	ALBEDO = pack_data(reveal, v_world, v_nworld, CAMERA_POSITION_WORLD);
}
