export const symiLang = {
	"$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
	"name": "symi",
	"scopeName": "source.symi",
	"fileTypes": ["symi"],
	"patterns": [
		{ "include": "#comments" },
		{ "include": "#macro-definition" },
		{ "include": "#ghost-line" },
		{ "include": "#base-pitch" },
		{ "include": "#time-signature" },
		{ "include": "#bpm" },
		{ "include": "#duration" },
		{ "include": "#quantize" },
		{ "include": "#macro-invoke" },
		{ "include": "#pitch" },
		{ "include": "#punctuation" },
		{ "include": "#identifier" }
	],
	"repository": {
		"comments": {
			"patterns": [
				{
					"name": "comment.line.double-slash.symi",
					"match": "//[^\\r\\n]*"
				}
			]
		},
		"macro-definition": {
			"name": "meta.definition.macro.symi",
			"begin": "^(\\s*)([A-Za-z_][A-Za-z0-9_]*)(\\s*)(\\(\\))?(\\s*)(=)",
			"beginCaptures": {
				"2": { "name": "entity.name.function.macro.symi" },
				"4": { "name": "storage.modifier.relative.symi" },
				"6": { "name": "keyword.operator.assignment.symi" }
			},
			"end": "$",
			"patterns": [
				{ "include": "#comments" },
				{ "include": "#duration" },
				{ "include": "#quantize" },
				{ "include": "#macro-invoke" },
				{ "include": "#pitch" },
				{ "include": "#punctuation" },
				{ "include": "#identifier" }
			]
		},
		"ghost-line": {
			"patterns": [
				{
					"name": "meta.line.ghost.symi",
					"match": "^(\\s*)(=)",
					"captures": {
						"2": { "name": "keyword.operator.assignment.symi" }
					}
				}
			]
		},
		"base-pitch": {
			"name": "meta.definition.base-pitch.symi",
			"begin": "<",
			"beginCaptures": {
				"0": { "name": "punctuation.section.angle.begin.symi" }
			},
			"end": ">",
			"endCaptures": {
				"0": { "name": "punctuation.section.angle.end.symi" }
			},
			"patterns": [
				{
					"name": "keyword.operator.assignment.symi",
					"match": "="
				},
				{ "include": "#pitch" }
			]
		},
		"time-signature": {
			"name": "meta.definition.time-signature.symi",
			"match": "\\(\\s*\\d+\\/\\d+\\s*\\)",
			"captures": {
				"0": { "name": "constant.numeric.time-signature.symi" }
			}
		},
		"bpm": {
			"name": "meta.definition.bpm.symi",
			"match": "\\(\\s*(?:\\[-?\\d+(?::\\d+)?\\]\\s*=\\s*)?\\d+(?:\\.\\d+)?\\s*\\)",
			"captures": {
				"0": { "name": "constant.numeric.bpm.symi" }
			}
		},
		"duration": {
			"patterns": [
				{
					"name": "constant.numeric.duration.commas.symi",
					"match": "\\[,+\\]"
				},
				{
					"name": "constant.numeric.duration.fraction.symi",
					"match": "\\[-?\\d+(?::\\d+)?\\]"
				}
			]
		},
		"quantize": {
			"patterns": [
				{
					"name": "constant.numeric.quantize.symi",
					"match": "\\{\\d+(?::\\d+)?\\}"
				}
			]
		},
		"macro-invoke": {
			"patterns": [
				{
					"name": "meta.invocation.macro.symi",
					"match": "\\b([A-Za-z_][A-Za-z0-9_]*)(\\s*)(\\()",
					"captures": {
						"1": { "name": "support.function.macro.symi" },
						"3": { "name": "punctuation.section.parens.begin.symi" }
					}
				}
			]
		},
		"pitch": {
			"patterns": [
				{
					"name": "constant.language.rest.symi",
					"match": "\\.+"
				},
				{
					"name": "constant.numeric.pitch.edo.symi",
					"match": "-?\\d+\\\\\\d+"
				},
				{
					"name": "constant.numeric.pitch.cents.symi",
					"match": "-?\\d+c\\b"
				},
				{
					"name": "constant.numeric.pitch.ratio.symi",
					"match": "\\b\\d+\\/\\d+\\b"
				},
				{
					"name": "constant.numeric.pitch.frequency.symi",
					"match": "\\b\\d+(?:\\.\\d+)?\\b"
				},
				{
					"name": "constant.language.pitch.spell-octave.symi",
					"match": "\\b[A-G](?:#|b)*(-[1-9]|1?[0-9])\\b"
				},
				{
					"name": "constant.language.pitch.spell.symi",
					"match": "\\b[A-G](?:#|b)*[+-]*\\b"
				},
				{
					"name": "keyword.operator.sustain.symi",
					"match": "(?<!\\S)-(?!\\d)"
				}
			]
		},
		"punctuation": {
			"patterns": [
				{
					"name": "keyword.operator.chain.symi",
					"match": "@"
				},
				{
					"name": "punctuation.separator.sequence.symi",
					"match": "[:;]"
				},
				{
					"name": "punctuation.separator.delimiter.symi",
					"match": ","
				},
				{
					"name": "keyword.operator.assignment.symi",
					"match": "="
				},
				{
					"name": "punctuation.section.parens.symi",
					"match": "[()]"
				}
			]
		},
		"identifier": {
			"patterns": [
				{
					"name": "variable.other.symi",
					"match": "\\b[A-Za-z_][A-Za-z0-9_]*\\b"
				}
			]
		}
	}
}

