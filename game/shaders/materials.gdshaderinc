// Material signatures — the shader side of the material concept.
// CARTOON EMULATION, not texture: each material is drawn the way a comic
// artist would shorthand it — sparse iconic marks on a near-black fill:
//   rock  — a crack network between stones (voronoi cell borders)
//   wood  — a few long, interrupted grain strokes
//   wool  — scattered soft round specks
//   cloth — sparse crosshatch weave lines
//   glass — the classic diagonal shine streaks
// The pattern multiplies the reveal: a dim base keeps the fill almost void,
// marks rise above it and pop when a wave sweeps the surface.
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

float mat_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = mat_hash(i);
	float b = mat_hash(i + vec2(1, 0));
	float c = mat_hash(i + vec2(0, 1));
	float d = mat_hash(i + vec2(1, 1));
	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// Surface-planar coordinates: project the world position onto the plane of
// the surface so marks lie flat on it (no streaking across faces).
vec2 mat_uv(vec3 p, vec3 n) {
	vec3 an = abs(n);
	if (an.y > 0.5) { return p.xz; }
	if (an.x > 0.5) { return p.zy; }
	return p.xy;
}

// Distance to the nearest voronoi cell border — the crack network.
float mat_cracks(vec2 uv) {
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

// Multiplier for the reveal: DIM_BASE keeps fills near-void; marks rise
// above 1.0 so they pop inside the swept area.
float material_pattern(vec3 p, vec3 n, int kind) {
	vec2 uv = mat_uv(p, n);
	const float DIM_BASE = 0.72;
	if (kind == MAT_ROCK) {
		// stones: thin bright cracks where voronoi cells meet
		float crack = smoothstep(0.10, 0.03, mat_cracks(uv * 1.4));
		return DIM_BASE + 1.1 * crack;
	}
	if (kind == MAT_WOOD) {
		// a few long grain strokes, wobbling, randomly interrupted
		float t = uv.y * 2.4 + 0.4 * mat_noise(uv * vec2(1.1, 0.3));
		float band = abs(fract(t) - 0.5);
		float stroke = smoothstep(0.10, 0.03, band)
				* step(0.45, mat_hash(vec2(floor(t), floor(uv.x * 0.8))));
		return DIM_BASE + 1.0 * stroke;
	}
	if (kind == MAT_WOOL) {
		// scattered soft specks
		vec2 cell = floor(uv * 6.0);
		vec2 pt = mat_hash2(cell);
		float d = length(fract(uv * 6.0) - pt);
		float speck = smoothstep(0.22, 0.05, d) * step(0.55, mat_hash(cell + 7.0));
		return DIM_BASE + 0.7 * speck;
	}
	if (kind == MAT_CLOTH) {
		// sparse crosshatch: thin weave lines in both directions
		float lu = smoothstep(0.08, 0.02, abs(fract(uv.x * 2.8) - 0.5))
				* step(0.5, mat_hash(vec2(floor(uv.x * 2.8), 3.0)));
		float lv = smoothstep(0.08, 0.02, abs(fract(uv.y * 2.8) - 0.5))
				* step(0.5, mat_hash(vec2(floor(uv.y * 2.8), 11.0)));
		return DIM_BASE + 0.8 * max(lu, lv);
	}
	// MAT_GLASS: two parallel diagonal shine streaks, cartoon-style
	float s = fract((uv.x + uv.y) * 0.55);
	float streak = smoothstep(0.05, 0.015, abs(s - 0.45))
			+ 0.7 * smoothstep(0.03, 0.008, abs(s - 0.58));
	return 0.9 + 1.2 * streak;
}
