import { type Extension, type Range } from "@codemirror/state";
import { Decoration, EditorView } from "@codemirror/view";

/**
 * Token 上色主题（十六进制颜色码）
 *
 * Key 必须与后端返回的 token type 字符串一致。
 * 建议直接返回 Rust `SyntaxKind` 的变体名（例如 "Comma"、"PitchSpellOctave"）。
 */
export const TOKEN_STYLES = {
    // Trivia
    "Whitespace": "#00000000", // transparent
    "Newline": "#00000000", // transparent
    "Comment": "#64748B", // slate-500

    // Punctuation / operators
    "Comma": "#94A3B8", // slate-400
    "Colon": "#94A3B8",
    "Semicolon": "#94A3B8",
    "At": "#94A3B8",
    "Equals": "#94A3B8",

    // Paired / brackets
    "ParenthesisPair": "#A78BFA", // violet-400
    "LParen": "#A78BFA",
    "RParen": "#A78BFA",
    "LAngle": "#FBBF24", // amber-400
    "RAngle": "#FBBF24",

    // Identifiers
    "Identifier": "#E2E8F0", // slate-200

    // Pitch tokens
    "PitchSpellOctave": "#34D399", // emerald-400
    "PitchSpellSimple": "#34D399",
    "PitchFrequency": "#60A5FA", // blue-400
    "PitchRatio": "#22D3EE", // cyan-400
    "PitchEdo": "#2DD4BF", // teal-400
    "PitchCents": "#F472B6", // pink-400
    "PitchRest": "#94A3B8", // slate-400
    "PitchSustain": "#94A3B8", // slate-400

    // Durations / quantize
    "DurationCommas": "#cfbf96", // amber-400
    "DurationFraction": "#ffd876",
    "Quantize": "#FB7185", // rose-400
};

/**
 * 构建 token 高亮的 Decorations。
 */
export function buildTokenDecorations(tokens: [string, number, number][]): Range<Decoration>[] {
    const ranges: Range<Decoration>[] = [];

    for (const [type, from, to] of tokens) {
        const color = TOKEN_STYLES[type as keyof typeof TOKEN_STYLES];
        if (color) {
            // 用 mark decoration 给 [from, to) 这段文本添加 class，由 theme 统一上色。
            const decoration = Decoration.mark({
                class: `cm-symi-highlight-${type}`,
            }).range(from, to);
            ranges.push(decoration);
        }
    }

    return ranges;
}

export function createTokenTheme(): Extension {
    let styles: Record<string, any> = {};

    for (const [type, color] of Object.entries(TOKEN_STYLES)) {
        styles[`.cm-symi-highlight-${type}`] = { color };
    }

    styles = {
        ".cm-cursor": { boxShadow: "0 0 2px 2px lime", border: "none" },
        ".cm-gutters": { backgroundColor: "#1E293B", border: "none" }, // slate-800
        ".cm-line": { color: "#E2E8F0" }, // slate-200
        ...styles,
    };

    return EditorView.baseTheme(styles);
}
