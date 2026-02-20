import { StateEffect, StateField, type Extension } from "@codemirror/state";
import { Decoration, EditorView } from "@codemirror/view";
import type { ActiveNoteHighlight, NoteEvent } from "./types";

export const addActiveNoteEffect = StateEffect.define<ActiveNoteHighlight>();
export const removeActiveNoteEffect = StateEffect.define<string>();

export const activeNotesField = StateField.define<ActiveNoteHighlight[]>({
    create() {
        return [];
    },
    update(value, tr) {
        let next = value;
        for (const effect of tr.effects) {
            if (effect.is(addActiveNoteEffect)) {
                next = [...next, effect.value];
            } else if (effect.is(removeActiveNoteEffect)) {
                next = next.filter((n) => n.id !== effect.value);
            }
        }

        if (tr.docChanged) {
            next = next
                .map((n) => ({
                    ...n,
                    from: tr.changes.mapPos(n.from, 1),
                    to: tr.changes.mapPos(n.to, -1),
                }))
                .filter((n) => n.from < n.to);
        }

        return next;
    },
    provide: (field) => EditorView.decorations.from(field, (value) => {
        const ranges = value.map((n) =>
            Decoration.mark({
                class: "cm-symi-active-note",
                attributes: { style: `--symi-hold: ${n.holdMs}ms;` },
            }).range(n.from, n.to),
        );
        return Decoration.set(ranges, true);
    }),
});

const FADE_OUT_MS = 300;

export function createActiveNoteTheme(): Extension {
    return EditorView.baseTheme({
        "@keyframes symi-note-hold": {
            from: { backgroundColor: "rgba(81, 130, 246, 0.8)" },
            to: { backgroundColor: "rgba(81, 132, 213, 0.8)" },
        },
        "@keyframes symi-note-fade": {
            from: { backgroundColor: "rgba(81, 130, 246, 0.8)" },
            to: { backgroundColor: "rgba(81, 130, 246, 0)" },
        },
        ".cm-symi-active-note": {
            borderRadius: "4px",
            animation: `symi-note-hold var(--symi-hold) linear 0ms 1 forwards, symi-note-fade ${FADE_OUT_MS}ms ease var(--symi-hold) 1 forwards`,
        },
    });
}

export function activateNoteHighlight(view: EditorView, note: NoteEvent) {
    const holdMs = Math.max(0, Math.round(note.duration_sec * 1000));
    const id = `${Date.now()}-${Math.random().toString(36).slice(2)}`;

    view.dispatch({
        effects: addActiveNoteEffect.of({
            id,
            from: note.span_from,
            to: note.span_to,
            holdMs,
        }),
    });

    window.setTimeout(() => {
        view.dispatch({ effects: removeActiveNoteEffect.of(id) });
    }, holdMs + FADE_OUT_MS);

    if (note.span_invoked_from && note.span_invoked_to) {
        const invokedId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
        view.dispatch({
            effects: addActiveNoteEffect.of({
                id: invokedId,
                from: note.span_invoked_from,
                to: note.span_invoked_to,
                holdMs,
            }),
        });
        window.setTimeout(() => {
            view.dispatch({ effects: removeActiveNoteEffect.of(invokedId) });
        }, holdMs + FADE_OUT_MS);

    }

}
