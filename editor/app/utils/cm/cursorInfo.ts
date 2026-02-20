import { type Extension } from "@codemirror/state";
import { EditorView, ViewPlugin, ViewUpdate } from "@codemirror/view";
import { reactive } from "vue";
import { findEventsAtPos, findEventsInRange } from "./events";

export type CursorInfo = {
    line: number;
    column: number;
    bar: number | null;
    tick: [number, number] | null;
    seconds: number | null;
};

export const cursorInfo = reactive<CursorInfo>({
    line: 1,
    column: 1,
    bar: null,
    tick: null,
    seconds: null,
});

function updateCursorInfo(view: EditorView) {
    const pos = view.state.selection.main.head;
    const line = view.state.doc.lineAt(pos);
    cursorInfo.line = line.number;
    cursorInfo.column = pos - line.from + 1;

    let events = findEventsAtPos(view.state, pos);
    if (events.length == 0) {
        let nextLine = view.state.doc.line(
            view.state.doc.lines > line.number ? line.number + 1 : line.number
        );
        let endOfNextLinePos = nextLine.to;
        events = findEventsInRange(view.state, pos + 1, endOfNextLinePos);
    }
    if (events.length > 0) {
        const event = events[0]!;
        cursorInfo.bar = event.start_bar + 1;
        cursorInfo.tick = event.start_tick;
        cursorInfo.seconds = event.start_sec;
    } else {
        cursorInfo.bar = null;
        cursorInfo.tick = null;
        cursorInfo.seconds = null;

    }
}

export function createCursorInfoPlugin(): Extension {
    return ViewPlugin.fromClass(class {
        constructor(view: EditorView) {
            updateCursorInfo(view);
        }

        update(update: ViewUpdate) {
            if (update.selectionSet || update.docChanged || update.viewportChanged) {
                updateCursorInfo(update.view);
            }
        }
    });
}
