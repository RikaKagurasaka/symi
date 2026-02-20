import { type Extension } from "@codemirror/state";
import { EditorView, ViewPlugin, ViewUpdate } from "@codemirror/view";
import { invoke } from "@tauri-apps/api/core";
import { useDebounceFn } from "@vueuse/core";
import { createAnimatedCursor, createAnimatedCursorTheme } from "./animatedCursor";
import { activeNotesField, createActiveNoteTheme } from "./activeNote";
import { createDiagnosticsHoverTooltip, diagnosticsField, setDiagnosticsEffect } from "./diagnostics";
import { buildDecorations, decorationsField, setDecorationsEffect } from "./decorations";
import {
    createCtrlClickEventLogger,
    eventsField,
    setEventsEffect,
} from "./events";
import { createShiftSpacePlayHandler } from "./play";
import { createTokenTheme } from "./tokenTheme";
import type { Diagnostic, NoteEvent } from "./types";
import { createCursorInfoPlugin } from "./cursorInfo";

export const FILE_ID_TMP = "000";

type ViewPluginOptions = {
    getFileId?: () => string;
};

export const createViewPlugin = (options?: ViewPluginOptions): Extension => {
    const getFileId = options?.getFileId ?? (() => FILE_ID_TMP);
    const DOC_UPDATE_DEBOUNCE_MS = 120;

    const plugin = ViewPlugin.fromClass(class {
        #reqId = 0;
        #destroyed = false;
        #runUpdateDebounced: (view: EditorView) => void;

        constructor(view: EditorView) {
            this.#runUpdateDebounced = useDebounceFn((nextView: EditorView) => {
                void this.#runUpdate(nextView);
            }, DOC_UPDATE_DEBOUNCE_MS);
            void this.#runUpdate(view);
        }

        update(update: ViewUpdate) {
            if (!update.docChanged) return;
            this.#runUpdateDebounced(update.view);
        }

        destroy() {
            this.#destroyed = true;
        }

        async #runUpdate(view: EditorView) {
            if (this.#destroyed) return;
            const myReqId = ++this.#reqId;
            const source = view.state.doc.toString();
            const fileId = getFileId();

            try {
                await invoke("file_update", { fileId, source });
                const [tokens, diagnostics, events] = await Promise.all([
                    invoke("get_tokens", { fileId }) as Promise<[string, number, number][]>,
                    invoke("get_diagnostics", { fileId }) as Promise<Diagnostic[]>,
                    invoke("get_events", { fileId }) as Promise<NoteEvent[]>,
                ]);
                const decos = buildDecorations(tokens, diagnostics);
                if (myReqId !== this.#reqId) {
                    return;
                }

                view.dispatch({
                    effects: [
                        setDecorationsEffect.of(decos),
                        setDiagnosticsEffect.of(diagnostics),
                        setEventsEffect.of(events),
                    ],
                });

            } catch (error) {
                console.error("[viewPlugin] scheduleUpdate failed", {
                    myReqId,
                    fileId,
                    error,
                });
            }
        }
    });

    const tokenTheme = createTokenTheme();
    const activeNoteTheme = createActiveNoteTheme();
    const diagnosticsTooltip = createDiagnosticsHoverTooltip();
    const ctrlClickLogger = createCtrlClickEventLogger();
    const shiftSpacePlay = createShiftSpacePlayHandler();
    const animatedCursorTheme = createAnimatedCursorTheme();
    const animatedCursor = createAnimatedCursor();
    const cursorInfoPlugin = createCursorInfoPlugin();

    return [
        decorationsField,
        diagnosticsField,
        eventsField,
        activeNotesField,
        tokenTheme,
        activeNoteTheme,
        animatedCursorTheme,
        diagnosticsTooltip,
        shiftSpacePlay,
        ctrlClickLogger,
        animatedCursor,
        cursorInfoPlugin,
        plugin,
    ];
};