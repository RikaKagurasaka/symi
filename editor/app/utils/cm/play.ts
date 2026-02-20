import { type Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import type { NoteEvent } from "./types";
import { getEvents, playNotes } from "./events";

let isPlaying = false;
let currentSessionId = 0;
let scheduledTimers: number[] = [];
let endTimer: number | null = null;

function clearScheduled() {
	for (const id of scheduledTimers) {
		clearTimeout(id);
	}
	scheduledTimers = [];
	if (endTimer != null) {
		clearTimeout(endTimer);
		endTimer = null;
	}
}

export function stopPlayback() {
	currentSessionId += 1;
	isPlaying = false;
	clearScheduled();
}

function getEventSpan(note: NoteEvent): [number, number] {
	const from = note.span_invoked_from ?? note.span_from;
	const to = note.span_invoked_to ?? note.span_to;
	return from <= to ? [from, to] : [to, from];
}

function getEventsInRange(view: EditorView, from: number, to: number): NoteEvent[] {
	const [a, b] = from <= to ? [from, to] : [to, from];
	const events = getEvents(view.state)
	return events.filter((e) => {
		if (e.type !== "Note") return false;
		const [ef, et] = getEventSpan(e);
		return !(et < a || ef > b);
	});
}


export function playNotesInSelection(view: EditorView) {
	if (isPlaying) {
		stopPlayback();
		return;
	}
	const sel = view.state.selection.main;
	const hasSelection = !sel.empty;
	const from = hasSelection ? Math.min(sel.anchor, sel.head) : sel.head;
	const to = hasSelection ? Math.max(sel.anchor, sel.head) : view.state.doc.length;
	const notes = getEventsInRange(view, from, to);
	if (notes.length === 0) return;
	void playNotesScheduled(view, notes);
}

async function playNotesScheduled(view: EditorView, notes: NoteEvent[]) {
	clearScheduled();
	currentSessionId += 1;
	const sessionId = currentSessionId;
	isPlaying = true;

	const sorted = [...notes].sort((a, b) => a.start_sec - b.start_sec);
	const first_start_delay = sorted[0]!.start_sec * 1000;
	const startTime = performance.now() - first_start_delay;

	let maxEndMs = 0;
	for (const note of sorted) {
		const startMs = note.start_sec * 1000;
		const endMs = startMs + note.duration_sec * 1000 - first_start_delay;
		if (endMs > maxEndMs) maxEndMs = endMs;
		const delay = Math.max(0, startMs - (performance.now() - startTime));
		const id = window.setTimeout(() => {
			if (!isPlaying || sessionId !== currentSessionId) return;
			if (!note.span_invoked_to) {
				const safePos = Math.min(note.span_to, view.state.doc.length);
				view.dispatch({
					selection: { anchor: safePos },
				});
			}
			playNotes(view, note);
		}, delay);
		scheduledTimers.push(id);
	}

	endTimer = window.setTimeout(() => {
		if (sessionId !== currentSessionId) return;
		isPlaying = false;
		clearScheduled();
	}, maxEndMs + 50);
}

/**
 * Shift + Space：播放选中区域内所有 events；无选区则从光标到文档尾。
 */
export function createShiftSpacePlayHandler(): Extension {
	return EditorView.domEventHandlers({
		keydown(event, view) {
			const isShiftSpace = (event as KeyboardEvent).shiftKey
				&& ((event as KeyboardEvent).code === "Space" || (event as KeyboardEvent).key === " ");
			if (!isShiftSpace) return false;
			event.preventDefault();
			playNotesInSelection(view);
			return true;
		},
	});
}
