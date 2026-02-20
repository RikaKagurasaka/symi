/**
 * The color of note is determined by its pitch ratio to the base frequency. 
 * x = log2(pitch_ratio) % 1
 * Then we map x to a color in oklab color space. Anchor points are:
 * 
 * x=0: black
 * x=log2(5/4)=0.321928: green
 * x=log2(3/2)=0.5849625: red
 * x=log2(7/4)=0.807355: blue
 * x=1: black
 * 
 * After interpolation, we mix the resulting color with white as 50/50 to get the final color.
 */

type Oklab = {
	l: number;
	a: number;
	b: number;
};

type Rgb = {
	r: number;
	g: number;
	b: number;
};

function clamp01(value: number): number {
	return Math.min(1, Math.max(0, value));
}

function frac01(value: number): number {
	const frac = value - Math.floor(value);
	return frac < 0 ? frac + 1 : frac;
}

function srgbToLinear(channel: number): number {
	const c = clamp01(channel);
	return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function linearToSrgb(channel: number): number {
	const c = clamp01(channel);
	return c <= 0.0031308 ? 12.92 * c : 1.055 * (c ** (1 / 2.4)) - 0.055;
}

function rgbToOklab(rgb: Rgb): Oklab {
	const lr = srgbToLinear(rgb.r);
	const lg = srgbToLinear(rgb.g);
	const lb = srgbToLinear(rgb.b);

	const l = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb;
	const m = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb;
	const s = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb;

	const lRoot = Math.cbrt(l);
	const mRoot = Math.cbrt(m);
	const sRoot = Math.cbrt(s);

	return {
		l: 0.2104542553 * lRoot + 0.793617785 * mRoot - 0.0040720468 * sRoot,
		a: 1.9779984951 * lRoot - 2.428592205 * mRoot + 0.4505937099 * sRoot,
		b: 0.0259040371 * lRoot + 0.7827717662 * mRoot - 0.808675766 * sRoot,
	};
}

function oklabToRgb(oklab: Oklab): Rgb {
	const lRoot = oklab.l + 0.3963377774 * oklab.a + 0.2158037573 * oklab.b;
	const mRoot = oklab.l - 0.1055613458 * oklab.a - 0.0638541728 * oklab.b;
	const sRoot = oklab.l - 0.0894841775 * oklab.a - 1.291485548 * oklab.b;

	const l = lRoot * lRoot * lRoot;
	const m = mRoot * mRoot * mRoot;
	const s = sRoot * sRoot * sRoot;

	const lr = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
	const lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
	const lb = -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s;

	return {
		r: linearToSrgb(lr),
		g: linearToSrgb(lg),
		b: linearToSrgb(lb),
	};
}

function lerp(a: number, b: number, t: number): number {
	return a + (b - a) * t;
}

function lerpOklab(from: Oklab, to: Oklab, t: number): Oklab {
	return {
		l: lerp(from.l, to.l, t),
		a: lerp(from.a, to.a, t),
		b: lerp(from.b, to.b, t),
	};
}

function toHex(rgb: Rgb): string {
	const r = Math.round(clamp01(rgb.r) * 255);
	const g = Math.round(clamp01(rgb.g) * 255);
	const b = Math.round(clamp01(rgb.b) * 255);

	return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

const BLACK = rgbToOklab({ r: 0, g: 0, b: 0 });
const BLUE = rgbToOklab({ r: 0, g: 0, b: 1 });
const RED = rgbToOklab({ r: 1, g: 0, b: 0 });
const GREEN = rgbToOklab({ r: 0, g: 1, b: 0 });

const LOG2_5_OVER_4 = Math.log2(5 / 4);
const LOG2_3_OVER_2 = Math.log2(3 / 2);
const LOG2_7_OVER_4 = Math.log2(7 / 4);

const ANCHORS: Array<{ x: number; color: Oklab }> = [
	{ x: 0, color: BLACK },
	{ x: LOG2_5_OVER_4, color: GREEN },
	{ x: LOG2_3_OVER_2, color: RED },
	{ x: LOG2_7_OVER_4, color: BLUE },
	{ x: 1, color: BLACK },
];

function colorAtUnitX(x: number): Oklab {
	const u = frac01(x);

	for (let i = 0; i < ANCHORS.length - 1; i += 1) {
		const left = ANCHORS[i]!;
		const right = ANCHORS[i + 1]!;
		if (u >= left.x && u <= right.x) {
			const span = right.x - left.x;
			const t = span > 0 ? (u - left.x) / span : 0;
			return lerpOklab(left.color, right.color, t);
		}
	}

	return BLACK;
}

export function getNoteColorFromPitchRatio(pitchRatio: number): string {
	if (!Number.isFinite(pitchRatio) || pitchRatio <= 0) {
		return "#808080";
	}

	const x = frac01(Math.log2(pitchRatio));
	const baseOklab = colorAtUnitX(x);
	const baseRgb = oklabToRgb(baseOklab);

	const mixedRgb: Rgb = {
		r: (baseRgb.r + 1) * 0.5,
		g: (baseRgb.g + 1) * 0.5,
		b: (baseRgb.b + 1) * 0.5,
	};

	return toHex(mixedRgb);
}

export function getNoteColorFromFreq(freq: number, baseFreq: number): string {
	if (!Number.isFinite(freq) || freq <= 0 || !Number.isFinite(baseFreq) || baseFreq <= 0) {
		return "#808080";
	}

	return getNoteColorFromPitchRatio(freq / baseFreq);
}

