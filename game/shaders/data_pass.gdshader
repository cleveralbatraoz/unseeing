shader_type spatial;
render_mode unshaded, cull_disabled, shadows_disabled, ambient_light_disabled, fog_disabled;

// DATA PASS — the WORLD skin of the data core: walls, floor/ceiling slabs,
// props, the cat, the hero's body. Real depth write + test (the world
// resolves its own hidden surfaces in hardware); everything else — the
// pulse loop, the object-id packing — lives in data_core.gdshaderinc,
// shared with the skins that ride the always-on-top depth hack.

#include "res://shaders/data_core.gdshaderinc"

void vertex() {
	v_world = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	v_nworld = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	float reveal = max(reveal_at(v_world), u_base);
	ALBEDO = pack_data(reveal, v_world, v_nworld, CAMERA_POSITION_WORLD);
}
