// Material signatures — the shader side of the material concept.
// The absolute minimum of cartoon shorthand: surfaces stay fully
// transparent, and a material is just a few TINY GLYPHS scattered
// sparsely across it, drawn in the same thin white stroke as the
// outlines (they are engraved into the distance channel, so the edge
// detector renders them itself):
//   rock  — little tildes  ~ ~ ~
//   wood  — short straight grain dashes  - - -
//   wool  — small arcs  ⌒ ⌒
//   cloth — tiny crosses  x x
//   glass — tiny slashes  / /
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
// the surface so glyphs lie flat on it.
vec2 mat_uv(vec3 p, vec3 n) {
	vec3 an = abs(n);
	if (an.y > 0.5) { return p.xz; }
	if (an.x > 0.5) { return p.zy; }
	return p.xy;
}

// One jittered glyph slot per grid cell, kept with probability `keep`.
// Returns 1 inside the glyph's unit box and writes local coords to l.
float mat_glyph_cell(vec2 uv, float cell, float size, float keep, out vec2 l) {
	vec2 c = floor(uv / cell);
	vec2 center = (c + 0.3 + 0.4 * mat_hash2(c)) * cell;
	l = (uv - center) / size;
	float inside = step(max(abs(l.x), abs(l.y)), 1.0);
	return inside * step(1.0 - keep, mat_hash(c + 3.3));
}

float mat_stroke(float d) {
	return smoothstep(0.30, 0.14, d);
}

// A glyph mask in [0,1] — sparse little marks, nothing else.
float material_pattern(vec3 p, vec3 n, int kind) {
	vec2 uv = mat_uv(p, n);
	vec2 l = vec2(0.0);
	if (kind == MAT_ROCK) {
		// ~   a loose little tilde
		float g = mat_glyph_cell(uv, 1.1, 0.16, 0.4, l);
		return g * mat_stroke(abs(l.y - 0.4 * sin(l.x * 3.14159)));
	}
	if (kind == MAT_WOOD) {
		// -   a short straight grain dash
		float g = mat_glyph_cell(uv, 0.55, 0.10, 0.4, l);
		return g * mat_stroke(abs(l.y)) * step(abs(l.x), 0.9);
	}
	if (kind == MAT_WOOL) {
		// ⌒   a small upward arc
		float g = mat_glyph_cell(uv, 0.8, 0.12, 0.4, l);
		return g * mat_stroke(abs(length(l) - 0.62)) * step(0.0, l.y);
	}
	if (kind == MAT_CLOTH) {
		// x   a tiny cross
		float g = mat_glyph_cell(uv, 0.7, 0.09, 0.4, l);
		float a = abs(l.y - l.x);
		float b = abs(l.y + l.x);
		return g * mat_stroke(min(a, b)) * step(max(abs(l.x), abs(l.y)), 0.75);
	}
	// MAT_GLASS: /   a tiny shine slash
	float g = mat_glyph_cell(uv, 0.9, 0.12, 0.35, l);
	return g * mat_stroke(abs(l.y - l.x * 0.9)) * step(abs(l.x), 0.8);
}
