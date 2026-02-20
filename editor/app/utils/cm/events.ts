import { EditorState, type Extension, StateEffect, StateField } from "@codemirror/state";
import { EditorView, hoverTooltip } from "@codemirror/view";
import { invoke } from "@tauri-apps/api/core";
import { activateNoteHighlight } from "./activeNote";
import type { NoteEvent } from "./types";

export const setEventsEffect = StateEffect.define<NoteEvent[]>();

export const eventsField = StateField.define<NoteEvent[]>({
    create() {
        return [];
    },
    update(value, tr) {
        for (const effect of tr.effects) {
            if (effect.is(setEventsEffect)) return effect.value;
        }
        return value;
    },
});


export function getEvents(state: EditorState): NoteEvent[] {
    return state.field(eventsField);
}

export function findEventsAtPos(state: EditorState, pos: number): NoteEvent[] {
    const events = getEvents(state);
    return events.filter((e) => {
        const eventFrom = e.span_invoked_from ?? e.span_from;
        const eventTo = e.span_invoked_to ?? e.span_to;
        return pos >= eventFrom && pos <= eventTo;
    });
}

export function findEventsInRange(state: EditorState, from: number, to: number): NoteEvent[] {
    const events = getEvents(state);
    return events.filter((e) => {
        const eventFrom = e.span_invoked_from ?? e.span_from;
        const eventTo = e.span_invoked_to ?? e.span_to;
        return !(eventTo < from || eventFrom > to);
    });
}

export function playNotesAt(view: EditorView, pos: number) {
    const hits = findEventsAtPos(view.state, pos);
    for (const note of hits) {
        playNotes(view, note);
    }
}

export async function playNotes(view: EditorView, note: NoteEvent | NoteEvent[]) {
    // sort the events by start_sec
    const notes = Array.isArray(note) ? note : [note];
    notes.sort((a, b) => a.start_sec - b.start_sec);
    if (notes.length === 0) return;
    const first_start_delay = notes[0]!.start_sec * 1000;
    const startTime = performance.now();
    for (const n of notes) {
        const delay = (n.start_sec * 1000 - first_start_delay) - (performance.now() - startTime);
        if (delay > 0) {
            await new Promise((resolve) => setTimeout(resolve, delay));
        }
        playNote(view, n);
    }
}

export function playNote(view: EditorView, note: NoteEvent) {
    activateNoteHighlight(view, note);
    invoke("play_note", { frequency: note.freq, durationSec: note.duration_sec });
}

/**
 * Ctrl + 点击事件：播放该位置的音符事件。
 */
export function createCtrlClickEventLogger(): Extension {
    return EditorView.domEventHandlers({
        mousedown(event, view) {
            if (!(event as MouseEvent).ctrlKey) return false;
            const pos = view.posAtCoords({
                x: (event as MouseEvent).clientX,
                y: (event as MouseEvent).clientY,
            });
            if (pos == null) return false;
            playNotesAt(view, pos);
            return true;
        },
    });
}