const symiTokenColorsDark = [
	{ scope: ["comment.line.double-slash.symi"], settings: { foreground: "#64748B" } },
	{ scope: ["punctuation.separator.delimiter.symi"], settings: { foreground: "#94A3B8" } },
	{ scope: ["punctuation.separator.sequence.symi"], settings: { foreground: "#94A3B8" } },
	{ scope: ["keyword.operator.chain.symi"], settings: { foreground: "#94A3B8" } },
	{ scope: ["keyword.operator.assignment.symi"], settings: { foreground: "#94A3B8" } },
	{ scope: ["punctuation.section.parens.symi", "punctuation.section.parens.begin.symi"], settings: { foreground: "#A78BFA" } },
	{ scope: ["punctuation.section.angle.begin.symi", "punctuation.section.angle.end.symi"], settings: { foreground: "#FBBF24" } },
	{ scope: ["variable.other.symi", "support.function.macro.symi", "entity.name.function.macro.symi"], settings: { foreground: "#E2E8F0" } },
	{ scope: ["constant.language.pitch.spell-octave.symi", "constant.language.pitch.spell.symi"], settings: { foreground: "#34D399" } },
	{ scope: ["constant.numeric.pitch.frequency.symi"], settings: { foreground: "#60A5FA" } },
	{ scope: ["constant.numeric.pitch.ratio.symi"], settings: { foreground: "#22D3EE" } },
	{ scope: ["constant.numeric.pitch.edo.symi"], settings: { foreground: "#2DD4BF" } },
	{ scope: ["constant.numeric.pitch.cents.symi"], settings: { foreground: "#F472B6" } },
	{ scope: ["constant.language.rest.symi", "keyword.operator.sustain.symi"], settings: { foreground: "#94A3B8" } },
	{ scope: ["constant.numeric.duration.commas.symi"], settings: { foreground: "#CFBF96" } },
	{ scope: ["constant.numeric.duration.fraction.symi"], settings: { foreground: "#FFD876" } },
	{ scope: ["constant.numeric.quantize.symi"], settings: { foreground: "#FB7185" } },
	{ scope: ["constant.numeric.bpm.symi", "constant.numeric.time-signature.symi"], settings: { foreground: "#60A5FA" } },
	{ scope: ["storage.modifier.relative.symi"], settings: { foreground: "#A78BFA" } }
];

const symiTokenColorsLight = [
	{ scope: ["comment.line.double-slash.symi"], settings: { foreground: "#64748B" } },
	{ scope: ["punctuation.separator.delimiter.symi"], settings: { foreground: "#475569" } },
	{ scope: ["punctuation.separator.sequence.symi"], settings: { foreground: "#475569" } },
	{ scope: ["keyword.operator.chain.symi"], settings: { foreground: "#475569" } },
	{ scope: ["keyword.operator.assignment.symi"], settings: { foreground: "#475569" } },
	{ scope: ["punctuation.section.parens.symi", "punctuation.section.parens.begin.symi"], settings: { foreground: "#7C3AED" } },
	{ scope: ["punctuation.section.angle.begin.symi", "punctuation.section.angle.end.symi"], settings: { foreground: "#B45309" } },
	{ scope: ["variable.other.symi", "support.function.macro.symi", "entity.name.function.macro.symi"], settings: { foreground: "#334155" } },
	{ scope: ["constant.language.pitch.spell-octave.symi", "constant.language.pitch.spell.symi"], settings: { foreground: "#059669" } },
	{ scope: ["constant.numeric.pitch.frequency.symi"], settings: { foreground: "#2563EB" } },
	{ scope: ["constant.numeric.pitch.ratio.symi"], settings: { foreground: "#0E7490" } },
	{ scope: ["constant.numeric.pitch.edo.symi"], settings: { foreground: "#0F766E" } },
	{ scope: ["constant.numeric.pitch.cents.symi"], settings: { foreground: "#BE185D" } },
	{ scope: ["constant.language.rest.symi", "keyword.operator.sustain.symi"], settings: { foreground: "#64748B" } },
	{ scope: ["constant.numeric.duration.commas.symi"], settings: { foreground: "#A16207" } },
	{ scope: ["constant.numeric.duration.fraction.symi"], settings: { foreground: "#B45309" } },
	{ scope: ["constant.numeric.quantize.symi"], settings: { foreground: "#E11D48" } },
	{ scope: ["constant.numeric.bpm.symi", "constant.numeric.time-signature.symi"], settings: { foreground: "#2563EB" } },
	{ scope: ["storage.modifier.relative.symi"], settings: { foreground: "#7C3AED" } }
];

export const symiThemeDark = {
	name: "symi-dark",
	type: "dark",
	colors: {
		"editor.background": "#0F172A",
		"editor.foreground": "#E2E8F0"
	},
	tokenColors: symiTokenColorsDark
};

export const symiThemeLight = {
	name: "symi-light",
	type: "light",
	colors: {
		"editor.background": "#F8FAFC",
		"editor.foreground": "#334155"
	},
	tokenColors: symiTokenColorsLight
};
