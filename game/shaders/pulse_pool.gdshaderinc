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
// it, which is safe only because the room is 20 m across.
const float DIST_PACK_RANGE = 40.0;

uniform float u_time = 0.0;
uniform float u_flick = 1.0;
uniform int u_count = 0;
uniform vec3 u_ppos[MAXP];
uniform vec4 u_pdat[MAXP];
uniform vec4 u_pdir[MAXP];

// The sound source's room as an XZ rect (min_x, min_z, max_x, max_z), set
// by the composition root from level data. A source's waves REVEAL only
// geometry inside its own room — air can't strike what walls enclose —
// while its shells are still felt through the walls, muffled. Zero rect
// means no source, so nothing is clipped.
uniform vec4 u_hum_room = vec4(0.0);

// 1.0 where a pulse of this type may reveal geometry at world point p.
float hum_room_mask(float typ, vec3 p) {
	if (typ < 2.5 || u_hum_room.z <= u_hum_room.x) { return 1.0; }
	bool inside = p.x >= u_hum_room.x && p.x <= u_hum_room.z
			&& p.z >= u_hum_room.y && p.z <= u_hum_room.w;
	return inside ? 1.0 : 0.0;
}

float pulse_type(vec4 d) {
	return floor(d.w / 10.0);
}

float pulse_gain(vec4 d) {
	return mod(d.w, 10.0) / 9.0;
}

// Directed sound: only the cone the sound was aimed into carries information.
// dir_from_src must be the unit vector from the pulse origin toward the point.
float pulse_cone(vec4 cn, vec3 dir_from_src) {
	return smoothstep(cn.w - 0.15, cn.w + 0.05, dot(dir_from_src, cn.xyz));
}
