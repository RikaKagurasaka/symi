import { type Range, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, type DecorationSet } from "@codemirror/view";
import type { Diagnostic } from "./types";
import { buildDiagnosticDecorations } from "./diagnostics";
import { buildTokenDecorations } from "./tokenTheme";

export const setDecorationsEffect = StateEffect.define<DecorationSet>();

/**
 * 用 StateField 存储 DecorationSet，并把它“提供”为 EditorView.decorations。
 */
export const decorationsField = StateField.define<DecorationSet>({
    create() {
        return Decoration.set([], true);
    },
    update(value, tr) {
        for (const effect of tr.effects) {
            if (effect.is(setDecorationsEffect)) return effect.value;
        }
        return tr.docChanged ? value.map(tr.changes) : value;
    },
    provide: (field) => EditorView.decorations.from(field),
});

/**
 * 调用后端获取 tokens，并构建 DecorationSet。
 */
export function buildDecorations(tokens: [string, number, number][], diagnostics: Diagnostic[]): DecorationSet {
    const decorations: Range<Decoration>[] = [];
    const tokenDecorations = buildTokenDecorations(tokens);
    for (const deco of tokenDecorations) {
        decorations.push(deco);
    }
    const diagDecorations = buildDiagnosticDecorations(diagnostics);
    for (const deco of diagDecorations) {
        decorations.push(deco);
    }
    decorations.sort((a, b) => a.from - b.from || a.to - b.to);
    return Decoration.set(decorations, true);
}
