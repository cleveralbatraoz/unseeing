// Material signatures — the shader side of the material concept.
// A material modulates the REVEAL with a minimal, monochrome, procedural
// pattern: not a texture map, just enough surface character (roughness,
// grain, weave) for an echo to tell rock from wood from cloth.
// Ids must match Materials.Kind in scripts/materials.gd.

const int MAT_ROCK = 0;
const int MAT_WOOD = 1;
const int MAT_WOOL = 2;
const int MAT_CLOTH = 3;
const int MAT_GLASS = 4;

float mat_hash(vec3 p) {
	p = fract(p * 0.3183099 + vec3(0.11, 0.23, 0.37));
	p *= 17.0;
	return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float mat_noise(vec3 p) {
	vec3 i = floor(p);
	vec3 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = mat_hash(i);
	float b = mat_hash(i + vec3(1, 0, 0));
	float c = mat_hash(i + vec3(0, 1, 0));
	float d = mat_hash(i + vec3(1, 1, 0));
	float e = mat_hash(i + vec3(0, 0, 1));
	float g = mat_hash(i + vec3(1, 0, 1));
	float h = mat_hash(i + vec3(0, 1, 1));
	float k = mat_hash(i + vec3(1, 1, 1));
	return mix(mix(mix(a, b, f.x), mix(c, d, f.x), f.y),
			mix(mix(e, g, f.x), mix(h, k, f.x), f.y), f.z);
}

// Returns a multiplier around 1.0 for the reveal. Surfaces have no fill —
// the signature shows as ROUGHNESS OF THE OUTLINE (edge brightness varies
// along the line), so contrast is tuned to read on a thin stroke.
float material_pattern(vec3 p, int kind) {
	if (kind == MAT_ROCK) {
		// coarse, harsh speckle: a rough surface answers unevenly
		float n = mat_noise(p * 7.0);
		return mix(0.45, 1.0, smoothstep(0.35, 0.75, n));
	}
	if (kind == MAT_WOOD) {
		// thin grain lines flowing with a slow wobble
		float grain = 0.5 + 0.5 * sin((p.x + p.y * 0.6) * 40.0 + mat_noise(p * 3.0) * 6.0);
		return mix(1.0, 0.45, smoothstep(0.78, 0.95, grain));
	}
	if (kind == MAT_WOOL) {
		// soft, blobby fuzz: no hard features at all
		float n = 0.5 * mat_noise(p * 9.0) + 0.5 * mat_noise(p * 21.0);
		return mix(0.68, 1.0, n);
	}
	if (kind == MAT_CLOTH) {
		// fine crosshatch weave
		float w = sin(p.x * 90.0 + p.y * 30.0) * sin(p.z * 90.0 + p.y * 90.0);
		return mix(0.72, 1.0, 0.5 + 0.5 * w);
	}
	// MAT_GLASS: smooth and clean, with a rare bright diagonal streak
	float streak = smoothstep(0.965, 1.0, 0.5 + 0.5 * sin((p.x + p.y + p.z) * 18.0));
	return mix(0.99, 1.25, streak);
}
