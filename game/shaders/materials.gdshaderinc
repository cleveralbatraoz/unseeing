// Material signatures — the shader side of the material concept.
// A material is the CONSTRUCTION of the object: the pieces it is made of,
// each piece outlined — nothing else. No textures, no fills, no marks;
// just the boundary lines between pieces, drawn in the same thin white
// stroke as every other outline (they are engraved into the distance
// channel, so the edge detector renders them itself):
//   rock  — each stone of the wall outlined (irregular polygonal cells)
//   wood  — each plank outlined (staggered boards with butt joints)
//   wool  — soft rounded tufts
//   cloth — quilted diamond panels
//   glass — rectangular panes
// Ids must match Materials.Kind in scripts/materials.gd.

const int MAT_ROCK = 0;
const int MAT_WOOD = 1;
const int MAT_WOOL = 2;
const int MAT_CLOTH = 3;
const int MAT_GLASS = 4;

float mat_hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

vec2 mat_hash2(vec2 p) {
	return vec2(mat_hash(p), mat_hash(p + vec2(17.13, 3.7)));
}

// Surface-planar coordinates: project the world position onto the plane of
// the surface so piece boundaries lie flat on it.
vec2 mat_uv(vec3 p, vec3 n) {
	vec3 an = abs(n);
	if (an.y > 0.5) { return p.xz; }
	if (an.x > 0.5) { return p.zy; }
	return p.xy;
}

// Distance to the nearest voronoi cell border — the joints between stones.
float mat_cell_border(vec2 uv) {
	vec2 i = floor(uv);
	vec2 f = fract(uv);
	float f1 = 8.0;
	float f2 = 8.0;
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 cell = vec2(float(x), float(y));
			vec2 pt = cell + mat_hash2(i + cell) - f;
			float d = dot(pt, pt);
			if (d < f1) { f2 = f1; f1 = d; }
			else if (d < f2) { f2 = d; }
		}
	}
	return sqrt(f2) - sqrt(f1);
}

// Thin line helper: 1 on the boundary, 0 elsewhere.
float mat_line(float d, float w) {
	return smoothstep(w, w * 0.45, d);
}

// Boundary mask in [0,1]: the outlines of the pieces the surface is made of.
float material_pattern(vec3 p, vec3 n, int kind) {
	vec2 uv = mat_uv(p, n);
	if (kind == MAT_ROCK) {
		// each stone outlined: irregular polygonal cells ~1.2 m across
		return mat_line(mat_cell_border(uv * 0.85), 0.045);
	}
	if (kind == MAT_WOOD) {
		// each plank outlined: boards ~0.26 m wide, ~1.7 m long, staggered
		float row = floor(uv.y / 0.26);
		float seam = mat_line(abs(fract(uv.y / 0.26) - 0.5), 0.06);
		float bu = uv.x / 1.7 + mat_hash(vec2(row, 5.0));
		float butt = mat_line(abs(fract(bu) - 0.5), 0.02);
		return max(seam, butt);
	}
	if (kind == MAT_WOOL) {
		// soft rounded tufts: chunky cells with a gentle, wider stroke
		return mat_line(mat_cell_border(uv * 1.9), 0.11);
	}
	if (kind == MAT_CLOTH) {
		// quilted diamonds: seams on the 45-degree grid
		vec2 q = vec2(uv.x + uv.y, uv.x - uv.y) * (0.5 / 0.32);
		float du = mat_line(abs(fract(q.x) - 0.5), 0.05);
		float dv = mat_line(abs(fract(q.y) - 0.5), 0.05);
		return max(du, dv);
	}
	// MAT_GLASS: rectangular panes ~0.55 m
	float gu = mat_line(abs(fract(uv.x / 0.55) - 0.5), 0.035);
	float gv = mat_line(abs(fract(uv.y / 0.55) - 0.5), 0.035);
	return max(gu, gv);
}
