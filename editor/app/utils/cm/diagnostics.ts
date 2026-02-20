import { EditorState, type Extension, type Range, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, hoverTooltip } from "@codemirror/view";
import type { Diagnostic } from "./types";

export const setDiagnosticsEffect = StateEffect.define<Diagnostic[]>();

export const diagnosticsField = StateField.define<Diagnostic[]>({
    create() {
        return [];
    },
    update(value, tr) {
        for (const effect of tr.effects) {
            if (effect.is(setDiagnosticsEffect)) return effect.value;
        }
        return value;
    },
});

export function getDiagnostics(state: EditorState): Diagnostic[] {
    return state.field(diagnosticsField);
}

/**
 * 构建诊断信息的 Decorations：使用橙色/红色波浪下划线。
 */
export function buildDiagnosticDecorations(diagnostics: Diagnostic[]): Range<Decoration>[] {
    const ranges: Range<Decoration>[] = [];

    for (const diag of diagnostics) {
        const color = diag.severity === "Warning" ? "#F59E0B" : "#EF4444"; // amber-500 / red-500
        const decoration = Decoration.mark({
            attributes: {
                style: `text-decoration: underline wavy ${color}; text-decoration-thickness: 1px;`,
            },
        }).range(diag.from, diag.to);
        ranges.push(decoration);
    }

    return ranges;
}

/**
 * 鼠标悬浮在诊断区间上时显示提示框。
 */
export function createDiagnosticsHoverTooltip(): Extension {
    return hoverTooltip((view, pos) => {
        const diagnostics = getDiagnostics(view.state);
        const hits = diagnostics.filter((d) => pos >= d.from && pos <= d.to);
        if (hits.length === 0) return null;

        const from = Math.min(...hits.map((d) => d.from));
        const to = Math.max(...hits.map((d) => d.to));

        return {
            pos: from,
            end: to,
            above: true,
            create() {
                const dom = document.createElement("div");
                dom.style.maxWidth = "360px";
                dom.style.padding = "6px 8px";
                dom.style.fontSize = "12px";
                dom.style.lineHeight = "1.4";

                for (const diag of hits) {
                    const row = document.createElement("div");
                    row.style.display = "flex";
                    row.style.gap = "6px";
                    row.style.alignItems = "baseline";

                    const badge = document.createElement("span");
                    badge.textContent = diag.severity;
                    badge.style.fontWeight = "600";
                    badge.style.color = diag.severity === "Warning" ? "#F59E0B" : "#EF4444";

                    const msg = document.createElement("span");
                    msg.textContent = diag.message;

                    row.appendChild(badge);
                    row.appendChild(msg);
                    dom.appendChild(row);
                }

                return { dom };
            },
        };
    });
}
