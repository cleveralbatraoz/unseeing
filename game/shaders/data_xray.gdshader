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

// THIS source's standing image, in its two independent halves: how loud its
// silhouette stands on its own, and what survives of it across the walls
// between it and the eye (SOURCE_THROUGH per crossing). The level computes
// both per source, per frame, on the CPU (WaveLevel's tick_sources).
//
// They are INSTANCE uniforms, and both halves of that matter:
//   - per OBJECT rather than per fragment, so a source dims as a coherent
//     whole instead of splitting bright/dim along a wall's screen edge,
//     which is exactly how a per-fragment sight test would tear it;
//   - per INSTANCE rather than per material, because every source in the
//     world shares this one skin — a material uniform would make the quiet
//     fan and the loud radio brighten and dim as one object.
// (The reveal from waves in flight is still per-fragment wall-occluded in
// reveal_at, shared in the core.)
instance uniform float u_source_volume = 0.0;
instance uniform float u_source_muffle = 1.0;

void vertex() {
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_label = CUSTOM0.x;
}

// rust/src/render/reveal.rs::source_image is the cargo-pinned reference.
//
// The ORDER is the law. The muffle multiplies the whole acoustic image —
// the standing silhouette AND any wave currently washing this body — so a
// wall can take something away. It used to arrive pre-multiplied into the
// volume and enter as a floor under reveal_at, and a floor always lost: a
// source's hub is unwalled from its own body by construction, so reveal_at
// reads near 1.0 there however many walls stand between that source and the
// player, and max() handed back the 1.0. The documented 0.30 / 0.09 / 0.027
// ladder held only while the source happened to be silent.
float source_image(float wave, float volume, float muffle) {
	return clamp(muffle * max(wave, volume), 0.0, 1.0);
}

void fragment() {
	// inside the acoustic-image band, ordered by TRUE distance: over every
	// world fragment, but never over a nearer source than itself
	DEPTH = source_depth(length(v_world - CAMERA_POSITION_WORLD));
	float reveal = source_image(reveal_at(v_world), u_source_volume, u_source_muffle);
	ALBEDO = pack_data(reveal, v_world, CAMERA_POSITION_WORLD);
}
